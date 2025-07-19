use std::io::Read;

use crate::tools::*;
use iced::{widget::{image as iced_image, Text, button}, advanced};
use iced_aw::{menu::{self, Item, Menu}, menu_bar, menu_items, MenuBar};

use image::DynamicImage;
use luna::img_manipulator as luna_imgman;
use rfd::FileDialog;

#[derive(Debug, Clone)]
pub enum IM_Message{
    Nothing, // TODO is nothing really needed? or just use None?
    Request_LoadImage,
    Request_SaveImage,
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

    // TODO add image buffer and layers functionality
    layers: Vec<(Layer, bool)>, // holds the layers and their on-off toggle
    og_image: Option<DynamicImage>, // holds the original image, if any
    res_image: Option<DynamicImage>, // holds the resulting image after changes, if any


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
                                    LunaMessage::ShouldUpdate(2) // HACK change to id
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
            Some(msg) => 
                match msg {
                    IM_Message::Nothing => (),
                    IM_Message::Request_LoadImage => {
                        load_image_rfd().map(|path| {
                            match luna_imgman::open_image_from_path(path) {
                                luna_imgman::ImgOpenResult::Success(img) => {
                                    self.og_image = Some(img);
                                    self.res_image = self.og_image.clone(); // TODO do we need this?
                                },
                                luna_imgman::ImgOpenResult::Failure(e) => {
                                    eprintln!("Failed to load image: {}", e); // HACK change eprintln to show error on screen inside image preview or something
                                }
                            }
                        }).unwrap_or_else(|e| {
                            eprintln!("Error loading image: {}", e); // HACK change eprintln to show error on screen inside image preview or something
                        });
                    },
                    IM_Message::Request_SaveImage => (),
                    _ => todo!()
            },
            None => (),
        }
    }

    fn layout(&self) -> Container<IM_Message> {

        /*
        tab (if you wanna process multiple images at once))
        top bar
        ---------------------------------------
        image section     | layer section
        og_btn   |   info | layer options
        ---------------------------------------
        bottom section
        */
        // or optionally edited and og side by side
        
        // TODO add file selector windows thingy
        // TODO add menu items
        // TODO either disable zoom or add fit to screen buttons etc

        let menu_item = |items| Menu::new(items).max_width(180.0).offset(0.0).spacing(5.0);

        // for saving and loading and stuff
        let top_bar = MenuBar::new(menu_items![
            (button("File").on_press(IM_Message::Nothing), {
                menu_item(menu_items!(
                    (button("Save").on_press(IM_Message::Request_SaveImage).width(Length::Fill))
                    (button("Load").on_press(IM_Message::Request_LoadImage).width(Length::Fill))
                    (button("Exit").on_press(IM_Message::Nothing).width(Length::Fill)) // TODO add exit functionality
                ))
            })

        ])
        .height(Length::Fixed(28.0))
        .width(Length::Fill)
        .padding(0); // BUG slight overshoot on the left over the sidebar, prolly needs .style()




        let mut img_info = (0, 0);

        let img_rgba = match &self.res_image {
            Some(img) => { 
                img_info = (img.width(), img.height()); // TODO add more info like format, bytesize, etc

                luna_imgman::into_rgba8(img)
            },
            None => {
                vec![0_u8; 0]
            },
        };

        let img_handle = advanced::image::Handle::from_rgba(
            img_info.0, 
            img_info.1, 
            img_rgba
        );

        let img_info_text = format!("{}x{}", img_info.0, img_info.1); // TODO Check for 0 and do NA or something

        // show final image, even if it is the same as the original
        // holds the image and info like pixels and format
        let image_preview = Container::new(
            self::column![
                Text::new("Image preview"), // TODO add image previewl
                iced_image::viewer(img_handle)
                    .filter_method(iced_image::FilterMethod::Nearest) // TODO add button for nearest or linear
                    .width(Length::Fill)
                    .height(Length::Fill),
                Text::new(img_info_text), // TODO add image info
            ])
            .width(Length::FillPortion(4))
            .height(Length::FillPortion(4));






        // holds the layers and their on-off toggle and possibly their value
        let layers = Container::new(
            self::column![
                Text::new("Layers").height(Length::FillPortion(1)), // TODO add layers
                Text::new("Layer options").height(Length::FillPortion(1)), // TODO add layer info
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
        og_image: None,
        res_image: None, 
    };
}

// TODO add load image by drag and drop

fn load_image_rfd() -> Result<String, String> {
    let file =  FileDialog::new()
        .set_title("Open Image")
        .add_filter("Images", &["png", "jpg", "jpeg", "gif", "bmp"])
        .set_directory(std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")))
        .pick_file();

    return file.map(|file| file.as_path().to_string_lossy().to_string()) // HACK make it not lossy
        .ok_or_else(|| "No file selected".to_string()); // NOTE can be used to present error to user

}

       