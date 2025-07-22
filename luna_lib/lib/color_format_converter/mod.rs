/// This module provides a function to convert a vector of bytes representing pixel data from one color model to another.
/// 
/// The conversion is done by mapping the channels of the source color model to the destination color model.

pub const VERSION: crate::Version = crate::Version::new(1, 0, 0);


// TODO maybe add lume and other formats? needs calculations etc but its fine i think
/// Packed-byte color models.
#[repr(usize)]
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum ColorFormat {
    RGBA = 0,
    RGB,
    BGRA,
    BGR,
    GRBA,
    GRB,
    GBRA,
    GBR,
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
fn channel_order(format: ColorFormat) -> &'static [Channel] { // TODO why is this static?
    match format {
        ColorFormat::RGBA => &[Channel::R, Channel::G, Channel::B, Channel::A],
        ColorFormat::RGB  => &[Channel::R, Channel::G, Channel::B],
        ColorFormat::BGRA => &[Channel::B, Channel::G, Channel::R, Channel::A],
        ColorFormat::BGR  => &[Channel::B, Channel::G, Channel::R],
        ColorFormat::GRBA => &[Channel::G, Channel::R, Channel::B, Channel::A],
        ColorFormat::GRB  => &[Channel::G, Channel::R, Channel::B],
        ColorFormat::GBRA => &[Channel::G, Channel::B, Channel::R, Channel::A],
        ColorFormat::GBR  => &[Channel::G, Channel::B, Channel::R],
    }
}

/// Convert raw pixel bytes from one color model to another.
///
/// ## Parameters
/// - `data`: input byte-slice, length must be a multiple of `src.channel_count()`
/// - `from`: source color model (e.g. `ColorFormat::RGBA`)
/// - `to`: destination color model (e.g. `ColorFormat::BGR`)
///
/// ## Returns
/// A new `Vec<u8>` whose length is `pixel_count * to.channel_count()`, or an empty vector
/// if the input length isn’t a multiple of the source channel count.
///
/// ## Examples
///
/// ```rust
/// # use luna::img_manipulator::color_conversion::{convert_vec_color_model, ColorFormat};
///
/// // Single pixel RGBA → BGR
/// let rgba = vec![128, 55, 88, 255];
/// let bgr = convert_vec_color_model(&rgba, ColorFormat::RGBA, ColorFormat::BGR);
/// assert_eq!(bgr, vec![88, 55, 128]);
///
/// // Single pixel RGB → BGRA (alpha default to 255)
/// let rgb = vec![10, 20, 30];
/// let bgra = convert_vec_color_model(&rgb, ColorFormat::RGB, ColorFormat::BGRA);
/// assert_eq!(bgra, vec![30, 20, 10, 255]);
///
/// // Two pixels RGBA → RGB (drops alpha)
/// let two_rgba = vec![1, 2, 3, 255,  4, 5, 6, 128];
/// let two_rgb = convert_vec_color_model(&two_rgba, ColorFormat::RGBA, ColorFormat::RGB);
/// assert_eq!(two_rgb, vec![1, 2, 3,  4, 5, 6]);
/// ```
pub fn convert_vec_color_model(data: &[u8], from: ColorFormat, to: ColorFormat) -> Vec<u8> {
    if from == to { return data.to_vec() }
    if data.len() < 3 { return vec![]; } // TODO handle this better, maybe return an error or something

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
                        _ => return vec![], // TODO handle this better, maybe return an error or something
                    }
                }
            };
            out.push(byte);
        }
    }

    return out;
}