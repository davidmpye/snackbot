use glib::clone::Upgrade;
use glib::object::Cast;
use gtk4::glib;
use gtk4::glib::Object;
use gtk4::subclass::prelude::*;
mod imp;

glib::wrapper! {
    pub struct MakeSelectionBox(ObjectSubclass<imp::MakeSelectionBox>)
        @extends gtk4::Box, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Orientable, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl MakeSelectionBox {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn set_row(&self, row: Option<char>) {
        let char = match row {
            Some(c) => c,
            None => '_'
        };
        let i = imp::MakeSelectionBox::from_obj(self);
        i.row.set_label(&format!("<span font=\"Arial Rounded MT 80\">{}</span>", char));
    }

    pub fn set_col(&self, row: Option<char>) {
        let char = match row {
            Some(c) => c,
            None => '_'
        };
        let i = imp::MakeSelectionBox::from_obj(self);
        i.col.set_label(&format!("<span font=\"Arial Rounded MT 80\">{}</span>", char));
    }

}
