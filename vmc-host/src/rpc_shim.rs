use tokio::runtime::Runtime;  //We use the Tokio runtime to run the postcard-rpc async functions
use tokio::time::{sleep, Duration};

use std::sync::OnceLock;
use glib_macros::clone;
use async_channel::{Sender, Receiver};

use crate::{VmcDriver};
use crate::{LcdDriver, LcdCommand};

use vmc_icd::{VendCommand, VendError, VendResult, VendProgressTopic, VendProgress, ChillerTopic, chiller::ChillerStatus};

pub enum VmcCommand {
    ItemAvailable(VendCommand),
    Vend(VendCommand),
    ForceDispense(VendCommand),
    //CancelVend
}

pub enum VmcResponse {
    VendResponse(VendResult),
    VendAwaitingPayment,
    VendDispensing,
}

//Spawn a tokio runtime instance for the postcard-rpc device handlers
fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        Runtime::new().expect("Failed to spawn tokio runtime")
    })
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
            match VmcDriver::new() {
                Ok(mut vmc) => {
                    println!("VMC connection successful");
                    //Subscribe to the topics
                    let mut vend_progress_topic = vmc.driver.subscribe_multi::<VendProgressTopic>(8).await.unwrap();
                    let mut chiller_topic = vmc.driver.subscribe_multi::<ChillerTopic>(8).await.unwrap();

                    'recvpoll: loop {
                        //Driver sits here in a select!, waiting for a topic from the VMC, or a command from vmc-host
                        tokio::select! {
                            //Vend progress topic message arrived
                            val = vend_progress_topic.recv() => {
                                match val {
                                    Ok(msg) => {
                                        println!("Vend progress topic message received");
                                        //Propagate message via command
                                        match msg {
                                            VendProgress::AwaitingPayment => {
                                                println!("Awaiting payment");
                                                let _ = vmc_response_channel_tx.send(VmcResponse::VendAwaitingPayment).await;
                                            },
                                            VendProgress::Dispensing => {
                                                println!("Dispense in progress");
                                                let _ = vmc_response_channel_tx.send(VmcResponse::VendDispensing).await;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        println!("Subscription error - reinitialising VMC connection");
                                        break 'recvpoll;
                                    }
                                }
                            },
                            val = chiller_topic.recv() => {
                                match val {
                                    Ok(chiller_status) => {
                                        println!("Received chiller status: Temp {:.2}'C, Setpoint {:.2}'C, chiller_on: {}",
                                            chiller_status.current_temperature, chiller_status.setpoint, chiller_status.on);
                                        //NB need to propagate this to event loop
                                    }
                                    Err(e) => {
                                        println!("Subscription error - reinitialising VMC connection");
                                        break 'recvpoll;
                                    }
                                }
                            }
                            val = vmc_command_channel_rx.recv() => {
                                //Received command from vmc-host
                                if let Ok(cmd) = val {
                                    println!("Processing cmd");
                                    match cmd {
                                        VmcCommand::ItemAvailable(cmd) =>{
                                            let res = vmc.item_available(cmd).await;
                                            let _ =  vmc_response_channel_tx.send(VmcResponse::VendResponse(res)).await;
                                        },  
                                        VmcCommand::Vend(cmd) => {
                                            let res = vmc.vend(cmd).await;
                                            let _ =  vmc_response_channel_tx.send(VmcResponse::VendResponse(res)).await;
                                        },
                                        VmcCommand::ForceDispense(cmd) => {
                                            let res = vmc.force_dispense(cmd).await;
                                            let _ =  vmc_response_channel_tx.send(VmcResponse::VendResponse(res)).await;            
                                        }         
                                    }
                                }
                            }
                        }   
                    }
                }
                Err(e)=> {
                    println!("Vmc connection failure - will retry in 1 sec");
                    sleep(Duration::from_secs(1)).await;
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
 