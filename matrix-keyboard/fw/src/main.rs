#![no_std]
#![no_main]

use core::sync::atomic::{AtomicBool, Ordering};
use defmt::*;

use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_time::{Delay, Duration, Timer};
use embassy_usb::class::hid::{
    HidReader, HidReaderWriter, HidWriter, ReportId, RequestHandler, State,
};
use embassy_usb::control::OutResponse;
use embassy_usb::{Config as UsbConfig, Handler, UsbDevice};

use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::i2c::{self, Config, InterruptHandler};
use embassy_rp::peripherals::{I2C0, PIN_16, PIN_17, USB};
use embassy_rp::usb;
use embassy_rp::usb::{Driver as UsbDriver, InterruptHandler as UsbInterruptHandler};

use usbd_hid::descriptor::{KeyboardReport, SerializedDescriptor};

use lcd_lcm1602_i2c::sync_lcd::Lcd;
use static_cell::{ConstStaticCell, StaticCell};

use embassy_sync::signal::Signal;
//NB if we use second core, this mutex is not suitable
static LINE_1_TEXT: Signal<ThreadModeRawMutex, [u8; 32]> = Signal::new();
static LINE_2_TEXT: Signal<ThreadModeRawMutex, [u8; 32]> = Signal::new();
static BACKLIGHT_SETTING: Signal<ThreadModeRawMutex, bool> = Signal::new();

use postcard_rpc::{
    define_dispatch,
    header::VarHeader,
    server::{
        impls::embassy_usb_v0_3::{
            dispatch_impl::{
                WireRxBuf, WireRxImpl, WireSpawnImpl, WireStorage, WireTxImpl,
            },
            PacketBuffers,
        },
        Dispatch, Server,
    },
};

use keyboard_icd::{
    SetBacklight, SetLine1Text, SetLine2Text, ENDPOINT_LIST, TOPICS_IN_LIST, TOPICS_OUT_LIST,
};

use {defmt_rtt as _, panic_probe as _};

type AppDriver = usb::Driver<'static, USB>;
type BufStorage = PacketBuffers<1024, 1024>;
static PBUFS: ConstStaticCell<BufStorage> = ConstStaticCell::new(BufStorage::new());
static STORAGE: AppStorage = AppStorage::new();
type AppStorage = WireStorage<ThreadModeRawMutex, AppDriver, 256, 256, 64, 256>;
type AppTx = WireTxImpl<ThreadModeRawMutex, AppDriver>;
type AppRx = WireRxImpl<AppDriver>;
type AppServer = Server<AppTx, AppRx, WireRxBuf, MyApp>;

const KEYPAD_MATRIX: [[u8; 7]; 3] = [
    [0x04u8, 0x05u8, 0x06u8, 0x07u8, 0x08u8, 0x09u8, 0x0Au8], //A->G
    [0x27u8, 0x1eu8, 0x1fu8, 0x20u8, 0x21u8, 0x52u8, 0x51u8], //0->4, up arrow, down arrow
    [0x22u8, 0x23u8, 0x24u8, 0x25u8, 0x26u8, 0x28u8, 0x29u8], //5->9, enter, escape
];

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
    I2C0_IRQ => InterruptHandler<I2C0>;
});

pub struct Context {
    //Probably not useful
}

