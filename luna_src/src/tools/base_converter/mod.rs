use std::cell::RefCell;

use luna::number_converter;
use slint::{ ComponentHandle, Model, ModelRc, SharedString, Weak };
use crate::{LunaAppUi, GlobalConversionCallback };


// TODO instead of invalid input on invalid input lmao, make the box red with the text somewhere above or below
// saying the same thing. i just dont want the other bases to change for whatever reason

// TODO add optional lock for some box, making it not change when other boxes change
// TODO add copy button
// TODO add popup for symbols like π, φ, etc
// TODO add help menu or something that explains the logic behind number conversion

pub const VERSION: luna::Version = luna::Version::new(1, 0, 1);

#[derive(Debug, Clone)]
pub enum BC_Message{
    Nothing,
    TLChanged(String),
    TRChanged(String),
    BLChanged(String),
    BRChanged(String),
    CustomNumChanged(String, usize, u8), // num base index
    CustomBaseChanged(String, u8),       // base index
    CustomBaseAdded,
    CustomBaseRemoved(u8),               // index
}

pub struct UI_BaseConverter{

    ui_handle: Weak<LunaAppUi>,

    last_msg: RefCell<Option<BC_Message>>,

    tl: String,
    tr: String,
    bl: String,
    br: String,
    cbCount: u8,
    cbNums: Vec<String>,
    cbBases: Vec<String>,
}

impl UI_BaseConverter {

    pub fn register_widget(ui_handle: Weak<LunaAppUi>) -> Self{

        let mut base_converter = UI_BaseConverter{
            ui_handle,
            last_msg: RefCell::new(None),
            tl: String::new(),
            tr: String::new(),
            bl: String::new(),
            br: String::new(),
            cbCount: 0,
            cbNums: vec![],
            cbBases: vec![],
        };

        base_converter.ui_handle.unwrap().global::<GlobalConversionCallback>().on_request_convert_number({ 
            move | in_nums, in_bases, edited_index, edited_num | {
                let new_numbers: Vec<SharedString> = vec![];

                let in_nums_vec: Vec<SharedString> = in_nums.iter().collect();
                let in_bases_vec: Vec<i32> = in_bases.iter().collect();
                let edited_number = edited_num.to_string();


                let new_numbers: Vec<String> = in_nums_vec.iter().zip(in_bases_vec.iter())
                    .enumerate().map(|(idx, (num, base))| 
                        if idx != edited_index as usize {
                            convert_number(in_bases_vec[edited_index as usize], *base, &edited_number)
                        } else { 
                            edited_number.to_string()
                        }
                    ).collect()
                ;
                
                let new_numbers: Vec<SharedString> = new_numbers.iter().map(
                    |s| SharedString::from(s)
                ).collect();

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