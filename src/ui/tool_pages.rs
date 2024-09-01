use std::default;



pub trait ToolPage{
    // pub enabled: bool,
    // pub sidebar_name: String,
    // pub title_name: String

    // fn add_to_sidebar(&self){
    //     add_toolpage(self)
    // }

    // fn remove_from_sidebar(&self){
    //     remove_toolpage(self)
    // }

    fn add_to_sidebar(&self);
    fn remove_from_sidebar(&self);

    // fn set_enabled_copy(&self, enable: bool) -> Self{ // why is this even present?
    //     return Self {
    //         enabled: enable,
    //         ..self
    //     };
    // }

    fn set_enabled(&mut self, enable: bool);
    // fn set_enabled(&mut self, enable: bool) -> &Self {
    //     self.enabled = enable;
    //     return self;
    // }

    fn show_page(&self, ui: &mut egui::Ui);

    fn get_index(&self) -> usize;
    fn set_index(&self, index: usize);

    fn get_sidebar_title(&self) -> String;
    fn get_main_title(&self) -> String;
    fn is_enabled(&self) -> bool;
}