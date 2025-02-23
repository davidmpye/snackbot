mod stock_info;


mod lcd_driver;
use lcd_driver::{LcdCommand, LcdDriver};

mod vmc_driver;
use vmc_driver::{VmcCommand, VmcDriver, VmcResponse};

mod rpc_shim;
use rpc_shim::{spawn_lcd_driver, spawn_vmc_driver};

use vmc_icd::coinacceptor::{CoinAcceptorEvent, CoinInserted, CoinRouting};
use vmc_icd::dispenser::{Dispenser, DispenserAddress};
use vmc_icd::EventTopic;

const APP_ID: &str = "uk.org.makerspace.snackbot";

const KEYBOARD_DEVICE_NAME: &str = "matrix-keyboard";
const VMC_DEVICE_NAME: &str = "vmc";

use glib::clone;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box, Button, Label, Stack, Image};

use std::path::Path;

use async_channel::{Receiver, Sender};

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

        if c.is_ascii_alphanumeric() || c == '\n' || c == '\x1B' {
            match sender.send_blocking(Event::Keypress(c)) {
                Ok(()) => {}
                Err(e) => {
                    println!("Error - unable to send keypress event {}", e);
                }
            }
        }
        glib::Propagation::Proceed
    });
    event_controller
}

enum Event {
    Clicked,
    Keypress(char),
}

enum AppState {
    Idle,
    AwaitingConfirmation,
    AwaitingPayment,
    Vending,
}

struct App {
    pub state: AppState,
    pub credit: u16,
    pub row_selected: Option<char>,
    pub col_selected: Option<char>,
    pub select_item_box: Box,

    pub item_label: Label,
    pub tick_or_cross_label: Label,
    pub stack: Stack,

    pub lcd_channel: Sender<LcdCommand>,
    pub vmc_command_channel: Sender<VmcCommand>,
    pub vmc_response_channel: Receiver<VmcResponse>,
    pub event_channel_tx: Sender<Event>,
    pub event_channel_rx: Receiver<Event>,
}

impl App {
    pub fn new(
        app: &Application,
        event_channel_tx: Sender<Event>,
        event_channel_rx: Receiver<Event>,
        lcd_channel: Sender<LcdCommand>,
        vmc_command_channel: Sender<VmcCommand>,
        vmc_response_channel: Receiver<VmcResponse>
    ) -> Self {
        //All the pages are stored in this widget stack
        let stack = Stack::builder().build();

        let select_item_box = Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .name("select_item_box")
            .build();

        select_item_box.append(
            &Label::builder()
                .use_markup(true)
                .justify(gtk4::Justification::Center)
                .label("<span font=\"Arial Rounded MT 60\">Please\nselect\nan item\n\n</span>")
                .build(),
        );

        let item_label = Label::builder()
            .use_markup(true)
            .justify(gtk4::Justification::Center)
            .label("<span font=\"Arial Rounded MT Bold 80\">_ _\n</span>")
            .build();

        select_item_box.append(&item_label);

        //Starts life hidden
        let tick_or_cross_label = Label::builder()
            .use_markup(true)
            .justify(gtk4::Justification::Center)
            .label("<span font=\"Arial Rounded MT 50\">\n✅ to vend\n❌ to cancel</span>")
            .visible(false)
            .build();

        select_item_box.append(&tick_or_cross_label);
        let _ = stack.add_named(&select_item_box,Some("select_item_box"));
        
        let confirm_item_box  = Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .build();


        let file = Path::new("./scampi.jpg");
        let e = Image::new();
        e.set_from_file(Some(file));
        e.set_size_request(200,300);
        
        confirm_item_box.append(&e);

        let _ = stack.add_named(&confirm_item_box, Some("confirm_item_box"));

        let window = ApplicationWindow::builder()
            .application(app)
            .title("SnackBot")
            .child(&stack)
            .width_request(480)
            .height_request(800)
            .build();

        window.add_controller(keypress_listener(event_channel_tx.clone()));

        window.present();

        Self {
            state: AppState::Idle,
            credit: 0,
            row_selected: None,
            col_selected: None,
            select_item_box,
            stack,
            item_label,
            tick_or_cross_label,
            lcd_channel,
            vmc_command_channel,
            vmc_response_channel,
            event_channel_rx,
            event_channel_tx,
        }
    }

    pub fn handle_event(&mut self, event: Event) {
        match self.state {
            AppState::Idle => {
                //In idle, we are waiting for key press events to select an item
                match event {
                    Event::Keypress(key) => {
                        if key.is_alphabetic() {
                            //Row selected
                            if self.row_selected.is_some() {
                                self.col_selected = None;
                            }
                            self.row_selected = Some(key);
                        } else if key.is_ascii_digit() {
                            //Col selected
                            if self.col_selected.is_some() {
                                self.row_selected = None;
                            }
                            self.col_selected = Some(key);
                        } else if key == '\n' {
                            //Green tick!
                            //Move into Confirm state if row and column selected, else ignore
                            if self.row_selected.is_some() && self.col_selected.is_some() {
                                self.state = AppState::Vending;
                            }
                        } else if key == '\x1B' {
                            //Red X - clear selection
                            self.row_selected = None;
                            self.col_selected = None;
                        }
                    }
                    //Only keypress events accepted in idle state
                    _ => {
                        println!("Unexpected event in idle state")
                    }
                }
            }
            _ => {}
        }
        self.update_ui();
    }

    async fn main_loop(&mut self) {
        loop {
            if let Ok(event) = self.event_channel_rx.recv().await {
                self.handle_event(event);
            }
        }
    }

    fn update_ui(&mut self) {
        //Display appropriate state
        match self.state {
            AppState::Idle => {
                println!("INIDLE");
                //In this state, we should be showing the select item widgetstack 'page'
                self.stack.set_visible_child(&self.stack.child_by_name("select_item_box").unwrap());
                //get it to update its' info
                //self.item_label.set_label("");
            },
            AppState::Vending => {
                println!("INVEND");
                self.stack.set_visible_child(&self.stack.child_by_name("confirm_item_box").unwrap());
            }
            _=>{},
        }
    }
}

fn main() -> glib::ExitCode {
    //Create VMC command and response channels
    let (vmc_response_channel_tx, vmc_response_channel_rx) =
        async_channel::unbounded::<VmcResponse>();
    let (vmc_command_channel_tx, vmc_command_channel_rx) = async_channel::unbounded::<VmcCommand>();
    //Spawn the VMC driver with two-way channels
    spawn_vmc_driver(
        vmc_response_channel_tx.clone(),
        vmc_command_channel_rx.clone(),
    );

    //Lcd command channel
    let (lcd_command_channel_tx, lcd_command_channel_rx) = async_channel::unbounded::<LcdCommand>();
    //Spawn LCD driver with its' one-way command channel
    spawn_lcd_driver(lcd_command_channel_rx);

    let (event_channel_tx, event_channel_rx) = async_channel::unbounded::<Event>();

    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(move |app| {
        let mut app = App::new(
            &app,
            event_channel_tx.clone(),
            event_channel_rx.clone(),
        lcd_command_channel_tx.clone(),
            vmc_command_channel_tx.clone(),
            vmc_response_channel_rx.clone(),
        );
        
        //Spawn the main loop onto the GLib event loop
        glib::MainContext::default().spawn_local(
            async move {
                app.main_loop().await;
            }
        );
    });

    app.run()
}
