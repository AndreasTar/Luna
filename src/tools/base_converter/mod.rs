mod BE_base_converter;

use std::cell::RefCell;

use iced::{alignment, widget::{column, container, keyed::column, row, scrollable, Container, Text}, Color, Element};

use crate::{helpers::positioner::{self, PositionInfo}, ui::{LunaMessage, ToolPage}};


#[derive(Debug, Clone)]
enum BC_Message{
    TLChanged(String),
    TRChanged(String),
    BLChanged(String),
    BRChanged(String),
    CustomNumChanged(String, u8),
    CustomBaseChanged(String, String)
}

pub struct UI_BaseConverter{

    side_title: String,
    main_title: String,
    enabled: bool,

    tl: String,
    tr: String,
    bl: String,
    br: String,
    cbCount: u8,
    cbNums: Vec<String>,
    cbBases: Vec<String>,
}

impl ToolPage for UI_BaseConverter {
    fn get_side_title(&self) -> String {
        return self.side_title.clone();
    }

    fn get_main_title(&self) -> String {
        return self.main_title.clone();
    }

    fn is_enabled(&self) -> bool {
        return self.enabled;
    }

    fn render(&self) -> Element<LunaMessage> {
        return Element::new(self.layout())
                        .map(move |msg| LunaMessage::Nothing);
    }
}

impl UI_BaseConverter {
    fn layout(&self) -> Container<BC_Message> {

        let title_section = container(
            Text::new(&self.main_title)
                .size(30)
                .center()
                .color(Color::WHITE)
        )
        .center(0)
        .width(iced::Length::Fill)
        .style(container::rounded_box)
        .padding(10);

        let predef_converters = Container::new(
            column![
                row![
                    iced::widget::text_input("Base 10", &self.tl)
                        .on_input(|text| BC_Message::TLChanged(text)),
                    iced::widget::text_input("Base 2", &self.tl)
                        .on_input(|text| BC_Message::TRChanged(text)),
                ],
                row![
                    iced::widget::text_input("Base 8", &self.tl)
                        .on_input(|text| BC_Message::BLChanged(text)),
                    iced::widget::text_input("Base 16", &self.tl)
                        .on_input(|text| BC_Message::BRChanged(text)),
                ]
            ]
        );

        let custom_converters = Container::new(
            scrollable(column(
                (0..self.cbCount).map(|i| {
                    let base = self.cbBases.get(i as usize).unwrap();
                    let basestr = "Base ".to_owned() + base;
                    let mut numstr = String::new();
                    if base == "" {
                        numstr = "Input Base -->".to_string();
                    }
                    else {
                        numstr = "Base ".to_owned() + base;
                    }
                    let numstr = numstr;

                    row![
                        iced::widget::text_input(&numstr, &self.tl)
                            .on_input(BC_Message::TLChanged),
                        iced::widget::text_input(&basestr, &self.tl)
                            .on_input(BC_Message::TLChanged),
                    ].into()
                })
            ))
        );


        return Container::new(column![
            title_section,
            predef_converters,
            custom_converters
        ]).into();
    }
}


pub fn get() -> UI_BaseConverter {

    let ui_bc = UI_BaseConverter {

        side_title: "Base Converter".to_string(),
        main_title: "Base Converter".to_string(),
        enabled: true,

        tl: String::new().to_owned(),
        tr: String::new().to_owned(),
        bl: String::new().to_owned(),
        br: String::new().to_owned(),
        cbCount: 1,
        cbNums: vec![String::new()],
        cbBases: vec![String::new()],

    };
    
    return ui_bc;
}

