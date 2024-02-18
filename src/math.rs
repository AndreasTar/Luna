mod base_converter;

// from any

use core::num;

use self::base_converter::ConversionResult;

pub fn convert_number_base(from:u32, to:u32, num:String) -> String{
    return match base_converter::to_decimal(from, num){
        ConversionResult::ParseError => String::from("Invalid input number!"),
        ConversionResult::InvalidBase => String::from("Invalid input base!"),
        ConversionResult::NormalConverted(n) => n.first().unwrap().to_string(),
        ConversionResult::CustomConverted(n) => n.first().unwrap().to_string(),
    };
}

fn convert_to_decimal(from:f32, num:String){

}

fn convert_to_binary(from:f32, num:String){

}

fn convert_to_hex(from:f32, num:String){

}

// from decimal

fn convert_decimal_to_base(to:f32, num:String){

}

fn convert_decimal_to_binary(num:String){

}

fn convert_decimal_to_hex(num:String){

}

// from binary

fn convert_binary_to_base(to:f32, num:String){

}

fn convert_binary_to_decimal(num:String){

}

fn convert_binary_to_hex(num:String){

}

// from hex

fn convert_hex_to_base(to:f32, num:String){

}

fn convert_hex_to_decimal(num:String){

}

fn convert_hex_to_binary(num:String){

}