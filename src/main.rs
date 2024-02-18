#![allow(unused, dead_code)]

use math::convert_number_base;
use slint::SharedString;

mod math;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let ui = MyWindow::new()?;

    ui.on_request_convert_number({
        let ui_handle = ui.as_weak();
        move |in_from, in_to, in_num:SharedString| {
            let ui = ui_handle.unwrap();
            let number = convert_number_base(in_from as u32, in_to as u32, in_num.to_string());
            if number.starts_with("Invalid"){
                return SharedString::from("");
            }
            println!("In : {} {} {} Out {}", in_from,in_to,in_num, number);
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
