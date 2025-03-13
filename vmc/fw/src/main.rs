#![no_std]
#![no_main]

use assign_resources::assign_resources;

mod mdb_driver;
mod motor_driver;
mod usb_device_handler;
mod chiller_driver;

use mdb_driver::coinacceptor_poll_task;
use motor_driver::{motor_driver_dispense_task, motor_driver_dispenser_status};
use chiller_driver::chiller_task;

use embassy_sync::mutex::Mutex;

use embassy_executor::Spawner;

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;

use embassy_usb::Config as UsbConfig;

use embassy_rp::peripherals::USB;
use embassy_rp::usb;
use embassy_rp::usb::{Driver as UsbDriver, InterruptHandler as UsbInterruptHandler};
use embassy_rp::{adc, adc::{Adc, Async, Config, InterruptHandler}, bind_interrupts, peripherals};
use embassy_rp::gpio::Pull;

use embassy_time::Duration;

use static_cell::{ConstStaticCell, StaticCell};

use vmc_icd::{
    CoinInsertedTopic, DispenseEndpoint, DispenserStatusEndpoint, EventTopic, ENDPOINT_LIST, TOPICS_IN_LIST, TOPICS_OUT_LIST,
};

use crate::motor_driver::MotorDriver;
use pio_9bit_uart_async::PioUart;
use postcard_rpc::{
    define_dispatch,
    server::{
        impls::embassy_usb_v0_4::{
            dispatch_impl::{
                spawn_fn, WireRxBuf, WireRxImpl, WireSpawnImpl, WireStorage, WireTxImpl,
            },
            EUsbWireTx, PacketBuffers,
        },
        Dispatch, Sender, Server, SpawnContext,
    },
};

use usb_device_handler::usb_task;
use usb_device_handler::UsbDeviceHandler;

use {defmt_rtt as _, panic_probe as _};

use mdb_async::coin_acceptor::{CoinAcceptor, PollEvent};
use mdb_async::Mdb;

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
    ADC_IRQ_FIFO => InterruptHandler;
});

pub struct Context {}
pub struct SpawnCtx {}

impl SpawnContext for Context {
    type SpawnCtxt = SpawnCtx;
    fn spawn_ctxt(&mut self) -> Self::SpawnCtxt {
        SpawnCtx {}
    }
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
        | EndpointTy                | kind        | handler                       |
        | ----------                | ----        | -------                       |
        | DispenseEndpoint          | spawn       | motor_driver_dispense_task    | //spawn due to duration of action
        | DispenserStatusEndpoint   | async       | motor_driver_dispenser_status |
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

assign_resources! {
    motor_driver_pins: MotorDriverResources {
        p0: PIN_0,
        p1: PIN_1,
        p2: PIN_2,
        p3: PIN_3,
        p4: PIN_4,
        p5: PIN_5,
        p6: PIN_6,
        p7: PIN_7,
        clk0: PIN_8,
        clk1: PIN_9,
        clk2: PIN_10,
        oe: PIN_11,
        clr: PIN_12,
    },
    adc_pin: AdcResources {
        pin: PIN_26,
    }
}

static MDB_DRIVER: Mutex<CriticalSectionRawMutex, Option<Mdb<PioUart<0>>>> = Mutex::new(None);    
static DISPENSER_DRIVER: Mutex<ThreadModeRawMutex, Option<MotorDriver>> = Mutex::new(None);

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    // Create the driver from the HAL.
    let driver = UsbDriver::new(p.USB, Irqs);
    static CONFIG: StaticCell<embassy_usb::Config> = StaticCell::new();
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
  
    let resources = split_resources!(p);

    {
        //Set up the dispenser motor driver struct - the task that uses it is spawned by postcard-rpc
        let mut m = DISPENSER_DRIVER.lock().await;
        *m = Some(MotorDriver::new(resources.motor_driver_pins).await);
    }

    //Set up the ADC for the chiller thermistor
    let adc = Adc::new(p.ADC, Irqs, Config::default());
    let p26 = adc::Channel::new_pin(resources.adc_pin.pin, Pull::None);
    spawner.must_spawn(chiller_task(adc, p26));

    //Set up the multi-drop bus peripheral (and its' PIO backed 9 bit uart) and place into MutexGuard
    {
        //Set up the 9-bit PIO backed UART the MDB library requires
        let uart: PioUart<'_, 0> = PioUart::new(
            p.PIN_21,
            p.PIN_20,
            p.PIO0,
            Duration::from_millis(25),
            Duration::from_millis(3),
        );
        let mdb = Mdb::new(uart);
        let mut m = MDB_DRIVER.lock().await;
        *m = Some(mdb);
    }

    //Set up the Postcard RPC server
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
    let usb = builder.build();
    spawner.must_spawn(usb_task(usb));

    //Spawn the coin acceptor poll task
    spawner.must_spawn(coinacceptor_poll_task(server.sender().clone()));
    
    //Postcard server mainloop runs here
    loop {
        let _ = server.run().await;
    }
}
