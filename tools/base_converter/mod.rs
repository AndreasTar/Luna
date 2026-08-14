
use std::cell::RefCell;
use luna::number_converter;
use luna_core::ToolManifest;
use slint::{ ComponentHandle, Model, ModelRc, SharedString, Weak };
use crate::tools::{ BoundTool, ToolView };
use crate::{ Global_NumberConversion_Callback, LunaAppUi };


// TODO instead of invalid input on invalid input lmao, make the box red with the text somewhere above or below
// saying the same thing. i just dont want the other bases to change for whatever reason

// TODO add optional lock for some box, making it not change when other boxes change
// TODO add copy button
// TODO add popup for symbols like π, φ, etc
// TODO add help menu or something that explains the logic behind number conversion

pub const VERSION: luna::Version = luna::Version::new(1, 0, 1);


pub struct Tool{

    ui_handle: Weak<LunaAppUi>,

    cbCount: u8,
    cbNums: Vec<String>,
    cbBases: Vec<String>,
}

impl BoundTool for Tool {
    fn tool_id(&self) -> &'static str {
        return "luna.base_converter";
    }
}

impl ToolView for Tool {

    fn manifest() -> ToolManifest {
        return ToolManifest::from_toml(include_str!("manifest.toml"))
            .expect("base_converter manifest.toml is malformed");
    }

    fn bind(ui_handle: Weak<LunaAppUi>) -> Self {

        let mut base_converter = Tool {
            ui_handle,
            cbCount: 0,
            cbNums: vec![],
            cbBases: vec![],
        };

        base_converter.ui_handle.unwrap().global::<Global_NumberConversion_Callback>().on_request_convert_number({ 
            move | in_nums, in_bases, edited_index, edited_num | {
                let new_numbers: Vec<SharedString> = vec![];

                let in_nums_vec: Vec<SharedString> = in_nums.iter().collect();
                let in_bases_vec: Vec<i32> = in_bases.iter().collect();
                let edited_number = edited_num.to_string();


                let new_numbers: Vec<SharedString> = in_nums_vec.iter().zip(in_bases_vec.iter())
                    .enumerate().map(|(idx, (num, base))| 
                        if idx != edited_index as usize {
                            SharedString::from(convert_number(in_bases_vec[edited_index as usize], *base, &edited_number))
                        } else { 
                            SharedString::from(edited_number.to_string())
                        }
                    ).collect()
                ;

                return ModelRc::from(new_numbers.as_slice());
            }
        });

        return base_converter;
    }
}


#[inline]
fn convert_number(from: i32, to: i32, num: &String) -> String {
    if num.is_empty(){
        return String::new();
    }
    let from: usize = from.try_into().unwrap();
    let to: usize = to.try_into().unwrap();
    return match number_converter::convert_number_base(from, to, num){
        Ok(n) => n,
        Err(_e) => String::from("Invalid Input"),
    };
}

// fn manage_customBoxes(from: usize, num: &String, cbBases: Vec<String>, cbCount: u8, currentlyAt: u8, fromCustom: bool) -> Vec<String> {
//
//     let mut newNums: Vec<String> = vec![];
//     for i in 0..cbCount{
//         if fromCustom {
//             if i == currentlyAt { 
//                 newNums.push(num.to_string()); 
//                 continue; 
//             }
//         };
//
//         let base = cbBases.get(i as usize).unwrap();
//         if !(base.is_empty() || base.parse::<u8>().is_err()) {
//             let cbBase: usize = u32::from_str_radix(base, 10).unwrap().try_into().unwrap();
//             newNums.push(convert_number(from, cbBase, num));
//         } else {
//             newNums.push(String::new());
//         }
  //      
//     }
//     return newNums;
// }

#[inline]
fn base_to_num(base: String) -> usize { // TODO change to float or double etc
    if !(base.is_empty() || base.parse::<u8>().is_err()) {
        return u32::from_str_radix(&base, 10).unwrap().try_into().unwrap();
    } else {
        return 0;
    }
}