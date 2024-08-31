use std::default;

use super::{add_toolpage, remove_toolpage};



pub struct ToolPage{
    pub enabled: bool,
    pub sidebar_name: String,
    pub title_name: String
}

impl Default for ToolPage {
    fn default() -> Self {
        return Self{
            enabled: false,
            sidebar_name: String::from("Default sidebar name"),
            title_name: String::from("Default title name")
        };
    }
}

impl PartialEq for ToolPage {
    fn eq(&self, other: &Self) -> bool {
        return self.title_name == other.title_name;
    }
}

impl ToolPage{
    pub fn new(sidebar: &str, title: &str) -> Self {
        return Self{
            sidebar_name: sidebar.to_string(),
            title_name: title.to_string(),
            ..Default::default()
        };
    }

    pub fn add_to_sidebar(self){
        add_toolpage(self)
    }

    pub fn remove_from_sidebar(self){
        remove_toolpage(self)
    }

    pub fn set_enabled(self, enable: bool) -> Self{ // why is this even present?
        return Self {
            enabled: enable,
            ..self
        };
    }

    pub fn set_enabled_as_ref(&mut self, enable: bool) -> &Self {
        self.enabled = enable;
        return self;
    }

    pub fn show_page(&self, ui: &mut egui::Ui){

    }
}