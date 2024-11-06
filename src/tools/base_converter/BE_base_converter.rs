use core::num;

pub enum ConversionResult <T>{
    ParseError,
    Converted(T),
}

const VALUES: [&str; 36] = ["0","1","2","3","4","5","6","7","8","9","A","B","C","D","E","F","G","H","I","J","K","L","M","N","O","P","Q","R","S","T","U","V","W","X","Y","Z"];

pub enum InvalidBase{}

pub fn convert_number_base(from: usize, to: usize, num: &String) -> String {

    /*
    take number and its base
    if base  <= 36, use our own with the letters
    if base > 36, convert each digit to decimal and return each digit like
        [58] [95] [1] [18]
        
        example with base 16, assuming we had only up to base 10
        number FF26
        [15] [15] [2] [6]
    
    if base isnt integer, we must make our own system (remember that one vid)
    */

    // check if bases are between currently implemented ones
    // if they are, get string, seperate into each digit, and get the int value of it
    //      for example, in hex, num ef would become [14][15]
    // then convert each digit and add em together
    // if output base is <= 36, use proper representation
    // if its > 36, then output each number seperately in base 10 (or base 36 optionally?)


    let result: ConversionResult<Vec<String>> = match (from, to) {
        (..=1, ..=1)        => return String::from("Invalid Base"), // TODO an error?
        (2..=36, 2..=36)    => 'case: {
            let res1 = match convert_to_decimal(from, num) {
                ConversionResult::ParseError => break 'case ConversionResult::ParseError,
                ConversionResult::Converted(n) => n,
            };
            convert_from_decimal(to.into(), res1)
        },
        (_, _)              => todo!()
    };

    
    match result {
        ConversionResult::ParseError => String::from("Invalid Input"),
        ConversionResult::Converted(n) => n.join(""),
    }
}



/// Converts a string from a radix to a number.
/// 
/// Radix **must** be an integer between 2 and 36 (inclusive).
/// 
/// ## Arguments
/// * `base` - The radix of the input number (between 2 and 36 inclusive)
/// * `num` - The input number
/// 
/// ## Examples
/// 
/// ```rust
/// asserteq!(to_decimal(16, "e"), Ok(14));
/// 
/// // If radix isnt between 2 and 36 (inclusive):
/// asserteq!(to_decimal(1, "9"), ParseError);
/// ```
pub fn convert_to_decimal(from: usize, num: &String) -> ConversionResult<u32> {

    return match u32::from_str_radix(num.as_str(), from.try_into().unwrap()) {
        Ok(n) => ConversionResult::Converted(n),
        Err(e) => ConversionResult::ParseError,
    };
}

/// Converts a string from a decimal number to a number in the given radix.
/// 
/// Radix **must** be an integer between 2 and 36 (inclusive).
/// 
/// ## Arguments
/// * `to` - The radix of the output number (between 2 and 36 inclusive)
/// * `num` - The input number
/// 
/// ## Examples
/// 
/// ```rust
/// asserteq!(from_decimal(16, "17"), Ok("f2"));
/// 
/// // If radix isnt between 2 and 36 (inclusive):
/// asserteq!(from_decimal(1, "9"), ParseError);
/// ```
pub fn convert_from_decimal(to: usize, num: u32) -> ConversionResult<Vec<String>> {

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


// // TODO temporary implementation. doesnt work correctly
// fn custom_conversion_to_decimal(base: u32, num: String) -> Result<Vec<u32>, ParseIntError> {
//     return match u32::from_str_radix(num.as_str(), base.into()) {
//         Ok(n) => Ok(vec![n]),
//         Err(e) => Err(e),
//     };
// }