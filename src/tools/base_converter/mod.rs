mod BE_base_converter;

use crate::ui::ToolPage;


pub fn get() -> ToolPage{
    return ToolPage {
        enabled: false,
        side_title: "temp".to_string(),
        main_title: "temp".to_string(),
        render: Box::new(|ui| temp()),
    };
}

fn temp(){

}
