mod stock_info;
use crate::stock_info::get_stock_item;

mod make_selection_box;
use crate::make_selection_box::MakeSelectionBox;
mod confirm_item_box;
use crate::confirm_item_box::ConfirmItemBox;
mod make_payment_box;
use crate::make_payment_box::MakePaymentBox;

mod make_another_selection_box;
use crate::make_another_selection_box::MakeAnotherSelectionBox;

mod vend_in_progress_box;
use vend_in_progress_box::VendInProgressBox;
mod vend_ok_box;
use vend_ok_box::VendOkBox;
mod vend_failed_box;
use vend_failed_box::VendFailedBox;

mod lcd_driver;
use gtk4::builders::ImageBuilder;
use lcd_driver::{LcdCommand, LcdDriver};

mod vmc_driver;
use vmc_driver::{VmcCommand, VmcDriver, VmcResponse};

mod rpc_shim;
use rpc_shim::{spawn_lcd_driver, spawn_vmc_driver};

use vmc_icd::coin_acceptor::{CoinAcceptorEvent, CoinInserted, CoinRouting};
use vmc_icd::dispenser::{Dispenser, DispenserAddress};
use vmc_icd::EventTopic;
use vmc_icd::cashless_device::{CashlessDeviceCommand, CashlessDeviceEvent};

const APP_ID: &str = "uk.org.makerspace.snackbot";

const KEYBOARD_DEVICE_NAME: &str = "matrix-keyboard";
const VMC_DEVICE_NAME: &str = "vmc";

const IDLE_MESSAGE_L1: &str = "Plz buy m0ar";
const IDLE_MESSAGE_L2: &str = "snackz kthx";

const PAY_MESSAGE_L1: &str = "Please pay:";

const APP_TIMEOUT_SECONDS: u16 = 30;

use glib::ControlFlow::Continue;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box, Button, Image, Label, Stack};

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
    Timeout_Poll_Event,
    ChangeState(AppState),
    CashlessEvent(CashlessDeviceEvent),
    VendSuccess,
    VendFailed,
}

enum AppState {
    Idle,
    MakeAnotherSelection,
    AwaitingConfirmation,
    AwaitingPayment,
    Vending,
    VendSuccess,
    VendFailed,
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
    pub make_another_selection_box: MakeAnotherSelectionBox,

    pub lcd_channel: Sender<LcdCommand>,
    pub vmc_command_channel: Sender<VmcCommand>,
    pub event_channel_tx: Sender<Event>,
    pub event_channel_rx: Receiver<Event>,

    pub seconds_since_last_event: u16,
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
        let confirm_item_box = ConfirmItemBox::new();
        let make_payment_box = MakePaymentBox::new();
        let make_another_selection_box = MakeAnotherSelectionBox::new();
        let vend_in_progress_box = VendInProgressBox::new();
        let vend_ok_box = VendOkBox::new();
        let vend_failed_box = VendFailedBox::new();

        stack.add_named(&make_selection_box, Some("make_selection_box"));
        stack.add_named(&confirm_item_box, Some("confirm_item_box"));
        stack.add_named(&make_payment_box, Some("make_payment_box"));
        stack.add_named(&make_another_selection_box, Some("make_another_selection_box"));
        stack.add_named(&vend_in_progress_box, Some("vend_in_progress_box"));
        stack.add_named(&vend_ok_box, Some("vend_ok_box"));
        stack.add_named(&vend_failed_box, Some("vend_failed_box"));

        let window = ApplicationWindow::builder()
            .application(app)
            .title("SnackBot")
            .child(&stack)
            .width_request(480)
            .height_request(800)
            .build();

        window.add_controller(keypress_listener(event_channel_tx.clone()));

        window.present();

        let _ = lcd_channel.send_blocking(LcdCommand::SetText(String::from(IDLE_MESSAGE_L1), String::from(IDLE_MESSAGE_L2)));

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
            make_another_selection_box,

            lcd_channel,
            vmc_command_channel,
            event_channel_rx,
            event_channel_tx,

