//! This module provides a function to convert a vector of bytes representing pixel data from one color model to another.
//! 
//! The conversion is done by mapping the channels of the source color model to the destination color model.
//! Currently, only models with RGB, CMYK, HSL and Grayscale are supported, with and without an Alpha channel,
//! with all their respective permutations.

pub const VERSION: crate::Version = crate::Version::new(1, 1, 0);

/// Possible errors that can occur during color format conversion.
/// - `InvalidInputLength`: The input data length is not a multiple of the source format's channel count.
/// - `SameFormat`: The source and destination formats are the same.
/// 
/// # Examples
/// ```rust
/// # use luna::color_format_converter::{convert_vec_color_model, ColorFormat, ColorFormatConverterError};
/// 
/// // Invalid input length: eg. RG -> RGBA (returns `Err(ColorFormatConverterError::InvalidInputLength)`)
/// let rg = vec![128, 55];
/// let length_err = convert_vec_color_model(&rg, ColorFormat::RGB, ColorFormat::RGBA);
/// assert_eq!(length_err, Err(ColorFormatConverterError::InvalidInputLength)); 
/// 
/// // Same format: eg. RGBA -> RGBA (returns `Err(ColorFormatConverterError::SameFormat)`)
/// let rgba = vec![128, 55, 88, 255];
/// let format_err = convert_vec_color_model(&rgba, ColorFormat::RGBA, ColorFormat::RGBA);
/// assert_eq!(format_err, Err(ColorFormatConverterError::SameFormat));
/// ```
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum ColorFormatConverterError {
    /// Error signifying that the input data length is not a multiple of the source format's channel count.
    InvalidInputLength,
    /// Error signifying that the source and destination formats are the same.
    SameFormat
}

impl std::fmt::Display for ColorFormatConverterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColorFormatConverterError::InvalidInputLength => write!(f, "Input data length is not a multiple of the source format's channel count."),
            ColorFormatConverterError::SameFormat => write!(f, "Source and destination formats are the same."),
        }
    }
}


// TODO maybe add lume and other formats? needs calculations etc but its fine i think
/// Packed-byte color models.
#[repr(usize)]
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum ColorFormat {
    RGBA = 0,
    RGB, RBGA, RBG,

    GRBA, GRB, GBRA, GBR,

    BRGA, BRG, BGRA, BGR,

    Gray, GrayA,
    CMYK, CMKY, CKYM, CKMY, CYMK, CYKM,
    KCMY, KCYM, KYCM, KYMC, KMCY, KMYC,
    MCYK, MCKY, MYCK, MYKC, MKCY, MKYC,
    YCMK, YCKM, YKCM, YKMC, YMCK, YMKC,

    CMYKA, CMKYA, CKYMA, CKMYA, CYMKA, CYKMA,
    KCMYA, KCYMA, KYCMA, KYMCA, KMCYA, KMYCA,
    MCYKA, MCKYA, MYCKA, MYKCA, MKCYA, MKYCA,
    YCMKA, YCKMA, YKCMA, YKMCA, YMCKA, YMKCA,

    HSL, HLS, SHL, SLH, LHS, LSH,
    HSLA, HLSA, SHLA, SLHA, LHSA, LSHA,
}

impl ColorFormat {
    /// Returns the number of channels in the color format.
    /// 
    /// # Examples
    /// ```rust
    /// # use luna::color_format_converter::ColorFormat;
    /// 
    /// assert_eq!(ColorFormat::RGBA.channel_count(), 4);
    /// assert_eq!(ColorFormat::RGB.channel_count(), 3);
    /// assert_eq!(ColorFormat::Gray.channel_count(), 1);
    /// assert_eq!(ColorFormat::CMYK.channel_count(), 4);
    /// assert_eq!(ColorFormat::HSL.channel_count(), 3);
    /// ```
    pub fn channel_count(&self) -> usize {
        return channel_order(*self).len();
    }
}

/// A color channel identifier.
/// Used to specify the order of channels in a color model.
#[derive(PartialEq, Debug, Clone, Copy)]
enum Channel {
    R, G, B, A,
    Gray,
    C, M, Y, K,
    H, S, L,
}

