use gtk4::prelude::{BoxExt, OrientableExt, WidgetExt};
use gtk4::subclass::prelude::*;
use gtk4::{Box, Image, Label};

#[derive(Default)]
pub struct MakeSelectionBox {
    pub row: Label,
    pub col: Label,
}

#[glib::object_subclass]
impl ObjectSubclass for MakeSelectionBox {
    const NAME: &'static str = "SnackBoxMakeSelectionBox";
    type Type = super::MakeSelectionBox;
    type ParentType = gtk4::Box;
}

/*
 */

// Trait shared by all GObjects
impl ObjectImpl for MakeSelectionBox {
    fn constructed(&self) {
        self.parent_constructed();
        self.obj().set_orientation(gtk4::Orientation::Vertical);

        let itembox = Box::new(gtk4::Orientation::Horizontal, 50);
        self.obj().append(&itembox);

        self.row.set_use_markup(true);
        self.row.set_label("<span font=\"Arial Rounded MT 80\">_</span>");

        self.col.set_use_markup(true);
        self.col.set_label("<span font=\"Arial Rounded MT 80\">_</span>");
        
        itembox.append(&self.row);
        itembox.append(&self.col);
        itembox.set_baseline_position(gtk4::BaselinePosition::Center);

        self.obj().set_spacing(50);
        self.obj().append(
            &Label::builder()
                .justify(gtk4::Justification::Center)
                .use_markup(true)
                .label("<span font=\"Arial Rounded MT 50\">Please select\nan item</span>")
                .build(),
        );

    }

}

// Trait shared by all widgets
impl WidgetImpl for MakeSelectionBox {}

impl BoxImpl for MakeSelectionBox {}
