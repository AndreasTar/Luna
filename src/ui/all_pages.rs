use super::{default_page, tool_pages::ToolPage};
use crate::tools::base_converter;

pub fn get_pages() -> Vec<ToolPage> {
    return vec![
        default_page::get(),
        base_converter::get(),
        
    ];
}