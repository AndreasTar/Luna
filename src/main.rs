#![allow(unused, dead_code)]

mod math;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;

    ui.on_request_increase_value({
        let ui_handle = ui.as_weak();
        move || {
            let ui = ui_handle.unwrap();
            ui.set_counter(ui.get_counter() + 1);
        }
    });

    ui.run()
}




/*\*
 * the layout i have thought of:
 * 
 * UI will have many buttons and pages etc
 * those will be handles by the main
 * the main will call the respective general module files
 * the module files will call the sub-modules to process
 * they will process, and return the result back up to UI
*/
