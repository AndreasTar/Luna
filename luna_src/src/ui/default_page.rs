use std::cell::RefCell;

use super::{tool_pages::ToolPage, Luna, LunaMessage};

pub struct default_page{}

impl ToolPage for default_page {
    fn get_side_title(&self) -> String {
        return "tempside".to_string();
    }

    fn get_main_title(&self) -> String {
        return "tempmain".to_string();
    }

    fn render(&self) -> iced::Element<LunaMessage> {
        return iced::widget::text("temptext").into();
    }

    fn is_enabled(&self) -> bool {
        return true;
    }
    
    fn update_state(&mut self) {
        ();
    }
}

pub fn get() -> impl ToolPage{
    return default_page{};
}