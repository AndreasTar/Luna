mod BE_base_converter;

use std::cell::RefCell;

use egui::{Grid, TextEdit, Ui, Vec2};

use crate::ui::ToolPage;


struct UI_BaseConverter{

    tl: String,
    tr: String,
    bl: String,
    br: String,

}


pub fn get() -> ToolPage {

    let mut ui_bc = UI_BaseConverter {
        tl: String::new().to_owned(),
        tr: String::new().to_owned(),
        bl: String::new().to_owned(),
        br: String::new().to_owned(),
    };
    
    return ToolPage {
        enabled: false,
        side_title: "Number Converter".to_string(),
        main_title: "Number Converter".to_string(),
        render: Box::new(RefCell::new(move |ui: &mut Ui| layout(ui, &mut ui_bc))),
    };
}

fn layout(ui: &mut Ui, bc: &mut UI_BaseConverter){
    ui.vertical_centered_justified(|ui| {

        // There are other ways to do the line below, check ctrl+space on it
        egui::Grid::min_col_width(
            egui::Grid::new("base_converter_top"), 300_f32
        )
        .num_columns(2)
        .striped(true)
        .spacing(Vec2::new(50.0,50.0))
        .show(ui, |ui| {
            
            ui.add(
                egui::TextEdit::singleline(&mut bc.tl)
                    .clip_text(false)
                    .hint_text("Base 10")
                    .min_size(Vec2::new(100.0, 30.0))
            );

            ui.add(
                egui::TextEdit::singleline(&mut bc.tr)
                    .clip_text(false)
                    .hint_text("Base 2")
                    .min_size(Vec2::new(100.0, 30.0))
            );

            ui.end_row();

            ui.add(
                egui::TextEdit::singleline(&mut bc.bl)
                    .clip_text(false)
                    .hint_text("Base 8")
                    .min_size(Vec2::new(100.0, 30.0))
            );
            
            ui.add(
                egui::TextEdit::singleline(&mut bc.br)
                    .clip_text(false)
                    .hint_text("Base 16")
                    .min_size(Vec2::new(100.0, 30.0))
            );
        });

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.label("scrollable")
        });
    });
}
