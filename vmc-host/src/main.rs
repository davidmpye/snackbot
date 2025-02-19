
mod lcd_driver;
use lcd_driver::{LcdDriver, LcdCommand};

mod vmc_driver;
use vmc_driver::VmcDriver;

mod postcard_shim;
use postcard_shim::{spawn_vmc_driver, spawn_lcd_driver};

use vmc_icd::dispenser::{DispenserAddress, Dispenser};
use vmc_icd::EventTopic;
use vmc_icd::coinacceptor::{CoinAcceptorEvent, CoinInserted, CoinRouting,};

const APP_ID: &str = "uk.org.makerspace.snackbot";

const KEYBOARD_DEVICE_NAME:&str = "matrix-keyboard";
const VMC_DEVICE_NAME:&str = "vmc";

use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box, Button, Stack, Label};
use gtk4::glib;
use glib::clone;

use async_channel::Sender;


#[derive (Copy, Clone)]
pub enum VmcCommand {
    VendItem(char,char),
    ForceVendItem(char, char),
    GetMachineMap(),                //Get a vec of dispenser
    GetDispenser(char,char),            //Get information about a specific dispenser
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

fn keypress_listener(sender: Sender<Event>) -> gtk4::EventControllerKey {
    let event_controller = gtk4::EventControllerKey::new();
    event_controller.connect_key_pressed(move |_, key, _, _| {
        let c = match key {
            gdk4::Key::Escape => '\x1B',
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
        if c.is_ascii_alphanumeric() || c == '\n' || c == '\x1B'  { 
            match sender.send_blocking(Event::Keypress(c)) {
                Ok(()) => {},
                Err(e) => {
                    println!("Error - unable to send keypress event {}", e);
                },
            }
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
    pub credit: u16,
    pub row_selected: Option<char>,
    pub col_selected: Option<char>,
    pub select_item_box: Box,

    pub item_label: Label,
    pub tick_or_cross_label: Label,
    pub stack: Stack,
}

impl App {
    pub fn new(app: &Application, gui_tx: Sender<Event>) -> Self {

        let select_item_box = Box::builder().orientation(gtk4::Orientation::Vertical).name("select_item_box").build();
      
        select_item_box.append(
            &Label::builder().use_markup(true).justify(gtk4::Justification::Center)
            .label("<span font=\"Arial Rounded MT 60\">Please\nselect\nan item\n\n</span>").build()
        );

        let item_label = Label::builder().use_markup(true).justify(gtk4::Justification::Center)
            .label("<span font=\"Arial Rounded MT Bold 80\">_ _\n</span>").build();

        select_item_box.append(&item_label);
    
        //Starts life hidden
        let tick_or_cross_label = Label::builder().use_markup(true).justify(gtk4::Justification::Center)
        .label("<span font=\"Arial Rounded MT 50\">\n✅ to vend\n❌ to cancel</span>").visible(false).build();
        
        select_item_box.append(&tick_or_cross_label);

        let stack = Stack::builder().build();
        let _ = stack.add_child(&select_item_box);

        let window = ApplicationWindow::builder()
            .application(app)
            .title("SnackBot")
            .child(&stack)
            .width_request(480)
            .height_request(800)
            .build();

        window.add_controller(keypress_listener(gui_tx.clone()));

        window.present();

        Self { credit: 0 , row_selected: None, col_selected: None, select_item_box, stack, item_label, tick_or_cross_label}
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
            spawn_vmc_driver(vmc_response_channel_tx.clone(), vmc_command_channel_rx.clone());
    
            //Spawn the LCD driver channel so we can post messages to screen
            let (lcd_command_channel_tx, lcd_command_channel_rx) = async_channel::unbounded::<LcdCommand>();
            spawn_lcd_driver(lcd_command_channel_rx);

            //GUI channel
            let (gui_tx, gui_rx) = async_channel::unbounded();
            let mut app = App::new(&app,gui_tx);
            let gui_event_handler = clone! (
                #[strong]
                vmc_command_channel_tx,
                #[strong]
                vmc_response_channel_rx,
                #[strong]
                lcd_command_channel_tx,
                async move {

                //Welcome message
                let _ = lcd_command_channel_tx.send_blocking(LcdCommand::SetText(String::from("Snackbot (C)"), String::from("I sell snacks")));

                while let Ok(event)= gui_rx.recv().await {  
                    match event {
                        Event::Keypress(key) => {
                            if key.is_ascii_digit() {
                                if app.col_selected.is_some() {
                                    app.row_selected = None;
                                }
                                app.col_selected = Some(key);
                            }
                            else if key.is_alphabetic() {
                                if app.row_selected.is_some() {
                                    app.col_selected = None;
                                }
                                app.row_selected = Some(key);
                            }
                            else  if key == '\n' { //Enter
                                if app.row_selected.is_some() && app.col_selected.is_some() {
                                    println!("Sending vend command");
                                    let _ = vmc_command_channel_tx.send_blocking(VmcCommand::VendItem(app.row_selected.unwrap(), app.col_selected.unwrap()));
                                }
                            }
                            else if key == '\x1B' { //Esc
                                app.row_selected = None;
                                app.col_selected = None;
                            }

                            let col_char = match app.col_selected {
                                Some(x) => x,
                                _=> ' ',
                            };

                            let row_char = match app.row_selected {
                                Some(x) => x,
                                _=> ' ',
                            };
                            app.item_label.set_label(format!("<span font=\"Arial Rounded MT Bold 80\">{}{}\n</span>", row_char, col_char).as_str());
                            
                            if app.row_selected.is_some() && app.col_selected.is_some() {
                                app.tick_or_cross_label.set_visible(true);
                            }
                            else {
                                app.tick_or_cross_label.set_visible(false);
                            }
                        }
                        _ => {
                            println!("Unhandled event");
                        }   
                    }
                }
            });
            //Spawn the GUI event handler onto the main thread
            glib::MainContext::default().spawn_local(gui_event_handler);

            let vmc_event_handler = async move { 
                while let Ok(event)= vmc_response_channel_rx.recv().await {
                    match event {
                        VmcResponse::CoinInsertedEvent(e) => {
                            app.credit = app.credit + e.value;
                            println!("Coin {}", e.value);
                        },
                        VmcResponse::CoinAcceptorEvent(e) => {
                            match e {
                                CoinAcceptorEvent::EscrowPressed => {
                                    println!("Escrow pressed");
                                    let _ = vmc_command_channel_tx.send(VmcCommand::RefundCoins(app.credit)).await;
                                    //Should wait for confirmation back.
                                    app.credit = 0;
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

