
mod tool_pages;
mod default_page;

use tool_pages::ToolPage;


pub struct Luna {
    pages: Vec<Box<dyn ToolPage>>,
    active_index: usize
}

impl Default for Luna {
    fn default() -> Self {
        return Self {
            pages: vec![],
            active_index: 0
        };
    }
}

impl eframe::App for Luna {

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {

        // Sidepanel
        egui::SidePanel::left("tools_sidepanel_window")
            .resizable(false)
            .default_width(150.0)
            .show(ctx, |ui| {
                ui.vertical_centered_justified(|ui|{
                    ui.heading("Tools");
                });
                egui::Grid::new("tools_sidepanel_content")
                    .num_columns(2)
                    .striped(true)
                    .spacing([50.0, 5.0])
                    .show(ui, |ui| {
                        self.show_sidebar_pages(ui);
                    });
            });

        // Main window
        egui::CentralPanel::default()
            .show(ctx, |ui|{
                //ui.text_edit_singleline(&mut "text")
                self.show_active_tool(ui)
            });
    }
}

impl Luna {

    pub fn new(cc: &eframe::CreationContext) -> Self {
        let mut l = Self::default();


    }

    fn show_sidebar_pages(&mut self, ui: &mut egui::Ui){
        let mut i: usize = 0;
        let mut active: usize = 0;
        let mut changed: bool = false;
        for page in &mut self.pages{

            ui.label(&page.get_sidebar_title());
            let button = ui.checkbox(&mut page.is_enabled(), "").on_hover_text("description");

            if button.changed(){ 
                active = i;
                changed = true
            }
            ui.end_row();
            i += 1;
        }
        if changed{
            self.pages.get_mut(self.active_index).expect("ERROR").set_enabled(false);
            self.pages.get_mut(active).expect("ERROR").set_enabled(true);
            self.active_index = active;
        }
        
    }

    fn change_active(&mut self, to: &mut impl ToolPage){
        self.pages.get_mut(self.active_index)
            .expect("ERROR")
            .set_enabled(false);
        to.set_enabled(true);
    }

    pub fn add_page(&mut self, tp: impl ToolPage + 'static) {
        self.pages.push(Box::new(tp));
    }

    pub fn remove_page(&mut self, tp: impl ToolPage) {
        // self.pages.remove(
        //     self.pages.iter().position(|x| *x == tp).unwrap()
        // );
        self.pages.remove(tp.get_index());
    }

    pub fn show_active_tool(&self, ui: &mut egui::Ui){
        let page = match self.pages.get(self.active_index){
            Some(p) => p,
            None => todo!("Wrong index"),
        };
        page.show_page(ui);

    }
}


// pub(crate) fn add_toolpage(tp: impl ToolPage + Sized + 'static) {
//     unsafe { LUNA_INSTANCE.add_page(tp) };
// }

// pub(crate) fn remove_toolpage(tp: impl ToolPage + Sized) {
//     unsafe  { LUNA_INSTANCE.remove_page(tp) };
// }