use defmt::*;

use embassy_rp::usb::{Driver as UsbDriver};
use embassy_time::{Duration, Timer};

use postcard_rpc::{
    server::{
        impls::embassy_usb_v0_4::{
            dispatch_impl::WireTxImpl,
        EUsbWireTx},
        Sender, WireTx,

    },
};
use crate::AppDriver;
type AppTx = WireTxImpl<ThreadModeRawMutex, AppDriver>;

use embassy_rp::peripherals::USB;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use mdb_async::coin_acceptor::{CoinAcceptor, PollEvent};
use mdb_async::{coin_acceptor, Mdb};
use vmc_icd::EventTopic;

use vmc_icd::{
    CoinInsertedTopic,
};

use vmc_icd::coinacceptor::{CoinAcceptorEvent, CoinInserted, CoinRouting};

use crate::MDB_DRIVER;

pub struct ChannelMessage {
    pub message: CoinAcceptorMessage,
    pub reply_channel: Channel<CriticalSectionRawMutex, CoinAcceptorResponse, 1>,
}

pub enum CoinAcceptorMessage {
    SetEnabled(bool),
    DispenseCoins(u16),
}
pub enum CoinAcceptorResponse {
    Ok,
    Err,
    CoinsDispensed(u16),
}

pub static COIN_ACCEPTOR_CHANNEL: Channel<CriticalSectionRawMutex, ChannelMessage, 1> = Channel::new();

#[embassy_executor::task]
pub async fn coinacceptor_poll_task(postcard_sender:  Sender<EUsbWireTx<ThreadModeRawMutex, UsbDriver<'static, USB>>>) {
    loop {
        match coin_acceptor_init().await {
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
                loop {
                    //Task main loop runs here
                    //See if any channel messages have arrived.
                    //Run a poll task
                    let response = {
                        //Unlock MDB
                        let mut b = MDB_DRIVER.lock().await;
                        let bus = b.as_mut().expect("MDB driver not present");    
                        acceptor.poll(bus).await
                    };
                    //Process the poll events
                    coinacceptor_process_poll_events(response, &postcard_sender).await;
                    
                    //If we have any messages from the channel, handle them
                    match COIN_ACCEPTOR_CHANNEL.try_receive() {
                        Ok(msg) => {
                            match msg.message {
                                CoinAcceptorMessage::SetEnabled(enable) => {
                                    let mask = {
                                        if enable {
                                            0xFFFFu16
                                        }
                                        else {
                                            0x0000u16
                                        }
                                    };
                                    {
                                        let mut b = MDB_DRIVER.lock().await;
                                        let bus = b.as_mut().expect("MDB driver not present");   
                                        match acceptor.enable_coins(bus, mask).await {
                                            Ok(_e) => {
                                                msg.reply_channel.send(CoinAcceptorResponse::Ok).await;
                                            },
                                            Err(_e) => {
                                                msg.reply_channel.send(CoinAcceptorResponse::Err).await;
                                            },
                                        }
                                    }
                                },
                                CoinAcceptorMessage::DispenseCoins(amount) => {
                                    //No can do yet.

                                },

                            }
                        },
                        Err(x) => {},
                    }
                    //Wait 50mS.
                    Timer::after_millis(50).await;
                }
            }
            None => {
                //Wait 10 seconds.
                Timer::after(Duration::from_secs(10)).await;
            }
        }
    }
}

pub async fn coin_acceptor_init() -> Option<CoinAcceptor> {
    //Unlock MDB                 
    let mut b = MDB_DRIVER.lock().await;
    let bus = b.as_mut().expect("MDB driver not present"); 
    //Try to initialise the coin acceptor
    CoinAcceptor::init(bus).await
}

pub async fn coinacceptor_process_poll_events(events: [Option<PollEvent>;16], postcard_sender:  &Sender<EUsbWireTx<ThreadModeRawMutex, UsbDriver<'static, USB>>>)  {
    let mut seq = 0x0000u16;
    for e in events.iter() {
        match e {
            Some(event) => {
                match event {
                    PollEvent::Status(bytes) => {
                        info!("Let's pretend it was always escrow....");
                        let _ = postcard_sender.publish::<EventTopic>(seq.into(), &CoinAcceptorEvent::EscrowPressed).await;   
                        seq = seq + 1;         
                    }
                    PollEvent::Coin(x) => {
                        info!("Coin inserted - unscaled value: {}", x.unscaled_value);     
                        let coinevent = CoinInserted {
                            value: x.unscaled_value,
                            routing: CoinRouting::CashBox //fixme!
                        };
                        let _ = postcard_sender.publish::<CoinInsertedTopic>(seq.into(), &coinevent).await;
                        seq = seq + 1;         
                    }
                    _=> {},
                }
            },
            _ =>{},
        }
    }
}
   