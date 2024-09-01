use super::{tool_pages::ToolPage, Luna};

pub fn get(l: &Luna) -> TP_default_page{
    return TP_default_page{
        side_title: String::from("temp"),
        main_title: String::from("temp"),
        side_descr: String::from("temp"),
        luna_instance: l
    };
}

pub struct TP_default_page<'a> {

    side_title: String,
    main_title: String,
    side_descr: String,
    luna_instance: &'a Luna,
}

impl <'a>ToolPage for TP_default_page<'a> {

    
    fn get_sidebar_title(&self) -> String{
        return self.side_title.to_string();
    }
    
    fn get_main_title(&self) -> String {
        return self.main_title.to_string();
    }

    fn is_enabled(&self) -> bool {
        todo!()
    }
    
    fn get_index(&self) -> usize {
        todo!()
    }
    
    fn set_index(&self, index: usize) {
        todo!()
    }
    
    fn set_enabled(&mut self, enable: bool) {
        todo!()
    }
    
    fn show_page(&self, ui: &mut egui::Ui) {
        todo!()
    }
    
    fn add_to_sidebar(&self) {
        todo!()
    }
    
    fn remove_from_sidebar(&self) {
        todo!()
    }

}