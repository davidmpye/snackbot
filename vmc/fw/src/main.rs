#![no_std]
#![no_main]

use core::sync::atomic::{AtomicBool, Ordering};
use defmt::*;

use embassy_executor::Spawner;

use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, signal::Signal};
use embassy_time::{Delay, Duration, Timer};

use embassy_usb::class::hid::{
    HidReader, HidReaderWriter, HidWriter, ReportId, RequestHandler, State,
};
use embassy_usb::control::OutResponse;
use embassy_usb::{Config as UsbConfig, Handler, UsbDevice};

use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{AnyPin, Flex, Input, Level, Output, Pin, Pull};
use embassy_rp::peripherals::USB;
use embassy_rp::usb;
use embassy_rp::usb::{Driver as UsbDriver, InterruptHandler as UsbInterruptHandler};

use static_cell::{ConstStaticCell, StaticCell};

use postcard_rpc::{
    define_dispatch,
    header::VarHeader,
    server::{
        impls::embassy_usb_v0_3::{
            dispatch_impl::{WireRxBuf, WireRxImpl, WireSpawnImpl, WireStorage, WireTxImpl},
            PacketBuffers,
        },
        Dispatch, Sender, Server,
    },
};

use vmc_icd::{
    Dispense, DispenseResult, DispenserOption, ForceDispense, GetDispenserInfo, ENDPOINT_LIST,
    TOPICS_IN_LIST, TOPICS_OUT_LIST,
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

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
});

