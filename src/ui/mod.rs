
struct Luna {
    pages: Vec<ToolPages>,
}

impl Default for Luna {
    fn default() -> Self {
        return Self { 
            pages: () 
        }
    }
}

impl eframe::App for Luna {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        
    }
}