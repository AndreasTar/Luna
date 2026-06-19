
use crate::{ LunaAppUi, GlobalConversionCallback, WidgetTrait };
use slint::{ ComponentHandle, Model, ModelRc, SharedString, Weak };

pub const VERSION: luna::Version = luna::Version::new(1, 0, 1);

pub struct UI_Calendar {
    ui_handle: Weak<LunaAppUi>,
}

impl WidgetTrait for UI_Calendar {
    
    fn register_widget(ui_handle: Weak<LunaAppUi>) -> Self {
        todo!();
    }
}