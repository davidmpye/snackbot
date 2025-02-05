mod lcd_driver;
use lcd_driver::LcdDriver;
mod vmc_driver;
use vmc_driver::VmcDriver;

use vmc_icd::dispenser::{DispenserAddress, Dispenser};

const APP_ID: &str = "uk.org.makerspace.snackbot";

const KEYBOARD_DEVICE_NAME:&str = "matrix-keyboard";
const VMC_DEVICE_NAME:&str = "vmc";

use tokio::runtime::Runtime;  //We use the Tokio runtime to run the postcard-apc async functions

use std::sync::OnceLock;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box, Button};
use glib_macros::clone;
use gtk4::glib;

use vmc_icd::EventTopic;
use vmc_icd::coinacceptor::{CoinAcceptorEvent, CoinInserted, CoinRouting};

//Spawn a tokio runtime instance for the postcard-rpc device handlers
fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        Runtime::new().expect("Failed to spawn tokio runtime")
    })
}

pub enum VmcCommand {
    VendItem(u8,u8),
    ForceVendItem(u8,u8),
    GetMachineMap(),
    GetDispenser(u8,u8),
}

pub enum VmcResponse {
    MachineMap(Vec<Dispenser>),
    Dispenser(Dispenser),

    //Vend result for a vend request
    CoinAcceptorEvent(u8),
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
fn build_ui(app: &Application) {
    // Create a button with label and margins
    let button = Button::builder()
        .label("Press me!")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    // Create a window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Snackbot")
        .child(&button)
        .width_request(480)
        .height_request(800)
    
        .build();

    // Present window
    window.present();
}

 fn main() -> glib::ExitCode {
    // Create a new application
    let app = Application::builder().application_id(APP_ID).build();

    // Connect to "activate" signal of `app`
    app.connect_activate(build_ui);
/*
    //Start up the VMC handler's channels
    let (vmc_response_channel_tx, vmc_response_channel_rx) = async_channel::bounded::<VmcResponse>(1);
    let (vmc_command_channel_tx, vmc_command_channel_rx) = async_channel::bounded::<VmcCommand>(1);

    //Set up the keyboard 'screen' driver channels
    let (lcd_command_channel_tx, lcd_command_channel_rx, ) = async_channel::bounded::<LcdCommand>(1);
    

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


    runtime().spawn( clone! (
        #[strong]
        vmc_command_channel_rx,
        #[strong]
        vmc_response_channel_tx,
        async move {
            if let Ok(Vmc) = VmcDriver::new() {
                println!("VMC task connected");
                //Await a message
                if let Ok(message) = Vmc.driver.subscribe_multi::<EventTopic>(8).await {

                }
                else {

                }
            } 
            else {
                println!("Error - unable to connect to VMC Driver");
            }
            //Await the channel messages, and operate with the VMC driver...
            vmc_response_channel_tx.send(VmcResponse::CoinAcceptorEvent(0xFFu8)).await;
            //let resp = sub.recv().await;       
        },
    ));





    glib::spawn_future_local(async move {
        while let Ok(result) = vmc_response_channel_rx.recv().await {
            match result {
                VmcResponse::CoinAcceptorEvent(x)=> {
                    println!("Got X = {}", x);
                }
                _=> {},
            }


        }
    });
 */
    //Start the keyboard handler
    // Run the application
    app.run()
}

