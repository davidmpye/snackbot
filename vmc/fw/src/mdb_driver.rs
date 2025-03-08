use defmt::*;

use embassy_rp::usb::Driver as UsbDriver;
use embassy_time::{Duration, Timer};

use crate::AppDriver;
use postcard_rpc::server::{
    impls::embassy_usb_v0_4::{dispatch_impl::WireTxImpl, EUsbWireTx},
    Sender, WireTx,
};
type AppTx = WireTxImpl<ThreadModeRawMutex, AppDriver>;

use embassy_rp::peripherals::USB;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;

use mdb_async::coin_acceptor::{CoinAcceptor, PollEvent};
use mdb_async::{coin_acceptor, Mdb};
use vmc_icd::EventTopic;

use vmc_icd::CoinInsertedTopic;

use vmc_icd::coinacceptor::{CoinAcceptorEvent, CoinInserted, CoinRouting};

use crate::MDB_DRIVER;

//Task will:
//Init the coin acceptor, or keep retrying every ten seconds
//Poll the coin acceptor every 100mS 
//If it fails to repond to a poll, it will get reinitialised
#[embassy_executor::task]
pub async fn coinacceptor_poll_task(
    postcard_sender: Sender<EUsbWireTx<ThreadModeRawMutex, UsbDriver<'static, USB>>>,
    ) {
    loop {
        while coin_acceptor_try_init().await != true {
            Timer::after(Duration::from_secs(10)).await;
        }
        'poll_loop: loop {
            let response = {
                let mut b = MDB_DRIVER.lock().await;
                let bus = b.as_mut().expect("MDB driver not present");
                let mut acceptor = bus.coin_acceptor.take().expect("Coin acceptor vanished");
                let response = acceptor.poll(bus).await;
                bus.coin_acceptor = Some(acceptor);
                response
            };

            match response {
                Ok(events) => {
                    coinacceptor_process_poll_events(events, &postcard_sender).await;
                    Timer::after_millis(100).await;
                },
                Err(()) => {
                    error!("Coinacceptor failed to reply to poll - will try to reinitialise");
                    break 'poll_loop;
                },
            }
        }  
    }
}

pub async fn coin_acceptor_try_init() -> bool {
    //Unlock MDB
    let mut b = MDB_DRIVER.lock().await;
    let bus = b.as_mut().expect("MDB driver not present");
    match CoinAcceptor::init(bus).await {
        Some(mut acceptor) => {
            if let Some(features) = &acceptor.l3_features {
                info!(
                    "L3 coin acceptor OK, Manufacturer: {}, Model {}, S/N {}",
                    features.manufacturer_code.as_str(),
                    features.model.as_str(),
                    features.serial_number.as_str()
                );
            } else {
                info!("Level 2 coin acceptor OK");
            }
            //Store the coin acceptor in the bus
            bus.coin_acceptor = Some(acceptor);
            true
        },
        None => {
            info!("Coin acceptor init failed");
            bus.coin_acceptor = None;
            false
        },
    }
}

pub async fn coinacceptor_process_poll_events(
    events: [Option<PollEvent>; 16],
    postcard_sender: &Sender<EUsbWireTx<ThreadModeRawMutex, UsbDriver<'static, USB>>>,
) {
    let mut seq = 0x0000u16;
    for e in events.iter() {
        match e {
            Some(event) => {
                match event {
                    PollEvent::Status(bytes) => {
                        info!("Let's pretend it was always escrow....");
                        let _ = postcard_sender
                            .publish::<EventTopic>(seq.into(), &CoinAcceptorEvent::EscrowPressed)
                            .await;
                        seq = seq + 1;
                    }
                    PollEvent::Coin(x) => {
                        info!("Coin inserted - unscaled value: {}", x.unscaled_value);
                        let coinevent = CoinInserted {
                            value: x.unscaled_value,
                            routing: CoinRouting::CashBox, //fixme!
                        };
                        let _ = postcard_sender
                            .publish::<CoinInsertedTopic>(seq.into(), &coinevent)
                            .await;
                        seq = seq + 1;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}
