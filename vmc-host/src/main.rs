
mod lcd_driver;
use lcd_driver::LcdDriver;

mod vmc_driver;
use vmc_driver::VmcDriver;
use vmc_icd::dispenser::{DispenserAddress, Dispenser};

use vmc_icd::EventTopic;
use vmc_icd::coinacceptor::{CoinAcceptorEvent, CoinInserted, CoinRouting};

const APP_ID: &str = "uk.org.makerspace.snackbot";

const KEYBOARD_DEVICE_NAME:&str = "matrix-keyboard";
const VMC_DEVICE_NAME:&str = "vmc";

use tokio::runtime::Runtime;  //We use the Tokio runtime to run the postcard-rpc async functions

use std::sync::OnceLock;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box, Button};
use glib_macros::clone;
use gtk4::glib;

use async_channel::Sender;

//Spawn a tokio runtime instance for the postcard-rpc device handlers
fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        Runtime::new().expect("Failed to spawn tokio runtime")
    })
}

#[derive (Copy, Clone)]
pub enum VmcCommand {
    VendItem(u8,u8),
    ForceVendItem(u8,u8),
    GetMachineMap(),
    GetDispenser(u8,u8),
    SetCoinAcceptorEnabled(bool),
}

pub enum VmcResponse {
    MachineMap(Vec<Dispenser>),
    Dispenser(Dispenser),
    //Vend result for a vend request
    CoinAcceptorEvent(CoinAcceptorEvent),
    CoinInsertedEvent(CoinInserted)
}

pub enum LcdCommand {
    SetText(String, String),
    SetBackLight(bool),
}

/*
fn keypress_listener(sender: ComponentSender<Self>) -> gtk4::EventControllerKey {
    let event_controller = gtk4::EventControllerKey::new();
    event_controller.connect_key_pressed(move |_, key, _, _| {
        let c = match key {
            gdk::Key::Escape => 'X',
            gdk::Key::Return => '\n',
            gdk::Key::a => 'A',
            gdk::Key::b => 'B',
            gdk::Key::c => 'C',
            gdk::Key::d => 'D',
            gdk::Key::e => 'E',
            gdk::Key::f => 'F',
            gdk::Key::g => 'G',
            gdk::Key::h => 'H',
            gdk::Key::_0 => '0',
            gdk::Key::_1 => '1',
            gdk::Key::_2 => '2',
            gdk::Key::_3 => '3',
            gdk::Key::_4 => '4',
            gdk::Key::_5 => '5',
            gdk::Key::_6 => '6',
            gdk::Key::_7 => '7',
            gdk::Key::_8 => '8',
            gdk::Key::_9 => '9',      
            _ => ' ',
        };
        if c.is_alphabetic() { 
           // sender.input(AppMsg::RowSelected(c));
        }
        else {
            //sender.input(AppMsg::ColSelected(c));
        }
        glib::Propagation::Proceed
    });
    event_controller
}
 */

enum Event {
    Clicked,
}

struct App {
    pub button: gtk4::Button,
    pub clicked: u32,
    pub credit: u16,
}

impl App {
    pub fn new(app: &Application, gui_tx: Sender<Event>) -> Self {
        let button = Button::builder()
        .label("Press me!")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

        // Connect to "clicked" signal of `button`
        button.connect_clicked(move |_| {
            let _ = gui_tx.send_blocking(Event::Clicked);
        });


        let window = ApplicationWindow::builder()
            .application(app)
            .title("SnackBot")
            .child(&button)
            .width_request(480)
            .height_request(800)
            .build();

        window.present();

        Self { button, clicked: 0, credit: 0 }
    }
}

 fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(
        |app| {
            //Start up the VMC handler's channels
            let (vmc_response_channel_tx, vmc_response_channel_rx) = async_channel::unbounded::<VmcResponse>();
            let (vmc_command_channel_tx, vmc_command_channel_rx) = async_channel::unbounded::<VmcCommand>();
            //GUI channel
            let (gui_tx, gui_rx) = async_channel::unbounded();
          
            
            let mut app = App::new(&app,gui_tx);
            let b = app.button.clone();
            let gui_event_handler = async move {
                while let Ok(event)= gui_rx.recv().await {
                    let _ = vmc_command_channel_tx.send_blocking(VmcCommand::VendItem(b'a', b'4'));
                }
            };
            //Spawn the GUI event handler 
            glib::MainContext::default().spawn_local(gui_event_handler);

            //Spawn off the VMC task on the tokio runtime
            runtime().spawn(clone!(
                #[strong] 
                vmc_response_channel_tx,
                #[strong]
                vmc_command_channel_rx,
                async move {
                    if let Ok(mut Vmc) = VmcDriver::new() {
                        println!("VMC task connected");
                        //Await a message
                        let mut event_topic = Vmc.driver.subscribe_multi::<EventTopic>(8).await.unwrap();
                        let mut coin_inserted_topic = Vmc.driver.subscribe_multi::<vmc_icd::CoinInsertedTopic>(8).await.unwrap();

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
                                    println!("Got something here");
                                    if let Ok(cmd) = val {
                                        match cmd {
                                            VmcCommand::VendItem(row,col) => {
                                                println!("Asked to vend {}{}",row,col);
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

            //Runs on the Glib event loop
            let b = app.button.clone();
            let vmc_event_handler = async move { 
                while let Ok(event)= vmc_response_channel_rx.recv().await {
                    match event {
                        VmcResponse::CoinInsertedEvent(e) => {
                            //Coin inserted event here
                            app.credit = app.credit + e.value;
                            b.set_label(&app.credit.to_string());
                            println!("Coin");
                        },
                        VmcResponse::CoinAcceptorEvent(e) => {
                            match e {
                                CoinAcceptorEvent::EscrowPressed => {
                                    println!("Escrow pressed");

                                }
                                _=> {
                                    println!("Other event");
                                }
                            }
                        }
                        _=>{
                                
                        },
                    }
                }
            }; 
           glib::MainContext::default().spawn_local(vmc_event_handler);
        }
    );  


/*
    //Spawn the LCD task
    runtime().spawn( clone! (
        #[strong]
        lcd_command_channel_rx,
        async move {
            if let Ok(Lcd) = LcdDriver::new() {
                println!("LCD task connected");
            } 
            else {
                println!("Error - unable to connect to LCD");
            }
            //Await the channel messages, and operate with the VMC driver...
           // let mut sub = vmcclient.driver.subscribe_multi::<EventTopic>(8).await.unwrap();
            //let resp = sub.recv().await;       
        },
    ));
*/
    app.run()
}

