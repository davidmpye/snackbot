#![no_std]
#![no_main]

use core::sync::atomic::{AtomicBool, Ordering};
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Input, Pull};

use embassy_rp::i2c::{self, Config, InterruptHandler};
use embassy_rp::peripherals::I2C0;

use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver as UsbDriver, InterruptHandler as UsbInterruptHandler};
use embassy_usb::class::hid::{HidWriter, HidReader, HidReaderWriter, ReportId, RequestHandler, State};
use embassy_usb::control::OutResponse;
use embassy_usb::{Builder, Config as UsbConfig, Handler, UsbDevice};
use usbd_hid::descriptor::{KeyboardReport, SerializedDescriptor};
use {defmt_rtt as _, panic_probe as _};
use embassy_rp::gpio::{Level, Output};
use static_cell::StaticCell;
use embassy_time::{Delay, Duration, Timer};
use embassy_rp::peripherals::{PIN_16, PIN_17};
use lcd_lcm1602_i2c::sync_lcd::Lcd;

use postcard_rpc::{
    define_dispatch,
    header::VarHeader,
    server::{
        impls::embassy_usb_v0_3::{
            dispatch_impl::{
                spawn_fn, WireRxBuf, WireRxImpl, WireSpawnImpl, WireStorage, WireTxImpl,
            },
            PacketBuffers,
        },
        Dispatch, Sender, Server, SpawnContext,
    },
};

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
    I2C0_IRQ => InterruptHandler<I2C0>;
});


#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());  

    // Create the driver, from the HAL.
    let driver = UsbDriver::new(p.USB, Irqs);

    // Create embassy-usb Config
    let mut config = UsbConfig::new(0xdead, 0xbeef);
    config.manufacturer = Some("SnackBot");
    config.product = Some("VendMatrixKeyboard");
    config.serial_number = Some("12345678");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();

    let request_handler = MyRequestHandler{};
    static DEVICE_HANDLER:StaticCell<MyDeviceHandler> = StaticCell::new();
    
    let device_handler = DEVICE_HANDLER.init(MyDeviceHandler::new());

    static STATE: StaticCell<State> = StaticCell::new();
    let state = STATE.init(State::new());

    let mut builder = Builder::new(
        driver,
        config,
        CONFIG_DESCRIPTOR.init([0; 256]),
        BOS_DESCRIPTOR.init([0; 256]),
        &mut [], // no msos descriptors
        CONTROL_BUF.init([0; 64])
    );

    builder.handler(device_handler);

    // Create classes on the builder.
    let config = embassy_usb::class::hid::Config {
        report_descriptor: KeyboardReport::desc(),
        request_handler: None,
        poll_ms: 60,
        max_packet_size: 64,
    };
    let hid: HidReaderWriter<'_, UsbDriver<'_, USB>, 1, 8> = HidReaderWriter::<_, 1, 8>::new(&mut builder, state, config);

    // Build the builder - USB device will be run by usb_task
    let usb: UsbDevice<'_, UsbDriver<'_, USB>> = builder.build();

    //Description for the matrix keyboard and how the row/columns map to GPIOs
    let col_pins = [ Output::new(p.PIN_19, Level::Low), Output::new(p.PIN_20, Level::Low), Output::new(p.PIN_21, Level::Low) ] ;
    let row_pins =  [ 
        Input::new(p.PIN_0, Pull::Down),  Input::new(p.PIN_1, Pull::Down),Input::new(p.PIN_2, Pull::Down),Input::new(p.PIN_3, Pull::Down),
        Input::new(p.PIN_4, Pull::Down),  Input::new(p.PIN_5, Pull::Down),Input::new(p.PIN_6, Pull::Down),Input::new(p.PIN_7, Pull::Down),
    ];

    let led_pin = Output::new(p.PIN_25, Level::Low);

    let (reader, writer) = hid.split();

    unwrap!(spawner.spawn(usb_task(usb)));
    unwrap!(spawner.spawn(reader_task(reader, request_handler)));
    unwrap!(spawner.spawn(writer_task(writer, col_pins, row_pins, led_pin)));
    unwrap!(spawner.spawn(i2c_task(p.I2C0, p.PIN_17, p.PIN_16)));
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
async fn reader_task(reader: MyHidReader, mut request_handler: MyRequestHandler)  -> ! {
    reader.run(false, &mut request_handler).await;
}

