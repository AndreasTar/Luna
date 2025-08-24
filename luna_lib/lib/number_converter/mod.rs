//! Module for converting numbers between different bases (radices).
//! Implemented bases are between 2 and 36 (inclusive).
//! 
//! It will eventually be able to convert from any arbitrary base to any other arbitrary base, even fractional and negative bases,
//! if it makes sense mathematically and is also able to be handled by a computer.


pub const VERSION: crate::Version = crate::Version::new(1, 1, 2);

/// An error which can be returned when attempting to convert a number to a different radix (base).
/// 
/// This error is used as the error type for the functions in the `number_converter` module,
/// such as [`convert_number_base`], [`convert_to_decimal`], and [`convert_from_decimal`].
/// 
/// ## Potential causes
/// `ConversionError` can be thrown because of an invalid usage of the functions in this module.
/// For example:
/// * If the input number is not a valid number in the given radix, e.g. trying to convert "9" in base 2 to any other radix.
/// * If the input radix is not valid, e.g. trying to convert a number from and/or to base 1.
/// * If the input number is too large to fit in the target type, e.g. trying to convert "10e33" in base 10 to base 2, which would overflow a `u32`. 
///     * This is not currently handled, but it will be in the future.
/// * If the current target base is not implemented. (Currently, integer bases 2 to 36 are supported.)
/// 
/// # Examples
/// ```
/// # use luna::number_converter::convert_number_base;
/// # use luna::number_converter::ConversionError;
/// 
/// assert_eq!(convert_number_base(2, 10, &"9".to_string()), Err(ConversionError::ParseError));
/// assert_eq!(convert_number_base(1, 10, &"5".to_string()), Err(ConversionError::BaseError));
/// ```
#[derive(Debug, PartialEq)]
pub enum ConversionError{
    /// The input number could not be parsed in the given base.
    ParseError,
    /// The base is not supported or is invalid.
    BaseError,
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            ConversionError::ParseError => write!(f, "Failed to parse the input number in the given base."),
            ConversionError::BaseError => write!(f, "The base is not supported or is invalid."),
        };
    }
    
}

