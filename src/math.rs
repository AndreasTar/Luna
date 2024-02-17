mod base_converter;

// from any

use core::num;

use self::base_converter::ConversionResult;

pub fn convert_number_base(from:u32, to:u32, num:String) -> String{
    return match base_converter::convert_to_decimal(2, num){
        ConversionResult::ParseError => String::from("-1"),
        ConversionResult::InvalidBase => String::from("-2"),
        ConversionResult::NormalConverted(n) => n.first().unwrap().to_string(),
        ConversionResult::CustomConverted(n) => n.first().unwrap().to_string(),
    };
}

pub fn convert_to_decimal(from:f32, num:String){

}

pub fn convert_to_binary(from:f32, num:String){

}

pub fn convert_to_hex(from:f32, num:String){

}

// from decimal

pub fn convert_decimal_to_base(to:f32, num:String){

}

pub fn convert_decimal_to_binary(num:String){

}

pub fn convert_decimal_to_hex(num:String){

}

// from binary

pub fn convert_binary_to_base(to:f32, num:String){

}

pub fn convert_binary_to_decimal(num:String){

}

pub fn convert_binary_to_hex(num:String){

}

// from hex

pub fn convert_hex_to_base(to:f32, num:String){

}

pub fn convert_hex_to_decimal(num:String){

}

pub fn convert_hex_to_binary(num:String){

}