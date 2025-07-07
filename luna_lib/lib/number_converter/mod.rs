
// TODO add doc for the errors
#[derive(Debug, PartialEq)]
pub enum ConversionError{
    ParseError,
    BaseError,
}

//const VALUES: [&str; 36] = ["0","1","2","3","4","5","6","7","8","9","A","B","C","D","E","F","G","H","I","J","K","L","M","N","O","P","Q","R","S","T","U","V","W","X","Y","Z"];


// TODO add returns and panics doc sections
// TODO make try_ versions
// TODO make arbitrary bases work

/// Converts a string from a radix to a string of a number in another radix.
/// This is the about same as chaining `convert_to_decimal(..)` and `convert_from_decimal(..)`,
/// which is exactly what it does under the hood, like so:
/// ```ignore
/// let number = "some number".to_string();
/// let num_decimal = convert_to_decimal(base1, number).unwrap();
/// let num_base2 = convert_from_decimal(base2, num_decimal).unwrap();
/// 
/// asserteq!(num_base2, convert_number_base(base1, base2, number));
/// ```
/// with the exception that it shouldn't panic in any case, and it will return an error instead.
/// 
/// Radices **must** be integer between 2 and 36 (inclusive).
/// Number **must** be a string of digits and letters (0-9, A-Z), representing an integer.
/// 
/// ## Arguments
/// * `from` - The integer radix of the input number (between 2 and 36 inclusive)
/// * `to`   - The integer radix of the output number (between 2 and 36 inclusive)
/// * `num`  - The input number as a string
/// 
/// ## Examples
/// 
/// ```rust
/// # use luna::number_converter::convert_number_base;
/// # use luna::number_converter::ConversionError;
/// 
/// let n = convert_number_base(16, 10, &"e".to_string());
/// assert_eq!(n.unwrap(), "14");
/// let n = convert_number_base(2, 16, &"1111".to_string());
/// assert_eq!(n.unwrap(), "f");
/// 
/// // If radix isnt between 2 and 36 (inclusive):
/// let n = convert_number_base(1, 8, &"9".to_string());
/// assert_eq!(n, Err(ConversionError::BaseError));
/// 
/// // If number isnt within the `from` radix:
/// let n = convert_number_base(2, 8, &"9".to_string());
/// assert_eq!(n, Err(ConversionError::ParseError));
/// ```
pub fn convert_number_base(from: usize, to: usize, num: &String) -> Result<String, ConversionError> {

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


    let result: Result<String, ConversionError> = match (from, to) {
        (..=1, ..=1)        => return Err(ConversionError::BaseError),
        (2..=36, 2..=36)    => 'case: {
            let res1 = match convert_to_decimal(from, num) {
                Err(_e) => break 'case Err(ConversionError::ParseError),
                Ok(n) => n,
            };
            convert_from_decimal_joined(to.into(), res1)
        },
        (_, _)              => return Err(ConversionError::BaseError),
    };

    return result;
}



/// Converts a string from a radix to a number in base 10.
/// 
/// Radix **must** be an integer between 2 and 36 (inclusive). Otherwise, it panics!
/// Number **must** be a string of digits and letters (0-9, A-Z), representing an integer.
/// 
/// ## Arguments
/// * `from` - The integer radix of the input number (between 2 and 36 inclusive)
/// * `num` - The input number
/// 
/// ## Examples
/// 
/// ```rust
/// # use luna::number_converter::convert_to_decimal;
/// # use luna::number_converter::ConversionError::*;
/// let n = convert_to_decimal(16, &"e".to_string());
/// assert_eq!(n, Ok(14));
/// 
/// // If number is invalid, either due to being outside the radix, or by containing invalid chars:
/// let n = convert_to_decimal(2, &"9".to_string());
/// assert_eq!(n, Err(ParseError));
/// 
/// ```
/// ```should_panic
/// # use luna::number_converter::convert_to_decimal;
/// // If radix isnt between 2 and 36 (inclusive) it panics:
/// convert_to_decimal(1, &"9".to_string());
/// ```
pub fn convert_to_decimal(from: usize, num: &String) -> Result<u32, ConversionError> {

    return match u32::from_str_radix(num.as_str(), from.try_into().unwrap()){
        Ok(n) => Ok(n),
        Err(_e) => Err(ConversionError::ParseError),
    };
}

/// Converts a string from base 10 to number in the given radix (aka base), as a vector of strings.
/// Each string (element of the vector) represents a digit of the number in order from larger to smaller.
/// 
/// Radix **must** be an integer between 2 and 36 (inclusive). Otherwise, it panics!
/// Number **must** be a valid integer.
/// 
/// #### WARNING: this function will be refactored soon, to remove hacky logic.
/// 
/// ## Arguments
/// * `to` - The integer radix of the output number (between 2 and 36 inclusive)
/// * `num` - The input integer number
/// 
/// 
/// ## Examples
/// 
/// ```rust
/// # use luna::number_converter::convert_from_decimal;
/// # use luna::number_converter::ConversionError::*;
/// assert_eq!(convert_from_decimal(5, 4), Ok(vec!["4".to_string()]));
/// 
/// let c = vec!["1".to_string(), "c".to_string()]; // Vec["1", "c"]
/// assert_eq!(convert_from_decimal(16, 28), Ok(c));
/// 
/// ```
/// ```should_panic
/// # use luna::number_converter::convert_from_decimal;
/// // If radix isnt between 2 and 36 (inclusive) it panics:
/// convert_from_decimal(37, 5);
/// ```
pub fn convert_from_decimal(to: usize, num: u32) -> Result<Vec<String>, ConversionError> { // HACK the vec may need to be changed

    let mut result = vec![];
    let mut number = num;
    let radix = to as u32;

    let mut tries = 64; // TODO remove this, its just for testing
    loop {
        tries -= 1;
        let digit = number % radix;
        number = number / radix;

        // will panic if you use a bad radix (< 2 or > 36).
        result.push( 
            match char::from_digit(digit, radix) {
                Some(c) => c.to_string(),
                None => return Err(ConversionError::ParseError),
            }.to_string()
        );
        if tries <= 0 {
            return Err(ConversionError::BaseError); // HACK is it def a base error? idk
        }

        if number <= 0 {
            break;
        }
    }
    let result = result.into_iter().rev().collect::<Vec<String>>();
    return Ok(result);
}

/// Converts a string from base 10 to a number in the given radix (aka base) as a string.
/// Works the same as `convert_from_decimal` but instead of returning a vector,
/// it returns a single string, with the digits concatenated together.
/// 
/// Radix **must** be an integer between 2 and 36 (inclusive). Otherwise, it panics!
/// Number **must** be a valid integer.
/// 
/// ## Arguments
/// * `to` - The integer radix of the output number (between 2 and 36 inclusive)
/// * `num` - The input integer number
/// 
/// ## Examples
/// 
/// ```rust
/// # use luna::number_converter::convert_from_decimal_joined;
/// # use luna::number_converter::ConversionError::*;
/// assert_eq!(convert_from_decimal_joined(5, 4), Ok("4".to_string()));
/// 
/// assert_eq!(convert_from_decimal_joined(16, 28), Ok("1c".to_string()));
/// 
/// ```
/// ```should_panic
/// # use luna::number_converter::convert_from_decimal_joined;
/// // If radix isnt between 2 and 36 (inclusive) it panics:
/// convert_from_decimal_joined(37, 5);
/// ```
pub fn convert_from_decimal_joined(to: usize, num: u32) -> Result<String, ConversionError> {
    let result = convert_from_decimal(to, num)?;
    return Ok(result.join(""));
}