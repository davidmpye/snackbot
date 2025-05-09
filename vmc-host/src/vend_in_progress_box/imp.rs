use gtk4::prelude::{BoxExt, OrientableExt, WidgetExt};
use gtk4::subclass::prelude::*;
use gtk4::{Box, Image, Label};

#[derive(Default)]
pub struct VendInProgressBox {
    pub reason: Label,
}

#[glib::object_subclass]
impl ObjectSubclass for VendInProgressBox {
    const NAME: &'static str = "VendInProgressBox";
    type Type = super::VendInProgressBox;
    type ParentType = gtk4::Box;
}

// Trait shared by all GObjects
impl ObjectImpl for VendInProgressBox {
    fn constructed(&self) {
        self.parent_constructed();
        self.obj().set_orientation(gtk4::Orientation::Vertical);

        self.obj().set_spacing(50);
        self.obj().append(
            &Label::builder()
                .use_markup(true)
                .justify(gtk4::Justification::Center)
                .label("<span font=\"Arial Rounded MT 60\" color=\"red\">\n\nVending\n-\nPlease Wait</span>")
                .build(),
        );
        self.reason.set_use_markup(true);
        self.obj().append(&self.reason);
    }
}

// Trait shared by all widgets
impl WidgetImpl for VendInProgressBox {}

impl BoxImpl for VendInProgressBox {}