#[embassy_executor::task]
async fn i2c_task(interface: I2C0, scl: PIN_17, sda: PIN_16) {
    //Initialise the I2C0 peripheral on GPIO16(SDA) and GPIO17(SCL)
    let mut i2c = i2c::I2c::new_async(interface, scl, sda, Irqs, Config::default());
    let mut delay = Delay;
    let line1 = "  SnackBot (C)";
    let line2 = "Powered by Makerspace";

    //Try to find the LCD
    if let Ok(mut lcd) = Lcd::new(&mut i2c, &mut delay)
    .with_address(0x27)
    .with_cursor_on(false) // no visible cursor
    .with_rows(2) // two rows
    .init() {
        info!("Found I2C LCD at address 0x27");
        let _ = lcd.backlight(lcd_lcm1602_i2c::Backlight::On);
        lcd.clear();
        //Write top line            
        lcd.set_cursor(0,0);
        lcd.write_str(line1);
        loop {
            for i in 0..line2.len() {
                //Write scrolling message on second row
                lcd.set_cursor(1,0);
                lcd.write_str("                ");
                lcd.set_cursor(1,0);
                lcd.write_str(&line2[i..line2.len()]);
                Timer::after(Duration::from_millis(500)).await;
            }
        }
    }
    else {
        warn!("Unable to locate I2C LCD at address 0x27");
    }
}

#[embassy_executor::task]
async fn writer_task(mut writer: MyHidWriter, mut col_pins: [Output<'static>;3], mut row_pins: [Input<'static>;8], mut led_pin: Output<'static>) -> ! {
    //This needs to match the same grid as the row_pins/col_pins above, or badness.
    let keypad_matrix = [ 
        [ 0x22u8, 0x23u8, 0x24u8, 0x25u8, 0x26u8, 0x28u8, 0x29u8],  //5->9, enter, escape
        [ 0x27u8, 0x1eu8, 0x1fu8, 0x20u8, 0x21u8, 0x52u8, 0x51u8],  //0->4, up arrow, down arrow
        [ 0x04u8, 0x05u8, 0x06u8, 0x07u8, 0x08u8, 0x09u8, 0x0Au8],  //A->G
    ];

    loop {
        let pressed_keys =  get_pressed_keys(&mut col_pins, &mut row_pins, keypad_matrix);

        if pressed_keys != [0x00u8;6] {
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

fn get_pressed_keys(col_pins:&mut [Output;3], row_pins: &mut[Input;8], keypad_matrix:[[u8;7];3]) -> [u8;6] {
    let mut pressed_keys = [0x00u8;6];
    let mut pressed_key_count = 0;
    for (col_pin, keypad_col) in core::iter::zip(col_pins.iter_mut(), keypad_matrix) {
        //For each column of the matrix keypad, set the column pin high, then scan the row pins to see 
        //which buttons are pressed
        col_pin.set_high();
        for (row_pin, keypad_val) in core::iter::zip(row_pins.iter(), keypad_col) {
            if row_pin.is_high() {
                if pressed_key_count == 6 {
                    //If we already have 6 keys pressed, we cannot accept another keypress
                    //Return an array of KEY_ERR_OVF to indicate this to the OS
                    warn!("Too many keys pressed");
                    return [0x01;6];
                }
                pressed_keys[pressed_key_count] = keypad_val;
                pressed_key_count+=1;
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
            info!("Device configured, it may now draw up to the configured current limit from Vbus.")
        } else {
            info!("Device is no longer configured, the Vbus current limit is 100mA.");
        }
    }
}
