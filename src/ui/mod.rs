mod tool_pages;
mod default_page;
mod all_pages;


use iced::Element;
use iced_aw::{sidebar::SidebarPosition, widget::SidebarWithContent};
pub use tool_pages::ToolPage;

use crate::tools::base_converter::BC_Message;

static mut LUNA_INSTANCE: Luna = Luna{ pages: vec![], active_index: 0 };

pub(crate) struct Luna {
    pages: Vec<Box<dyn ToolPage>>,
    active_index: usize
}

impl Default for Luna {
    fn default() -> Self {
        return Self { 
            pages: all_pages::get_pages(),
            active_index: 0
        }
    }
}

#[derive(Clone, Debug)]
pub enum LunaMessage{
    Nothing,
    PageSelected(usize),
    ShouldUpdate(usize),
    //PageDeactivated(usize),
    //PageActivated(usize),
}

impl Luna {

    pub fn update(&mut self, msg: LunaMessage) {
        match msg {
            LunaMessage::Nothing => (),
            LunaMessage::PageSelected(i) => self.active_index = i,
            LunaMessage::ShouldUpdate(i) => self.pages[self.active_index].update_state(), // HACK change indexing            
        }
    }

    pub fn view(&self) -> Element<LunaMessage>{

        let mut sbwc = SidebarWithContent::new(LunaMessage::PageSelected)
            .sidebar_position(SidebarPosition::Start);

        let mut i = 0;
        for page in self.pages.iter(){
            sbwc = sbwc.push(i, iced_aw::sidebar::TabLabel::Text(page.get_side_title()), page.render());
            i += 1;
        }
        let sbwc = sbwc.set_active_tab(&self.active_index);
        return sbwc.into();

            
    }

    pub fn add_page<T: ToolPage + 'static>(&mut self, tp: T) {
        self.pages.push(Box::new(tp));
    }

    pub fn remove_page(&mut self, tp: Box<dyn ToolPage>) {
        self.pages.remove(
            self.pages.iter().position(|x| x.as_ref() == tp.as_ref()).unwrap()
        );
    }

    pub fn get_page(&self, index: usize) -> &Box<dyn ToolPage> {
        return &self.pages[index];
    }

    pub fn get_active_tool(&self) -> &Box<dyn ToolPage> {
        match self.pages.get(self.active_index){
            Some(p) => p,
            None => todo!(),
        }
    }
}


#[allow(static_mut_refs)]
pub(crate) fn add_toolpage<T: ToolPage + 'static>(tp: T) {
    unsafe { LUNA_INSTANCE.add_page(tp) };
}
#[allow(static_mut_refs)]
pub(crate) fn remove_toolpage(tp: Box<dyn ToolPage>) {
    unsafe  { LUNA_INSTANCE.remove_page(tp) };
}