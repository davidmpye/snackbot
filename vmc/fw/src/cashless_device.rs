use defmt::*;

use embassy_rp::usb::Driver as UsbDriver;
use embassy_time::{Duration, Timer};

use postcard_rpc::server::{impls::embassy_usb_v0_4::EUsbWireTx, Sender};

use embassy_rp::peripherals::USB;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use mdb_async::cashless_device::{CashlessDevice, PollEvent};

use vmc_icd::cashless_device::CashlessDeviceCommand::*;
use vmc_icd::dispenser::DispenserAddress;
use vmc_icd::EventTopic;

use vmc_icd::cashless_device::*;

use postcard_rpc::header::VarHeader;

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

use crate::Context;
use crate::MDB_DRIVER;

const CASHLESS_DEVICE_INIT_RETRY_INTERVAL: Duration = Duration::from_secs(10);
const CASHLESS_DEVICE_POLL_INTERVAL: Duration = Duration::from_millis(100);

pub enum CashlessCommand {
    Reset,
    Enable,
    Disable,
    RecordCashTransaction(u16, DispenserAddress),
    StartTransaction(u16, DispenserAddress),
    CancelTransaction,
    VendSuccess(DispenserAddress),
    VendFailed,
}

static CASHLESS_COMMAND_CHANNEL: Channel<ThreadModeRawMutex, CashlessCommand, 2> = Channel::new();
static CASHLESS_RESPONSE_CHANNEL: Channel<
    ThreadModeRawMutex,
    mdb_async::cashless_device::PollEvent,
    32,
> = Channel::new();

//Task will:
//Init the cashless device, or keep retrying every ten seconds
//Poll the cashless device every 100mS
//If it fails to repond to a poll, it will get reinitialised
#[embassy_executor::task]
pub async fn cashless_device_task() -> ! {
    loop {
        //Try to initialise the device
        let d = {
            let mut b = MDB_DRIVER.lock().await;
            let bus = b.as_mut().expect("MDB driver not present");
            CashlessDevice::init(bus).await
        };

        match d {
            Some(device) => {
                info!("Initialised cashless device");
                'main : loop {
                    //This is the main device poll loop
                    let poll_events = {
                        //Unlock the bus, do the poll
                        let mut b = MDB_DRIVER.lock().await;
                        let bus = b.as_mut().expect("MDB driver not present");
                        device.poll(bus).await
                    };
                    //Collect poll events and send them over the response channel to the handler function
                    for event in poll_events {
                        if let Some(e) = event {
                            debug!("Processing poll event {}", e);
                            //Send this event down the command channel
                            CASHLESS_RESPONSE_CHANNEL.send(e).await;
                        }
                    }
                    //Handle any pending commands
                    while let Ok(cmd) = CASHLESS_COMMAND_CHANNEL.try_receive() {
                        let mut b = MDB_DRIVER.lock().await;
                        let bus = b.as_mut().expect("MDB driver not present");
                        debug!("Processing command {}",cmd);
                        match cmd {
                            CashlessCommand::Enable => {
                                device.set_device_enabled(bus, true).await;
                            }
                            CashlessCommand::Disable => {
                                device.set_device_enabled(bus, false).await;
                            }
                            CashlessCommand::RecordCashTransaction(amount, address) => {
                                device
                                    .record_cash_transaction(
                                        bus,
                                        amount,
                                        [address.row as u8, address.col as u8],
                                    )
                                    .await;
                            }
                            CashlessCommand::StartTransaction(amount, address) => {
                                device.start_transaction(bus, amount,[address.row as u8, address.col as u8]).await;
                            }
                            CashlessCommand::CancelTransaction => {
                                device.cancel_transaction(bus).await;
                            }
                            CashlessCommand::VendSuccess(address) => {
                                device.vend_success(bus, [address.row as u8, address.col as u8]).await;
                            },
                            CashlessCommand::VendFailed => {
                                device.vend_failed(bus).await;
                            },
                            CashlessCommand::Reset => {
                                //Break the main loop, and we will end up reinitialising the device.
                                break 'main;
                            }
                        }
                    }
                    Timer::after(CASHLESS_DEVICE_POLL_INTERVAL).await;
                }
            }
            None => {
                info!("Cashless device not found");
                Timer::after(CASHLESS_DEVICE_INIT_RETRY_INTERVAL).await;
                //loop will now try to reinitialise the device again
            }
        }
    }
}
