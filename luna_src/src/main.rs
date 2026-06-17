#![allow(unused, dead_code, non_snake_case, non_camel_case_types)]
//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use slint;

mod tools;
pub mod helpers;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    // env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    // TODO add tests for each ui page (tools should have doctests ready)
    // TODO add example and info etc page for each tool (maybe widget it or something)
    // TODO also all calls to luna_lib need to be asyncronous, so we can use them in the UI without blocking the main thread
    // TODO fix imports and uses etc into one prolly, most likely the tools/mod.rs file

    let luna_app_ui = LunaAppUi::new()?;

    let widgets_vec = vec![
        tools::base_converter::UI_BaseConverter::register_widget(luna_app_ui.as_weak()),
        // tools::img_manipulator::ImgManipulator::new(test.clone()),
    ];

    return luna_app_ui.run();
}