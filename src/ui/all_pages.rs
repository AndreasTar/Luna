use super::{default_page, tool_pages::ToolPage};
use crate::tools::base_converter;

pub fn get_pages() -> Vec<Box<dyn ToolPage>> {
    return vec![
            Box::new(default_page::get()),
            Box::new(base_converter::get()),
            
        ];
}