/*
fn layout(ui: &mut Ui, bc: &mut UI_BaseConverter, ctx: &Context){

    egui::TopBottomPanel::top("base_converter_title")
        .show_inside(ui, |ui| {
            ui.put(positioner::create_rectangle(
                ui, PositionInfo {
                    defaultSize: [100,30], 
                    offset: [0;2],
                    anchor: positioner::AnchorAt::Center, 
                    scaled: positioner::ScaledOn::Nothing,
                    ..Default::default()
                },
                false
            ), 
                Label::new("Base Converter")
            ).paint_debug_info();
        }
    );

    // ui.debug_paint_cursor();
    egui::TopBottomPanel::top("base_converter_center")
        .resizable(false)
        .min_height(180.0)
        .show_inside(ui, |ui| {

            let tl_box = ui.put(
                positioner::create_rectangle(
                    &ui, PositionInfo {
                        defaultSize: [300,50], 
                        offset: [0;2],
                        anchor: positioner::AnchorAt::TopLeft, 
                        scaled: positioner::ScaledOn::Nothing,
                        ..Default::default()
                    },
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
                    &ui, PositionInfo {
                        defaultSize: [300,50], 
                        offset: [0;2],
                        anchor: positioner::AnchorAt::TopRight, 
                        scaled: positioner::ScaledOn::Nothing,
                        ..Default::default()
                    },
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
                    &ui, PositionInfo {
                        defaultSize: [300,50], 
                        offset: [0;2],
                        anchor: positioner::AnchorAt::BottomLeft, 
                        scaled: positioner::ScaledOn::Nothing,
                        ..Default::default()
                    },
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
                    &ui, PositionInfo {
                        defaultSize: [300,50], 
                        offset: [0;2],
                        anchor: positioner::AnchorAt::BottomRight, 
                        scaled: positioner::ScaledOn::Nothing,
                        ..Default::default()
                    },
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

                let oldBase = bc.cbBases.get(i as usize).unwrap();
                let mut inputBase = oldBase.clone();

                let cbBase = ui.put(
                    positioner::create_rectangle(
                        &ui, PositionInfo {
                            defaultSize: [30,30], 
                            offset: [500, 0],
                            anchor: positioner::AnchorAt::TopCenter, 
                            scaled: positioner::ScaledOn::Nothing,
                            ..Default::default()
                        },
                        false
                    ),
                    egui::TextEdit::singleline(&mut inputBase)
                        .clip_text(false)
                        .hint_text(oldBase.clone())
                        .min_size(Vec2::new(30.0, 30.0))
                );

                if cbBase.has_focus() && cbBase.changed(){
                    if (inputBase.is_empty() || inputBase.parse::<u8>().is_err()){
                        inputBase = String::new();
                    } else {
                        inputBase = u32::from_str_radix(&inputBase, 10).unwrap().to_string();
                    }
                }

                let mut enabled = false;
                let mut numboxText: String;
                if inputBase.is_empty() {
                    numboxText = "Input Base -->".to_string();
                } else if !(u32::from_str_radix(&inputBase, 10).unwrap() > 1 && u32::from_str_radix(&inputBase, 10).unwrap() < 37) {
                        numboxText = "Input a valid Base (2-36) -->".to_string();
                } else {
                    numboxText = format!("Base {}", inputBase);
                    enabled = true;
                }; 

                let mut oldNum = bc.cbNums.get(i as usize).unwrap().clone();

                let cbBox = ui.put(
                    positioner::create_rectangle(
                        &ui, PositionInfo {
                            defaultSize: [500,90], 
                            offset: [-300, 0],
                            anchor: positioner::AnchorAt::TopCenter, 
                            scaled: positioner::ScaledOn::Nothing,
                            ..Default::default()
                        },
                        false
                    ),
                    egui::TextEdit::singleline(&mut oldNum)
                        .clip_text(false)
                        .hint_text(numboxText)
                        .min_size(Vec2::new(100.0, 30.0))
                );

                if cbBox.has_focus() && cbBox.changed() && enabled {
                    let inputNum = oldNum;
                    let base = u32::from_str_radix(&inputBase, 10).unwrap();
                    bc.tl = convert_number(16, 10, &inputNum);
                    bc.tr = convert_number(16, 2, &inputNum);
                    bc.bl = convert_number(16, 8, &inputNum);
                    bc.cbNums = manage_customBoxes(16, &inputNum, bc.cbBases.clone(), bc.cbCount, i, true);
                    
                    println!("{}", bc.cbNums.len());
                    *bc.cbNums.get_mut(i as usize).unwrap() = inputNum;
                }

                *bc.cbBases.get_mut(i as usize).unwrap() = inputBase;
            }
        })
    });

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
}
*/

fn convert_number(from: usize, to: usize, num: &String) -> String {
    if num.is_empty(){
        return String::new();
    }
    return BE_base_converter::convert_number_base(from, to, num);
}

fn manage_customBoxes(from: usize, num: &String, cbBases: Vec<String>, cbCount: u8, currentlyAt: u8, fromCustom: bool) -> Vec<String> {

    let mut newNums: Vec<String> = vec![];
    for i in 0..cbCount{
        if fromCustom {
            if i == currentlyAt {newNums.push(num.to_string()); continue;}
        };

        let base = cbBases.get(i as usize).unwrap();
        if !(base.is_empty() || base.parse::<u8>().is_err()) {
            let cbBase: usize = u32::from_str_radix(base, 10).unwrap().try_into().unwrap();
            newNums.push(convert_number(from, cbBase, num));
        } else {
            newNums.push(String::new());
        }
        
    }
    return newNums;
}

fn run_once() {
    // Add tool page dependent stuff here if needed
}
