
use std::num::ParseIntError;

const DIGITS: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";

enum ConversionResult <T>{
    ParseError,
    InvalidBase,
    NormalConverted(T),
    CustomConverted(T)
}


pub fn convert_to_decimal(from: u32, num: String) -> ConversionResult<Vec<u32>>{

    /*

    take number and its base
    if base is 2, 8 or 16, use rusts internal method
    if base anything else but <= 36, use our own with the letters
    if base > 36, convert each digit to decimal and return each digit like
        [58] [95] [1] [18]
        
        example with base 16, assuming we had only up to base 10
        number FF26
        [15] [15] [2] [6]
    
    if base isnt integer, we must make our own system (remember that one vid)
    
    */
    let result = if from <= 36 && from >= 2 {
        
        let base: u8 = from as u8;

        let converted_num =  match normal_conversion_to_decimal(base, num) {
            Ok(n) => ConversionResult::NormalConverted(vec![n]),
            Err(e) => ConversionResult::ParseError,
        };
        converted_num

        // number from String -> &str
        // then &str -> number in decimal
        // then number -> char in radix

        // // converts a digit in the given radix to a char like
        // // char::from_digit(4, 10) ==  Some('4');
        // let char =  char::from_digit(
        //
        //     
        //     u32::from_str_radix(
        //         num.as_str(),
        //         from
        //     ).unwrap(), 
        //     from
    }
    else if from > 36 {
        let base = from;
        let converted_num =  match custom_conversion_to_decimal(base, num) {
            Ok(n) => ConversionResult::CustomConverted(n),
            Err(e) => ConversionResult::ParseError,
        };
        converted_num
    }
    else {
        ConversionResult::InvalidBase
    };

    return result;
}

pub fn convert_from_decimal(to:f32, num:String){
    
}

/// Converts a string from a radix to a number.
/// 
/// Radix **must** be between 2 and 36 (inclusive).
/// 
/// ## Arguments
/// * `base` - The base of the input number (between 2 and 36 inclusive)
/// * `num` - The input number
/// 
/// ## Examples
/// 
/// ```rust
/// asserteq!(normal_conversion_to_decimal(16, "e"), Ok(14));
/// 
/// // If radix isnt between 2 and 36 inclusive:
/// asserteq!(normal_conversion_to_decimal(1, "9"), ParseIntError);
/// ```
fn normal_conversion_to_decimal(base: u8, num: String) -> Result<u32, ParseIntError> {
    return u32::from_str_radix(num.as_str(), base.into());
}

fn custom_conversion_to_decimal(base: u32, num: String) -> Result<Vec<u32>, ParseIntError> {
    
}