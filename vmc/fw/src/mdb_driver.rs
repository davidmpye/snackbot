use defmt::*;

use pio_9bit_uart_async::PioUart;
use embedded_io_async::{Read, Write};
use embassy_time::{Timer,Duration};
use postcard_rpc::{
    define_dispatch,
    header::VarHeader,
    server::{
        impls::embassy_usb_v0_4::{
            dispatch_impl::{WireRxBuf, WireRxImpl, WireSpawnImpl, WireStorage, WireTxImpl},
            PacketBuffers, EUsbWireTx,
        },
        Dispatch, Sender, Server, WireTx
    },
};
use embassy_rp::usb::{Driver as UsbDriver, InterruptHandler as UsbInterruptHandler};

use embassy_rp::peripherals::{USB, PIO0};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;

use mdb_async::Mdb;
use mdb_async::coin_acceptor::{CoinAcceptor, PollEvent};
use vmc_icd::EventTopic;

use vmc_icd::{
    ENDPOINT_LIST, TOPICS_IN_LIST, TOPICS_OUT_LIST, ForceDispense, Dispense, GetDispenserInfo, CoinInsertedTopic, SetCoinAcceptorEnabled,
};

use vmc_icd::coinacceptor::{CoinInserted, CoinRouting, CoinAcceptorEvent};

use crate::MDB_DRIVER;


#[embassy_executor::task]
pub async fn coinacceptor_poll_task(sender: Sender<EUsbWireTx<ThreadModeRawMutex, UsbDriver<'static, USB>>>) {
    {
        let mut b = MDB_DRIVER.lock().await;
        let mdb = b.as_mut().expect("MDB driver not present");  
        match mdb.coin_acceptor.take() {
            Some(mut coinacceptor) =>  {
                if let Some(features) = &coinacceptor.l3_features {
                    info!("L3 coin acceptor OK, Manufacturer: {}, Model {}, S/N {}", features.manufacturer_code.as_str(), features.model.as_str(), features.serial_number.as_str());
                }
                else {
                    info!("Level 2 coin acceptor OK");
                }
                //Return the coin acceptor
                mdb.coin_acceptor = Some(coinacceptor);   
            },
            None => {
                info!("No coin acceptor present")
            }
        }
    }

    let mut seq = 0x00u16;
    loop {
            //Perform the poll
            let poll_response = {
                let mut b = MDB_DRIVER.lock().await;
                let mdb = b.as_mut().expect("MDB driver not present");  
                match mdb.coin_acceptor.take() {
                    Some(mut coin_acceptor) => {
                        let response = coin_acceptor.poll(mdb).await;
                        mdb.coin_acceptor = Some(coin_acceptor);
                        response       
                    },
                    None => {
                        [None;16]
                    },
                }
            };

            //Handle the poll events
            for e in poll_response.iter() {
                match e {
                    Some(event) => {
                        match event {
                            PollEvent::Status(bytes) => {
                                info!("Let's pretend it was always escrow");
                                let _ = sender.publish::<EventTopic>(seq.into(), &CoinAcceptorEvent::EscrowPressed).await;   
                                seq = seq.wrapping_add(1);         
                            }
                            PollEvent::Coin(x) => {
                                info!("Coin inserted - unscaled value: {}", x.unscaled_value);     
                                let coinevent = CoinInserted {
                                    value: x.unscaled_value,
                                    routing: CoinRouting::CashBox //fixme!
                                };
                                let _ = sender.publish::<CoinInsertedTopic>(seq.into(), &coinevent).await;
                                seq = seq.wrapping_add(1);         
                            }
                            _=> {},
                        }
                    },
                    _ =>{},
                }
            }
            //Re-poll every 100mS
            Timer::after_millis(100).await;
        } 



   
}
