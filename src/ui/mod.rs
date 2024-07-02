
pub struct Luna {
    //pages: Vec<ToolPages>,
}

impl Default for Luna {
    fn default() -> Self {
        return Self { 
            //pages: () 
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
            });
    }
}