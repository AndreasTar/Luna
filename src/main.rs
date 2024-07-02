#![allow(unused, dead_code)]
//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release


use eframe::egui;

use math::convert_number_base;
use ui::Luna;

mod math;
mod ui;


fn main() -> eframe::Result<()> {
    // env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0]),
            // optional
            // .with_icon( 
            //     eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
            //         .expect("Failed to load icon"),
            // ),
        ..Default::default()
    };
    eframe::run_native(
        "Luna",
        native_options,
        Box::new(|cc| Box::new(Luna::default())),
    )
}


/*\*
 * the layout i have thought of:
 * 
 * main.rs will launch the app and windows, and also call function to fetch any saved data to restore
 * it will then create the threads, and one of them will be fore UI
 * UI will be stored inside ui/ and will contain whatever it needs to show stuff on screen\
 * also under ui/pages will prolly be each individual page? idk for sure
 * all the tools will be inside tools/ into their own respective subfolder
 * BUT WHO WILL CONNECT THESE TWO?
 * maybe direct connection? aka UI calls tools?
 * do we need middle-man?
*/
