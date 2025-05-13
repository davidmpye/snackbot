use glib::clone::Upgrade;
use glib::object::Cast;
use gtk4::glib;
use gtk4::glib::Object;
use gtk4::subclass::prelude::*;
mod imp;

glib::wrapper! {
    pub struct VendInProgressBox(ObjectSubclass<imp::VendInProgressBox>)
        @extends gtk4::Box, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Orientable, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl VendInProgressBox {
    pub fn new() -> Self {
        Object::builder().build()
    }
}
