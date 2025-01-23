mod lcd_driver;
use lcd_driver::LcdDriver;
mod vmc_driver;
use relm4::gtk::prelude::WidgetExt;
use vmc_driver::VmcDriver;

use vmc_icd::DispenserAddress;

const KEYBOARD_DEVICE_NAME:&str = "matrix-keyboard";
const VMC_DEVICE_NAME:&str = "vmc";

use tokio::runtime::Runtime;  //We use the Tokio runtime to run the postcard-apc async functions
use gtk::glib::clone;
use gtk::gdk;
use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt};
use relm4::{gtk, ComponentParts, ComponentSender, RelmApp, RelmWidgetExt, SimpleComponent};
/* 
//Spawn a tokio runtime instance for the postcard-rpc device handlers
fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        Runtime::new().expect("Failed to spawn tokio runtime")
    })
}*/
struct AppModel {
    row_selected:Option<char>,
    col_selected:Option<char>,
}

#[derive(Debug)]
enum AppMsg {
    RowSelected(char),
    ColSelected(char),
    Dispense,
    ClearSelection,
}

struct AppWidgets {
    selected_item_label: gtk::Label,
}

impl SimpleComponent for AppModel {

    /// The type of the messages that this component can receive.
    type Input = AppMsg;
    /// The type of the messages that this component can send.
    type Output = ();
    /// The type of data with which this component will be initialized.
    type Init = u8;
    /// The root GTK widget that this component will create.
    type Root = gtk::Window;
    /// A data structure that contains the widgets that you will need to update.
    type Widgets = AppWidgets;

    fn init_root() -> Self::Root {
        gtk::Window::builder()
            .title("Snackbot")
            .default_width(480)
            .default_height(800)
            .build()
    }

    /// Initialize the UI and model.
    fn init(
        counter: Self::Init,
        window: Self::Root,
        sender: ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        let model = AppModel { row_selected: None, col_selected: None };

        let vbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(5)
            .build();

        //Install the keypress listener
        window.add_controller(Self::keypress_listener(sender.clone()));


        let vend_button = gtk::Button::with_label("Press TICK to buy");

  
        let selected_item_label = gtk::Label::new(Some("__"));
        selected_item_label.set_margin_all(5);

        window.set_child(Some(&vbox));
        vbox.set_margin_all(5);
        vbox.append(&vend_button);
        vbox.append(&selected_item_label);

        vend_button.connect_clicked(clone!(
            #[strong]
            sender,
            move |_| {
                sender.input(AppMsg::RowSelected('A'));
            }
        ));

        let widgets = AppWidgets { selected_item_label };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            AppMsg::RowSelected(x) => {
                if self.row_selected.is_some() {
                    self.col_selected = None;
                }
                self.row_selected = Some(x);
                }
            AppMsg::ColSelected(x) => {
                if self.col_selected.is_some() {
                    self.row_selected = None;
                }
                self.col_selected = Some(x);
            }
            _ => {},
        }
    }

    /// Update the view to represent the updated model.
    fn update_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {

        let row_char = match self.row_selected {
            Some(c) => c,
            None => '_'
        };

        let col_char = match self.col_selected {
            Some(c) => c,
            None => '_'
        };

        widgets
            .selected_item_label
            .set_label(&format!("{}{}", row_char, col_char));
         }


}

impl AppModel {

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
            sender.input(AppMsg::RowSelected(c));
        }
        else {
            sender.input(AppMsg::ColSelected(c));
        }
        glib::Propagation::Proceed
    });
    event_controller
}

}


fn main() {
    let app = RelmApp::new("uk.org.makerspace.snackbot");
    app.run::<AppModel>(0);
}