            seconds_since_last_event: 0,
        }
    }

    pub fn handle_event(&mut self, event: Event) {
        //Handle timeout events separately from main state machine
        match event {
            Event::Timeout_Poll_Event => {
                if !matches!(self.state, AppState::Idle) {
                    if self.seconds_since_last_event == APP_TIMEOUT_SECONDS {
                        println!("Timeout - return to idle state");
                        self.state = AppState::Idle;
                        self.seconds_since_last_event = 0;
                        self.update_ui();
                    }
                    else {
                        self.seconds_since_last_event += 1;
                    }
                }
                return;
            }
            Event::ChangeState(state) => {
                self.state = state;
                self.update_ui();
                return;
            }
            _ => {
                //Another event occurred - reset timer
                self.seconds_since_last_event = 0;
            }
        }
        //Handle other events
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
                                    }
                                    None => {
                                        println!("Error - item no longer found - shouldnt happen!");
                                    }
                                }
                                //Enable coin acceptor
                                //let _ = self.vmc_command_channel.send_blocking(VmcCommand::SetCoinAcceptorEnabled(true));

                                //Set amount for card reader
                                let _ = self.vmc_command_channel.send_blocking(VmcCommand::CashlessCmd(
                                    vmc_icd::cashless_device::CashlessDeviceCommand::StartTransaction(
                                        self.amount_due, 
                                        DispenserAddress {
                                            row: self.row_selected.unwrap(),
                                            col : self.col_selected.unwrap()
                                        }
                                    )
                                ));
                            }
                            '\x1b' => {
                                //Cancel
                                self.row_selected = None;
                                self.col_selected = None;
                                self.state = AppState::Idle;                   
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

                                //Cancel card reader transaction
                                let _ = self.vmc_command_channel.send_blocking(VmcCommand::CashlessCmd(
                                    vmc_icd::cashless_device::CashlessDeviceCommand::CancelTransaction));            
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
                         //Cancel the cashless transaction
                         let _ = self.vmc_command_channel.send_blocking(VmcCommand::CashlessCmd(CashlessDeviceCommand::CancelTransaction));
                         //Need to refund coins if any inserted
                    },/*
                    Event::CoinInserted(value) => {
                        //Update the credit
                        self.credit += value;
                        if self.credit >= self.amount_due {
                            //Move to vend
                            self.state = AppState::Vending;
                            let _ = self.vmc_command_channel.send_blocking(VmcCommand::VendItem(self.row_selected.unwrap(), self.col_selected.unwrap()));
                        }
                    }*/
                    Event::CashlessEvent(e) => {
                        match e {
                            CashlessDeviceEvent::VendApproved(amount) => {
                                println!("Vend approved for amount: {}",amount);
                                if amount == self.amount_due {
                                    let _ = self.vmc_command_channel.send_blocking(VmcCommand::VendItem(self.row_selected.unwrap(), self.col_selected.unwrap()));
                                    self.state = AppState::Vending;
                                }
                                else {
                                    //This shouldn't happen - cancel.
                                    println!("Cashless device approved for wrong amount");
                                    let _ = self.vmc_command_channel.send_blocking(VmcCommand::CashlessCmd(CashlessDeviceCommand::CancelTransaction));
                                }
                            }
                            _ => {
                                println!("Other event");
                            },
                        }
                    },

                    //Card payment event here...-
                    _ => {
                        println!("Other event - not handled");
                    }
                }
            }
            AppState::Vending => {
                //Only two events acceptable here - success or failed.
                match event {
                    Event::VendSuccess => {
                        //Send massage to cashless device to confirm vend successful, to end transaction
                        let _ = self.vmc_command_channel.send_blocking(VmcCommand::CashlessCmd(CashlessDeviceCommand::VendSuccess(DispenserAddress { row: self.row_selected.unwrap(), col: self.col_selected.unwrap() })));
                        self.state = AppState::VendSuccess;              
                    },
                    Event::VendFailed => {
                        //Cancel the cashless device transaction with vend failed (not sure what it will say if it didnt handle the transaction)
                        let _ = self.vmc_command_channel.send_blocking(VmcCommand::CashlessCmd(CashlessDeviceCommand::VendFailed));
                        self.state = AppState::VendFailed;                   
                    },
                    //Fixme - need a timeout if vmc has gone wrong
                    _ => {},
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
                
                let _ = self.lcd_channel.send_blocking(LcdCommand::SetText(String::from(IDLE_MESSAGE_L1),
                    String::from(IDLE_MESSAGE_L2)));

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
                let _ = self.lcd_channel.send_blocking(LcdCommand::SetText(String::from(IDLE_MESSAGE_L1), String::from(IDLE_MESSAGE_L2)));
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
                        self.state = AppState::MakeAnotherSelection;
                        self.update_ui();
                    }
                }
            }
            AppState::AwaitingPayment => {

                let balance_due = self.amount_due - self.credit;
                
                let _ = self.lcd_channel.send_blocking(LcdCommand::SetText(String::from(PAY_MESSAGE_L1), 
                format!("{}.{:02}", balance_due/100, balance_due%100)));

                self.stack.set_visible_child(
                    &self
                        .stack
                        .child_by_name("make_payment_box")
                        .expect("Error: Make payment box is missing from stack!"),
                );
            }
            AppState::MakeAnotherSelection => {
                self.stack.set_visible_child(
                    &self
                        .stack
                        .child_by_name("make_another_selection_box")
                        .expect("Error: Make another selection box is missing from stack!"),
                );
                //Queue a message to leave this state after 3 seconds
                let ch = self.event_channel_tx.clone();
                glib::timeout_add_seconds(2, move || {
                    let _ = ch.send_blocking(Event::ChangeState(AppState::Idle));
                    glib::ControlFlow::Break
                });
            }
            AppState::Vending => {
                 self.stack.set_visible_child(
                    &self.stack.child_by_name("vend_in_progress_box").expect("vend_in_progress_box missing from stack"));
            }
            AppState::VendSuccess => {
                self.stack.set_visible_child(
                    &self.stack.child_by_name("vend_ok_box").expect("Vendsuccess missing from stack"));
                //Queue a message to leave this state after 3 seconds
                let ch = self.event_channel_tx.clone();
                glib::timeout_add_seconds(2, move || {
                    let _ = ch.send_blocking(Event::ChangeState(AppState::Idle));
                    glib::ControlFlow::Break
                }); 
            }
            AppState::VendFailed => {
                self.stack.set_visible_child(
                    &self.stack.child_by_name("vend_failed_box").expect("Vendfailed missing from stack"));
                //Queue a message to leave this state after 3 seconds
                let ch = self.event_channel_tx.clone();
                glib::timeout_add_seconds(2, move || {
                    let _ = ch.send_blocking(Event::ChangeState(AppState::Idle));
                    glib::ControlFlow::Break
                }); 
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
        
        //Spawn a task to receive events from the VMC response channel, and repost them onto the app's main event loop
        let rx = vmc_response_channel_rx.clone();
        let tx = event_channel_tx.clone();
        glib::MainContext::default().spawn_local( async move {
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        match event {
                            VmcResponse::CoinInsertedEvent(coin) => {
                                let _ = tx.send(Event::CoinInserted(coin.value)).await;
                            },
                            VmcResponse::CoinAcceptorEvent(CoinAcceptorEvent::EscrowPressed) => {
                                let _ = tx.send(Event::EscrowPressed).await;
                            }
                            VmcResponse::CashlessEvent(e) => {
                                let _ = tx.send(Event::CashlessEvent(e)).await;
                            }
                            VmcResponse::DispenseSuccessEvent => {
                                let _ = tx.send(Event::VendSuccess).await;
                            }
                            VmcResponse::DispenseFailedEvent => {
                                let _ = tx.send(Event::VendFailed).await;
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

        let ch = event_channel_tx.clone();
        glib::timeout_add_seconds(1, move || {
            let _ = ch.send_blocking(Event::Timeout_Poll_Event);
            glib::ControlFlow::Continue
        });

    });

    app.run()
}
