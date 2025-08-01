use iced::{theme::Palette, Color};


pub struct LunaColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl LunaColor {
    pub const WHITE: Self = Self            { r: 255, g: 255, b: 255, a: 255 };
    pub const BLACK: Self = Self            { r: 0  , g: 0  , b: 0  , a: 255 };
    pub const TRANSPARENT: Self = Self      { r: 0  , g: 0  , b: 0  , a: 0   };
    pub const RED: Self = Self              { r: 255, g: 0  , b: 0  , a: 255 };
    pub const GREEN: Self = Self            { r: 0  , g: 255, b: 0  , a: 255 };
    pub const BLUE: Self = Self             { r: 0  , g: 0  , b: 255, a: 255 };

    pub const PURPLE: Self = Self           { r: 128, g: 0  , b: 128, a: 255 };


    pub const PURPLE_NIGHT: Self = Self     { r: 43 , g: 10 , b: 94 , a: 255 };
    pub const PURPLE_MIDNIGHT: Self = Self  { r: 51 , g: 17 , b: 102, a: 255 };
    pub const PURPLISH_WHITE: Self = Self   { r: 219, g: 197, b: 252, a: 255 };
    pub const PURPLE_LIGHT: Self = Self     { r: 123, g: 100, b: 158, a: 255 };

}

pub struct LunaPallete {
    pub primary: Color,
    pub secondary: Color,
    pub tertiary: Color,
    pub quaternary: Color,

    pub text: Color,
    pub text_secondary: Color,
    pub text_tertiary: Color,
    pub text_quaternary: Color,

    pub background: Color,
    pub background_secondary: Color,
    pub background_tertiary: Color,
    pub background_quaternary: Color,

    pub border: Color,
    pub border_secondary: Color,
    pub border_tertiary: Color,
    pub border_quaternary: Color,

    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
    pub danger: Color,

    pub inactive: Color,
    pub disabled: Color,

    pub highlight: Color,
}

impl LunaPallete {

    pub const DARK: Self = Self {
        primary:                Color::from_rgb(0   as f32 / 255.0, 0   as f32 / 255.0, 0   as f32 / 255.0), // pure black
        secondary:              Color::from_rgb(50  as f32 / 255.0, 50  as f32 / 255.0, 50  as f32 / 255.0), // dark gray
        tertiary:               Color::from_rgb(100 as f32 / 255.0, 100 as f32 / 255.0, 100 as f32 / 255.0), // medium gray
        quaternary:             Color::from_rgb(150 as f32 / 255.0, 150 as f32 / 255.0, 150 as f32 / 255.0), // light gray

        text:                   Color::WHITE,
        text_secondary:         Color::from_rgb(200 as f32 / 255.0, 200 as f32 / 255.0, 200 as f32 / 255.0), // lighter gray
        text_tertiary:          Color::from_rgb(180 as f32 / 255.0, 180 as f32 / 255.0, 180 as f32 / 255.0),
        text_quaternary:        Color::from_rgb(160 as f32 / 255.0, 160 as f32 / 255.0, 160 as f32 / 255.0),

        background:             Color::from_rgb(30  as f32 / 255.0, 30  as f32 / 255.0, 30  as f32 / 255.0),
        background_secondary:   Color::from_rgb(40  as f32 / 255.0, 40  as f32 / 255.0, 40  as f32 / 255.0),
        background_tertiary:    Color::from_rgb(50  as f32 / 255.0, 50  as f32 / 255.0, 50  as f32 / 255.0),
        background_quaternary:  Color::from_rgb(60  as f32 / 255.0, 60  as f32 / 255.0, 60  as f32 / 255.0),

        border:                 Color::from_rgb(70  as f32 / 255.0, 70  as f32 / 255.0, 70  as f32 / 255.0),
        border_secondary:       Color::from_rgb(80  as f32 / 255.0, 80  as f32 / 255.0, 80  as f32 / 255.0),
        border_tertiary:        Color::from_rgb(90  as f32 / 255.0, 90  as f32 / 255.0, 90  as f32 / 255.0),
        border_quaternary:      Color::from_rgb(100 as f32 / 255.0, 100 as f32 / 255.0, 100 as f32 / 255.0),

        success:                Color::from_rgb(0   as f32 / 255.0, 128 as f32 / 255.0, 0   as f32 / 255.0),
        warning:                Color::from_rgb(255 as f32 / 255.0, 165 as f32 / 255.0, 0   as f32 / 255.0),
        error:                  Color::from_rgb(255 as f32 / 255.0, 0   as f32 / 255.0, 0   as f32 / 255.0),
        info:                   Color::from_rgb(0   as f32 / 255.0, 0   as f32 / 255.0, 255 as f32 / 255.0),
        danger:                 Color::from_rgb(255 as f32 / 255.0, 20  as f32 / 255.0, 147 as f32 / 255.0),

        inactive:               Color::from_rgb(120 as f32 / 255.0, 120 as f32 / 255.0, 120 as f32 / 255.0),
        disabled:               Color::from_rgb(150 as f32 / 255.0, 150 as f32 / 255.0, 150 as f32 / 255.0),

        highlight:              Color::from_rgb(255 as f32 / 255.0, 255 as f32 / 255.0, 0 as f32 / 255.0),
    };

    pub fn new() -> Self {
        LunaPallete {
            primary:                Color::from_rgb8(255, 255, 255),
            secondary:              Color::from_rgb8(255, 255, 255),
            tertiary:               Color::from_rgb8(255, 255, 255),
            quaternary:             Color::from_rgb8(255, 255, 255),

            text:                   Color::WHITE,
            text_secondary:         Color::from_rgb8(255, 255, 255),
            text_tertiary:          Color::from_rgb8(255, 255, 255),
            text_quaternary:        Color::from_rgb8(255, 255, 255),

            background:             Color::from_rgb8(255, 255, 255),
            background_secondary:   Color::from_rgb8(255, 255, 255),
            background_tertiary:    Color::from_rgb8(255, 255, 255),
            background_quaternary:  Color::from_rgb8(255, 255, 255),

            border:                 Color::from_rgb8(255, 255, 255),
            border_secondary:       Color::from_rgb8(255, 255, 255),
            border_tertiary:        Color::from_rgb8(255, 255, 255),
            border_quaternary:      Color::from_rgb8(255, 255, 255),

            success:                Color::from_rgb8(255, 255, 255),
            warning:                Color::from_rgb8(255, 255, 255),
            error:                  Color::from_rgb8(255, 255, 255),
            info:                   Color::from_rgb8(255, 255, 255),
            danger:                 Color::from_rgb8(255, 255, 255),

            inactive:               Color::from_rgb8(255, 255, 255),
            disabled:               Color::from_rgb8(255, 255, 255),

            highlight:              Color::from_rgb8(255, 255, 255),
        }
    }


}

mod buttons {
    
}