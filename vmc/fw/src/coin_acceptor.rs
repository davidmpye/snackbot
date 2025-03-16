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
use embassy_sync::channel::Channel;

use mdb_async::coin_acceptor::{CoinAcceptor, PollEvent};
use vmc_icd::EventTopic;

use vmc_icd::CoinInsertedTopic;

use vmc_icd::coinacceptor::{CoinAcceptorEvent, CoinInserted, CoinRouting};

use postcard_rpc::header::VarHeader;

use crate::MDB_DRIVER;
use crate::Context;

static TASK_COMMAND_CHANNEL: Channel<ThreadModeRawMutex, CoinAcceptorDriverCommand, 2> =
    Channel::new();

const COIN_ACCEPTOR_INIT_RETRY_INTERVAL: Duration = Duration::from_secs(10);
const COIN_ACCEPTOR_POLL_INTERVAL: Duration = Duration::from_millis(100);

pub enum CoinAcceptorDriverCommand {
    Enable,
    Disable,
}

//Task will:
//Init the coin acceptor, or keep retrying every ten seconds
//Poll the coin acceptor every 100mS
//If it fails to repond to a poll, it will get reinitialised
#[embassy_executor::task]
pub async fn coin_acceptor_task(
    postcard_sender: Sender<EUsbWireTx<ThreadModeRawMutex, UsbDriver<'static, USB>>>,
) {
    loop {
        let a = {
            let mut b = MDB_DRIVER.lock().await;
            let bus = b.as_mut().expect("MDB driver not present");
            CoinAcceptor::init(bus).await
        };
        match a {
            Some(mut acceptor) => 'poll_loop: loop {
                let events = {
                    let mut b = MDB_DRIVER.lock().await;
                    let bus = b.as_mut().expect("MDB driver not present");
                    acceptor.poll(bus).await
                };
                match events {
                    Ok(events) => {
                        coinacceptor_process_poll_events(events, &postcard_sender).await;
                        Timer::after(COIN_ACCEPTOR_POLL_INTERVAL).await;
                    }
                    Err(()) => {
                        error!("Coinacceptor failed to reply to poll - will try to reinitialise");
                        break 'poll_loop;
                    }
                }
                //Handle any incoming requests and send those messages to the coin acceptor.
                match TASK_COMMAND_CHANNEL.try_receive() {
                    Ok(msg) => {
                        let mut b = MDB_DRIVER.lock().await;
                        let bus = b.as_mut().expect("MDB driver not present");
                        match msg {
                            CoinAcceptorDriverCommand::Enable => {
                                debug!("Sending coin acceptor enable command");
                                let _ = acceptor.enable_coins(bus, 0xFFFFu16).await;
                            }
                            CoinAcceptorDriverCommand::Disable => {
                                debug!("Sending coin acceptor disable command");
                                let _ = acceptor.enable_coins(bus, 0x00u16).await;
                            }
                        }
                    }
                    Err(_e) => {
                        error!("Task Command Channel rx error");
                    }
                }
            },
            None => {
                info!("Coin acceptor not initialised");
                Timer::after(COIN_ACCEPTOR_INIT_RETRY_INTERVAL).await;
            }
        }
    }
}

//Process the potential list of poll events, and send these as event via postcard-rpc
pub async fn coinacceptor_process_poll_events(
    events: [Option<PollEvent>; 16],
    postcard_sender: &Sender<EUsbWireTx<ThreadModeRawMutex, UsbDriver<'static, USB>>>,
) {
    let mut seq = 0x0000u16;
    for e in events.iter() {
        match e {
            Some(event) => {
                match event {
                    PollEvent::Status(byte) => {
                        let _ = postcard_sender
                            .publish::<EventTopic>(seq.into(), &CoinAcceptorEvent::from(*byte))
                            .await;
                        seq += 1;
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
                        seq += 1;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

pub async fn set_coin_acceptor_enabled(_context: &mut Context, _header: VarHeader, enable: bool) {
    //Send a message to the task via its' channel.
    let message = match enable {
        true => CoinAcceptorDriverCommand::Enable,
        false => CoinAcceptorDriverCommand::Disable,
    };
    TASK_COMMAND_CHANNEL.send(message).await;
}
