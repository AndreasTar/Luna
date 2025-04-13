use luna::number_converter;

use std::{any::Any, cell::RefCell};

use iced::{alignment, widget::{button, column, container, keyed::column, row, scrollable, text::LineHeight, text_input, Container, Text, TextInput}, Border, Color, Element, Theme};

use crate::{helpers::positioner::{self, PositionInfo}, ui::{LunaMessage, ToolPage}};

// TODO instead of invalid input on invalid input lmao, make the box red with the text somewhere above or below
// saying the same thing. i just dont want the other bases to change for whatever reason

// TODO add optional lock for some box, making it not change when other boxes change
// TODO add copy button
// TODO add popup for symbols like π, φ, etc
// TODO add help menu or something that explains the logic behind number conversion


#[derive(Debug, Clone)]
pub enum BC_Message{
    Nothing,
    TLChanged(String),
    TRChanged(String),
    BLChanged(String),
    BRChanged(String),
    CustomNumChanged(String, usize, u8), // num base index
    CustomBaseChanged(String, u8),       // base index
    CustomBaseAdded,
    CustomBaseRemoved(u8),               // index
}

pub struct UI_BaseConverter{

    side_title: String,
    main_title: String,
    enabled: bool,

    last_msg: RefCell<Option<BC_Message>>,

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
                                    LunaMessage::ShouldUpdate(1) // HACK change to id
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
                        self.tr = convert_number(*b, 2, n);
                        self.bl = convert_number(*b, 8, n);
                        self.br = convert_number(*b, 16, n);
                        self.cbNums = manage_customBoxes(*b, n,
                            self.cbBases.clone(), self.cbCount, *i, true);
                    },
                    BC_Message::CustomBaseChanged(b, i) => { // HACK this is not good code, also doesnt check base
                        self.cbBases[*i as usize] = b.to_string();
                        let base = base_to_num(b.to_string());
                    },
                    BC_Message::CustomBaseAdded => {
                        self.cbCount += 1;
                        self.cbNums.push(String::new());
                        self.cbBases.push(String::new());
                    },
                    BC_Message::CustomBaseRemoved(i) => {
                        self.cbBases.remove(*i as usize);
                        self.cbNums.remove(*i as usize);
                        self.cbCount -= 1;
                    }

                }
            },
            None => (),
        }
    }

    fn layout(&self) -> Container<BC_Message> {

        let title_section = Container::new(
            Text::new(&self.main_title)
                .size(20)
                .center()
                .color(Color::WHITE)
                .wrapping(iced::widget::text::Wrapping::None)
        )
        .center(50)
        .width(iced::Length::Fill)
        .height(iced::Length::FillPortion(1))
        .style(|theme: &Theme| {
            let mut style = container::Style::default()
                .background(Color::from_rgb(0.1, 0.1, 0.1))
                .border(Border::default().rounded(10));
            return style;
        })
        .padding(1);

        // let title_section = 
        //     Text::new(&self.main_title)
        //         .size(30)
        //         .center()
        //         .color(Color::WHITE)
        //         .wrapping(iced::widget::text::Wrapping::None);

        let predef_converters = Container::new(
            column![
                row![
                    container("").width(iced::Length::FillPortion(1)),
                    iced::widget::text_input("Base 10", &self.tl)
                        .on_input(|text| BC_Message::TLChanged(text))
                        .width(iced::Length::FillPortion(2))
                        .line_height(LineHeight::Absolute(iced::Pixels(40.0)))
                        .style(|theme: &Theme, status| {
                            converter_style(theme, status)
                        }),
                    container("").width(iced::Length::FillPortion(1)),
                    iced::widget::text_input("Base 2", &self.tr)
                        .on_input(|text| BC_Message::TRChanged(text))
                        .width(iced::Length::FillPortion(2))
                        .line_height(LineHeight::Absolute(iced::Pixels(40.0)))
                        .style(|theme: &Theme, status| {
                           converter_style(theme, status)
                        }),
                    container("").width(iced::Length::FillPortion(1))
                ].height(iced::Length::FillPortion(2)),

                container("").height(iced::Length::FillPortion(1)),
                
                row![
                    container("").width(iced::Length::FillPortion(1)),
                    iced::widget::text_input("Base 8", &self.bl)
                        .on_input(|text| BC_Message::BLChanged(text))
                        .width(iced::Length::FillPortion(2))
                        .line_height(LineHeight::Absolute(iced::Pixels(40.0)))
                        .style(|theme: &Theme, status| {
                            converter_style(theme, status)
                        }),
                    container("").width(iced::Length::FillPortion(1)),
                    iced::widget::text_input("Base 16", &self.br)
                        .on_input(|text| BC_Message::BRChanged(text))
                        .width(iced::Length::FillPortion(2))
                        .line_height(LineHeight::Absolute(iced::Pixels(40.0)))
                        .style(|theme: &Theme, status| {
                            converter_style(theme, status)
                        }),
                    container("").width(iced::Length::FillPortion(1)),
                ]
                .height(iced::Length::FillPortion(2))
            ]
        )
        .center(50)
        .height(iced::Length::FillPortion(2))
        .width(iced::Length::Fill)
        .style(|theme: &Theme| {
            let mut style = container::Style::default()
                .background(Color::from_rgb8(22,2,54))
                .border(Border::default().rounded(10));
            return style;
        })
        .padding(10);

        let custom_converters = Container::new(
            row![
                container("").width(iced::Length::FillPortion(1)).height(iced::Length::Fill), // TODO theres a `vertical_space` function

                scrollable(column(
                    (0..self.cbCount).map(|i| {
                        let base = self.cbBases.get(i as usize).unwrap();
                        let basestr = "Base ".to_owned() + base;
                        let mut numstr = String::new();
                        if base == "" {
                            numstr = "Input a Base first -->".to_string();
                        }
                        else {
                            numstr = "Base ".to_owned() + base;
                        }
                        let numstr = numstr;

                        let rem_button = button("-").on_press_maybe(
                            if self.cbCount <= 1 {
                                None
                            } else {
                                Some(BC_Message::CustomBaseRemoved(i))
                            }
                        )
                            .width(iced::Length::Fixed(30.0))
                            .height(iced::Length::Fixed(30.0))
                            .style(|theme: &Theme, status| {
                                let palette = theme.extended_palette();

                                let mut style = button::Style { 
                                    background: Some(Color::from_rgb8(43,10,94).into()), 
                                    text_color: Color::from_rgb8(219, 197, 252).into(), 
                                    border: Border::default().rounded(5), 
                                    shadow: iced::Shadow::default()
                                };

                                match status {
                                    button::Status::Active => (),
                                    button::Status::Hovered => style.border = Border::default()
                                        .width(1)
                                        .rounded(5)
                                        .color(Color::from_rgb8(123, 100, 158)),
                                    button::Status::Pressed => {
                                        style.background = Some(Color::from_rgb8(51, 17, 102).into());
                                        style.border = Border::default()
                                            .width(1)
                                            .rounded(5)
                                            .color(Color::from_rgb8(167, 145, 201));
                                    },
                                    button::Status::Disabled => {
                                        style.background = Some(Color::from_rgb8(28, 8, 59).into());
                                        style.text_color = Color::from_rgb8(199, 58, 161).into();
                                        style.border = Border::default()
                                            .width(1)
                                            .rounded(5)
                                            .color(Color::from_rgb8(87, 26, 70));
                                    },
                                }

                                return style;
                            });
                            
                        let add_button = button("+").on_press(BC_Message::CustomBaseAdded)
                            .width(iced::Length::Fixed(30.0))
                            .height(iced::Length::Fixed(30.0))
                            .style(|theme: &Theme, status| {
                                let palette = theme.extended_palette();

                                let mut style = button::Style { 
                                    background: Some(Color::from_rgb8(43,10,94).into()), 
                                    text_color: Color::from_rgb8(219, 197, 252).into(), 
                                    border: Border::default().rounded(5), 
                                    shadow: iced::Shadow::default()
                                };

                                match status {
                                    button::Status::Active => (),
                                    button::Status::Hovered => style.border = Border::default()
                                        .width(1)
                                        .rounded(5)
                                        .color(Color::from_rgb8(123, 100, 158)),
                                    button::Status::Pressed => {
                                        style.background = Some(Color::from_rgb8(51, 17, 102).into());
                                        style.border = Border::default()
                                            .width(1)
                                            .rounded(5)
                                            .color(Color::from_rgb8(167, 145, 201));
                                    },
                                    button::Status::Disabled => (),
                                }
                                
                                return style;
                            });
    
                        row![
                            iced::widget::text_input(&numstr, &self.cbNums.get(i as usize).unwrap()) // TODO convert to text_editor
                                .on_input_maybe(
                                    if base.is_empty() {
                                        None
                                    } else {
                                        Some( move |text| BC_Message::CustomNumChanged(text, base_to_num(base.to_string()), i))
                                    },
                                )
                                .align_x(alignment::Horizontal::Left)
                                .width(iced::Length::FillPortion(7))
                                .line_height(LineHeight::Absolute(iced::Pixels(90.0)))
                                .style(|theme: &Theme, status| {
                                    converter_style(theme, status)
                                }),

                            container("").width(iced::Length::FillPortion(1)),

                            iced::widget::text_input(&basestr, &self.cbBases.get(i as usize).unwrap())
                                .on_input(move |text| BC_Message::CustomBaseChanged(text, i))
                                .align_x(alignment::Horizontal::Right)
                                .width(iced::Length::FillPortion(2))
                                .line_height(LineHeight::Absolute(iced::Pixels(30.0)))
                                .style(|theme: &Theme, status| {
                                    converter_style(theme, status)
                                }),
                            
                            container("").width(iced::Length::FillPortion(1)),
                            add_button,
                            rem_button,

                        ].into()
                    })
                ))
                .width(iced::Length::FillPortion(7))
                .height(iced::Length::FillPortion(5)),

                container("").width(iced::Length::FillPortion(1)).height(iced::Length::Fill),
            ]
        )
        .center(50)
        .width(iced::Length::Fill)
        .height(iced::Length::FillPortion(3))
        .style(|theme: &Theme| {
            let mut style = container::Style::default()
                .background(Color::from_rgb8(22,2,54))
                .border(Border::default().rounded(10));
            return style;
        })
        .padding(1);


        return Container::new(column![
            title_section,
            predef_converters,
            custom_converters
        ])
        .into();
    }
}