/// The per-pixel channel order.
fn channel_order(format: ColorFormat) -> &'static [Channel] {
    match format {
        ColorFormat::RGBA => &[Channel::R, Channel::G, Channel::B, Channel::A],
        ColorFormat::RGB  => &[Channel::R, Channel::G, Channel::B],
        ColorFormat::RBGA => &[Channel::R, Channel::B, Channel::G, Channel::A],
        ColorFormat::RBG  => &[Channel::R, Channel::B, Channel::G],

        ColorFormat::BRGA => &[Channel::B, Channel::R, Channel::G, Channel::A],
        ColorFormat::BRG  => &[Channel::B, Channel::R, Channel::G],
        ColorFormat::BGRA => &[Channel::B, Channel::G, Channel::R, Channel::A],
        ColorFormat::BGR  => &[Channel::B, Channel::G, Channel::R],

        ColorFormat::GBRA => &[Channel::G, Channel::B, Channel::R, Channel::A],
        ColorFormat::GBR  => &[Channel::G, Channel::B, Channel::R],
        ColorFormat::GRBA => &[Channel::G, Channel::R, Channel::B, Channel::A],
        ColorFormat::GRB  => &[Channel::G, Channel::R, Channel::B],

        ColorFormat::Gray => &[Channel::Gray],
        ColorFormat::GrayA => &[Channel::Gray, Channel::A],

        ColorFormat::CMYK => &[Channel::C, Channel::M, Channel::Y, Channel::K],
        ColorFormat::CMKY => &[Channel::C, Channel::M, Channel::K, Channel::Y],
        ColorFormat::CKYM => &[Channel::C, Channel::K, Channel::Y, Channel::M],
        ColorFormat::CKMY => &[Channel::C, Channel::K, Channel::M, Channel::Y],
        ColorFormat::CYMK => &[Channel::C, Channel::Y, Channel::M, Channel::K],
        ColorFormat::CYKM => &[Channel::C, Channel::Y, Channel::K, Channel::M],

        ColorFormat::KCMY => &[Channel::K, Channel::C, Channel::M, Channel::Y],
        ColorFormat::KCYM => &[Channel::K, Channel::C, Channel::Y, Channel::M],
        ColorFormat::KYCM => &[Channel::K, Channel::Y, Channel::C, Channel::M],
        ColorFormat::KYMC => &[Channel::K, Channel::Y, Channel::M, Channel::C],
        ColorFormat::KMCY => &[Channel::K, Channel::M, Channel::C, Channel::Y],
        ColorFormat::KMYC => &[Channel::K, Channel::M, Channel::Y, Channel::C],

        ColorFormat::MCYK => &[Channel::M, Channel::C, Channel::Y, Channel::K],
        ColorFormat::MCKY => &[Channel::M, Channel::C, Channel::K, Channel::Y],
        ColorFormat::MYCK => &[Channel::M, Channel::Y, Channel::C, Channel::K],
        ColorFormat::MYKC => &[Channel::M, Channel::Y, Channel::K, Channel::C],
        ColorFormat::MKCY => &[Channel::M, Channel::K, Channel::C, Channel::Y],
        ColorFormat::MKYC => &[Channel::M, Channel::K, Channel::Y, Channel::C],

        ColorFormat::YCMK => &[Channel::Y, Channel::C, Channel::M, Channel::K],
        ColorFormat::YCKM => &[Channel::Y, Channel::C, Channel::K, Channel::M],
        ColorFormat::YKCM => &[Channel::Y, Channel::K, Channel::C, Channel::M],
        ColorFormat::YKMC => &[Channel::Y, Channel::K, Channel::M, Channel::C],
        ColorFormat::YMCK => &[Channel::Y, Channel::M, Channel::C, Channel::K],
        ColorFormat::YMKC => &[Channel::Y, Channel::M, Channel::K, Channel::C],

        ColorFormat::CMYKA => &[Channel::C, Channel::M, Channel::Y, Channel::K, Channel::A],
        ColorFormat::CMKYA => &[Channel::C, Channel::M, Channel::K, Channel::Y, Channel::A],
        ColorFormat::CKYMA => &[Channel::C, Channel::K, Channel::Y, Channel::M, Channel::A],
        ColorFormat::CKMYA => &[Channel::C, Channel::K, Channel::M, Channel::Y, Channel::A],
        ColorFormat::CYMKA => &[Channel::C, Channel::Y, Channel::M, Channel::K, Channel::A],
        ColorFormat::CYKMA => &[Channel::C, Channel::Y, Channel::K, Channel::M, Channel::A],

        ColorFormat::KCMYA => &[Channel::K, Channel::C, Channel::M, Channel::Y, Channel::A],
        ColorFormat::KCYMA => &[Channel::K, Channel::C, Channel::Y, Channel::M, Channel::A],
        ColorFormat::KYCMA => &[Channel::K, Channel::Y, Channel::C, Channel::M, Channel::A],
        ColorFormat::KYMCA => &[Channel::K, Channel::Y, Channel::M, Channel::C, Channel::A],
        ColorFormat::KMCYA => &[Channel::K, Channel::M, Channel::C, Channel::Y, Channel::A],
        ColorFormat::KMYCA => &[Channel::K, Channel::M, Channel::Y, Channel::C, Channel::A],

        ColorFormat::MCYKA => &[Channel::M, Channel::C, Channel::Y, Channel::K, Channel::A],
        ColorFormat::MCKYA => &[Channel::M, Channel::C, Channel::K, Channel::Y, Channel::A],
        ColorFormat::MYCKA => &[Channel::M, Channel::Y, Channel::C, Channel::K, Channel::A],
        ColorFormat::MYKCA => &[Channel::M, Channel::Y, Channel::K, Channel::C, Channel::A],
        ColorFormat::MKCYA => &[Channel::M, Channel::K, Channel::C, Channel::Y, Channel::A],
        ColorFormat::MKYCA => &[Channel::M, Channel::K, Channel::Y, Channel::C, Channel::A],

        ColorFormat::YCMKA => &[Channel::Y, Channel::C, Channel::M, Channel::K, Channel::A],
        ColorFormat::YCKMA => &[Channel::Y, Channel::C, Channel::K, Channel::M, Channel::A],
        ColorFormat::YKCMA => &[Channel::Y, Channel::K, Channel::C, Channel::M, Channel::A],
        ColorFormat::YKMCA => &[Channel::Y, Channel::K, Channel::M, Channel::C, Channel::A],
        ColorFormat::YMCKA => &[Channel::Y, Channel::M, Channel::C, Channel::K, Channel::A],
        ColorFormat::YMKCA => &[Channel::Y, Channel::M, Channel::K, Channel::C, Channel::A],

        ColorFormat::HSL => &[Channel::H, Channel::S, Channel::L],
        ColorFormat::HLS => &[Channel::H, Channel::L, Channel::A],
        ColorFormat::SHL => &[Channel::S, Channel::H, Channel::L],
        ColorFormat::SLH => &[Channel::S, Channel::L, Channel::H],
        ColorFormat::LHS => &[Channel::L, Channel::H, Channel::S],
        ColorFormat::LSH => &[Channel::L, Channel::S, Channel::H],
        ColorFormat::HSLA => &[Channel::H, Channel::S, Channel::L, Channel::A],
        ColorFormat::HLSA => &[Channel::H, Channel::L, Channel::A, Channel::A],
        ColorFormat::SHLA => &[Channel::S, Channel::H, Channel::L, Channel::A],
        ColorFormat::SLHA => &[Channel::S, Channel::L, Channel::H, Channel::A],
        ColorFormat::LHSA => &[Channel::L, Channel::H, Channel::S, Channel::A],
        ColorFormat::LSHA => &[Channel::L, Channel::S, Channel::H, Channel::A],
    }
}