use core::assert;
use core::unreachable;
define_dispatch! {
    app: MyApp;
    spawn_fn: spawn_fn;
    tx_impl: AppTx;
    spawn_impl: WireSpawnImpl;
    context: Context;

    endpoints: {
        list: ENDPOINT_LIST;
        | EndpointTy                | kind        | handler                     |
        | ----------                | ----        | -------                     |
        | SetBacklight              | blocking    | set_backlight               |
        | SetLine1Text              | blocking    | set_line1_text              |
        | SetLine2Text              | blocking    | set_line2_text              |

    };
    topics_in: {
        list: TOPICS_IN_LIST;

        | TopicTy                   | kind      | handler                       |
        | ----------                | ----      | -------                       |
    };
    topics_out: {
        list: TOPICS_OUT_LIST;
    };
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // Create the driver, from the HAL.
    let driver = UsbDriver::new(p.USB, Irqs);

    // Create embassy-usb Config
    let mut config = UsbConfig::new(0x16c0, 0x27DD);

    config.manufacturer = Some("OneVariable");
    config.product = Some("ov-twin");
    config.serial_number = Some("12345678");

    config.device_class = 0xEF;
    config.device_sub_class = 0x02;
    config.device_protocol = 0x01;
    config.composite_with_iads = true;

    let request_handler = MyRequestHandler {};
    static DEVICE_HANDLER: StaticCell<MyDeviceHandler> = StaticCell::new();

    static STATE: StaticCell<State> = StaticCell::new();
    let state = STATE.init(State::new());

    let pbufs = PBUFS.take();

    //We use init_without_build because init consumes the driver, and creates (and doesn't return) a builder otherwise
    let (mut builder, tx_impl, rx_impl) =
        STORAGE.init_without_build(driver, config, pbufs.tx_buf.as_mut_slice());
    let device_handler = DEVICE_HANDLER.init(MyDeviceHandler::new());
    builder.handler(device_handler);

    let context = Context {};

    let dispatcher = MyApp::new(context, spawner.into());
    let vkk = dispatcher.min_key_len();

    let mut server: AppServer = Server::new(
        tx_impl,
        rx_impl,
        pbufs.rx_buf.as_mut_slice(),
        dispatcher,
        vkk,
    );

    //Set up our HID device on the builder
    let config = embassy_usb::class::hid::Config {
        report_descriptor: KeyboardReport::desc(),
        request_handler: None,
        poll_ms: 60,
        max_packet_size: 64,
    };
    let hid: HidReaderWriter<'_, UsbDriver<'_, USB>, 1, 8> =
        HidReaderWriter::<_, 1, 8>::new(&mut builder, state, config);

    // Build the builder - USB device will be run by usb_task
    let usb: UsbDevice<'_, UsbDriver<'_, USB>> = builder.build();

    //Description for the matrix keyboard and how the row/columns map to GPIOs
    let col_pins = [
        Output::new(p.PIN_21, Level::Low),
        Output::new(p.PIN_19, Level::Low),
        Output::new(p.PIN_20, Level::Low),
    ];
    let row_pins = [
        Input::new(p.PIN_0, Pull::Down),
        Input::new(p.PIN_1, Pull::Down),
        Input::new(p.PIN_2, Pull::Down),
        Input::new(p.PIN_3, Pull::Down),
        Input::new(p.PIN_4, Pull::Down),
        Input::new(p.PIN_5, Pull::Down),
        Input::new(p.PIN_6, Pull::Down),
        Input::new(p.PIN_7, Pull::Down),
    ];

    let led_pin = Output::new(p.PIN_25, Level::Low);

    let (reader, writer) = hid.split();

    unwrap!(spawner.spawn(usb_task(usb)));
    unwrap!(spawner.spawn(reader_task(reader, request_handler)));
    unwrap!(spawner.spawn(writer_task(writer, col_pins, row_pins, led_pin)));
    unwrap!(spawner.spawn(i2c_task(p.I2C0, p.PIN_17, p.PIN_16)));

    loop {
        // If the host disconnects, we'll return an error here.
        // If this happens, just wait until the host reconnects
        let _ = server.run().await;
    }
}

type MyUsbDriver = UsbDriver<'static, USB>;
type MyUsbDevice = UsbDevice<'static, MyUsbDriver>;
type MyHidWriter = HidWriter<'static, MyUsbDriver, 8>;
type MyHidReader = HidReader<'static, MyUsbDriver, 1>;

#[embassy_executor::task]
async fn usb_task(mut usb: MyUsbDevice) -> ! {
    usb.run().await
}

#[embassy_executor::task]
async fn reader_task(reader: MyHidReader, mut request_handler: MyRequestHandler) -> ! {
    reader.run(false, &mut request_handler).await;

}


//These are handlers for the Postcard-RPC endpoints
fn set_backlight(_context: &mut Context, _header: VarHeader, rqst: bool) {
    BACKLIGHT_SETTING.signal(rqst);
}