pub fn get() -> UI_BaseConverter {

    let ui_bc = UI_BaseConverter {

        side_title: "Base Converter side".to_string(),
        main_title: "Base Converter main".to_string(),
        enabled: true,

        last_msg: None.into(),

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

fn convert_number(from: usize, to: usize, num: &String) -> String {
    if num.is_empty(){
        return String::new();
    }
    return match number_converter::convert_number_base(from, to, num){
        Ok(n) => n,
        Err(_e) => String::from("Invalid Input"),
    };
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

#[inline]
fn converter_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let palette = theme.extended_palette();

    // TODO extract the colors into variables, also for the code above
    // TODO also for the future, extact all style managing into its own widget
    match status {
        text_input::Status::Active => text_input::Style {
            background: Color::from_rgb8(43,10,94).into(),
            border: Border::default()
                .width(1)
                .color(Color::from_rgb(0.1, 0.1, 0.1))
                .rounded(10),
            icon: Color::from_rgb(1.0, 0.0, 0.0).into(),
            placeholder: Color::from_rgb8(123, 96, 166).into(),
            value: Color::from_rgb8(219, 197, 252).into(),
            selection: Color::from_rgba8(143, 111, 191, 0.5).into(),
        },

        text_input::Status::Hovered => text_input::Style {
            background: Color::from_rgb8(50, 11, 110).into(),
            border: Border::default()
                .width(1)
                .color(Color::from_rgb8(123, 100, 158))
                .rounded(10),
            icon: Color::from_rgb(1.0, 0.0, 0.0).into(),
            placeholder: Color::from_rgb8(123, 96, 166).into(),
            value: Color::from_rgb8(219, 197, 252).into(),
            selection: Color::from_rgba8(143, 111, 191, 0.5).into(),
        },

        text_input::Status::Focused => text_input::Style {
            background: Color::from_rgb8(62, 12, 138).into(),
            border: Border::default()
                .width(1)
                .color(Color::from_rgb8(167, 145, 201))
                .rounded(10),
            icon: Color::from_rgb(1.0, 0.0, 0.0).into(),
            placeholder: Color::from_rgb8(123, 96, 166).into(),
            value: Color::from_rgb8(219, 197, 252).into(),
            selection: Color::from_rgba8(143, 111, 191, 0.5).into(),
        },

        text_input::Status::Disabled => text_input::Style {
            background: Color::from_rgb8(28, 8, 59).into(),
            border: Border::default()
                .width(1)
                .color(Color::from_rgb8(87, 26, 70))
                .rounded(10),
            icon: Color::from_rgb(1.0, 0.0, 0.0).into(),
            placeholder: Color::from_rgb8(199, 58, 161).into(),
            value: Color::from_rgb(1.0, 0.0, 0.0).into(),
            selection: Color::from_rgb(1.0, 1.0, 1.0).into(),
        },
    }
}