use embassy_time::{Duration, Timer, with_timeout};
use postcard_rpc::server::{impls::embassy_usb_v0_4::EUsbWireTx};
use postcard_rpc::header::VarHeader;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver as UsbDriver;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;


use vmc_icd::{Vend, VendCommand, VendError, VendResult};

use crate::motor_driver::{CanStatus, DispenserAddress, DispenserType, MotorStatus};

use crate::{AppTx, MotorDriverResources, Sender, SpawnCtx, Context};

use defmt::*;

use crate::DISPENSER_DRIVER;
use crate::cashless_device::{CashlessDeviceCommand, CashlessDeviceResponse, CASHLESS_COMMAND_SIGNAL, CASHLESS_RESPONSE_SIGNAL};

static PAYMENT_TIMEOUT:Duration = Duration::from_secs(30);

enum PaymentType {
    Cash(u16),//amount received
    Card(u16),//amount authorised
}

enum PaymentError {
    Cancelled,
    Denied,
    TimedOut,
    DeviceFault,
    NoPaymentDevice,
}

//Spawned in response to the a vend request to handle the process
#[embassy_executor::task]
pub async fn vend_handler(
    _context: SpawnCtx,
    header: VarHeader,
    cmd: VendCommand,
    sender: Sender<AppTx>,
) {
    {
        let mut r = DISPENSER_DRIVER.lock().await;
        let driver = r.as_mut().expect("Motor driver must be stored in mutex");
        
        match driver.get_dispenser(DispenserAddress { row: cmd.row as char, col:cmd.col as char}).await {

            Some(dispenser) => {       
                //Check the dispenser is dispensable - if not, we'll return that now.
                match driver.is_dispensable(dispenser) {
                    Ok(()) => {
                        //Dispenser exists, now we collect payment
                        match collect_payment(dispenser.address, cmd.price).await {
                            Ok(payment) => {
                                //Now dispense item
                                match driver.dispense(dispenser, false).await {
                                    Ok(_) => {
                                        //Notify the payment subsystem
                                        vend_success(dispenser.address, cmd.price).await;   
                                        match sender.reply::<Vend>(header.seq_no, &Ok(())).await {
                                            Ok(_) => debug!("Vend success reply sent OK"),
                                            Err(_) => error!("Vend success reply did not send")
                                        }
                                    },
                                    Err(e) => {
                                        //Notify the payment subsystem to do refund
                                        vend_failed(dispenser.address, cmd.price).await;
                                        match sender.reply::<Vend>(header.seq_no, &Err(e)).await {
                                            Ok(_) => debug!("Vend failed reply sent OK"),
                                            Err(_) => error!("Vend failed reply did not send")
                                        }
                                    },   
                                }
                            },
                            Err (e) => {
                                match sender.reply::<Vend>(header.seq_no, &Err(VendError::PaymentFailed)).await {
                                    Ok(_) => debug!("Payment reply sent OK"),
                                    Err(_) => error!("Payment reply did not send")
                                }
                                return;
                            }
                        }       
                    }
                    Err(e)=> {
                        match sender.reply::<Vend>(header.seq_no, &Err(e)).await {
                            Ok(_) => debug!("Not vendable reply sent OK"),
                            Err(_) => error!("Not vendable reply did not send")
                        }
                        //propagate error to sender
                        return
                    },
                }
            },
            None => {
                //There is no dispenser at this address - you've asked for an invalid address
                match sender.reply::<Vend>(header.seq_no, &Err(VendError::InvalidAddress)).await {
                    Ok(_) => debug!("Invalid address reply sent OK"),
                    Err(_) => error!("Invalid address reply did not send")
                }
                return
            },
        };
    }
}

async fn collect_payment(addr: DispenserAddress, amount:u16) -> Result<PaymentType, PaymentError> {
    CASHLESS_COMMAND_SIGNAL.signal(CashlessDeviceCommand::StartTransaction(amount, addr.row as u8, addr.col as u8));
    CASHLESS_RESPONSE_SIGNAL.reset();
    //Now yield
    match with_timeout(PAYMENT_TIMEOUT, CASHLESS_RESPONSE_SIGNAL.wait() ).await {
        Ok(reply) => {
            match reply {
                CashlessDeviceResponse::VendApproved(amount_authorised) => {
                    debug!("Cashless device- Vend approved");
                    if amount == amount_authorised {
                        debug!("Card authorised for {}", amount_authorised);
                        return Ok(PaymentType::Card(amount));
                    }
                    else {
                        error!("Card auth issue - requested {}, got {}", amount, amount_authorised);
                        return Err(PaymentError::Denied);
                    }
                },
                CashlessDeviceResponse::VendDenied => {
                    debug!("Cashless device- Vend denied");
                    return Err(PaymentError::Denied);
                },
                CashlessDeviceResponse::Unavailable => {
                    debug!("Cashless device - no payment device");
                    return Err(PaymentError::NoPaymentDevice);
                }
                _ => { 
                    error!("Unexpected response to collect payment");
                    return Err(PaymentError::DeviceFault);
                }
            }
        },
        Err(_) => {
            debug!("Collect payment timed out");
            return Err(PaymentError::TimedOut);
        }
    }  
}   

async fn vend_success(addr: DispenserAddress, amount:u16) {
    //For now we assume we're using cashless, but we need to be smart and handle coin stuff one day
    CASHLESS_COMMAND_SIGNAL.signal(CashlessDeviceCommand::VendSuccess(addr.row as u8, addr.col as u8));
}   


async fn vend_failed(addr: DispenserAddress, amount:u16) {
    //if card payment, cancel
    CASHLESS_COMMAND_SIGNAL.signal(CashlessDeviceCommand::VendFailed);
}


