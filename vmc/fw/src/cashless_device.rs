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
use vmc_icd::cashless_device::CashlessDeviceCommand::*;

use vmc_icd::cashless_device::*;

use postcard_rpc::header::VarHeader;

use embassy_sync::mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;

use crate::MDB_DRIVER;
use crate::Context;

const CASHLESS_DEVICE_INIT_RETRY_INTERVAL: Duration = Duration::from_secs(10);
const CASHLESS_DEVICE_POLL_INTERVAL: Duration = Duration::from_millis(100);


static CASHLESS_DEVICE: Mutex<CriticalSectionRawMutex, Option<CashlessDevice>> = Mutex::new(None);


//Task will:
//Init the cashless device, or keep retrying every ten seconds
//Poll the cashless device every 100mS
//If it fails to repond to a poll, it will get reinitialised
#[embassy_executor::task]
pub async fn cashless_device_poll_task (
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
                info!("Initialised cashless device: {}", device);
                //Place it in the mutex
                {
                    let mut m = CASHLESS_DEVICE.lock().await;
                    *m = Some(device);
                }
                loop {
                    Timer::after(CASHLESS_DEVICE_POLL_INTERVAL).await;
                    {
                        let mut b = MDB_DRIVER.lock().await;
                        let bus = b.as_mut().expect("MDB driver not present");
                        let mut r =CASHLESS_DEVICE.lock().await;

                        match r.as_mut()  {
                            Some(device) => {
                                device.poll_heartbeat(bus).await;
                            },
                            None => {
                                //Device has gone
                                error!("Cashless device no longer valid - will attempt to reinit");
                                break;
                            }
                        }
                    }
                }
            },
            None => {
                info!("Cashless device not found");
                Timer::after(CASHLESS_DEVICE_INIT_RETRY_INTERVAL).await;
                //loop will now try to reinitialise the device again
            }
        }
    }
}

pub async fn cashless_device_cmd_handler (    
    _context: &mut Context,
    _header: VarHeader,
    c : CashlessDeviceCommand) -> Result<(),()> {
        let mut b = MDB_DRIVER.lock().await;
        let bus = b.as_mut().expect("MDB driver not present");
        let mut r =CASHLESS_DEVICE.lock().await;

        match r.as_mut()  {
            Some(device) => {
                let result = match c {
                    StartTransaction(unscaled_amount, d) => {
                        debug!("Start contactless transaction, amount {}, addr {}{}", unscaled_amount, d.row, d.col);
                        device.start_transaction(bus, unscaled_amount, [d.row as u8, d.col as u8]).await

                    },
                    CancelTransaction=> {
                        device.cancel_transaction(bus).await
                    },
                    EnableDevice => {
                        device.set_device_enabled(bus, true).await
                    },
                    DisableDevice => {
                        device.set_device_enabled(bus, false).await
                    },
                    EndSession=> {
                        device.end_session(bus).await
                    },
                    VendSuccess(d)=>{
                        device.vend_success(bus, [d.row as u8, d.col as u8]).await
                    },
                    VendFailed => {
                        device.vend_failed(bus).await
                    },
                    RecordCashTransaction(unscaled_amount, d) => {
                        device.record_cash_transaction(bus, unscaled_amount, [d.row as u8, d.col as u8]).await
                    },
                };

                if result {
                    Ok(())
                }
                else {
                    Err(())
                }
            },
            None => {
                //Device has gone
                error!("Unable to handle cashless device command - no device");
                Err(())
            }
        }
}