fn set_line1_text(_context: &mut Context, _header: VarHeader, rqst: [u8; 32]) {
    LINE_1_TEXT.signal(rqst);
}

fn set_line2_text(_context: &mut Context, _header: VarHeader, rqst: [u8; 32]) {
    LINE_2_TEXT.signal(rqst);
}

#[embassy_executor::task]
async fn i2c_task(interface: I2C0, scl: PIN_17, sda: PIN_16) {
    //Initialise the I2C0 peripheral on GPIO16(SDA) and GPIO17(SCL)
    let mut i2c=
        i2c::I2c::new_async(interface, scl, sda, Irqs, Config::default());
    let mut delay = Delay;

    //Try to find the LCD
    if let Ok(mut lcd) = Lcd::new(&mut i2c, &mut delay)
        .with_address(0x27)
        .with_cursor_on(false)
        .with_rows(2)
        .init()
    {
        info!("Found I2C LCD at address 0x27");
        let _ = lcd.backlight(lcd_lcm1602_i2c::Backlight::On);
        let _ = lcd.clear();

        let mut line1_text = "    SnackBot";
        let mut line2_text = "Initializing...";

        let mut line1_buf = [0x00u8; 32];
        let mut line2_buf = [0x00u8; 32];

        //Whether the lines need to be written to screen
        let mut changed_line1 = true;
        let mut changed_line2 = true;

        //Used to determine if line needs to be scrolled, and what position it is at
        let mut line1_scrolling = false;
        let mut line2_scrolling = false;
        let mut line1_scroll_index: usize = 0;
        let mut line2_scroll_index: usize = 0;

        loop {
            //if backlight setting has changed, apply it.
            if let Some(res) = BACKLIGHT_SETTING.try_take() {
                let _ = match res {
                    true => lcd.backlight(lcd_lcm1602_i2c::Backlight::On),
                    false => lcd.backlight(lcd_lcm1602_i2c::Backlight::Off),
                };
            }

            //New Line 1 text arrived
            if let Some(line) = LINE_1_TEXT.try_take() {
                line1_buf[..line.len()].copy_from_slice(&line);
                line1_text = if let Ok(text) = core::str::from_utf8(&line1_buf) {
                    text.trim_end()
                }
                else {
                    "Invalid ASCII"
                };
                changed_line1 = true;
                line1_scroll_index = 0;
                //if line longer than 16 chars (display length), it'll need to scroll
                line1_scrolling = line1_text.len() > 16;
            }

            //New Line 2 text arrived
            if let Some(line) = LINE_2_TEXT.try_take() {
                line2_buf[..line.len()].copy_from_slice(&line);
                line2_text = if let Ok(text) = core::str::from_utf8(&line2_buf) {
                    text.trim_end()
                }
                else {
                    "Invalid ASCII"
                };
                changed_line2 = true;
                line2_scroll_index = 0;
                //as above
                line2_scrolling = line2_text.len() > 16;
            }

            let line1_to_display = {
                if line1_scrolling {
                    let l = &line1_text[line1_scroll_index..];
                    let msg_len = line1_text.len();
                    line1_scroll_index = if line1_scroll_index == msg_len {
                        0
                    }
                    else {
                        line1_scroll_index+1
                    };
                    changed_line1 = true;
                    l
                } else {
                    line1_text
                }
            };

            let line2_to_display = {
                if line2_scrolling {
                    let l = &line2_text[line2_scroll_index..];
                    let msg_len = line2_text.len();
                    line2_scroll_index = if line2_scroll_index == msg_len {
                        0
                    }
                    else {
                        line2_scroll_index+1
                    };
                    changed_line2 = true;
                    l
                } else {
                    line2_text
                }
            };

            //Closure to update a single line
            let mut line_update = |row:u8, msg:&str| {
                let _ = lcd.set_cursor(row, 0);
                let _ = lcd.write_str("                ");
                let _ = lcd.set_cursor(row, 0);
                let _ = lcd.write_str(msg);
            };

            if changed_line1 {
                line_update(0, line1_to_display);
                changed_line1 = false;
            }

            if changed_line2 {
                line_update(1, line2_to_display);
                changed_line2 = false;
            }

            Timer::after(Duration::from_millis(500)).await;
        }
    } else {
        warn!("Unable to locate I2C LCD at address 0x27");
    }
}

