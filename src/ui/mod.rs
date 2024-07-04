
mod tool_pages;

use tool_pages::ToolPages;

pub struct Luna {
    pages: Vec<ToolPages>,
}

impl Default for Luna {
    fn default() -> Self {
        return Self { 
            pages: vec![ToolPages::new("temp", "test").set_enabled(true)]
        }
    }
}

impl eframe::App for Luna {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
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
    }
}

impl Luna {
    fn show_sidebar_pages(&self, ui: &mut egui::Ui){
        for page in &self.pages{
            ui.label(&page.sidebar_name);
            let mut temp = page.enabled;
            // ui.checkbox(&mut temp, "Enabled?").without_text(true);
            // ui.add(egui::Checkbox::without_text(&mut temp));
            ui.add_enabled(temp, egui::Checkbox::without_text(&mut temp));
            ui.end_row()

        }
    }
}