#![allow(unused, dead_code)]

use core::num;
use std::vec;

use math::convert_number_base;

mod math;


fn main(){
    return;
}


/*\*
 * THIS NEEDS UPDATING FOR EGUI AND EFRAME
 * the layout i have thought of:
 * 
 * UI will have many buttons and pages etc
 * those will be handled by the main
 * the main will call the respective general module files
 * the module files will call the sub-modules to process
 * they will process, and return the result back up to UI
*/
