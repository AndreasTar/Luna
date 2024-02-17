#![allow(unused, dead_code)]

use math::convert_number_base;

mod math;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let ui = MyWindow::new()?;

    ui.on_request_convert_number({
        let ui_handle = ui.as_weak();
        move |in_num| {
            let ui = ui_handle.unwrap();
            let number = convert_number_base(10, 16, in_num.to_string());
            ui.set_converted(number.into());
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
