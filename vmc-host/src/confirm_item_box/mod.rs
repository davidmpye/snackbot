use glib::clone::Upgrade;
use glib::object::Cast;
use gtk4::glib;
use gtk4::glib::Object;
use gtk4::subclass::prelude::*;
mod imp;

glib::wrapper! {
    pub struct ConfirmItemBox(ObjectSubclass<imp::ConfirmItemBox>)
        @extends gtk4::Box, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Orientable, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl ConfirmItemBox {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn set_price(&self, price: u16) {
        let i = imp::ConfirmItemBox::from_obj(self);
        i.item_price.set_label(&format!(
            "<span font=\"Arial Rounded MT 50\">Price: £{}.{:02}</span>",
            price / 100,
            price % 100
        ));
    }

    pub fn set_name(&self, label: String) {
        let i = imp::ConfirmItemBox::from_obj(self);
        i.item_name.set_label(&format!(
            "<span font=\"Arial Rounded MT 50\">{}</span>",
            &label
        ));
    }

    pub fn set_image(&self, path: String) {
        let i = imp::ConfirmItemBox::from_obj(self);
        i.item_image.set_from_file(Some(&path));
    }
}
