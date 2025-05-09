use defmt::*;

use embassy_rp::usb::Driver as UsbDriver;
use embassy_time::{Duration, Timer};

use postcard_rpc::server::{impls::embassy_usb_v0_4::EUsbWireTx, Sender};

use embassy_rp::peripherals::USB;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use mdb_async::cashless_device::{CashlessDevice, PollEvent};

use vmc_icd::cashless_device::{CashlessDeviceCommand, CashlessDeviceEvent};
use vmc_icd::CashlessEvent;

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
            CashlessDevice::init(bus).await
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
                    for event in poll_events {
                        let mut seq = 0x00u16;
                        if let Some(e) = event {
                            match e {
                                PollEvent::VendApproved(amount) => {
                                    let _ = postcard_sender
                                        .publish::<CashlessEvent>(
                                            seq.into(),
                                            &CashlessDeviceEvent::VendApproved(amount),
                                        )
                                        .await;
                                }
                                PollEvent::VendDenied => {
                                    let _ = postcard_sender
                                        .publish::<CashlessEvent>(
                                            seq.into(),
                                            &CashlessDeviceEvent::VendDenied,
                                        )
                                        .await;

                                    //End session
                                    let mut b = MDB_DRIVER.lock().await;
                                    let bus = b.as_mut().expect("MDB driver not present");
                                    device.end_session(bus).await;
                                }
                                PollEvent::Malfunction(code) => {
                                    error!("Received cashless device malfunction");
                                }
                                PollEvent::SessionCancelRequest | PollEvent::Cancelled => {
                                    let mut b = MDB_DRIVER.lock().await;
                                    let bus = b.as_mut().expect("MDB driver not present");
                                    device.end_session(bus).await;
                                }
                                PollEvent::CmdOutOfSequence => {
                                    error!("Cmd out of sequence, reinitialising device");
                                    break 'main;
                                }
                                _ => {
                                    debug!("Received poll event {}", e)
                                }
                            }
                            seq += 1;
                        }
                    }
                    //Handle any pending commands
                    while let Ok(cmd) = CASHLESS_COMMAND_CHANNEL.try_receive() {
                        let mut b = MDB_DRIVER.lock().await;
                        let bus = b.as_mut().expect("MDB driver not present");
                        debug!("Processing command {}", cmd);
                        match cmd {
                            CashlessDeviceCommand::Enable => {
                                device.set_device_enabled(bus, true).await;
                            }
                            CashlessDeviceCommand::Disable => {
                                device.set_device_enabled(bus, false).await;
                            }
                            CashlessDeviceCommand::RecordCashTransaction(amount, address) => {
                                device
                                    .record_cash_transaction(
                                        bus,
                                        amount,
                                        [address.row as u8, address.col as u8],
                                    )
                                    .await;
                            }
                            CashlessDeviceCommand::StartTransaction(amount, address) => {
                                device
                                    .start_transaction(
                                        bus,
                                        amount,
                                        [address.row as u8, address.col as u8],
                                    )
                                    .await;
                            }
                            CashlessDeviceCommand::CancelTransaction => {
                                device.cancel_transaction(bus).await;
                            }
                            CashlessDeviceCommand::VendSuccess(address) => {
                                device
                                    .vend_success(bus, [address.row as u8, address.col as u8])
                                    .await;
                            }
                            CashlessDeviceCommand::VendFailed => {
                                device.vend_failed(bus).await;
                            }
                            CashlessDeviceCommand::Reset => {
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

pub async fn cashless_device_cmd_handler(
    _context: &mut Context,
    //Send the command down the channel
    _header: VarHeader,
    cmd: CashlessDeviceCommand,
) {
    CASHLESS_COMMAND_CHANNEL.send(cmd).await;
}
