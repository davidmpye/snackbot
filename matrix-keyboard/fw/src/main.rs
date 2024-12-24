#![no_std]
#![no_main]

use core::sync::atomic::{AtomicBool, Ordering};
use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Input, Pull};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_usb::class::hid::{HidReaderWriter, ReportId, RequestHandler, State};
use embassy_usb::control::OutResponse;
use embassy_usb::{Builder, Config, Handler};
use usbd_hid::descriptor::{KeyboardReport, SerializedDescriptor};
use {defmt_rtt as _, panic_probe as _};
use embassy_rp::gpio::{Level, Output};

use embassy_time::{Duration, Timer};

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());   

    // Create the driver, from the HAL.
    let driver = Driver::new(p.USB, Irqs);

    // Create embassy-usb Config
    let mut config = Config::new(0xdead, 0xbeef);
    config.manufacturer = Some("SnackBot");
    config.product = Some("VendMatrixKeyboard");
    config.serial_number = Some("12345678");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    let mut config_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    // You can also add a Microsoft OS descriptor.
    let mut msos_descriptor = [0; 256]; 
    let mut control_buf = [0; 64];
    let mut request_handler = MyRequestHandler {};
    let mut device_handler = MyDeviceHandler::new();

    let mut state = State::new();

    let mut builder = Builder::new(
        driver,
        config,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut msos_descriptor,
        &mut control_buf,
    );

    builder.handler(&mut device_handler);

    // Create classes on the builder.
    let config = embassy_usb::class::hid::Config {
        report_descriptor: KeyboardReport::desc(),
        request_handler: None,
        poll_ms: 60,
        max_packet_size: 64,
    };
    let hid = HidReaderWriter::<_, 1, 8>::new(&mut builder, &mut state, config);

    // Build the builder.
    let mut usb = builder.build();

    // Run the USB device.
    let usb_fut = usb.run();

    //Description for the matrix keyboard and how the row/columns map to GPIOs
    let mut col_pins = [ Output::new(p.PIN_19, Level::Low), Output::new(p.PIN_20, Level::Low), Output::new(p.PIN_21, Level::Low) ] ;
    let mut row_pins =  [ 
        Input::new(p.PIN_0, Pull::Down),  Input::new(p.PIN_1, Pull::Down),Input::new(p.PIN_2, Pull::Down),Input::new(p.PIN_3, Pull::Down),
        Input::new(p.PIN_4, Pull::Down),  Input::new(p.PIN_5, Pull::Down),Input::new(p.PIN_6, Pull::Down),Input::new(p.PIN_7, Pull::Down),
    ];

    //This needs to match the same grid as the row_pins/col_pins above, or badness.
    let keypad_matrix = [ 
        [ 0x04u8, 0x05u8, 0x06u8, 0x07u8, 0x08u8, 0x09u8, 0x0Au8],  //A->G
        [ 0x27u8, 0x1eu8, 0x1fu8, 0x20u8, 0x21u8, 0x52u8, 0x51u8],  //0->4, up arrow, down arrow
        [ 0x22u8, 0x23u8, 0x24u8, 0x25u8, 0x26u8, 0x28u8, 0x29u8],  //5->9, enter, escape
    ];

    let (reader, mut writer) = hid.split();

    let in_fut = async {
        loop {
            let report = KeyboardReport {
                keycodes: get_pressed_keys(&mut col_pins, &mut row_pins, keypad_matrix),
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
        }
    };

    let out_fut = async {
       reader.run(false, &mut request_handler).await;
    };

    // Run everything concurrently.
    // If we had made everything `'static` above instead, we could do this using separate tasks instead.
    join(usb_fut, join(in_fut, out_fut)).await;
}

fn get_pressed_keys(col_pins:&mut [Output;3], row_pins: &mut[Input;8], keypad_matrix:[[u8;7];3]) -> [u8;6] {
    let mut pressed_keys = [0x00u8;6];
    let mut num_pressed_keys = 0;

    for (col_num, col_pin) in col_pins.iter_mut().enumerate() {
        col_pin.set_high();
        for (row_num, row_pin) in row_pins.iter_mut().enumerate()  {
            if row_pin.is_high() {
                if num_pressed_keys == 5 {
                    //If we already have 6 keys pressed, we cannot accept another keypress
                    //Return an array of KEY_ERR_OVF to indicate this to the OS
                    warn!("Too many keys pressed");
                    return [0x01;6];
                }
                pressed_keys[num_pressed_keys] = keypad_matrix[col_num][row_num];
                num_pressed_keys+=1;
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