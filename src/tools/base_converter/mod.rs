mod BE_base_converter;

use std::cell::RefCell;

use eframe::glow::Context;
use egui::{Align, Grid, Label, Pos2, Rangef, Rect, TextEdit, Ui, Vec2};

use crate::{helpers::positioner, ui::ToolPage};


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
        .with_cross_align(Align::Center)
        .with_main_align(Align::Center);

    // ui.debug_paint_cursor();
    
    // ui.with_layout(layout, |ui| {
    //     ui.add(Label::new("temp 1"));
    //     ui.label("temp 2");
    // });

    ui.with_layout(layout, |ui| {

        egui::TopBottomPanel::top("base_converter_title")
            .show_inside(ui, |ui| {
                ui.label("Base Converter");
            }
        );

        // ui.debug_paint_cursor();
        egui::TopBottomPanel::top("base_converter_top")
            .resizable(true)
            .show_inside(ui, |ui| {

                egui::Grid::min_col_width(
                    egui::Grid::new("base_converter_inner_top"), 300_f32
                )
                .num_columns(2)
                .striped(true)
                .spacing(Vec2::new(50.0,50.0))
                .show(ui, |ui| {
                    
                    // ui.debug_paint_cursor();
                    //let mut temp = Rect::from_two_pos(Pos2::new(0.0, 0.0), Pos2::new(40.0,20.0)).translate(Vec2::new(ui.min_rect().min.x, ui.min_rect().min.y));
                    //println!("{} {}", ui.min_rect().min, ui.min_rect().max);
                    let rect = positioner::create_rectangle(ui, [50,30], positioner::AnchorAt::Center, positioner::ScaledOn::Down(1));
                    ui.centered_and_justified(|ui|{
                        let tl_box = ui.put(rect,
                            egui::TextEdit::singleline(&mut bc.tl)
                                .clip_text(false)
                                .hint_text("Base 10")
                                //.min_size(Vec2::new(100.0, 30.0))
                        );
                        tl_box.paint_debug_info();

                        if tl_box.has_focus() && tl_box.changed(){
                            bc.tr = convert_number(10, 2, &bc.tl);
                            bc.bl = convert_number(10, 8, &bc.tl);
                            bc.br = convert_number(10, 16, &bc.tl);
                        };
                    });

                    ui.vertical_centered(|ui|{
                        let tr_box = ui.add(
                            egui::TextEdit::singleline(&mut bc.tr)
                                .clip_text(false)
                                .hint_text("Base 2")
                                .min_size(Vec2::new(100.0, 30.0))
                                .char_limit(32)
                        );

                        if tr_box.has_focus() && tr_box.changed(){
                            bc.tl = convert_number(2, 10, &bc.tr);
                            bc.bl = convert_number(2, 8, &bc.tr);
                            bc.br = convert_number(2, 16, &bc.tr);
                        };
                    });

                    ui.end_row();

                    

                    ui.vertical_centered(|ui|{
                        let bl_box = ui.add(
                            egui::TextEdit::singleline(&mut bc.bl)
                                .clip_text(false)
                                .hint_text("Base 8")
                                .min_size(Vec2::new(100.0, 30.0))
                        );

                        if bl_box.has_focus() && bl_box.changed(){
                            bc.tl = convert_number(8, 10, &bc.bl);
                            bc.tr = convert_number(8, 2, &bc.bl);
                            bc.br = convert_number(8, 16, &bc.bl);
                        };
                    });

                    // ui.debug_paint_cursor();

                    ui.vertical_centered(|ui|{
                        let br_box = ui.add(
                            egui::TextEdit::singleline(&mut bc.br)
                                .clip_text(false)
                                .hint_text("Base 16")
                                .min_size(Vec2::new(100.0, 30.0))
                                .char_limit(8)
                        );

                        // br_box.paint_debug_info();

                        if br_box.has_focus() && br_box.changed(){
                            bc.tl = convert_number(16, 10, &bc.br);
                            bc.tr = convert_number(16, 2, &bc.br);
                            bc.bl = convert_number(16, 8, &bc.br);
                        };
                    });
                });
            });
        
        ui.add_space(50.0);

        egui::TopBottomPanel::top("base_converter_bot")
            .height_range(Rangef::new(10.0, 300.0))
            .default_height(100.0)
            .resizable(true)
            .show_inside(ui, |ui| {

                // ui.debug_paint_cursor();
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

fn convert_number(from: usize, to: usize, num: &String) -> String {
    if num.is_empty(){
        return String::new();
    }
    return BE_base_converter::convert_number_base(from, to, num);
}
