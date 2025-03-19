mod stock_info;
use crate::stock_info::get_stock_item;

mod make_selection_box;
use crate::make_selection_box::MakeSelectionBox;
mod confirm_item_box;
use crate::confirm_item_box::ConfirmItemBox;
mod make_payment_box;
use crate::make_payment_box::MakePaymentBox;

mod lcd_driver;
use gtk4::builders::ImageBuilder;
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

const IDLE_MESSAGE_L1: &str = "Plz buy m0ar";
const IDLE_MESSAGE_L2: &str = "snackz kthx";

const PAY_MESSAGE_L1: &str = "Please pay:";

use glib::clone;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box, Button, Image, Label, Stack};

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

//These are events the main loop should respond to
enum Event {
    Keypress(char),
    EscrowPressed,
    CoinInserted(u16),
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
    pub amount_due: u16,
    pub row_selected: Option<char>,
    pub col_selected: Option<char>,

    pub stack: Stack,
    pub make_selection_box: MakeSelectionBox,
    pub confirm_item_box: ConfirmItemBox,
    pub make_payment_box: MakePaymentBox,

    pub lcd_channel: Sender<LcdCommand>,
    pub vmc_command_channel: Sender<VmcCommand>,
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
    ) -> Self {
        //All the pages are stored in this widget stack
        let stack = Stack::builder().build();

        let make_selection_box = MakeSelectionBox::new();
        stack.add_named(&make_selection_box, Some("make_selection_box"));

        let confirm_item_box = ConfirmItemBox::new();
        stack.add_named(&confirm_item_box, Some("confirm_item_box"));

        let make_payment_box = MakePaymentBox::new();
        stack.add_named(&make_payment_box, Some("make_payment_box"));

        //  let _ = stack.add_named(&confirm_item_box, Some("confirm_item_box"));

        let window = ApplicationWindow::builder()
            .application(app)
            .title("SnackBot")
            .child(&stack)
            .width_request(480)
            .height_request(800)
            .build();

        window.add_controller(keypress_listener(event_channel_tx.clone()));

        window.present();


        lcd_channel.send_blocking(LcdCommand::SetText(String::from(IDLE_MESSAGE_L1), String::from(IDLE_MESSAGE_L2)));

        Self {
            state: AppState::Idle,
            credit: 0,
            amount_due: 0,
            row_selected: None,
            col_selected: None,
            stack,

            make_selection_box,
            confirm_item_box,
            make_payment_box,

            lcd_channel,
            vmc_command_channel,
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
                        } else if key == '\x1b' {
                            //Red X - clear selection
                            self.row_selected = None;
                            self.col_selected = None;
                        }

                        //If now a row and col are selected, go to confirmation screen
                        if self.row_selected.is_some() && self.col_selected.is_some() {
                            self.state = AppState::AwaitingConfirmation;
                        }
                    }
                    //Only keypress events accepted in idle state
                    _ => {
                        println!("Unexpected event in idle state")
                    }
                }
            }
            AppState::AwaitingConfirmation => {
                match event {
                    Event::Keypress(key) => {
                        match key {
                            '\n' => {
                                //Into payment sate
                                self.state = AppState::AwaitingPayment;

                                //Find the item and set the balance
                                match get_stock_item(DispenserAddress {
                                    row: self.row_selected.unwrap(),
                                    col: self.col_selected.unwrap(),
                                }) {
                                    Some(item) => {
                                        self.amount_due = item.price;
                                        let balance_due = self.amount_due - self.credit;
                                        self.lcd_channel.send_blocking(LcdCommand::SetText(String::from(PAY_MESSAGE_L1), 
                                            format!("£{pound}.{pence}", pound = balance_due/100, pence = balance_due%100)));
                                    }
                                    None => {
                                        println!("Error - item no longer found - shouldnt happen!");
                                    }
                                }
                                //Enable coin acceptor
                                let _ = self.vmc_command_channel.send_blocking(VmcCommand::SetCoinAcceptorEnabled(true));

                            }
                            '\x1b' => {
                                //Cancel
                                self.row_selected = None;
                                self.col_selected = None;
                                self.state = AppState::Idle;
                                let _ = self.vmc_command_channel.send_blocking(VmcCommand::SetCoinAcceptorEnabled(false));
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            AppState::AwaitingPayment => {
                match event {
                    Event::Keypress(key) => {
                        match key {
                            '\x1b' => {
                                //Cancel
                                self.row_selected = None;
                                self.col_selected = None;
                                self.state = AppState::Idle;
                                //Disable coin acceptor
                                let _ = self.vmc_command_channel.send_blocking(VmcCommand::SetCoinAcceptorEnabled(false));
                            },
                            '\n' => {
                                let _ = self.vmc_command_channel.send_blocking(VmcCommand::VendItem(self.row_selected.unwrap(), self.col_selected.unwrap()));
                            },
                            _=> {},
                        }
                    },
                    Event::EscrowPressed => {
                        println!("Got escrow");
                        //Also acts as cancel.
                         //Cancel
                         self.row_selected = None;
                         self.col_selected = None;
                         self.state = AppState::Idle;
                         self.amount_due = 0;
                         //Disable coin acceptor
                         let _ = self.vmc_command_channel.send_blocking(VmcCommand::SetCoinAcceptorEnabled(false));
                         //Need to refund coins if any inserted
                    },
                    Event::CoinInserted(value) => {
                        //Update the credit
                        self.credit += value;
                        println!("Got paid {}, balance {}", value, self.credit);
                    }
                    _ => {
                        println!("Other event - not handled");
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
                //In this state, we should be showing the select item widgetstack 'page'
                self.stack.set_visible_child(
                    &self
                        .stack
                        .child_by_name("make_selection_box")
                        .expect("Error: Make selection box is missing from stack"),
                );
                let row_char = {
                    if self.row_selected.is_none() {
                        '_'
                    } else {
                        self.row_selected.unwrap()
                    }
                };
                let col_char = {
                    if self.col_selected.is_none() {
                        '_'
                    } else {
                        self.col_selected.unwrap()
                    }
                };
                //Display idle message
                self.lcd_channel.send_blocking(LcdCommand::SetText(String::from(IDLE_MESSAGE_L1), String::from(IDLE_MESSAGE_L2)));
            }
            AppState::AwaitingConfirmation => {
                match get_stock_item(DispenserAddress {
                    row: self.row_selected.unwrap(),
                    col: self.col_selected.unwrap(),
                }) {
                    Some(item) => {
                        self.confirm_item_box.set_name(item.name);
                        self.confirm_item_box.set_image(item.image_url);
                        self.confirm_item_box.set_price(item.price);
                        self.stack.set_visible_child(
                            &self
                                .stack
                                .child_by_name("confirm_item_box")
                                .expect("Error: Confirm item box is missing from stack"),
                        );
                    }
                    None => {
                        //Invalid, should say so.
                        self.row_selected = None;
                        self.col_selected = None;
                        self.state = AppState::Idle;
                        self.update_ui();
                    }
                }
            }
            AppState::AwaitingPayment => {
                self.stack.set_visible_child(
                    &self
                        .stack
                        .child_by_name("make_payment_box")
                        .expect("Error: Make payment box is missing from stack!"),
                );
            }
            _ => {}
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
        );

        //Spawn the main loop onto the GLib event loop
        glib::MainContext::default().spawn_local(async move {
            app.main_loop().await;
        });
        
        let rx = vmc_response_channel_rx.clone();
        let tx = event_channel_tx.clone();
        glib::MainContext::default().spawn_local( async move {
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        match event {
                            VmcResponse::CoinInsertedEvent(coin) => {
                                tx.send(Event::CoinInserted(coin.value)).await;
                            },
                            VmcResponse::CoinAcceptorEvent(CoinAcceptorEvent::EscrowPressed) => {
                                tx.send(Event::EscrowPressed).await;
                            }
                            _ => {
                                println!("Ignored an event");
                            },
                        }
                    },
                    Err(e) => {
                    
                    },
                }
            }

        });
        //Spawn the VMC listener loop onto the main glib event loop
        //glib::MainC

    });

    app.run()
}
