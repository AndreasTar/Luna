#![allow(unused, dead_code)]

use math::convert_number_base;
use slint::{ComponentHandle, SharedString, WindowPosition};

mod math;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let ui = MyWindow::new()?;

    //ui.window().set_position(WindowPosition::Logical(slint::LogicalPosition { x: (1000.0), y: (600.0) }));

    ui.global::<ConversionCallback>().on_request_convert_number({
        let ui_handle = ui.as_weak();

        move |in_from, in_to, in_num:SharedString| {
            if in_num.is_empty() {
                return SharedString::from("");
            }
            let ui = ui_handle.unwrap();
            let number = convert_number_base(in_from as u32, in_to as u32, in_num.to_string());

            return number.into();
        }
    });

    ui.run()
}




/*\*
 * the layout i have thought of:
 * 
 * UI will have many buttons and pages etc
 * those will be handled by the main
 * the main will call the respective general module files
 * the module files will call the sub-modules to process
 * they will process, and return the result back up to UI
*/
