use crate::tools::*;
use iced::widget::Text;

pub enum IM_Message{
    Nothing,
}

pub struct UI_ImgManipulator {

    side_title: String,
    main_title: String,
    enabled: bool,

    last_msg: RefCell<Option<IM_Message>>,

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
        // TODO implement layout
        return Container::new(Text::new("TODO implement layout"));
    }
}

pub fn get() -> UI_ImgManipulator {
    return UI_ImgManipulator { 

        side_title: "Image Manipulator side".to_string(),
        main_title: "Image Manipulator main".to_string(),
        enabled: true,
        last_msg: RefCell::new(None), 
    };
}