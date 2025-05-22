use defmt::*;

use embassy_time::{Duration, Timer};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use mdb_async::cashless_device::{CashlessDevice, PollEvent};

use crate::MDB_DRIVER;

const CASHLESS_DEVICE_INIT_RETRY_INTERVAL: Duration = Duration::from_secs(10);
const CASHLESS_DEVICE_POLL_INTERVAL: Duration = Duration::from_millis(100);


pub enum CashlessDeviceCommand {
    Reset,
    Enable,
    Disable,
    RecordCashTransaction(u16, u8, u8),
    StartTransaction(u16, u8,u8),
    CancelTransaction,
    VendSuccess(u8,u8),
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
            let d = CashlessDevice::init(bus).await;
            if let Some(ref device) = d {   
                let _ = device.set_device_enabled(bus, true).await;
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


                    for event in poll_events {
                        if let Some(e) = event {
                            match e {
                                PollEvent::BeginSessionLevelAdvanced(_) | PollEvent::BeginSessionLevelBasic(_) => {
                                    //No, we don't want it to initiate sessions itself,
                                    //so if it tries to do so, we cancel it.
                                    debug!("Terminated a reader-initiated begin session request");
                                    let mut b = MDB_DRIVER.lock().await;
                                    let bus = b.as_mut().expect("MDB driver not present");
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
                                    let mut b = MDB_DRIVER.lock().await;
                                    let bus = b.as_mut().expect("MDB driver not present");
                                    let _ = device.end_session(bus).await;
                                }
                                PollEvent::Malfunction(_code) => {
                                    error!("Received cashless device malfunction, resetting device");
                                    //Breaking the main loop will force a reinit
                                    break 'main;
                                }
                                PollEvent::SessionCancelRequest => {
                                    debug!("Session cancel request received");
                                    let mut b = MDB_DRIVER.lock().await;
                                    let bus = b.as_mut().expect("MDB driver not present");
                                    let _ = device.end_session(bus).await;
                                }
                                PollEvent::Cancelled => {
                                    debug!("Cancelled");
                                    let mut b = MDB_DRIVER.lock().await;
                                    let bus = b.as_mut().expect("MDB driver not present");
                                    let _ = device.end_session(bus).await;
                                }
                                PollEvent::CmdOutOfSequence => {
                                    error!("Cmd out of sequence, resetting device");
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
                        }
                    }
                    //Handle any pending commands
                    if let Some(cmd) = CASHLESS_COMMAND_SIGNAL.try_take() {
                        debug!("Locking mutex");
                        let mut b = MDB_DRIVER.lock().await;
                        let bus = b.as_mut().expect("MDB driver not present");
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
                                    .record_cash_transaction(
                                        bus,
                                        amount,
                                        [row,col],

                                    )
                                    .await;
                            }
                            CashlessDeviceCommand::StartTransaction(amount, row,col) => {
                                debug!("Entering start transaction");
                                let _ = device
                                    .start_transaction(
                                        bus,
                                        amount,
                                        [row,col],
                                    )
                                    .await;
                            }
                            CashlessDeviceCommand::CancelTransaction => {
                                debug!("Cancelling transaction");
                                let _ = device.cancel_transaction(bus).await;
                                //It should then say vend denied, then we send end session
                            }
                            CashlessDeviceCommand::VendSuccess(row,col) => {
                                debug!("Vend success");
                                let _ = device
                                    .vend_success(bus, [row,col])
                                    .await;

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
                                debug!("Resetting cashless device");
                                //Break the main loop, and we will end up reinitialising the device.
                                break 'main;
                            }
                            _ => {
                                error!("Unhandled command");
                            }
                        }
                    }
                    Timer::after(CASHLESS_DEVICE_POLL_INTERVAL).await;
                }
            }
            None => {
                //If we have anything in the message queue, reply device not found
                info!("Cashless device not found");
                for _i in 0..10u8 {
                    //We keep an eye on the incoming message queue so we can reply that we are unavailable in case anybody writes.
                    if let Some(_cmd) = CASHLESS_COMMAND_SIGNAL.try_take() {
                        CASHLESS_RESPONSE_SIGNAL.signal(CashlessDeviceResponse::Unavailable);
                    }
                    Timer::after(CASHLESS_DEVICE_INIT_RETRY_INTERVAL/10).await;
                }   
                //loop will now try to reinitialise the device again
            }
        }
    }
}
