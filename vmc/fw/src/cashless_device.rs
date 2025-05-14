use defmt::*;

use embassy_rp::usb::Driver as UsbDriver;
use embassy_time::{Duration, Timer};

use postcard_rpc::server::{impls::embassy_usb_v0_4::EUsbWireTx, Sender};

use embassy_rp::peripherals::USB;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use mdb_async::cashless_device::{CashlessDevice, PollEvent};

use vmc_icd::cashless_device::{CashlessDeviceCommand, CashlessDeviceEvent};

use vmc_icd::CashlessEventTopic;

use postcard_rpc::header::VarHeader;

use crate::Context;
use crate::MDB_DRIVER;

const CASHLESS_DEVICE_INIT_RETRY_INTERVAL: Duration = Duration::from_secs(10);
const CASHLESS_DEVICE_POLL_INTERVAL: Duration = Duration::from_millis(100);

static CASHLESS_COMMAND_CHANNEL: Channel<ThreadModeRawMutex, CashlessDeviceCommand, 2> =
    Channel::new();

//Task will:
//Init the cashless device, or keep retrying every ten seconds
//Poll the cashless device every 100mS
//If it fails to repond to a poll, it will get reinitialised
#[embassy_executor::task]
pub async fn cashless_device_task(
    postcard_sender: Sender<EUsbWireTx<ThreadModeRawMutex, UsbDriver<'static, USB>>>,
) -> ! {
    loop {
        //Try to initialise the device
        let d = {
            let mut b = MDB_DRIVER.lock().await;
            let bus = b.as_mut().expect("MDB driver not present");
            let d = CashlessDevice::init(bus).await;
            if let Some(ref device) = d {   
                device.set_device_enabled(bus, true).await;
            }
            d
        };

        match d {
            Some(device) => {
                info!("Initialised cashless device");
                'main: loop {
                    //This is the main device poll loop
                    let poll_events = {
                        //Unlock the bus, do the poll
                        let mut b = MDB_DRIVER.lock().await;
                        let bus = b.as_mut().expect("MDB driver not present");
                        device.poll(bus).await
                    };
                    //Collect poll events and send summary ones to postcard-rpc
                    let mut seq = 0x00u16;
                    for event in poll_events {
                        if let Some(e) = event {
                            match e {
                                PollEvent::BeginSessionLevelAdvanced(_) | PollEvent::BeginSessionLevelBasic(_) => {
                                    //No, we don't want it to initiate sessions.
                                    debug!("Terminated a reader-initiated begin session request");
                                    let mut b = MDB_DRIVER.lock().await;
                                    let bus = b.as_mut().expect("MDB driver not present");
                                    device.cancel_transaction(bus).await;
                                }
                                PollEvent::VendApproved(amount) => {
                                    debug!("Cashless device - vend approved for {}", amount);
                                    let _ = postcard_sender
                                        .publish::<CashlessEventTopic>(
                                            seq.into(),
                                            &CashlessDeviceEvent::VendApproved(amount),
                                        )
                                        .await;
                                }
                                PollEvent::VendDenied => {
                                    debug!("Cashless device - vend denied");
                                    let _ = postcard_sender
                                        .publish::<CashlessEventTopic>(
                                            seq.into(),
                                            &CashlessDeviceEvent::VendDenied,
                                        )
                                        .await;
                                    //End session
                                    let mut b = MDB_DRIVER.lock().await;
                                    let bus = b.as_mut().expect("MDB driver not present");
                                    device.end_session(bus).await;
                                }
                                PollEvent::Malfunction(_code) => {
                                    error!("Received cashless device malfunction");
                                }
                                PollEvent::SessionCancelRequest => {
                                    debug!("Session cancel request received");
                                    let mut b = MDB_DRIVER.lock().await;
                                    let bus = b.as_mut().expect("MDB driver not present");
                                    device.end_session(bus).await;
                                }
                                PollEvent::Cancelled => {
                                    debug!("Cancelled");
                                    let mut b = MDB_DRIVER.lock().await;
                                    let bus = b.as_mut().expect("MDB driver not present");
                                    device.end_session(bus).await;
                                }
                                PollEvent::CmdOutOfSequence => {
                                    error!("Cmd out of sequence, reinitialising device");
                                    //Breaking the main loop will force a reinit
                                    break 'main;
                                }
                                PollEvent::EndSession => {
                                    debug!("End session confirmed by reader");
                                }
                                _ => {
                                    debug!("Received unhandled poll event");
                                }
                            }
                            seq += 1;
                        }
                    }
                    //Handle any pending commands
                    while let Ok(cmd) = CASHLESS_COMMAND_CHANNEL.try_receive() {
                        debug!("Locking mutex");
                        let mut b = MDB_DRIVER.lock().await;
                        let bus = b.as_mut().expect("MDB driver not present");
                        match cmd {
                            CashlessDeviceCommand::Enable => {
                                debug!("Enabling reader");
                                device.set_device_enabled(bus, true).await;
                            }
                            CashlessDeviceCommand::Disable => {
                                debug!("Disabling reader");
                                device.set_device_enabled(bus, false).await;
                            }
                            CashlessDeviceCommand::RecordCashTransaction(amount, address) => {
                                debug!("Record cash transaction");
                                device
                                    .record_cash_transaction(
                                        bus,
                                        amount,
                                        [address.row as u8, address.col as u8],
                                    )
                                    .await;
                            }
                            CashlessDeviceCommand::StartTransaction(amount, address) => {
                                debug!("Entering start transaction");
                                device
                                    .start_transaction(
                                        bus,
                                        amount,
                                        [address.row as u8, address.col as u8],
                                    )
                                    .await;
                            }
                            CashlessDeviceCommand::CancelTransaction => {
                                debug!("Cancelling transaction");
                                device.cancel_transaction(bus).await;
                                //It should then say vend denied, then we send end session
                            }
                            CashlessDeviceCommand::VendSuccess(address) => {
                                debug!("Vend success");
                                device
                                    .vend_success(bus, [address.row as u8, address.col as u8])
                                    .await;
                                device.end_session(bus).await;
                            }
                            CashlessDeviceCommand::VendFailed => {
                                debug!("Vend failed");
                                device.vend_failed(bus).await;
                                device.end_session(bus).await;
                            }
                            CashlessDeviceCommand::Reset => {
                                debug!("Resetting cashless device");
                                //Break the main loop, and we will end up reinitialising the device.
                                break 'main;
                            }
                            _ => {
                                error!("Unhandled command");
                            }
                        }
                    }
                }
                Timer::after(CASHLESS_DEVICE_POLL_INTERVAL).await;
            }
            None => {
                info!("Cashless device not found");
                Timer::after(CASHLESS_DEVICE_INIT_RETRY_INTERVAL).await;
                //loop will now try to reinitialise the device again
            }
        }
    }
}

pub async fn cashless_device_cmd_handler(
    _context: &mut Context,
    //Send the command down the channel
    _header: VarHeader,
    cmd: CashlessDeviceCommand,
) {
    debug!("Command received, sending down channel");
    CASHLESS_COMMAND_CHANNEL.send(cmd).await;
    debug!("Command sent down channel");
}
