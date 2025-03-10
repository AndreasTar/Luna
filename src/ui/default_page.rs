use std::cell::RefCell;

use super::{tool_pages::ToolPage, Luna, Message};

pub struct default_page{}

impl ToolPage for default_page {
    fn get_side_title(&self) -> String {
        return "temp".to_string();
    }

    fn get_main_title(&self) -> String {
        return "temp".to_string();
    }

    fn render(&self) -> iced::Element<Message> {
        return iced::widget::text("temp").into();
    }

    fn is_enabled(&self) -> bool {
        return true;
    }
}

impl PartialEq for dyn ToolPage{
    fn eq(&self, other: &Self) -> bool {
        return self.get_main_title() == other.get_main_title();
    }
}

pub fn get() -> impl ToolPage{
    return default_page{};
}