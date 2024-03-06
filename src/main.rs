#![allow(unused, dead_code)]

use core::num;
use std::vec;

use math::convert_number_base;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel, WindowPosition};

mod math;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let ui = MyWindow::new()?;

    //ui.window().set_position(WindowPosition::Logical(slint::LogicalPosition { x: (1000.0), y: (600.0) }));

    ui.global::<ConversionCallback>().on_request_convert_number({
        let ui_handle = ui.as_weak();

        // move |in_from, in_to, in_num:SharedString| {
        //     if in_num.is_empty() {
        //         return SharedString::from("");
        //     }

        //     let ui = ui_handle.unwrap();
        //     let number = convert_number_base(in_from as u32, in_to as u32, in_num.to_string());

        //     return number.into();
        // }

        move |in_numbers, in_bases, in_index, in_num| {

            let mut out_numbers: Vec<SharedString> = vec![];
            for (i, base) in in_bases.iter().enumerate() {
                //if i == in_index as usize { continue; }

                out_numbers.insert(
                    i, 
                    { if (i == in_index as usize) { 
                                in_num.clone()
                            } else { 
                                convert_number_base(
                                    in_bases.iter().nth(in_index as usize).unwrap() as u32,
                                    base as u32,
                                    in_num.clone().to_string()
                                ).into()
                        }
                    }
                );
            }

            return out_numbers.as_slice().into();
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
