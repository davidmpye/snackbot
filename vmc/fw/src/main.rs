#![no_std]
#![no_main]

mod motor_driver;
mod usb_device_handler;

use embassy_sync::mutex::Mutex;

use defmt::*;

use embassy_executor::Spawner;

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;

use embassy_usb::Config as UsbConfig;

use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Pin};
use embassy_rp::peripherals::{USB, PIO0};
use embassy_rp::usb;
use embassy_rp::usb::{Driver as UsbDriver, InterruptHandler as UsbInterruptHandler};
use embassy_rp::pio;

use embassy_time::Duration;
use static_cell::{ConstStaticCell, StaticCell};

use vmc_icd::{
    CanStatus, Dispense, DispenseError, DispenseResult, Dispenser, DispenserAddress, DispenserOption, DispenserType, ForceDispense, 
    MotorStatus,GetDispenserInfo, ENDPOINT_LIST, TOPICS_IN_LIST, TOPICS_OUT_LIST
};

use pio_9bit_uart_async::PioUart;
use embedded_io_async::{Read, Write};
use crate::motor_driver::MotorDriver;
use postcard_rpc::{
    define_dispatch,
    header::VarHeader,
    server::{
        impls::embassy_usb_v0_4::{
            dispatch_impl::{WireRxBuf, WireRxImpl, WireSpawnImpl, WireStorage, WireTxImpl},
            PacketBuffers,
        },
        Dispatch, Sender, Server,
    },
};
use usb_device_handler::UsbDeviceHandler;
use usb_device_handler::usb_task;

use {defmt_rtt as _, panic_probe as _};

use mdb_async::Mdb;
use mdb_async::coin_acceptor::{CoinAcceptor, ChangerStatus, PollEvent};

type AppDriver = usb::Driver<'static, USB>;
type BufStorage = PacketBuffers<1024, 1024>;
static PBUFS: ConstStaticCell<BufStorage> = ConstStaticCell::new(BufStorage::new());
static STORAGE: AppStorage = AppStorage::new();
type AppStorage = WireStorage<ThreadModeRawMutex, AppDriver, 256, 256, 64, 256>;
type AppTx = WireTxImpl<ThreadModeRawMutex, AppDriver>;
type AppRx = WireRxImpl<AppDriver>;
type AppServer = Server<AppTx, AppRx, WireRxBuf, MyApp>;

static MOTOR_DRIVER: Mutex<
    CriticalSectionRawMutex,
    Option<MotorDriver>,
> = Mutex::new(None);

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
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
        | Dispense                  | async       | dispense_handler            |
        | ForceDispense             | async       | force_dispense_handler      |
        | GetDispenserInfo          | async       | get_dispenser_info_handler  |
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

async fn dispense_handler(_context: &mut Context, _header: VarHeader, address: DispenserAddress) -> DispenseResult {
    let mut c = MOTOR_DRIVER.lock().await;
    c.as_mut().expect("Motor controller missing from mutex").dispense(address).await
}

async fn force_dispense_handler(_context: &mut Context, _header: VarHeader, address: DispenserAddress) -> DispenseResult {
    let mut c = MOTOR_DRIVER.lock().await;
    c.as_mut().expect("Motor controller missing from mutex").force_dispense(address).await
}

async fn get_dispenser_info_handler(_context: &mut Context, _header: VarHeader, address: DispenserAddress) -> DispenserOption {
    let mut c = MOTOR_DRIVER.lock().await;
    c.as_mut().expect("Motor controller missing from mutex").getDispenser(address).await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    // Create the driver, from the HAL.
    let driver = UsbDriver::new(p.USB, Irqs);

    static CONFIG : StaticCell<embassy_usb::Config> = StaticCell::new();
    // Create embassy-usb Config
    let config = CONFIG.init(UsbConfig::new(0xDEAD, 0xBEEF));
    config.manufacturer = Some("Snackbot");
    config.product = Some("vmc");
    config.serial_number = Some("12345678");
    config.device_class = 0xEF;
    config.device_sub_class = 0x02;
    config.device_protocol = 0x01;
    config.composite_with_iads = true;

    let pbufs = PBUFS.take();

    //We use init_without_build because init consumes the driver, and creates (and doesn't return) a builder otherwise
    let (mut builder, tx_impl, rx_impl) =
        STORAGE.init_without_build(driver, *config, pbufs.tx_buf.as_mut_slice());

    static DEVICE_HANDLER: StaticCell<UsbDeviceHandler> = StaticCell::new();

    let device_handler = DEVICE_HANDLER.init(UsbDeviceHandler::new());
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

    let interface = MotorDriver::new(
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
    ).await;
    
    {
        //Move interface into the mutex
        let mut m = MOTOR_DRIVER.lock().await;
        *m = Some(interface);
    }

    // Build the builder - USB device will be run by usb_task
    let usb = builder.build();

    //USB device handler task
    spawner.must_spawn(usb_task(usb));

    let uart:PioUart<'_, 0> = PioUart::new(p.PIN_21, p.PIN_20, p.PIO0,Duration::from_millis(25), Duration::from_millis(3));
    let mut mdb = Mdb::new(uart);
    
    match CoinAcceptor::init(&mut mdb).await {
        Some(mut b) => {info!("Got {}",b);
        b.enable_coins(&mut mdb, 0xffff).await;
        loop {
            for (num, e) in  b.poll(&mut mdb).await.iter().enumerate() {
                match e {
                    Some(event) => {
                        match event {
                            PollEvent::Status(ChangerStatus::EscrowPressed) => {debug!("Escrow pressed - event number {}", num);}
                            PollEvent::Coin(x) => {debug!("Got a coin - event num {}",num);}
                            _=> {},
                        }
                    },
                     _ =>{},
                }
            }
        }
    },
        _ => {},
    }

 
    //Postcard server mainloop just runs here
    loop {
        let _ = server.run().await;
    }
}


