#![allow(unused, dead_code, non_snake_case, non_camel_case_types)]
//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release


use iced::{self, Theme};

use ui::Luna;

mod tools;
mod ui;
pub mod helpers;

fn main() -> iced::Result {
    // env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    // TODO add tests for each ui page (tools should have doctests ready)
    // TODO add example and info etc page for each tool (maybe widget it or something)
    // TODO also all calls to luna_lib need to be asyncronous, so we can use them in the UI without blocking the main thread
    // TODO fix imports and uses etc into one prolly, most likely the tools/mod.rs file

    iced::application("Default Title", Luna::update, Luna::view)
        .theme(|_| Theme::Dark)
        .window_size(iced::Size::new(500.0, 300.0)) // TODO need to set minimums
        .transparent(false)
        .resizable(true)
        .run()
}