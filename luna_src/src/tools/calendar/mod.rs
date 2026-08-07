
use crate::{ LunaAppUi, WidgetTrait };
use slint::{ ComponentHandle, Model, ModelRc, SharedString, Weak };

pub const VERSION: luna::Version = luna::Version::new(0, 0, 1);

pub struct UI_Calendar {
    ui_handle: Weak<LunaAppUi>,
}

impl WidgetTrait for UI_Calendar {
    
    fn register_widget(ui_handle: Weak<LunaAppUi>) -> Self {

        let mut calendar = UI_Calendar {
            ui_handle
        };

        calendar.ui_handle.unwrap().set_test2(true);

        return calendar;
    }
}