use std::{cell::RefCell, default};

use iced::Element;

use super::{add_toolpage, remove_toolpage, Message};

pub trait ToolPage{
    fn get_side_title(&self) -> String;
    fn get_main_title(&self) -> String;
    fn is_enabled(&self) -> bool;
    fn render(&self) -> Element<Message>;
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