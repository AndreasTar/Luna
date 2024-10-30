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
    cbCount: usize,
    cbNums: Vec<String>,
    cbBases: Vec<String>

}


pub fn get() -> ToolPage {

    let mut ui_bc = UI_BaseConverter {
        tl: String::new().to_owned(),
        tr: String::new().to_owned(),
        bl: String::new().to_owned(),
        br: String::new().to_owned(),
        cbCount: 1,
        cbNums: vec![String::new()],
        cbBases: vec![String::from("9")],
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
                ui.put(positioner::create_rectangle(
                    ui, [100,30], [0;2], 
                    positioner::AnchorAt::Center, positioner::ScaledOn::Nothing,
                    false
                ), 
                    Label::new("Base Converter")
                ).paint_debug_info();
            }
        );

        // ui.debug_paint_cursor();
        egui::TopBottomPanel::top("base_converter_center")
            .resizable(true)
            .min_height(180.0)
            .show_inside(ui, |ui| {

                let tl_box = ui.put(
                    positioner::create_rectangle(
                        &ui, [300,50], [0;2],
                        positioner::AnchorAt::TopLeft, positioner::ScaledOn::Nothing,
                        false
                    ),
                    egui::TextEdit::singleline(&mut bc.tl)
                        .clip_text(false)
                        .hint_text("Base 10")
                        .min_size(Vec2::new(100.0, 30.0))
                );
                
                //tl_box.paint_debug_info();

                if tl_box.has_focus() && tl_box.changed(){
                    bc.tr = convert_number(10, 2, &bc.tl);
                    bc.bl = convert_number(10, 8, &bc.tl);
                    bc.br = convert_number(10, 16, &bc.tl);
                    bc.cbNums = manage_customBoxes(10, &bc.tl, bc.cbBases.clone(), bc.cbCount, 0, false);
                };

                let tr_box = ui.put(
                    positioner::create_rectangle(
                        &ui, [300,50], [0;2],
                        positioner::AnchorAt::TopRight, positioner::ScaledOn::Nothing,
                        false
                    ),
                    egui::TextEdit::singleline(&mut bc.tr)
                        .clip_text(false)
                        .hint_text("Base 2")
                        .min_size(Vec2::new(100.0, 30.0))
                        .char_limit(32)
                );

                //tr_box.paint_debug_info();

                if tr_box.has_focus() && tr_box.changed(){
                    bc.tl = convert_number(2, 10, &bc.tr);
                    bc.bl = convert_number(2, 8, &bc.tr);
                    bc.br = convert_number(2, 16, &bc.tr);
                    bc.cbNums = manage_customBoxes(2, &bc.tr, bc.cbBases.clone(), bc.cbCount, 0, false);
                };

                let bl_box = ui.put(
                    positioner::create_rectangle(
                        &ui, [300,50], [0;2],
                        positioner::AnchorAt::BottomLeft, positioner::ScaledOn::Nothing,
                        false
                    ),
                    egui::TextEdit::singleline(&mut bc.bl)
                        .clip_text(false)
                        .hint_text("Base 8")
                        .min_size(Vec2::new(100.0, 30.0))
                );

                //bl_box.paint_debug_info();

                if bl_box.has_focus() && bl_box.changed(){
                    bc.tl = convert_number(8, 10, &bc.bl);
                    bc.tr = convert_number(8, 2, &bc.bl);
                    bc.br = convert_number(8, 16, &bc.bl);
                    bc.cbNums = manage_customBoxes(8, &bc.bl, bc.cbBases.clone(), bc.cbCount, 0, false);
                };
                

                let br_box = ui.put(
                    positioner::create_rectangle(
                        &ui, [300,50], [0;2],
                        positioner::AnchorAt::BottomRight, positioner::ScaledOn::Nothing,
                        false
                    ),
                    egui::TextEdit::singleline(&mut bc.br)
                        .clip_text(false)
                        .hint_text("Base 16")
                        .min_size(Vec2::new(100.0, 30.0))
                        .char_limit(8)
                );

                //br_box.paint_debug_info();

                if br_box.has_focus() && br_box.changed(){
                    bc.tl = convert_number(16, 10, &bc.br);
                    bc.tr = convert_number(16, 2, &bc.br);
                    bc.bl = convert_number(16, 8, &bc.br);
                    bc.cbNums = manage_customBoxes(16, &bc.br, bc.cbBases.clone(), bc.cbCount, 0, false);
                };

            });
        
        ui.add_space(50.0);

        egui::ScrollArea::vertical()

        .auto_shrink(false)
        .show(ui, |ui| {
            ui.vertical_centered(|ui|{
                //ui.label("scrollable");

                for i in 0..bc.cbCount {
                    ui.put(
                        positioner::create_rectangle(
                            &ui, [500,90], [-300, 0],
                            positioner::AnchorAt::TopCenter, positioner::ScaledOn::Nothing,
                            false
                        ),
                        egui::TextEdit::singleline(bc.cbNums.get_mut(i).unwrap())
                            .clip_text(false)
                            .hint_text("Base 9")
                            .min_size(Vec2::new(100.0, 30.0))
                    );

                    ui.put(
                        positioner::create_rectangle(
                            &ui, [30,30], [500, 0],
                            positioner::AnchorAt::TopCenter, positioner::ScaledOn::Nothing,
                            false
                        ),
                        egui::TextEdit::singleline(bc.cbBases.get_mut(i).unwrap())
                            .clip_text(false)
                            .hint_text("9")
                            .min_size(Vec2::new(30.0, 30.0))
                    );
                }
            })
        })

        // egui::TopBottomPanel::top("base_converter_bot")
        //     .height_range(Rangef::new(10.0, 300.0))
        //     .default_height(100.0)
        //     .resizable(true)
        //     .show_inside(ui, |ui| {

        //         // ui.debug_paint_cursor();
        //         egui::ScrollArea::vertical()

        //             .auto_shrink(false)
        //             .show(ui, |ui| {
        //                 ui.vertical_centered(|ui|{
        //                     ui.label("scrollable")
        //                 })
        //             });
        //     }
        // )
        
    });
}

fn convert_number(from: usize, to: usize, num: &String) -> String {
    if num.is_empty(){
        return String::new();
    }
    return BE_base_converter::convert_number_base(from, to, num);
}

fn manage_customBoxes(from: usize, num: &String, cbBases: Vec<String>, cbCount: usize, currentlyAt: usize, fromCustom: bool) -> Vec<String> {

    let mut newNums: Vec<String> = vec![];
    for i in 0..cbCount{
        if fromCustom {
            if i == currentlyAt {continue;}
        };

        let cbBase: usize = u32::from_str_radix(cbBases.get(i).unwrap(), 10).unwrap().try_into().unwrap();
        newNums.push(convert_number(from, cbBase, num));
        
    }
    return newNums;
}
