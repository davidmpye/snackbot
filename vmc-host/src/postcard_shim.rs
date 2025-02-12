use tokio::runtime::Runtime;  //We use the Tokio runtime to run the postcard-rpc async functions
use std::sync::OnceLock;
use glib_macros::clone;
use async_channel::{Sender, Receiver};

use crate::VmcDriver;
use crate::EventTopic;
use crate::VmcResponse;
use crate::VmcCommand;

use crate::DispenserAddress;
//Spawn a tokio runtime instance for the postcard-rpc device handlers
fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        Runtime::new().expect("Failed to spawn tokio runtime")
    })
}

pub(crate) fn spawn_postcard_shim(vmc_response_channel_tx:Sender<VmcResponse>, vmc_command_channel_rx:Receiver<VmcCommand>) {
     //Spawn off the VMC task on the tokio runtime
    runtime().spawn(clone!(
        #[strong] 
        vmc_response_channel_tx,
        #[strong]
        vmc_command_channel_rx,
        async move {
            if let Ok(mut vmc) = VmcDriver::new() {
                println!("VMC task connected OK");
                //Await a message
                let mut event_topic = vmc.driver.subscribe_multi::<EventTopic>(8).await.unwrap();
                let mut coin_inserted_topic = vmc.driver.subscribe_multi::<vmc_icd::CoinInsertedTopic>(8).await.unwrap();
                loop {
                    tokio::select! {
                        val = event_topic.recv()  => {
                            if let Ok(event) = val {
                                let _ = vmc_response_channel_tx.send(VmcResponse::CoinAcceptorEvent(event)).await;
                            }
                            else {
                                println!("Error receiving coinacceptor event");
                            }
                        }
                        val = coin_inserted_topic.recv() => {
                            if let Ok(coin) = val {
                                let _ = vmc_response_channel_tx.send(VmcResponse::CoinInsertedEvent(coin)).await;
                            }
                        else {
                                println!("Error receiving coininserted event")
                            }
                        }
                        val = vmc_command_channel_rx.recv() => {
                            if let Ok(cmd) = val {
                                match cmd {
                                    VmcCommand::VendItem(row, col) => {
                                        println!("Vend command received - {}{}",row,col);
                                        match vmc.dispense(DispenserAddress {row, col}).await {
                                            Ok(()) => {
                                                println!("Vend success");
                                                //We need to deduct the cost of the item from the remaining credit.

                                            },
                                            Err(e) => {
                                                println!("Error - failed to vend");
                                            },
                                        }
                                    },
                                    _ => {},
                                }
                            }   
                        } 
                    }
                }
            }
            else {
                println!("VMC task failed to connect");
            }
            
        }
    ));
}