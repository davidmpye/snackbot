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

fn build_ui(application: &gtk4::Application) {
    let window = gtk4::ApplicationWindow::new(application);

    window.set_title(Some("Snackbot"));
    window.set_default_size(480, 800);
    
    let event_controller = keyboard_listener();
    window.add_controller(event_controller);

    let vbox  = gtk4::Box::builder().orientation(gtk4::Orientation::Vertical).build();

    let button = gtk4::Button::with_label("Press test to vend");

    let (sender, receiver) = async_channel::bounded(1);

    button.connect_clicked(move |_| {
        runtime().spawn(clone!(
            #[strong]
            sender,
            async move {
                let c = LcdDriver::new(KEYBOARD_DEVICE_NAME);
                match c {
                    Ok(mut d) => {
                        let _  = d.set_text("TEST", "OK").await;
                    },
                    Err(msg) => {println!("LCD comms err: {}", msg);}
                }
               
               sender
               .send(Some(true))
               .await
               .expect("Channel closed");
            }
        ));
    });

    let label1 = gtk4::Label::new(None);
    label1.set_markup("<span font=\"36\">Please select</span>");
    
    let label2 = gtk4::Label::new(None);
    label2.set_markup("<span font=\"36\">an item</span>");


    glib::spawn_future_local(async move {
        while let Ok(response) = receiver.recv().await {

        }
    });

    vbox.append(&label1);
    vbox.append(&label2);
    vbox.append(&button);
    
    window.set_child(Some(&vbox));

    window.present();
}

fn keyboard_listener() -> gtk4::EventControllerKey {
    let event_controller = gtk4::EventControllerKey::new();
    event_controller.connect_key_pressed(|_, key, _, _| {
        match key {
            gdk::Key::Escape => {
                println!("CANCEL");
            },
            gdk::Key::Return => {
                println!("VEND");
            }
            gdk::Key::a => {
                println!("a");
            },
            gdk::Key::b => {
                println!("b");
            },
            gdk::Key::c => {
                println!("c");
            },
            gdk::Key::d => {
                println!("d");
            },
            gdk::Key::e => {
                println!("e");
            },
            gdk::Key::f => {
                println!("f");
            },
            gdk::Key::g => {
                println!("g");
            },
            gdk::Key::_0 => {
                println!("0");
            },
            gdk::Key::_1 => {
                println!("1");
            },
            gdk::Key::_2 => {
                println!("2");
            },
            gdk::Key::_3 => {
                println!("3");
            },
            gdk::Key::_4 => {
                println!("4");
            },
            gdk::Key::_5 => {
                println!("5");
            },
            gdk::Key::_6 => {
                println!("6");
            },
            gdk::Key::_7 => {
                println!("7");
            },
            gdk::Key::_8 => {
                println!("8");
            },
            gdk::Key::_9 => {
                println!("9");
            },
            _ => (),
        }
        glib::Propagation::Proceed
    });
    event_controller
}