//const VALUES: [&str; 36] = ["0","1","2","3","4","5","6","7","8","9","A","B","C","D","E","F","G","H","I","J","K","L","M","N","O","P","Q","R","S","T","U","V","W","X","Y","Z"];


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
/// ## Returns
/// A `Result<String, ConversionError>`, where the `Ok` variant contains the converted number as a string in the target radix,
/// and the `Err` variant contains a `ConversionError` if the conversion failed.
/// 
/// ## Examples
/// ```rust
/// # use luna::number_converter::convert_number_base;
/// # use luna::number_converter::ConversionError;
/// 
/// let n = convert_number_base(16, 10, &"e".to_string());
/// assert_eq!(n, Ok("14".to_string()));
/// let n = convert_number_base(2, 16, &"1111".to_string());
/// assert_eq!(n, Ok("f".to_string()));
/// 
/// // If radix isn't between 2 and 36 (inclusive):
/// let n = convert_number_base(1, 8, &"9".to_string());
/// assert_eq!(n, Err(ConversionError::BaseError));
/// 
/// // If number isn't within the `from` radix:
/// let n = convert_number_base(2, 8, &"9".to_string());
/// assert_eq!(n, Err(ConversionError::ParseError));
/// ```
pub fn convert_number_base(from: usize, to: usize, num: &String) -> Result<String, ConversionError> {

    /*
    take number and its base
    if base <= 36, use our own with the letters
    if base > 36, convert each digit to decimal and return each digit like
        [58] [95] [1] [18]
        
        example with base 16, assuming we had only up to base 10
        number FF26
        [15] [15] [2] [6]
    
    for bases > 36, as an input, we can seperate each digit with space, and then convert each digit to decimal
        example, for base 38, we dont have single-digit numbers or enough letters for it
        so instead, the user would input like 37 5 10 35 for each digit
        or maybe even Z2 5 A Z
        where Z2 = 35+2 = 37, 5 = 5, A = 10, Z = 35
        this needs a bit extra handling to recognize between 37 = 37 in base 10 and 37 = 115 = 3*36 + 7 = 108+7
        maybe a toggle on the UI, and a bool parameter on the function
        
        assuming we only had in total 10 digits and no letters for the sake of the example,
        for the number 255 which is FF in base 16, the user would input 15 15
    
    if base isnt integer, we must make our own system (remember that one vid)

    this whole thing will need a bit of changing the types of the functions
    if i have for example i8 numbers, aka 128 max num, and the user inputs a base 127
    then the number 1 2 is equal to 129, which overflows the i8.
    Now extrapolate to numbers like i128 which is 1.7e38, which is a lot of digits, 39 to be exact.
    instead, i64 is 9.2e18, which is still a lot, but not as much.
    additionally, this doesnt work for fractions and decimals, which need f64 etc.
    maybe convert each digit to string to be able to handle *almost* any number?
    maybe this will fix the input/output problem, but the bases are still a problem,
    which means i will probably need to restrict them to maximum f128 OR implement my own number type,
    which is a lot of work and math that i dont know well enough currently.

    the system implementing these will be able to handle any valid number, potentially also 
    making the currently implemented ones obsolete.
    However, it will most likely be slower, so it will be optional.
    I will keep all versions most likely

    also remember, there are ways to quickly calculate large numbers, for example the 1.7e38 from before,
    with reduced accuracy but much faster. it can be an optional function.

    also, larger numbers will need to have a seperator, like _ or . to make them easier to read.

    also, optional handling of scientific notation, like 1.7e38, which is 1.7 * 10^38.
    */

    // check if bases are between currently implemented ones
    // if they are, get string, seperate into each digit, and get the int value of it
    //      for example, in hex, num EF would become [14][15]
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
/// ## Returns
/// A `Result<u32, ConversionError>`, where the `Ok` variant contains the converted number as a `u32` in base 10,
/// and the `Err` variant contains a `ConversionError` if the conversion failed.
/// 
/// ## Examples
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
/// ## Returns
/// A `Result<Vec<String>, ConversionError>`, where the `Ok` variant contains a vector of strings,
/// each string representing a digit of the number in the target radix,
/// and the `Err` variant contains a `ConversionError` if the conversion failed.
/// 
/// ## Examples
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
/// // This panic will be removed in the future, and an error will be returned instead.
/// ```
pub fn convert_from_decimal(to: usize, num: u32) -> Result<Vec<String>, ConversionError> { // HACK the vec may need to be changed

    let mut result = vec![];
    let mut number = num;
    let radix = to as u32;

    let mut tries: u8 = 64; // TODO remove this, its just for testing
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
/// ## Returns
/// A `Result<String, ConversionError>`, where the `Ok` variant contains the converted number as a string in the target radix,
/// and the `Err` variant contains a `ConversionError` if the conversion failed.
/// 
/// ## Examples
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
/// // This panic will be removed in the future, and an error will be returned instead.
/// ```
pub fn convert_from_decimal_joined(to: usize, num: u32) -> Result<String, ConversionError> {
    return convert_from_decimal_joined_with_seperator(to, num, "");
}

/// Converts a string from base 10 to a number in the given radix (aka base) as a string.
/// Works the same as `convert_from_decimal` but instead of returning a vector,
/// it returns a single string, with the digits concatenated together using a custom seperator.
/// 
/// `convert_from_decimal_joined` does the same thing but with the seperator being an empty string: ""
/// Other than that, the two functions are identical. `convert_from_decimal_joined` even calls this function
/// with the same arguments but the seperator mentioned above.
/// 
/// Radix **must** be an integer between 2 and 36 (inclusive). Otherwise, it panics!
/// Number **must** be a valid integer.
/// 
/// ## Arguments
/// * `to` - The integer radix of the output number (between 2 and 36 inclusive)
/// * `num` - The input integer number
/// 
/// ## Returns
/// A `Result<String, ConversionError>`, where the `Ok` variant contains the converted number as a string in the target radix,
/// and the `Err` variant contains a `ConversionError` if the conversion failed.
/// 
/// ## Examples
/// ```rust
/// # use luna::number_converter::convert_from_decimal_joined;
/// # use luna::number_converter::convert_from_decimal_joined_with_seperator;
/// # use luna::number_converter::ConversionError::*;
/// assert_eq!(convert_from_decimal_joined_with_seperator(5, 4, ""), Ok("4".to_string()));
/// 
/// assert_eq!(convert_from_decimal_joined_with_seperator(16, 28, "-"), Ok("1-c".to_string()));
/// assert_eq!(convert_from_decimal_joined_with_seperator(16, 28, "- !& ."), Ok("1- !& .c".to_string()));
/// 
/// assert_eq!(convert_from_decimal_joined_with_seperator(16, 28, ""), convert_from_decimal_joined(16, 28));
/// 
/// ```
/// ```should_panic
/// # use luna::number_converter::convert_from_decimal_joined_with_seperator;
/// // If radix isnt between 2 and 36 (inclusive) it panics:
/// convert_from_decimal_joined_with_seperator(37, 5, " ");
/// // This panic will be removed in the future, and an error will be returned instead.
/// ```
pub fn convert_from_decimal_joined_with_seperator(to: usize, num: u32, sep: &str) -> Result<String, ConversionError> {
    let result = convert_from_decimal(to, num)?; 
    return Ok(result.join(sep));
}