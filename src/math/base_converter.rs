
use std::num::ParseIntError;

const DIGITS: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";



pub enum ConversionResult <T>{
    ParseError,
    Converted(T),
}


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
/// asserteq!(to_decimal(16, "e"), Ok(14));
/// 
/// // If radix isnt between 2 and 36 inclusive:
/// asserteq!(to_decimal(1, "9"), ParseIntError);
/// ```
pub fn to_decimal(from: u8, num: String) -> ConversionResult<u32> {

    return match u32::from_str_radix(num.as_str(), from.into()) {
        Ok(n) => ConversionResult::Converted(n),
        Err(e) => ConversionResult::ParseError,
    };
}

/// Converts a string from a decimal number to a number in the given radix.
/// 
/// Radix **must** be between 2 and 36 (inclusive).
/// 
/// ## Arguments
/// * `to` - The base the output number should be in (between 2 and 36 inclusive)
/// * `num` - The input number
/// 
/// ## Examples
/// 
/// ```rust
/// asserteq!(from_decimal(16, "17"), Ok("f2"));
/// 
/// // If radix isnt between 2 and 36 inclusive:
/// asserteq!(from_decimal(1, "9"), ParseIntError);
/// ```
pub fn from_decimal(to: u8, num:u32) -> ConversionResult<Vec<String>> {

    let mut result = vec![];
    let mut number = num;
    let radix = to as u32;

    loop {
        let digit = number % radix;
        number = number / radix;

        // will panic if you use a bad radix (< 2 or > 36).
        result.push( 
            match char::from_digit(digit, radix) {
                Some(c) => c.to_string(),
                None => return ConversionResult::ParseError,
            }.to_string()
        );
        if number == 0 {
            break;
        }
    }
    let result = result.into_iter().rev().collect::<Vec<String>>();
    return ConversionResult::Converted(result);
}

pub fn custom_to_decimal(){

}

pub fn custom_from_decimal(){

}

// TODO temporary implementation. doesnt work correctly
fn custom_conversion_to_decimal(base: u32, num: String) -> Result<Vec<u32>, ParseIntError> {
    return match u32::from_str_radix(num.as_str(), base.into()) {
        Ok(n) => Ok(vec![n]),
        Err(e) => Err(e),
    };
}