#[embassy_executor::task]
async fn writer_task(
    mut writer: MyHidWriter,
    mut col_pins: [Output<'static>; 3],
    mut row_pins: [Input<'static>; 8],
    mut led_pin: Output<'static>,
) -> ! {
    loop {
        let pressed_keys = get_pressed_keys(&mut col_pins, &mut row_pins);
        if pressed_keys != [0x00u8; 6] {
            //flash led
            led_pin.set_high();
        }

        let report = KeyboardReport {
            keycodes: pressed_keys,
            leds: 0,
            modifier: 0,
            reserved: 0,
        };

        // Send the report.
        match writer.write_serialize(&report).await {
            Ok(()) => {}
            Err(e) => warn!("Failed to send report: {:?}", e),
        };
        Timer::after(Duration::from_millis(5)).await;
        led_pin.set_low();
    }
}

fn get_pressed_keys(col_pins: &mut [Output; 3], row_pins: &mut [Input; 8]) -> [u8; 6] {
    let mut pressed_keys = [0x00u8; 6];
    let mut pressed_key_count = 0;
    for (col_pin, keypad_col) in core::iter::zip(col_pins.iter_mut(), KEYPAD_MATRIX) {
        //For each column of the matrix keypad, set the column pin high, then scan the row pins to see
        //which buttons are pressed
        col_pin.set_high();
        for (row_pin, keypad_val) in core::iter::zip(row_pins.iter(), keypad_col) {
            if row_pin.is_high() {
                if pressed_key_count == 6 {
                    //If we already have 6 keys pressed, we cannot accept another keypress
                    //Return an array of KEY_ERR_OVF to indicate this to the OS
                    warn!("Too many keys pressed");
                    return [0x01; 6];
                }
                pressed_keys[pressed_key_count] = keypad_val;
                pressed_key_count += 1;
            }
        }
        col_pin.set_low();
    }
    pressed_keys
}

struct MyRequestHandler {}

impl RequestHandler for MyRequestHandler {
    fn get_report(&mut self, id: ReportId, _buf: &mut [u8]) -> Option<usize> {
        info!("Get report for {:?}", id);
        None
    }

    fn set_report(&mut self, id: ReportId, data: &[u8]) -> OutResponse {
        info!("Set report for {:?}: {=[u8]}", id, data);
        OutResponse::Accepted
    }

    fn set_idle_ms(&mut self, id: Option<ReportId>, dur: u32) {
        info!("Set idle rate for {:?} to {:?}", id, dur);
    }

    fn get_idle_ms(&mut self, id: Option<ReportId>) -> Option<u32> {
        info!("Get idle rate for {:?}", id);
        None
    }
}

struct MyDeviceHandler {
    configured: AtomicBool,
}

impl MyDeviceHandler {
    fn new() -> Self {
        MyDeviceHandler {
            configured: AtomicBool::new(false),
        }
    }
}

impl Handler for MyDeviceHandler {
    fn enabled(&mut self, enabled: bool) {
        self.configured.store(false, Ordering::Relaxed);
        if enabled {
            info!("Device enabled");
        } else {
            info!("Device disabled");
        }
    }

    fn reset(&mut self) {
        self.configured.store(false, Ordering::Relaxed);
        info!("Bus reset, the Vbus current limit is 100mA");
    }

    fn addressed(&mut self, addr: u8) {
        self.configured.store(false, Ordering::Relaxed);
        info!("USB address set to: {}", addr);
    }

    fn configured(&mut self, configured: bool) {
        self.configured.store(configured, Ordering::Relaxed);
        if configured {
            info!(
                "Device configured, it may now draw up to the configured current limit from Vbus."
            )
        } else {
            info!("Device is no longer configured, the Vbus current limit is 100mA.");
        }
    }
}
