mod BE_base_converter;

use std::cell::RefCell;

use egui::{Align, Grid, Label, Rangef, TextEdit, Ui, Vec2};

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

    let layout = egui::Layout::top_down(egui::Align::Center)
        .with_main_wrap(false)
        .with_cross_align(Align::Center);

    ui.debug_paint_cursor();
    
    ui.with_layout(layout, |ui| {
        ui.add(Label::new("temp 1"));
        ui.label("temp 2");
    });


    ui.with_layout(layout, |ui| {

        ui.debug_paint_cursor();
        egui::TopBottomPanel::top("base_converter_top")
            .show_inside(ui, |ui| {

                egui::Grid::min_col_width(
                    egui::Grid::new("base_converter_inner_top"), 300_f32
                )
                .num_columns(2)
                .striped(true)
                .spacing(Vec2::new(50.0,50.0))
                .show(ui, |ui| {
                    
                    ui.centered_and_justified(|ui|{
                        ui.add(
                            egui::TextEdit::singleline(&mut bc.tl)
                                .clip_text(false)
                                .hint_text("Base 10")
                                .min_size(Vec2::new(100.0, 30.0))
                                .desired_width(190.0)
                        );
                    });

                    ui.vertical_centered(|ui|{
                        ui.add(
                            egui::TextEdit::singleline(&mut bc.tr)
                                .clip_text(false)
                                .hint_text("Base 2")
                                .min_size(Vec2::new(100.0, 30.0))
                        );
                    });

                    ui.end_row();

                    ui.vertical_centered(|ui|{
                        ui.add(
                            egui::TextEdit::singleline(&mut bc.bl)
                                .clip_text(false)
                                .hint_text("Base 8")
                                .min_size(Vec2::new(100.0, 30.0))
                        );
                    });

                    ui.vertical_centered(|ui|{
                        ui.add(
                            egui::TextEdit::singleline(&mut bc.br)
                                .clip_text(false)
                                .hint_text("Base 16")
                                .min_size(Vec2::new(100.0, 30.0))
                        ).paint_debug_info();
                    });
                });
            });
        
        ui.add_space(50.0);

        egui::TopBottomPanel::top("base_converter_bot")
            .height_range(Rangef::new(10.0, 300.0))
            .default_height(100.0)
            .resizable(true)
            .show_inside(ui, |ui| {

                ui.debug_paint_cursor();
                egui::ScrollArea::vertical()

                    .auto_shrink(false)
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui|{
                            ui.label("scrollable")
                        })
                    });
            })
        
    });
}
