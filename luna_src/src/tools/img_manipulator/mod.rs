use std::io::Read;

use crate::tools::*;
use iced::{advanced::{self, graphics::layer}, widget::{button, image as iced_image, scrollable, Text, pick_list}};
use iced_aw::{menu::{self, Item, Menu}, menu_bar, menu_items, MenuBar};

use image::DynamicImage;
use luna::img_manipulator as luna_imgman;
use rfd::FileDialog;

const VERSION: luna::Version = luna::Version::new(0, 2, 1);

#[derive(Debug, Clone)]
pub enum IM_Message{
    Nothing, // TODO is nothing really needed? or just use None?
    Request_LoadImage,
    Request_SaveImage,
    Request_ClearImage,
    Request_ToggleLayer(usize),
    Request_AddLayer(Layer), // TODO add layer
    Request_RemoveLayer(usize), // TODO remove layer
}

#[derive(Debug, Clone, PartialEq)]
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
    HueRotate(i32),
    Flip_Horizontal,
    Flip_Vertical,
}

impl Layer {
    const ALL: [Layer; 9] = [
        Layer::Brighten(0),
        Layer::Contrast(0.0),
        Layer::Dither,
        Layer::Grayscale,
        Layer::Invert,
        Layer::Blur(0.0),
        Layer::FastBlur(0.0),
        Layer::Unsharpen(0.0, 0),
        Layer::HueRotate(0),
    ];

    pub fn as_str(&self) -> &str {
        match self {
            Layer::Brighten(_)     => "Brighten",
            Layer::Contrast(_)     => "Contrast",
            Layer::Dither          => "Dither",
            Layer::Grayscale       => "Grayscale",
            Layer::Invert          => "Invert",
            Layer::Blur(_)         => "Blur",
            Layer::FastBlur(_)     => "Fast Blur",
            Layer::Unsharpen(_,_)  => "Unsharpen",
            Layer::Sharpen         => "Sharpen",
            Layer::HueRotate(_)    => "Hue Rotate",
            Layer::Flip_Horizontal => "Flip Horizontal",
            Layer::Flip_Vertical   => "Flip Vertical",
        }
    }
}

impl std::fmt::Display for Layer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "{}", self.as_str());
    }
}

pub struct UI_ImgManipulator {

    side_title: String,
    main_title: String,
    enabled: bool,
    last_msg: RefCell<Option<IM_Message>>,


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
    
