use crate::tools::*;
use iced::{widget::{image, Text}};
use iced_aw::{menu::{self, Item, Menu}, menu_bar, menu_items, MenuBar};

pub enum IM_Message{
    Nothing, // TODO is nothing really needed? or just use None?
}

pub enum Layer {
    // TODO add layer types and their values
    Brighten(i32),
    Contrast(f32),
    Dither, // TODO
    Grayscale,
    Invert,
    Blur(f32),
    FastBlur(f32),
    Unsharpen(f32, i32),
    Sharpen, // TODO
    HueRotate(i32)
}

pub struct UI_ImgManipulator {

    side_title: String,
    main_title: String,
    enabled: bool,
    last_msg: RefCell<Option<IM_Message>>,

    // TODO add image buffer and layers
    layers: Vec<(Layer, bool)>, // holds the layers and their on-off toggle


}

impl ToolPage for UI_ImgManipulator {
    
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
                                IM_Message::Nothing => LunaMessage::Nothing,
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

impl UI_ImgManipulator {
    pub fn update_state(&mut self) {
        match &self.last_msg.take() {
            Some(msg) => match msg {
                    _ => todo!()
            },
            None => (),
        }
    }

    fn layout(&self) -> Container<IM_Message> {

        /*
        top bar
        ---------------------------------------
        image section     | layer section
        og_btn   |   info | layer options
        ---------------------------------------
        bottom section
        */
        
        // TODO add file selector windows thingy
        // TODO add menu items

        // for saving and loading and stuff
        let top_bar = MenuBar::new(vec![
            Item::new("Save"),
            Item::new("Load"),
        ])
        .height(Length::Fixed(20.0))
        .width(Length::Fill)
        .padding(0); // BUG slight overshoot on the left over the sidebar, prolly needs .style()

        // holds the image and info like pixels and format
        let image_preview = Container::new(
            self::column![
                Text::new("Image preview"), // TODO add image preview
                Text::new("Image info"), // TODO add image info
            ])
            .width(Length::FillPortion(4))
            .height(Length::FillPortion(4)
        );

        // holds the layers and their on-off toggle and possibly their value
        let layers = Container::new(
            self::column![
                Text::new("Layers"), // TODO add layers
                Text::new("Layer options"), // TODO add layer info
            ])
            .width(Length::FillPortion(1))
            .height(Length::FillPortion(4)
        ); 

        // hold the sliders and whatnot for the selected layer
        //let layer_options = todo!(); 

        // holds the image and the layers
        let mid_section = self::row![ 
            image_preview,
            layers,
        ];


        let bottom_section = Text::new("Section"); // no idea what to put here yet


        return Container::new(self::column![
            top_bar,
            mid_section,
            bottom_section,
        ]).into();
    }
}

pub fn get() -> UI_ImgManipulator {
    return UI_ImgManipulator { 

        side_title: "Image Manipulator side".to_string(),
        main_title: "Image Manipulator main".to_string(),
        enabled: true,
        last_msg: RefCell::new(None),
        layers: vec![], 
    };
}