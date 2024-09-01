use std::default;

use super::{add_toolpage, remove_toolpage};



pub struct ToolPage{
    pub enabled: bool,
    pub side_title: String,
    pub main_title: String,
    pub render: Box<dyn Fn(&mut egui::Ui)>
}

impl PartialEq for ToolPage {
    fn eq(&self, other: &Self) -> bool {
        return self.main_title == other.main_title;
    }
}


impl ToolPage{

    pub fn add_to_sidebar(self){
        add_toolpage(self)
    }

    pub fn remove_from_sidebar(self){
        remove_toolpage(self)
    }

    fn set_sidebar_title(&mut self, title: &str){
        self.side_title = title.to_string();
    }
    
    fn set_main_title(&mut self, title: &str) {
        self.main_title = title.to_string();
    }

    pub fn set_enabled_copy(self, enable: bool) -> Self{ // why is this even present?
        return Self {
            enabled: enable,
            ..self
        };
    }

    pub fn set_enabled(&mut self, enable: bool) -> &Self {
        self.enabled = enable;
        return self;
    }

    pub fn show_page(&self, ui: &mut egui::Ui){
        (self.render)(ui);
    }
}