use iced::{theme::Palette, Color, Background};


pub const VERSION: luna::Version = luna::Version::new(0, 0, 3);

/// Custom colors for use in Luna UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LunaColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}


impl LunaColor {
    /// Pure `WHITE` color, rgb: 255 255 255
    #[inline]
    pub const WHITE: Self = Self            { r: 255, g: 255, b: 255, a: 255 };
    /// Pure `BLACK` color, rgb: 0 0 0
    #[inline]
    pub const BLACK: Self = Self            { r: 0  , g: 0  , b: 0  , a: 255 };
    /// Transparent color, alpha: 0
    #[inline]
    pub const TRANSPARENT: Self = Self      { r: 0  , g: 0  , b: 0  , a: 0   };
    /// Pure `RED` color, rgb: 255 0 0
    #[inline]
    pub const RED: Self = Self              { r: 255, g: 0  , b: 0  , a: 255 };
    /// Pure `GREEN` color, rgb: 0 255 0
    #[inline]
    pub const GREEN: Self = Self            { r: 0  , g: 255, b: 0  , a: 255 };
    /// Pure `BLUE` color, rgb: 0 0 255
    #[inline]
    pub const BLUE: Self = Self             { r: 0  , g: 0  , b: 255, a: 255 };
    /// Pure `YELLOW` color, rgb: 255 255 0
    #[inline]
    pub const YELLOW: Self = Self           { r: 255, g: 255, b: 0  , a: 255 };
    /// Pure `PINK` color, rgb: 255 0 255
    #[inline]
    pub const PINK: Self = Self             { r: 255, g: 0  , b: 255, a: 255 };
    /// Pure `CYAN` color, rgb: 255 0 255
    #[inline]
    pub const CYAN: Self = Self             { r: 0  , g: 255, b: 255, a: 255 };


    /// Slightly off-black color, rgb: 25 25 25
    #[inline]
    pub const SPACE_BLACK: Self = Self      { r: 25 , g: 25 , b: 25 , a: 255 };
    /// Dark gray color, rgb: 50 50 50
    #[inline]
    pub const DARK_GRAY: Self = Self        { r: 50 , g: 50 , b: 50 , a: 255 };
    /// Gray color, rgb: 100 100 100
    #[inline]
    pub const GRAY: Self = Self             { r: 100, g: 100, b: 100, a: 255 };
    /// Light gray color, rgb: 150 150 150
    #[inline]
    pub const LIGHT_GRAY: Self = Self       { r: 150, g: 150, b: 150, a: 255 };
    /// Slighty gray white color, rgb: 200 200 200
    #[inline]
    pub const GRAYISH_WHITE: Self = Self    { r: 200, g: 200, b: 200, a: 255 };


    /// Pure `PURPLE` color, rgb: 128 0 128
    #[inline]
    pub const PURPLE: Self = Self           { r: 128, g: 0  , b: 128, a: 255 };


    /// A deep purple-blue color mix, like the sea on a night clear sky, rgb: 43 10 94
    #[inline]
    pub const PURPLE_NIGHT: Self = Self     { r: 43 , g: 10 , b: 94 , a: 255 };
    /// A slightly purpler version of [PURPLE_NIGHT], rgb: 51 17 102
    #[inline]
    pub const PURPLER_NIGHT: Self = Self    { r: 51 , g: 17 , b: 102, a: 255 };
    /// A purple color that is more on the blue side, lighter than [PURPLER_NIGHT], rgb: 62 12 138
    #[inline]
    pub const PURPLE_AFTERNOON: Self = Self { r: 62 , g: 12 , b: 138, a: 255 };
    /// A white with a hint of purple, rgb: 219 197 252
    #[inline]
    pub const PURPLISH_WHITE: Self = Self   { r: 219, g: 197, b: 252, a: 255 };
    /// A light purple color, rgb: 123 96 166
    #[inline]
    pub const PURPLER_LIGHT: Self = Self    { r: 123, g: 96 , b: 166, a: 255 };
    /// A slightly faded light purple color, rgb: 123 100 158
    #[inline]
    pub const PURPLE_LIGHT: Self = Self     { r: 123, g: 100, b: 158, a: 255 };
    /// A lighter and purpler version of [PURPLE_LIGHT], rgb: 143 111 191
    #[inline]
    pub const PURPLE_LIGHTER: Self = Self   { r: 143, g: 111, b: 191, a: 255 };
    /// A very light purple color, rgb: 167 145 201
    #[inline]
    pub const LILY: Self = Self             { r: 167, g: 145, b: 201, a: 255 };


