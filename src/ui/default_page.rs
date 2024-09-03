use std::cell::RefCell;

use egui::Ui;

use super::{tool_pages::ToolPage, Luna};

pub fn get() -> ToolPage{
    return ToolPage {
        enabled: true,
        side_title: "temp".to_string(),
        main_title: "temp".to_string(),
        render: Box::new(RefCell::new(move |ui: &mut Ui| temp())),
    };
}

fn temp(){

}