    fn version(&self) -> luna::Version {
        return VERSION;
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
                                    self.update_image();
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
                    IM_Message::Request_ToggleLayer(i) => {
                        if let Some(layer) = self.layers.get_mut(*i) {
                            layer.1 = !layer.1; // toggle the layer on/off
                        };
                        self.update_image();
                    },
                    IM_Message::Request_AddLayer(layer) => {
                        self.layers.push((layer.clone(), true)); // add layer enabled
                        self.update_image();
                    }, 
                    IM_Message::Request_RemoveLayer(i) => {
                        if *i < self.layers.len() {
                            self.layers.remove(*i); // remove layer
                        };
                        self.update_image();
                    },
                    _ => todo!()
            },
            None => (),
        }
    }
    
    fn update_image(&mut self) {
        match self.apply_edit_layers(self.og_image.clone()) { // TODO this can be optimized, apply only the new layers instead of everything every frame
            Some(img) => self.res_image = self.apply_edit_layers(self.og_image.clone()),
            None => (),
        };
    }

    fn layout(&self) -> Container<IM_Message> {

        /*
        tab (if you wanna process multiple images at once)
        top bar
        ---------------------------------------
        image section     | layer section
        og_btn   |   info | layer options
        ---------------------------------------
        bottom section
        */

        // NOTE what will the bottom section be?
        // maybe you can open a folder and it shows all images in it, and you can select one to edit?
        // or maybe contains the layers and stuff? that may be an add button instead

        
        // TODO add menu items (save layers etc)
        // TODO either disable zoom or add fit to screen buttons etc
        // TODO add load image by drag and drop
        // TODO add 'show original' button AND/OR split view with original and edited side by side, with extra toggle to match movement of the two or not
        // TODO add image buffer and layers functionality
        // TODO style needs changing for everything

        // -------------------------------- FOR TOP MENU BAR --------------------------------

        let menu_item = |items| Menu::new(items).max_width(180.0).offset(0.0).spacing(5.0);

        // for saving and loading and stuff
        let top_bar = MenuBar::new(menu_items![
            (button("File").on_press(IM_Message::Nothing), {
                menu_item(menu_items!(
                    (button("Save").on_press(IM_Message::Request_SaveImage).width(Length::Fill))
                    (button("Load").on_press(IM_Message::Request_LoadImage).width(Length::Fill))
                    (button("Clear").on_press(IM_Message::Nothing).width(Length::Fill)) // TODO add clear functionality, reseting the image and layers
                ))
            })

        ])
        .height(Length::Fixed(28.0))
        .width(Length::Fill)
        .padding(0); // BUG slight overshoot on the left over the sidebar, prolly needs .style()


        // -------------------------------- FOR IMAGE PREVIEW --------------------------------

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
                Text::new("Image preview"), // TODO make it look better, as a title or something
                iced_image::viewer(img_handle)
                    .filter_method(iced_image::FilterMethod::Nearest) // TODO add button for nearest or linear
                    .width(Length::Fill)
                    .height(Length::Fill),
                Text::new(img_info_text), // TODO add more image info
            ])
            .width(Length::FillPortion(4))
            .height(Length::FillPortion(4));




        // -------------------------------- FOR LAYERS --------------------------------

        // holds the layers and their on-off toggle and possibly their value

        let selected_layer: Option<Layer> = None;
        let pick_layer_list = pick_list( 
                        Layer::ALL,
                        selected_layer,
                        IM_Message::Request_AddLayer,
        );
        
        let layers = Container::new(
            self::column![
                
                row![
                    Text::new("Layers").width(Length::FillPortion(3)).height(Length::Fill),
                    pick_layer_list
                        .width(Length::FillPortion(1))
                ].height(Length::FillPortion(1)),
                
                scrollable(column(
                    (0..self.layers.len()).map(|i| {
                        let layer = &self.layers[i];
                        let layer_name = layer.0.as_str();

                        button(layer_name)
                            .on_press(IM_Message::Request_ToggleLayer(i)) // TODO add functionality to toggle layer on/off
                            .width(Length::Fill)
                            .into()
                    })
                ))
                
                .height(Length::FillPortion(4)),

                
                Text::new("Layer options").height(Length::FillPortion(4)), // TODO add layer info
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


        let bottom_section = Text::new("Section"); // TODO no idea what to put here yet


        return Container::new(self::column![
            top_bar,
            mid_section,
            bottom_section,
        ]).into();
    }

    fn apply_edit_layers(&self, og_img: Option<DynamicImage>) -> Option<DynamicImage> {
        if og_img.is_none() {
            return None; 
        }

        let mut img = og_img.unwrap();

        for layer in self.layers.iter() {
            if layer.1 == false { continue; } // skip if layer is off

            match &layer.0 {
                Layer::Brighten(amount) => luna_imgman::brighten(&mut img, *amount),
                Layer::Contrast(amount) => luna_imgman::contrast(&mut img, *amount),
                Layer::Dither => todo!(),
                Layer::Grayscale => luna_imgman::grayscale(&mut img),
                Layer::Invert => luna_imgman::invert(&mut img),
                Layer::Blur(amount) => luna_imgman::blur(&mut img, *amount),
                Layer::FastBlur(amount) => luna_imgman::fast_blur(&mut img, *amount),
                Layer::Unsharpen(value, thresh) => luna_imgman::unsharpen(&mut img, *value, *thresh),
                Layer::Sharpen => todo!(),
                Layer::HueRotate(degrees) => luna_imgman::huerotate(&mut img, *degrees),
                Layer::Flip_Horizontal => luna_imgman::flip_horizontal(&mut img),
                Layer::Flip_Vertical => luna_imgman::flip_vertical(&mut img),
            }
        };

        return Some(img);
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


fn load_image_rfd() -> Result<String, String> {
    let file =  FileDialog::new()
        .set_title("Open Image")
        .add_filter("Images", &["png", "jpg", "jpeg", "gif", "bmp"])
        .set_directory(std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")))
        .pick_file();

    return file.map(|file| file.as_path().to_string_lossy().to_string()) // HACK make it not lossy
        .ok_or_else(|| "No file selected".to_string()); // NOTE can be used to present error to user

}



       