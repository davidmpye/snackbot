use gtk4::prelude::{BoxExt, OrientableExt, WidgetExt};
use gtk4::subclass::prelude::*;
use gtk4::{Box, Image, Label};

#[derive(Default)]
pub struct ConfirmItemBox {
    pub c: u16,
    pub item_image: Image,
    pub item_name: Label,
    pub item_price: Label,
}

#[glib::object_subclass]
impl ObjectSubclass for ConfirmItemBox {
    const NAME: &'static str = "SnackBoxConfirmItemBox";
    type Type = super::ConfirmItemBox;
    type ParentType = gtk4::Box;
}

// Trait shared by all GObjects
impl ObjectImpl for ConfirmItemBox {
    fn constructed(&self) {
        self.parent_constructed();
        self.obj().set_orientation(gtk4::Orientation::Vertical);

        self.item_image.set_width_request(200);
        self.item_image.set_height_request(300);

        self.item_name.set_justify(gtk4::Justification::Center);
        self.item_name.set_use_markup(true);

        self.item_price.set_use_markup(true);

        //Add the three items to the child pane
        self.obj().append(&self.item_name);
        self.obj().append(&self.item_image);
        self.obj().append(&self.item_price);

        self.obj().set_spacing(50);
        self.obj().append(
            &Label::builder()
                .use_markup(true)
                .label("<span font=\"Arial Rounded MT 50\">✅ to vend\n❌ to cancel</span>")
                .build(),
        );
    }
}

// Trait shared by all widgets
impl WidgetImpl for ConfirmItemBox {}

impl BoxImpl for ConfirmItemBox {}
