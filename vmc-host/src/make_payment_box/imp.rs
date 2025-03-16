use gtk4::prelude::{BoxExt, OrientableExt, WidgetExt};
use gtk4::subclass::prelude::*;
use gtk4::{Box, Image, Label};

#[derive(Default)]
pub struct MakePaymentBox {
    pub item_image: Image,
    pub item_name: Label,
    pub item_price: Label,
}

#[glib::object_subclass]
impl ObjectSubclass for MakePaymentBox {
    const NAME: &'static str = "SnackBoxMakePaymentBox";
    type Type = super::MakePaymentBox;
    type ParentType = gtk4::Box;
}

// Trait shared by all GObjects
impl ObjectImpl for MakePaymentBox {
    fn constructed(&self) {
        self.parent_constructed();
        self.obj().set_orientation(gtk4::Orientation::Vertical);

        self.obj().set_spacing(20);
        self.obj().append(
            &Label::builder()
                .use_markup(true)
                .justify(gtk4::Justification::Center)
                .label("<span font=\"Arial Rounded MT 50\">Please use\nContactless</span>")
                .build(),
        );
        self.obj().append(
            &Image::builder()
                .file("./contactless.gif")
                .height_request(200)
                .width_request(140)
                .build(),
        );
        self.obj().append(
            &Label::builder()
                .use_markup(true)
                .justify(gtk4::Justification::Center)
                .label("<span font=\"Arial Rounded MT 50\">or\nInsert Coins</span>")
                .build(),
        );
        self.obj().append(
            &Image::builder()
                .file("./coins.jpeg")
                .height_request(200)
                .width_request(140)
                .build(),
        );
    }
}

// Trait shared by all widgets
impl WidgetImpl for MakePaymentBox {}

impl BoxImpl for MakePaymentBox {}
