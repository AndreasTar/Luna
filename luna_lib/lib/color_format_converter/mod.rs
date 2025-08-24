//! This module provides a function to convert a vector of bytes representing pixel data from one color model to another.
//! 
//! The conversion is done by mapping the channels of the source color model to the destination color model.
//! Currently, only models with Red, Green, Blue, and Alpha channels are supported, with all their permutations.

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
    RGB,
    RBGA,
    RBG,

    GRBA,
    GRB,
    GBRA,
    GBR,

    BRGA,
    BRG,
    BGRA,
    BGR,
}

/// A color channel identifier.
/// Used to specify the order of channels in a color model.
#[derive(PartialEq, Debug, Clone, Copy)]
enum Channel {
    R,
    G,
    B,
    A,
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