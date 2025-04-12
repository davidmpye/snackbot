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

use vmc_icd::CoinInsertedTopic;

use vmc_icd::coinacceptor::{CoinAcceptorEvent, CoinInserted, CoinRouting};

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
    VendSuccess,
    VendFailed,
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
                let events = {
                    let mut b = MDB_DRIVER.lock().await;
                    let bus = b.as_mut().expect("MDB driver not present");
                   // cashless.poll(bus).await
                };    /*            

                match events {
                    Ok(events) => {
                 //       coinacceptor_process_poll_events(events, &postcard_sender).await;
                    }
                    Err(()) => {
                        error!("Cashless device failed to reply to poll - will try to reinitialise");
                        break 'poll_loop;
                    }
                }
                 */
                Timer::after(CASHLESS_DEVICE_POLL_INTERVAL).await;

            }
            None => {
                info!("Cashless device not found");
                Timer::after(CASHLESS_DEVICE_INIT_RETRY_INTERVAL).await;
            }
        }
    }
}


