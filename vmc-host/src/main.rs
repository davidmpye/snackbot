
mod lcd_driver;
use lcd_driver::LcdDriver;

mod vmc_driver;
use vmc_driver::VmcDriver;

mod postcard_shim;
use postcard_shim::spawn_postcard_shim;

use vmc_icd::dispenser::{DispenserAddress, Dispenser};
use vmc_icd::EventTopic;
use vmc_icd::coinacceptor::{CoinAcceptorEvent, CoinInserted, CoinRouting,};

const APP_ID: &str = "uk.org.makerspace.snackbot";

const KEYBOARD_DEVICE_NAME:&str = "matrix-keyboard";
const VMC_DEVICE_NAME:&str = "vmc";

use std::sync::OnceLock;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box, Button};
use glib_macros::clone;
use gtk4::glib;

use async_channel::Sender;

#[derive (Copy, Clone)]
pub enum VmcCommand {
    VendItem(u8,u8),
    ForceVendItem(u8,u8),
    GetMachineMap(),                //Get a vec of dispenser
    GetDispenser(u8,u8),            //Get information about a specific dispenser
    SetCoinAcceptorEnabled(bool),   //Whether the coin acceptor should accept coins
    RefundCoins(u16),               //Refund amount
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

fn keypress_listener(sender: Sender<Event>) -> gtk4::EventControllerKey {
    let event_controller = gtk4::EventControllerKey::new();
    event_controller.connect_key_pressed(move |_, key, _, _| {
        let c = match key {
            gdk4::Key::Escape => 'X',
            gdk4::Key::Return => '\n',
            gdk4::Key::a => 'A',
            gdk4::Key::b => 'B',
            gdk4::Key::c => 'C',
            gdk4::Key::d => 'D',
            gdk4::Key::e => 'E',
            gdk4::Key::f => 'F',
            gdk4::Key::g => 'G',
            gdk4::Key::h => 'H',
            gdk4::Key::_0 => '0',
            gdk4::Key::_1 => '1',
            gdk4::Key::_2 => '2',
            gdk4::Key::_3 => '3',
            gdk4::Key::_4 => '4',
            gdk4::Key::_5 => '5',
            gdk4::Key::_6 => '6',
            gdk4::Key::_7 => '7',
            gdk4::Key::_8 => '8',
            gdk4::Key::_9 => '9',      
            _ => ' ',
        };
        if c.is_ascii_alphanumeric() { 
            sender.send_blocking(Event::Keypress((c)));
        }
        //Otherwise ignore.
        glib::Propagation::Proceed
    });
    event_controller
}

enum Event {
    Clicked,
    Keypress(char),
}

struct App {
    pub button: gtk4::Button,
    pub clicked: u32,
    pub credit: u16,
    pub row_selected: Option<char>,
    pub col_selected: Option<char>,
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
        let tx: Sender<Event> = gui_tx.clone();
        button.connect_clicked(move |_| {
            let _ = tx.send_blocking(Event::Clicked);
        });


        let window = ApplicationWindow::builder()
            .application(app)
            .title("SnackBot")
            .child(&button)
            .width_request(480)
            .height_request(800)
            .build();

        window.add_controller(keypress_listener(gui_tx.clone()));

        window.present();

        Self { button, clicked: 0, credit: 0 , row_selected: None, col_selected: None}
    }
}

 fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(
        |app| {
            //Start up the VMC handler's channels
            let (vmc_response_channel_tx, vmc_response_channel_rx) = async_channel::unbounded::<VmcResponse>();
            let (vmc_command_channel_tx, vmc_command_channel_rx) = async_channel::unbounded::<VmcCommand>();
            //Spawn the Postcard-RPC shim, which will run on the Tokio executor
            spawn_postcard_shim(vmc_response_channel_tx, vmc_command_channel_rx);

            //GUI channel
            let (gui_tx, gui_rx) = async_channel::unbounded();
            let mut app = App::new(&app,gui_tx);
            let gui_event_handler = async move {
                while let Ok(event)= gui_rx.recv().await {  
                    match event {
                        Event::Keypress(key) => {
                            println!("Key pressed - {}", key);
                            if key.is_ascii_digit() {
                                app.col_selected = Some(key);
                            }
                            else {
                                app.row_selected = Some(key);
                            }
                        }
                        _ => {
                            println!("Unhandled event");
                        }   
                    }
                    //Process GUI events here.
                }
            };
            //Spawn the GUI event handler onto the main thread
            glib::MainContext::default().spawn_local(gui_event_handler);


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
                                    vmc_command_channel_tx.send(VmcCommand::RefundCoins(app.credit));
                                    //Should wait for confirmation back.
                                    app.credit = 0;
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

