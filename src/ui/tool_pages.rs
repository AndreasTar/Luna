use std::{cell::RefCell, default, fmt::Debug};

use iced::Element;

use super::{add_toolpage, remove_toolpage, LunaMessage};

pub trait ToolPage{

    fn get_side_title(&self) -> String;
    fn get_main_title(&self) -> String;
    fn is_enabled(&self) -> bool;
    fn render(&self) -> Element<LunaMessage>;
    fn update_state(&mut self);
}

impl Debug for dyn ToolPage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ToolPage") // HACK
    }
}



// impl ToolPage{

//     pub fn add_to_sidebar(self){
//         add_toolpage(self)
//     }

//     pub fn remove_from_sidebar(self){
//         remove_toolpage(self)
//     }

//     fn set_sidebar_title(&mut self, title: &str){
//         self.side_title = title.to_string();
//     }
    
//     fn set_main_title(&mut self, title: &str) {
//         self.main_title = title.to_string();
//     }

//     pub fn set_enabled_copy(self, enable: bool) -> Self{ // why is this even present?
//         return Self {
//             enabled: enable,
//             ..self
//         };
//     }

//     pub fn set_enabled(&mut self, enable: bool) -> &Self {
//         self.enabled = enable;
//         return self;
//     }

//     pub fn show_page(&self, ui: &mut egui::Ui, ctx: &Context){
//         (self.render.borrow_mut())(ui, ctx);
//     }
// }