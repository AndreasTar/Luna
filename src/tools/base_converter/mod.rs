mod base_converter;

// from any

use core::num;

use self::base_converter::ConversionResult;

pub enum InvalidBase{}

pub fn convert_number_base(from:u8, to:u8, num:String) -> String {
    // check if bases are between currently implemented ones
    // if they are, get string, seperate into each digit, and get the int value of it
    //      for example, in hex, num ef would become [14][15]
    // then convert each digit and add em together
    // if output base is <= 36, use proper representation
    // if its > 36, then output each number seperately in base 10 (or base 36 optionally?)


    let result = match (from, to) {
        (..=1, ..=1)        => return String::from      ("Invalid Base"), // TODO an error?
        (2..=36, 2..=36)    => todo!(),
        (_, _)              => todo!()
    };

    let result = match (from, to) {
        (..=1, ..=1)        => return String::from      ("Invalid Base"),
        (2, 10 )            => convert_binary_to_decimal(num),
        (2, 16 )            => convert_binary_to_hex    (num),
        (2, 2..=36)         => convert_binary_to_base   (to as u8, num),
        (10, 2 )            => convert_decimal_to_binary(num),
        (10, 16)            => convert_decimal_to_hex   (num),
        (10, 2..=36)        => convert_decimal_to_base  (to as u8, num),
        (16, 2 )            => convert_hex_to_binary    (num),
        (16, 10)            => convert_hex_to_decimal   (num),
        (16, 2..=36)        => convert_hex_to_base      (to as u8, num),
        (2..=36, 2..=36)    => 'case: {
            // num -> decimal -> base
            let in_dec = match convert_base_to_decimal(from as u8, num){
                ConversionResult::ParseError => break 'case ConversionResult::ParseError,
                ConversionResult::Converted(n) => n,
            };
            convert_decimal_to_base(to as u8, in_dec)
        },
        (2..=36, _)         => 'case: {
            // num -> decimal -> custom base
            // format for output
            let in_dec = match convert_base_to_decimal(from as u8, num){
                ConversionResult::ParseError => break 'case ConversionResult::ParseError,
                ConversionResult::Converted(n) => n,
            };
            convert_from_decimal(to as u32, in_dec)
        },
        (_, 2..=36)         => {
            todo!();
            // convert_to_decimal(from, num);
            // convert_decimal_to_base(to as u8, num);
            // unformat base num -> decimal -> base
        },
        (_, _)              => {
            todo!();
            // convert_to_decimal(from, num);
            // convert_from_decimal(to, num);
            // unformat base num -> decimal -> custom base
            // format for output
        }
    };

    match result {
        ConversionResult::ParseError => String::from("Invalid Input"),
        ConversionResult::Converted(n) => n,
    }
}

// x -c
fn convert_to_decimal(from:u32, num:String) -> ConversionResult<String> {
    todo!()
}

// c- x
fn convert_from_decimal(to:u32, num:String) -> ConversionResult<String> {
    todo!();
}

// from decimal

fn convert_decimal_to_base(to:u8, num:String) -> ConversionResult<String> {
    let as_int = match base_converter::to_decimal(10, num){
        ConversionResult::ParseError => return ConversionResult::ParseError,
        ConversionResult::Converted(n) => n,
    };
    return match base_converter::from_decimal(to, as_int){
        ConversionResult::ParseError => ConversionResult::ParseError,
        ConversionResult::Converted(n) => ConversionResult::Converted(n.concat()),
    };
}

fn convert_decimal_to_binary(num:String) -> ConversionResult<String> {
    return convert_decimal_to_base(2, num);
}

fn convert_decimal_to_hex(num:String) -> ConversionResult<String> {
    return convert_decimal_to_base(16, num);
}

// from binary

fn convert_binary_to_base(to:u8, num:String) -> ConversionResult<String> {
    let as_int = match base_converter::to_decimal(2, num){
        ConversionResult::ParseError => return ConversionResult::ParseError,
        ConversionResult::Converted(n) => n,
    };
    return match base_converter::from_decimal(to, as_int){
        ConversionResult::ParseError => ConversionResult::ParseError,
        ConversionResult::Converted(n) => ConversionResult::Converted(n.concat()),
    };
}

fn convert_binary_to_decimal(num:String) -> ConversionResult<String> {
    return convert_binary_to_base(10, num);

}

fn convert_binary_to_hex(num:String) -> ConversionResult<String> {
    return convert_binary_to_base(16, num);
}

// from hex

fn convert_hex_to_base(to:u8, num:String) -> ConversionResult<String> {
    let as_int = match base_converter::to_decimal(16, num){
        ConversionResult::ParseError => return ConversionResult::ParseError,
        ConversionResult::Converted(n) => n,
    };
    return match base_converter::from_decimal(to, as_int){
        ConversionResult::ParseError => ConversionResult::ParseError,
        ConversionResult::Converted(n) => ConversionResult::Converted(n.concat()),
    };
}

fn convert_hex_to_decimal(num:String) -> ConversionResult<String> {
    return convert_hex_to_base(10, num);

}

fn convert_hex_to_binary(num:String) -> ConversionResult<String> {
    return convert_hex_to_base(2, num);
}

// from base to 

fn convert_base_to_decimal(from:u8, num:String) -> ConversionResult<String> {
    return match base_converter::to_decimal(from, num) {
        ConversionResult::ParseError => ConversionResult::ParseError,
        ConversionResult::Converted(n) => ConversionResult::Converted(n.to_string()),
    };
}