pub struct MotorControllerInterface<'a> {
    bus: [Flex<'a>; 8],
    clks: [Output<'a>; 3],
    output_enable: Output<'a>,
    flipflop_clr: Output<'a>,
}

impl<'a> MotorControllerInterface<'a> {
    fn new(
        bus_pin0: AnyPin,
        bus_pin1: AnyPin,
        bus_pin2: AnyPin,
        bus_pin3: AnyPin,
        bus_pin4: AnyPin,
        bus_pin5: AnyPin,
        bus_pin6: AnyPin,
        bus_pin7: AnyPin,
        clk_pin1: AnyPin,
        clk_pin2: AnyPin,
        clk_pin3: AnyPin,

        oe_pin: AnyPin,
        flipflop_clr_pin: AnyPin,
    ) -> Self {
        let mut x = Self {
            bus: [
                Flex::new(bus_pin0),
                Flex::new(bus_pin1),
                Flex::new(bus_pin2),
                Flex::new(bus_pin3),
                Flex::new(bus_pin4),
                Flex::new(bus_pin5),
                Flex::new(bus_pin6),
                Flex::new(bus_pin7),
            ],
            clks: [
                Output::new(clk_pin1, Level::Low),
                Output::new(clk_pin2, Level::Low),
                Output::new(clk_pin3, Level::Low),
            ],
            output_enable: Output::new(oe_pin, Level::High),
            flipflop_clr: Output::new(flipflop_clr_pin, Level::High),
        };

        for pin in x.bus.iter_mut() {
            pin.set_low();
            pin.set_as_input();
        }
        x
    }

    async fn write_bytes(&mut self, bytes: [u8; 3]) {
        for (clk_pin, byte) in core::iter::zip(self.clks.iter_mut(), bytes.iter()) {
            //write out the data
            for (bit_index, gpio) in self.bus.iter_mut().enumerate() {
                if byte & (0x01 << bit_index) == 0 {
                    //output enable, to pull gpio low
                    gpio.set_as_output();
                } else {
                    //gpio floating, gpio pulled high by external resistors
                    gpio.set_as_input();
                }
            }
            //Pulse clock pin for 10uS to latch data
            clk_pin.set_high();
            //Pause to allow latching
            Timer::after_micros(10).await;
            //clk pin low
            clk_pin.set_low();
        }
        //Put bus pins back into input mode
        for clk_pin in self.bus.iter_mut() {
            clk_pin.set_as_input();
        }
    }

    async fn read_byte(&mut self) -> u8 {
        //Pull buffer OE to low to put it into read mode
        self.output_enable.set_low();
        //Allow 100uS for it to stabilise its' state
        Timer::after_micros(100).await;
        let mut byte = 0x00u8;
        for (bit_index, gpio) in self.bus.iter().enumerate() {
            if gpio.is_high() {
                byte |= 0x01 << bit_index;
            }
        }
        //Return buffer to write mode
        self.output_enable.set_high();
        byte  
    }

    fn calc_drive_bytes(row: char, col: char) -> Option<[u8; 3]> {
        /*
        Wiring is as follows
        U3:
        0x01 - Cols 0,1
        0x02 - Cols 2,3
        0x04 - Cols 4,5
        0x08 - Cols 6,7
        0x10 - Cols 8,9

        Rows are wired as follows:

        U4:
        0x01 - Row A Even
        0x02 - Row A Odd
        0x04 - Row B Even
        0x08 - Row B Odd
        0x10 - Row C Even
        0x20 - Row C Odd
        0x40 - Row D Even
        0x80 - Row D Odd

        U2:
        0x01 - Row E Even
        0x02 - Row E Odd
        0x04 - Row F Even
        0x08 - Row F Odd

        U3:
        0x20 - Row G (there's no odd!) - Gum and mint row drive (if fitted)
        */
        let mut drive_bytes:[u8;3] = [0x00;3];
        
        //Check row and col are calculable - note, NOT whether they are present in the machine
        if !row.is_ascii() || row < 'A' || row > 'G' {
            return None;
        }
        if !col.is_ascii() || col < '0' || col > '9' {
            return None;
        }

        let row_offset = row as u8 - b'A';
        let col_offset = match row as u8 {
            //Special handling for can chiller rows (E/F) due to discrepancy in numbering and wiring!
            //Row E +F cans are numbered E0, E1, E2, E3 but are wired E0, E2, E4, E6
            //G - Gum and Mint may need special handling if implemented as I suspect that's wired 0/2/4/6/8 also.
            //G is the optional Gum/Mint module.
            b'E' | b'F' => (col as u8 - b'0') * 2,
            //Standard column offset
            _ => col as u8 - b'0',
        };

        let even_odd_offset: u8 = col_offset % 2;

        //Set row drive bit on appropriate flipflop
        match row as u8 {
            b'A' | b'B' | b'C' | b'D' => {
                //U4
                drive_bytes[2] = 0x01 << (row_offset * 2 + even_odd_offset);
            }
            b'E' | b'F' => {
                //U2
                drive_bytes[0] = 0x01 << ((row_offset - 4) * 2 + even_odd_offset);
            }
            b'G' => {
                //U3
                drive_bytes[1] = 0x20;
            }
            _ => {
                //This shouldn't happen!
                defmt::panic!("Asked to apply invalid row calculation!")
            }
        }
        //Set column drive bit
        drive_bytes[1] |= 0x01 << (col_offset / 2);

        Some(drive_bytes)
    }

    pub async fn dispense(&mut self, row: char, col: char) -> DispenseResult {
        info!("Driving motor at {}{}", row, col);      
        if let Some(drive_bytes) = MotorControllerInterface::calc_drive_bytes(row,col) {
            //Send the bytes to the motor driver
            debug!("Calculated drive bytes: {=[u8]:#04x}", drive_bytes);
            self.write_bytes(drive_bytes).await;
            //Now start watching for the motor to leave home


            //Now wait for the motor to return home





        }
        else {
            error!("Unable to calculate drive bytes - aborted");
        }
        Ok(())
    }
}

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
    let mut config = UsbConfig::new(0xDEAD, 0xBEEF);

    config.manufacturer = Some("Snackbot");
    config.product = Some("vmc");
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

    // Build the builder - USB device will be run by usb_task
    let usb: UsbDevice<'_, UsbDriver<'_, USB>> = builder.build();

    //USB device handler task
    spawner.must_spawn(usb_task(usb));

    //Set up the pins for the VMC interface
    let x = MotorControllerInterface::new(
        //bus pins
        p.PIN_0.degrade(),
        p.PIN_1.degrade(),
        p.PIN_2.degrade(),
        p.PIN_3.degrade(),
        p.PIN_4.degrade(),
        p.PIN_5.degrade(),
        p.PIN_6.degrade(),
        p.PIN_7.degrade(),
        //clk pins
        p.PIN_8.degrade(),
        p.PIN_9.degrade(),
        p.PIN_10.degrade(),
        //oe pin
        p.PIN_11.degrade(),
        //clr pin
        p.PIN_12.degrade(),
    );
    //Postcard server mainloop just runs here
    loop {
        let _ = server.run().await;
    }
}

type MyUsbDriver = UsbDriver<'static, USB>;
type MyUsbDevice = UsbDevice<'static, MyUsbDriver>;

#[embassy_executor::task]
async fn usb_task(mut usb: MyUsbDevice) -> ! {
    usb.run().await
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