use tokio::runtime::Runtime;  //We use the Tokio runtime to run the postcard-rpc async functions
use tokio::time::{sleep, Duration};

use std::sync::OnceLock;
use glib_macros::clone;
use async_channel::{Sender, Receiver};

use crate::{VmcDriver};
use crate::{LcdDriver, LcdCommand};

use vmc_icd::{VendCommand, VendError, VendResult};

pub enum VmcCommand {
    ItemAvailable(VendCommand),
    Vend(VendCommand),
    ForceDispense(VendCommand),
    //CancelVend
}

pub enum VmcResponse {
    VendResponse(VendResult),
}

//Spawn a tokio runtime instance for the postcard-rpc device handlers
fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        Runtime::new().expect("Failed to spawn tokio runtime")
    })
}

async fn connect_to_vmc() -> Option<VmcDriver> {
    match VmcDriver::new() {
        Ok(driver) => {
            println!("VMC driver connected OK");
            Some(driver)
        }
        Err(e) => {
            println!("VMC driver connection failed - {}", e);
                None
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

            let mut vmc: Option<VmcDriver> = None;

            'outer: loop {
                if vmc.is_none() {
                    println!("Attempting VMC connection");
                    vmc = connect_to_vmc().await;
                }
                //Await a message
          //      let mut cashless_topic = vmc.driver.subscribe_multi::<CashlessEventTopic>(8).await.unwrap();
            //    let mut event_topic = vmc.driver.subscribe_multi::<EventTopic>(8).await.unwrap();
              //  let mut coin_inserted_topic = vmc.driver.subscribe_multi::<vmc_icd::CoinInsertedTopic>(8).await.unwrap();
                'recvpoll: loop {
                    tokio::select! {
                        /*
                        val = cashless_topic.recv() => {
                            if let Ok(event) = val {
                                println!("Got a cashless event");
                                let _ = vmc_response_channel_tx.send(VmcResponse::CashlessEvent(event)).await;
                            }
                        }*/
                        val = vmc_command_channel_rx.recv() => {
                            if let Ok(cmd) = val {
                                //Check we are connected to vmc
                                if let Some(ref mut v) = vmc {
                                    println!("Processing cmd");
                                    match cmd {
                                        VmcCommand::ItemAvailable(cmd) =>{
                                            let res = v.item_available(cmd).await;
                                            let _ =  vmc_response_channel_tx.send(VmcResponse::VendResponse(res)).await;
                                        },  
                                        VmcCommand::Vend(cmd) =>
                                            {
                                            let res = v.vend(cmd).await;
                                            let _ =  vmc_response_channel_tx.send(VmcResponse::VendResponse(res)).await;
                                        },
                                        VmcCommand::ForceDispense(cmd) => {
                                            let res = v.force_dispense(cmd).await;
                                            let _ =  vmc_response_channel_tx.send(VmcResponse::VendResponse(res)).await;            
                                        }         
                                    }
                                }
                                else {
                                  println!("Err - no vmc conn");
                                  let _ =  vmc_response_channel_tx.send(VmcResponse::VendResponse(Err(VendError::CommsFault))).await;  
                                  //This will cause it to try to reinitialise          
                                  break 'recvpoll;
                                }


                               
                            }  
                            else {
                                println!("VMC comms err - will attempt to reconnect");
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
 