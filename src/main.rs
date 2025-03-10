#![allow(unused, dead_code, non_snake_case, non_camel_case_types)]
//#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release


use iced::{self, Theme};

use ui::Luna;

mod tools;
mod ui;
pub mod helpers;

fn main() -> iced::Result {
    // env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    iced::application("Default Title", Luna::update, Luna::view)
        .theme(|_| Theme::Dark)
        .window_size(iced::Size::new(500.0, 300.0))
        .transparent(false)
        .resizable(true)
        .run()
}


/*\*
 * the layout i have thought of:
 * 
 * main.rs will launch the app and windows, and also call function to fetch any saved data to restore
 * it will then create the threads, and one of them will be for UI
 * UI will be stored inside ui/ and will contain whatever it needs to show stuff on screen\
 * also under ui/pages will prolly be each individual page? idk for sure
 * all the tools will be inside tools/ into their own respective subfolder
 * BUT WHO WILL CONNECT THESE TWO?
 * maybe direct connection? aka UI calls tools?
 * do we need middle-man?
*/

/*\*
 * New layout
 * 
 * under tools/ there will be folders of each tool, including its logic and ui
 * then ui will be handlers and stuff for making the ui easier for me
 * and there will also be a third folder for other stuff, like handlers for loading saving etc
 * that arent tool specific
 * 
 * page 0 will always be there, as a failsafe, and cant be deactivated
 */
