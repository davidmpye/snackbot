//This task owns the MDB and is responsible for trying to initialise devices that might be on it.
//Once a device is initialised, it should be polled regularly by this task.
//If a device stops replying, it should be destroyed.
use mdb_async::cashless_device::CashlessDevice;
use mdb_async::coin_acceptor::CoinAcceptor;

use crate::PioUart;
use crate::Mdb;
use crate::cashless_device::cashless_handler;

use defmt::*;

use crate::Duration;


const DEVICE_POLL_INTERVAL: Duration = Duration::from_millis(100);
use crate::Timer;

#[embassy_executor::task]
pub async fn mdb_task(mut bus: Mdb<PioUart<'static, 0>>) -> ! {
    //These devices don't exist at function start - if present, the handlers will init them
    let mut cashless:Option<CashlessDevice> = None;
   // let mut CoinAcceptor:Option<CoinAcceptor> = None;
    loop {
        cashless_handler(&mut bus, &mut cashless).await;
        //Await poll time.
        Timer::after(DEVICE_POLL_INTERVAL).await;
    }
}
