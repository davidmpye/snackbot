use gtk4::prelude::{BoxExt, OrientableExt, WidgetExt};
use gtk4::subclass::prelude::*;
use gtk4::{Box, Image, Label};

#[derive(Default)]
pub struct MakeSelectionBox {
    pub row_col: Label,
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


        self.row_col.set_use_markup(true);
        self.row_col.set_label("<span font=\"Arial Rounded MT 80\">_ _</span>");
        self.obj().append(&self.row_col);

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
