    mod lcd_driver;
use gtk4::glib::value;
use lcd_driver::LcdDriver;

mod vmc_driver;
use vmc_driver::VmcDriver;
use vmc_icd::DispenserAddress;

const KEYBOARD_DEVICE_NAME:&str = "matrix-keyboard";
const VMC_DEVICE_NAME:&str = "vmc";

use gtk4::{glib, prelude::*};
use gtk4::gdk;
use crate::glib::clone;
use tokio::runtime::Runtime;  //We use the Tokio runtime to run the postcard-apc async functions
use std::sync::OnceLock;

use std::sync::{Arc,Mutex};

use async_channel::{Sender, Receiver};

//Spawn a tokio runtime instance for the postcard-rpc device handlers
fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        Runtime::new().expect("Failed to spawn tokio runtime")
    })
}
fn main() -> glib::ExitCode {
    let application = gtk4::Application::builder()
        .application_id("uk.org.makerspace.snackbot")
        .build();

    application.connect_activate(build_ui);
    application.run()
}

async fn handle_vend(key_receiver: Receiver<char>) {
    let mut row:Option<char> = None;
    let mut col: Option<char> = None;

    while let Ok(response) = key_receiver.recv().await {
        if response.is_ascii_digit() && response >= '0' && response <= '9' {
            if col.is_some() {
                row = None;
            }
            col = Some(response);
        }
        else if response.is_ascii_alphabetic() && response >= 'A' && response <= 'H' {
            if row.is_some() {
                col = None;
            }
            row = Some(response); 
        }
        else {
            println!("Error - unexpected ascii character {}", response);
        }

        if row.is_some() && col.is_some() {
            //Valid rows and columns found
            let r = row.unwrap();
            let c = col.unwrap();
            //Got valid row and column, time to vend!
            let (sender, receiver) = async_channel::bounded(1);
            //Spawn a Postcard-RPC vend request on the Tokio runtime
            runtime().spawn(clone!(
                async move {
                    match VmcDriver::new(VMC_DEVICE_NAME) {
                        Ok(mut driver) => {
                            let result = driver.dispense(DispenserAddress{row: r, col: c}).await;
                            sender.send(result).await.expect("Vend channel failure");
                        },
                        Err(msg) => {
                            println!("VMC init failure: {}", msg);
                            //need to send something down the channel..
                            sender.send(Err(vmc_driver::VmcClientError::Comms(postcard_rpc::host_client::HostErr::BadResponse))).await.expect("Vend channel failure");
                        },
                    }
                }
            ));

            //This runs on the local event loop, and receives the result from the vend postcard RPC command running on tokio
            glib::spawn_future_local(async move {
                while let Ok(response) = receiver.recv().await {
                    if let Ok(()) = response {
                        println!("Vend success");
                    } else {
                        println!("Vend failed");
                    }
                }
            });
        }
    }
}

fn build_ui(application: &gtk4::Application) {
    let window = gtk4::ApplicationWindow::new(application);
    window.set_title(Some("Snackbot"));
    //Set to the resolution of the Pimoroni screen
    window.set_default_size(480, 800);

    let vbox  = gtk4::Box::builder().orientation(gtk4::Orientation::Vertical).build();

    let button = gtk4::Button::with_label("Press test to vend");

    let address_label =  gtk4::Label::new(None);
    address_label.set_markup("<span font=\"36\">Please select</span>");
    

    //This async channel is for the handle_vend function to receive key presses via the keyboard listener
    let (key_sender, key_receiver) = async_channel::bounded::<char>(1);
    //The keyboard listener will listen for A-G, 0-9, tick (enter), cross (esc), and UP/DOWN arrows
    //from the membrane keypad on the vending machine (appears as USB keyboard)
    window.add_controller(keyboard_listener(key_sender));
    //Spawn off the handle vend future, with a channel to receive key presses
    glib::spawn_future_local(async move { handle_vend(key_receiver).await; });

    vbox.append(&address_label);
    vbox.append(&button);
        
    window.set_child(Some(&vbox));
    window.present();
}

fn keyboard_listener(sender: Sender<char>) -> gtk4::EventControllerKey {
    let event_controller = gtk4::EventControllerKey::new();
    event_controller.connect_key_pressed(move |_, key, _, _| {
        let c = match key {
            gdk::Key::Escape => 'X',
            gdk::Key::Return => 'Y',
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
        let _ = sender.send_blocking(c);
        glib::Propagation::Proceed
    });
    event_controller
}