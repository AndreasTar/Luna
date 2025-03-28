mod BE_base_converter;

use std::{any::Any, cell::RefCell};

use iced::{alignment, widget::{column, container, keyed::column, row, scrollable, Container, Text}, Color, Element};

use crate::{helpers::positioner::{self, PositionInfo}, ui::{LunaMessage, ToolPage}};


#[derive(Debug, Clone)]
pub enum BC_Message{
    Nothing,
    TLChanged(String),
    TRChanged(String),
    BLChanged(String),
    BRChanged(String),
    CustomNumChanged(String, usize, u8), // num base index
    CustomBaseChanged(String, u8) // base index
}

pub struct UI_BaseConverter{

    last_msg: RefCell<Option<BC_Message>>,

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
                        .map(move |msg| {
                            match msg {
                                BC_Message::Nothing => LunaMessage::Nothing,
                                _ => {
                                    self.last_msg.replace(Some(msg));
                                    LunaMessage::ShouldUpdate(1) // HACK
                                }
                            }
                            
                        });
    }

    fn update_state(&mut self) {
        self.update_state();
    }
}

impl UI_BaseConverter {

    pub fn update_state(&mut self) {
        match &self.last_msg.take() {
            Some(msg) => {
                match msg {
                    BC_Message::Nothing => (),
                    BC_Message::TLChanged(numstr) => {
                        self.tl = numstr.to_string();
                        self.tr = convert_number(10, 2, numstr);
                        self.bl = convert_number(10, 8, numstr);
                        self.br = convert_number(10, 16, numstr);
                        self.cbNums = manage_customBoxes(10, numstr,
                            self.cbBases.clone(), self.cbCount, 0, false);
                    },
                    BC_Message::TRChanged(numstr) => {
                        self.tl = convert_number(2, 10, numstr);
                        self.tr = numstr.to_string();
                        self.bl = convert_number(2, 8, numstr);
                        self.br = convert_number(2, 16, numstr);
                        self.cbNums = manage_customBoxes(2, numstr,
                            self.cbBases.clone(), self.cbCount, 0, false);
                    },
                    BC_Message::BLChanged(numstr) => {
                        self.tl = convert_number(8, 10, numstr);
                        self.tr = convert_number(8, 2, numstr);
                        self.bl = numstr.to_string();
                        self.br = convert_number(8, 16, numstr);
                        self.cbNums = manage_customBoxes(8, numstr,
                            self.cbBases.clone(), self.cbCount, 0, false);
                    },
                    BC_Message::BRChanged(numstr) => {
                        self.tl = convert_number(16, 10, numstr);
                        self.tr = convert_number(16, 2, numstr);
                        self.bl = convert_number(16, 8, numstr);
                        self.br = numstr.to_string();
                        self.cbNums = manage_customBoxes(16, numstr,
                            self.cbBases.clone(), self.cbCount, 0, false);
                    },
                    BC_Message::CustomNumChanged(n, b, i) => {
                        self.tl = convert_number(*b, 10, n);
                        self.tr = convert_number(*b, 8, n);
                        self.bl = convert_number(*b, 8, n);
                        self.br = convert_number(*b, 16, n);
                        self.cbNums = manage_customBoxes(*b, n,
                            self.cbBases.clone(), self.cbCount, *i, true);
                    },
                    BC_Message::CustomBaseChanged(b, i) => {
                        todo!()
                    },

                }
            },
            None => (),
        }
    }

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
                    iced::widget::text_input("Base 2", &self.tr)
                        .on_input(|text| BC_Message::TRChanged(text)),
                ],
                row![
                    iced::widget::text_input("Base 8", &self.bl)
                        .on_input(|text| BC_Message::BLChanged(text)),
                    iced::widget::text_input("Base 16", &self.br)
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
                        iced::widget::text_input(&numstr, &self.cbNums.get(i as usize).unwrap())
                            .on_input(move |text| BC_Message::CustomNumChanged(text, base_to_num(base.to_string()), i)),
                        iced::widget::text_input(&basestr, &self.cbBases.get(i as usize).unwrap())
                            .on_input(move |text| BC_Message::CustomBaseChanged(text, i)),
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

        last_msg: None.into(),

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

fn base_to_num(base: String) -> usize { // TODO change to float or double etc
    if !(base.is_empty() || base.parse::<u8>().is_err()) {
        return u32::from_str_radix(&base, 10).unwrap().try_into().unwrap();
    } else {
        return 0;
    }
}

fn run_once() {
    // Add tool page dependent stuff here if needed
}