/// Convert raw pixel bytes from one color model to another.
///
/// ## Parameters
/// - `data`: input byte-slice, length must be a multiple of `from.channel_count()`
/// - `from`: source color model (e.g. `ColorFormat::RGBA`)
/// - `to`: destination color model (e.g. `ColorFormat::BGR`)
///
/// ## Returns
/// A `Result` containing either a new `Vec<u8>` whose length is `pixel_count * to.channel_count()`, or an `ColorFormatConverterError` 
/// if the conversion failed, if the input length isn’t a multiple of the source channel count, or if the source and destination 
/// formats are the same.
///
/// ## Examples
///
/// ```rust
/// # use luna::color_format_converter::{convert_vec_color_model, ColorFormat, ColorFormatConverterError};
///
/// // Single pixel RGBA -> BGR (reorders & drops alpha)
/// let rgba = vec![128, 55, 88, 255];
/// let bgr = convert_vec_color_model(&rgba, ColorFormat::RGBA, ColorFormat::BGR);
/// assert_eq!(bgr, Ok(vec![88, 55, 128]));
///
/// // Single pixel RGB -> BGRA (alpha default to 255)
/// let rgb = vec![10, 20, 30];
/// let bgra = convert_vec_color_model(&rgb, ColorFormat::RGB, ColorFormat::BGRA);
/// assert_eq!(bgra, Ok(vec![30, 20, 10, 255]));
///
/// // Two pixels RGBA -> RGB (drops alpha)
/// let two_rgba = vec![1, 2, 3, 255,  4, 5, 6, 128];
/// let two_rgb = convert_vec_color_model(&two_rgba, ColorFormat::RGBA, ColorFormat::RGB);
/// assert_eq!(two_rgb, Ok(vec![1, 2, 3,  4, 5, 6]));
/// 
/// // Same format RGBA -> RGBA (returns `Err(ColorFormatConverterError::SameFormat)`)
/// let rgba = vec![128, 55, 88, 255];
/// let rgba_err = convert_vec_color_model(&rgba, ColorFormat::RGBA, ColorFormat::RGBA);
/// assert_eq!(rgba_err, Err(ColorFormatConverterError::SameFormat));
/// ```
pub fn convert_vec_color_model(data: &[u8], from: ColorFormat, to: ColorFormat) -> Result<Vec<u8>, ColorFormatConverterError> {
    if from == to { return Err(ColorFormatConverterError::SameFormat) }
    if data.len() < 3 { return Err(ColorFormatConverterError::InvalidInputLength) }

    let from_model = channel_order(from);
    let to_model = channel_order(to);

    let i_channels_num = if from as usize % 2 == 0 {4} else {3};
    let o_channels_num = if to as usize % 2 == 0 {4} else {3};

    let pixel_count = data.len() / i_channels_num;
    let mut out = Vec::with_capacity(pixel_count * o_channels_num);

    for pix in 0..pixel_count {
        let base = pix * i_channels_num;
        for &ch in to_model {
            let byte = match from_model.iter().position(|&c| c == ch) {
                Some(idx) => data[base + idx],
                None => {
                    // channel not in source: if it's alpha, default to 255; else error
                    match ch {
                        Channel::A => 255,
                        _ => unreachable!("This should never happen due to prior checks."),
                    }
                }
            };
            out.push(byte);
        }
    }

    return Ok(out);
}

// test these
fn from_rgb_to_cmyk(r: u8, g: u8, b: u8) -> (u8, u8, u8, u8) {

    let k = 255 - r.max(g).max(b);
    let c = (255 - r - k) / (255 - k);
    let m = (255 - g - k) / (255 - k);
    let y = (255 - b - k) / (255 - k);

    return (c,m,y,k);
}
// test these
fn from_rgb_to_hsl(r: u8, g: u8, b: u8) -> (u8, u8, u8) {

    let cmax = r.max(g).max(b);
    let cmin = r.min(g).min(b);
    let delta = cmax - cmin;

    let l = (cmax + cmin) / 2;

    let s = if delta == 0 { 0 } 
    else { delta / (255 - (2 * l - 255)) };

    let h = if delta == 0 { 0 } 
    else if cmax == r { 60 * (((g - b) / delta) % 6) }
    else if cmax == g { 60 * (((b - r) / delta) + 2) }
    else { 60 * (((r - g) / delta) + 4) };

    return (h, s, l);
}