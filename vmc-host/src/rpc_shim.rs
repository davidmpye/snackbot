use tokio::runtime::Runtime;  //We use the Tokio runtime to run the postcard-rpc async functions
use tokio::time::{sleep, Duration};

use std::sync::OnceLock;
use glib_macros::clone;
use async_channel::{Sender, Receiver};

use crate::EventTopic;

use crate::{VmcDriver, VmcCommand, VmcResponse};
use crate::{LcdDriver, LcdCommand};
use crate::DispenserAddress;

use vmc_icd::CashlessEventTopic;

//Spawn a tokio runtime instance for the postcard-rpc device handlers
fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        Runtime::new().expect("Failed to spawn tokio runtime")
    })
}

async fn get_vmc_driver() -> VmcDriver {
    loop {
        match VmcDriver::new() {
            Ok(driver) => {
                println!("VMC driver connected OK");
                return driver;
            }
            Err(_e) => {
                println!("VMC driver init failed, retrying in 15 seconds");
                tokio::time::sleep(Duration::from_secs(15)).await;   
            }
        }
    }
}

pub(crate) fn spawn_vmc_driver(vmc_response_channel_tx:Sender<VmcResponse>, vmc_command_channel_rx:Receiver<VmcCommand>) {
     //Spawn off the VMC task on the tokio runtime
    runtime().spawn(clone!(
        #[strong] 
        vmc_response_channel_tx,
        #[strong]
        vmc_command_channel_rx,
        async move {

            'outer: loop {
                let mut vmc = get_vmc_driver().await;
                //Await a message
                let mut cashless_topic = vmc.driver.subscribe_multi::<CashlessEventTopic>(8).await.unwrap();
                let mut event_topic = vmc.driver.subscribe_multi::<EventTopic>(8).await.unwrap();
                let mut coin_inserted_topic = vmc.driver.subscribe_multi::<vmc_icd::CoinInsertedTopic>(8).await.unwrap();
                'recvpoll: loop {
                    tokio::select! {
                        val = event_topic.recv()  => {
                            if let Ok(event) = val {
                                let _ = vmc_response_channel_tx.send(VmcResponse::CoinAcceptorEvent(event)).await;
                            }
                            else {
                                println!("Error receiving coinacceptor event");
                                break 'recvpoll;
                            }
                        }
                        val = coin_inserted_topic.recv() => {
                            if let Ok(coin) = val {
                                let _ = vmc_response_channel_tx.send(VmcResponse::CoinInsertedEvent(coin)).await;
                            }
                            else {
                                println!("Error receiving coininserted event");
                                break 'recvpoll;
                            }
                        }
                        val = cashless_topic.recv() => {
                            if let Ok(event) = val {
                                println!("Got a cashless event");
                                let _ = vmc_response_channel_tx.send(VmcResponse::CashlessEvent(event)).await;
                            }
                        }
                        val = vmc_command_channel_rx.recv() => {
                            if let Ok(cmd) = val {
                                match cmd {
                                    VmcCommand::VendItem(row, col) => {
                                        println!("Vend command received - {}{}",row,col);
                                        //Send VMC command
                                        match vmc.dispense(DispenserAddress {row, col}).await {
                                            Ok(()) => {
                                                println!("Vend success");
                                                let _ = vmc_response_channel_tx.send(VmcResponse::DispenseSuccessEvent).await;
                                            },
                                            Err(e) => {
                                                println!("Error - failed to vend");
                                                let _ = vmc_response_channel_tx.send(VmcResponse::DispenseFailedEvent).await;
                                            },
                                        }
                                    },
                                    VmcCommand::SetCoinAcceptorEnabled(enable) => {
                                        let _ = vmc.set_coinacceptor_enabled(enable).await;
                                    },  
                                    VmcCommand::CashlessCmd(cmd) => {
                                        println!("Sending cashless command");
                                        let _ = vmc.send_cashless_device_command(cmd).await;
                                    }
                                    _ => {},
                                }
                            }  
                            else {
                                println!("VMC comms err");
                                break 'recvpoll;
                            }
                        } 
                    }
                }
            }
        }
    ));
}

pub async fn get_lcd_driver() -> LcdDriver {
    loop {
        match LcdDriver::new() {
            Ok(driver) => {
                println!("LCD driver connected OK");
                return driver;
            }
            Err(_e) => {
                println!("LCD driver init failed, retrying in 15 seconds");
                tokio::time::sleep(Duration::from_secs(15)).await;   
            }
        }
    }
}

pub(crate) fn spawn_lcd_driver(lcd_command_channel_rx:Receiver<LcdCommand>) {
    runtime().spawn(clone!(
        #[strong] 
        lcd_command_channel_rx,
        async move {
            let mut lcd = get_lcd_driver().await;
            loop {
                if let Ok(cmd) = lcd_command_channel_rx.recv().await {
                    match cmd {
                        LcdCommand::SetText(l1,l2) => {
                            match lcd.set_text(l1,l2).await {
                                Ok(_x) => {},
                                Err(_x) => {
                                    println!("LCD set text error");
                                }
                            }
                        },
                        LcdCommand::SetBackLight(state) => {
                            match lcd.set_backlight(state).await {
                                Ok(_x) => {},
                                Err(_x) => {   
                                    println!("LCD set backlight error");
                                }
                            }
                        },
                    }
                }
                else {
                    println!("LCD driver command rx err");
                }             
            }
        }
    ));
}
 