    /// A deep dark blue with a hint of purple, rgb: 28 8 59
    #[inline]
    pub const DEEPSEA_MIDNIGHT: Self = Self { r: 28 , g: 8  , b: 59 , a: 255 };
    /// An almost-black deep blue color, rgb: 22 2 54
    #[inline]
    pub const DEEPSEA_BLUE: Self = Self     { r: 22 , g: 2  , b: 54 , a: 255 };


    /// A bright, almost-pink magenta color, rgb: 199 58 161
    #[inline]
    pub const LIGHT_MAGENTA: Self = Self    { r: 199, g: 58 , b: 161, a: 255 };
    /// A dark, slightly reddish magenta color, rgb: 87 26 70
    #[inline]
    pub const REDISH_MAGENTA: Self = Self   { r: 87 , g: 26 , b: 70 , a: 255 };



    #[inline]
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        return Self { r, g, b, a };
    }
    #[inline]
    pub const fn to_iced_color(&self) -> Color {
        return Color::from_rgba(
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        );
    }
}

impl Into<Color> for LunaColor {
    fn into(self) -> Color {
        return self.to_iced_color();
    }
}

impl From<Color> for LunaColor {
    fn from(c: Color) -> Self {
        return Self {
            r: (c.r *255.0) as u8,
            g: (c.g *255.0) as u8,
            b: (c.b *255.0) as u8,
            a: (c.a *255.0) as u8
        };
    }
}

impl Into<Background> for LunaColor {
    fn into(self) -> Background {
        return self.to_iced_color().into();
    }
}

// TODO convert these to LunaColor, and then add a method to convert them to iced::Color
pub struct LunaPallete {
    pub primary: LunaColor,
    pub secondary: LunaColor,
    pub tertiary: LunaColor,
    pub quaternary: LunaColor,

    pub text: LunaColor,
    pub text_secondary: LunaColor,
    pub text_tertiary: LunaColor,
    pub text_quaternary: LunaColor,

    pub background: LunaColor,
    pub background_secondary: LunaColor,
    pub background_tertiary: LunaColor,
    pub background_quaternary: LunaColor,

    pub border: LunaColor,
    pub border_secondary: LunaColor,
    pub border_tertiary: LunaColor,
    pub border_quaternary: LunaColor,

    pub success: LunaColor,
    pub warning: LunaColor,
    pub error: LunaColor,
    pub info: LunaColor,
    pub danger: LunaColor,

    pub inactive: LunaColor,
    pub disabled: LunaColor,

    pub highlight: LunaColor,
}

impl LunaPallete {

    pub const MONOCHROME_GRAY: Self = Self {
        primary:                LunaColor::BLACK,
        secondary:              LunaColor::DARK_GRAY,
        tertiary:               LunaColor::GRAY,
        quaternary:             LunaColor::LIGHT_GRAY,

        text:                   LunaColor::WHITE,
        text_secondary:         LunaColor::GRAYISH_WHITE,
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
        error:                  LunaColor::RED,
        info:                   LunaColor::BLUE,
        danger:                 Color::from_rgb(255 as f32 / 255.0, 20  as f32 / 255.0, 147 as f32 / 255.0),

        inactive:               Color::from_rgb(120 as f32 / 255.0, 120 as f32 / 255.0, 120 as f32 / 255.0),
        disabled:               Color::from_rgb(150 as f32 / 255.0, 150 as f32 / 255.0, 150 as f32 / 255.0),

        highlight:              LunaColor::YELLOW,
    };

    pub fn new() -> Self {
        return LunaPallete {
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
        };
    }

}

mod buttons {
    
}