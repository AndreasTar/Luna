mod tool_pages;
mod default_page;
mod all_pages;

pub use tool_pages::ToolPage;

static mut LUNA_INSTANCE: Luna = Luna{ pages: vec![], active_index: 0 };

pub(crate) struct Luna {
    pages: Vec<ToolPage>,
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
                // NOTE for some reason, RichText gets flagged as a Trojan by the AV
                //ui.text_edit_singleline(&mut "text")
                // ui.label(RichText::new(
                //     self.get_active_tool().main_title.clone()).size(20.0)
                // );
                self.show_active_tool(ui)
            });
    }
}

impl Luna {

    fn show_sidebar_pages(&mut self, ui: &mut egui::Ui){
        let mut i: usize = 0;
        let mut active: usize = 0;
        let mut changed: bool = false;
        for page in &mut self.pages{

            ui.label(&page.side_title);
            let button = ui.checkbox(&mut page.enabled, "").on_hover_text("description");

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

    fn change_active(&mut self, to: &mut ToolPage){
        
        // check out radio values like ui.radio_value()
        self.pages.get_mut(self.active_index)
            .expect("ERROR")
            .set_enabled(false);
        to.set_enabled(true);
    }

    pub fn add_page(&mut self, tp: ToolPage) {
        self.pages.push(tp);
    }

    pub fn remove_page(&mut self, tp: ToolPage) {
        self.pages.remove(
            self.pages.iter().position(|x| *x == tp).unwrap()
        );
    }

    pub fn show_active_tool(&self, ui: &mut egui::Ui){
        let page = self.get_active_tool();
        page.show_page(ui);

    }

    pub fn get_active_tool(&self) -> &ToolPage {
        match self.pages.get(self.active_index){
            Some(p) => p,
            None => todo!(),
        }
    }
}


pub(crate) fn add_toolpage(tp: ToolPage) {
    unsafe { LUNA_INSTANCE.add_page(tp) };
}

pub(crate) fn remove_toolpage(tp: ToolPage) {
    unsafe  { LUNA_INSTANCE.remove_page(tp) };
}