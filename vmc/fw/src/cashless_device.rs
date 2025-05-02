use defmt::*;

use embassy_rp::usb::Driver as UsbDriver;
use embassy_time::{Duration, Timer};

use postcard_rpc::server::{
    impls::embassy_usb_v0_4::EUsbWireTx,
    Sender};

use embassy_rp::peripherals::USB;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use mdb_async::cashless_device::CashlessDevice;

use vmc_icd::dispenser::DispenserAddress;
use vmc_icd::EventTopic;

use postcard_rpc::header::VarHeader;

use crate::MDB_DRIVER;
use crate::Context;

const CASHLESS_DEVICE_INIT_RETRY_INTERVAL: Duration = Duration::from_secs(10);
const CASHLESS_DEVICE_POLL_INTERVAL: Duration = Duration::from_millis(100);

static TASK_COMMAND_CHANNEL: Channel<ThreadModeRawMutex, CashlessDeviceCommand, 2> =
    Channel::new();

pub enum CashlessDeviceCommand {
    StartTransaction(u16, DispenserAddress),
    CancelTransaction,
    EnableDevice,
    DisableDevice,
    EndSession,
    VendSuccess(DispenserAddress),
    VendFailed,
    RecordCashTransaction(u16, DispenserAddress),
}

//Task will:
//Init the cashless device, or keep retrying every ten seconds
//Poll the cashless device every 100mS
//If it fails to repond to a poll, it will get reinitialised
#[embassy_executor::task]
pub async fn cashless_device_task (
    postcard_sender: Sender<EUsbWireTx<ThreadModeRawMutex, UsbDriver<'static, USB>>>,
) {
    loop {
        //Try to initialise the device
        let a = {
            let mut b = MDB_DRIVER.lock().await;
            let bus = b.as_mut().expect("MDB driver not present");
            CashlessDevice::init(bus).await
        };

        if let Some(ref device) = a {
            info!("Initialised cashless device: {}", device);
        }

        match a {
            Some(mut cashless) => 'poll_loop: loop {
                //Handle any incoming commands from the VMC
                match TASK_COMMAND_CHANNEL.try_receive() {
                    Ok(msg) => {
                        let mut b = MDB_DRIVER.lock().await;
                        let bus = b.as_mut().expect("MDB driver not present");
                        match msg {
                            CashlessDeviceCommand::StartTransaction(amount, address) => {
                                debug!("Received Contactless StartTransaction");
                                cashless.start_transaction(bus, amount, [address.row as u8, address.col as u8]).await;
                            },
                            CashlessDeviceCommand::CancelTransaction => {
                                debug!("Received Contactless CancelTransaction");
                                cashless.cancel_transaction(bus).await;
                            },
                            CashlessDeviceCommand::VendSuccess(address) => {
                                debug!("Received Contactless VendSuccess");
                                cashless.vend_success(bus, [address.row as u8, address.col as u8]).await;
                            },
                            CashlessDeviceCommand::VendFailed => {
                                debug!("Received Contactless VendFailed");
                                cashless.vend_failed(bus).await;

                            },
                            CashlessDeviceCommand::EndSession => {
                                debug!("Received Contactless EndSession");
                                cashless.end_session(bus).await;
                            },
                            CashlessDeviceCommand::RecordCashTransaction(amount, address) => {
                                debug!("Received Contactless RecordCashTransaction");
                                cashless.record_cash_transaction(bus, amount, [address.row as u8, address.col as u8]).await;
                            },
                            _ => {
                                debug!("Unimplemented Contactless command");
                            }
                        }
                    },
                    Err(_e) => {
                        //Channel not open, etc.
                    }
                }
                
                {
                    let mut b = MDB_DRIVER.lock().await;
                    let bus = b.as_mut().expect("MDB driver not present");
                    cashless.poll_heartbeat(bus).await;
                }
                Timer::after(CASHLESS_DEVICE_POLL_INTERVAL).await;

            }
            None => {
                info!("Cashless device not found");
                Timer::after(CASHLESS_DEVICE_INIT_RETRY_INTERVAL).await;
            }
        }
    }
}


