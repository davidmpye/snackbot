use defmt::*;

use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};

use mdb_async::cashless_device::{CashlessDevice, PollEvent};

const CASHLESS_DEVICE_INIT_RETRY_INTERVAL: Duration = Duration::from_secs(10);

pub enum CashlessDeviceCommand {
    Reset,
    Enable,
    Disable,
    RecordCashTransaction(u16, u8, u8),
    StartTransaction(u16, u8, u8),
    CancelTransaction,
    VendSuccess(u8, u8),
    VendFailed,
}

pub enum CashlessDeviceResponse {
    Available,
    Unavailable,
    VendApproved(u16),
    VendDenied,
}

pub static CASHLESS_COMMAND_SIGNAL: Signal<ThreadModeRawMutex, CashlessDeviceCommand> =
    Signal::new();
pub static CASHLESS_RESPONSE_SIGNAL: Signal<ThreadModeRawMutex, CashlessDeviceResponse> =
    Signal::new();
use crate::{Mdb, PioUart};

pub async fn handle_cashless_events(
    bus: &mut Mdb<PioUart<'static, 0>>,
    device: &mut CashlessDevice,
    events: [Option<PollEvent>; 36],
) -> Result<(), ()> {
    for event in events {
        if let Some(e) = event {
            match e {
                PollEvent::BeginSessionLevelAdvanced(_) | PollEvent::BeginSessionLevelBasic(_) => {
                    //No, we don't want it to initiate sessions itself,
                    //so if it tries to do so, we cancel it.
                    debug!("Terminated a reader-initiated begin session request");
                    let _ = device.cancel_transaction(bus).await;
                }
                PollEvent::VendApproved(amount) => {
                    debug!("Cashless device - vend approved for {}", amount);
                    CASHLESS_RESPONSE_SIGNAL.signal(CashlessDeviceResponse::VendApproved(amount));
                }
                PollEvent::VendDenied => {
                    debug!("Cashless device - vend denied");
                    CASHLESS_RESPONSE_SIGNAL.signal(CashlessDeviceResponse::VendDenied);
                    //End session
                    let _ = device.end_session(bus).await;
                }
                PollEvent::SessionCancelRequest => {
                    debug!("Session cancel request received");
                    let _ = device.end_session(bus).await;
                }
                PollEvent::Cancelled => {
                    debug!("Cancelled");
                    let _ = device.end_session(bus).await;
                }
                PollEvent::EndSession => {
                    debug!("End session confirmed by reader");
                }
                PollEvent::Malfunction(_code) => {
                    error!("Received cashless device malfunction - deinit");
                    return Err(());
                }
                PollEvent::CmdOutOfSequence => {
                    error!("Cmd out of sequence, deinit");
                    return Err(());
                }
                _ => {
                    error!("Received unhandled event, deinit");
                    return Err(());
                }
            }
        }
    }
    Ok(())
}

pub async fn handle_cashless_commands(
    bus: &mut Mdb<PioUart<'static, 0>>,
    device: &mut CashlessDevice,
) -> Result<(), ()> {
    //Handle any pending commands
    if let Some(cmd) = CASHLESS_COMMAND_SIGNAL.try_take() {
        match cmd {
            CashlessDeviceCommand::Enable => {
                debug!("Enabling reader");
                let _ = device.set_device_enabled(bus, true).await;
            }
            CashlessDeviceCommand::Disable => {
                debug!("Disabling reader");
                let _ = device.set_device_enabled(bus, false).await;
            }
            CashlessDeviceCommand::RecordCashTransaction(amount, row, col) => {
                debug!("Record cash transaction");
                let _ = device
                    .record_cash_transaction(bus, amount, [row, col])
                    .await;
            }
            CashlessDeviceCommand::StartTransaction(amount, row, col) => {
                debug!("Entering start transaction");
                let _ = device.start_transaction(bus, amount, [row, col]).await;
            }
            CashlessDeviceCommand::CancelTransaction => {
                debug!("Cancelling transaction");
                let _ = device.cancel_transaction(bus).await;
                //It should then say vend denied, then we send end session
            }
            CashlessDeviceCommand::VendSuccess(row, col) => {
                debug!("Vend success");
                let _ = device.vend_success(bus, [row, col]).await;
                let _ = device.end_session(bus).await;
            }
            CashlessDeviceCommand::VendFailed => {
                debug!("Vend failed");
                //Report 'vend failed' to the device, so it will handle a refund
                let _ = device.vend_failed(bus).await;
                //End the session to return the device to the IDLE state
                let _ = device.end_session(bus).await;
            }
            CashlessDeviceCommand::Reset => {
                //If we return Err here, the MDB supervisor task will remove, and then reinit the device.
                return Err(());
            }
        }
    }
    Ok(())
}

pub(crate) async fn cashless_handler(
    bus: &mut Mdb<PioUart<'static, 0>>,
    maybe_device: &mut Option<CashlessDevice>,
) {
    match maybe_device {
        Some(device) => {
            match device.poll(bus).await {
                Ok(events) => {
                    debug!("Cashless device poll completed successfully");
                    //Handle the events
                    match handle_cashless_events(bus, device, events).await {
                        Ok(()) => {
                            debug!("Handled cashless events successfully");
                        }
                        Err(_) => {
                            error!("Cashless device event error - will reinit");
                            *maybe_device = None;
                            return;
                        }
                    }
                }
                Err(_) => {
                    error!("Cashless device poll error - will reinit");
                    *maybe_device = None;
                    return;
                }
            }

            match handle_cashless_commands(bus, device).await {
                Ok(_) => {}
                Err(_) => {
                    error!("Cashless device command error - will reinit");
                    *maybe_device = None;
                    return;
                }
            }
        }

        None => {
            //Device not initialised (or absent) - try to init it
            *maybe_device = CashlessDevice::init(bus).await;
            match maybe_device {
                Some(device) => {
                    info!("Cashless device initialised");
                    //Set the device as enabled so we can use it
                    let _ = device.set_device_enabled(bus, true).await;
                }
                None => {
                    CASHLESS_RESPONSE_SIGNAL.signal(CashlessDeviceResponse::Unavailable);
                }
            }
        }
    }
}
