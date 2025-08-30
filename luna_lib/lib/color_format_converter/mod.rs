//! This module provides a function to convert a vector of bytes representing pixel data from one color model to another.
//! 
//! The conversion is done by mapping the channels of the source color model to the destination color model.
//! Currently, only models with RGB, CMYK, HSL and Grayscale are supported, with and without an Alpha channel,
//! with all their respective permutations.

pub const VERSION: crate::Version = crate::Version::new(1, 2, 0);

/// Possible errors that can occur during color format conversion, throughout this module.
/// - `InvalidInputLength`: The input data length is not a multiple of the source format's channel count.
/// - `SameFormat`: The source and destination formats are the same.
/// - `OutOfRange`: The input channel value is out of range for the given color format.
/// 
/// # Examples
/// ```rust
/// # use luna::color_format_converter::{convert_vec_color_model, from_cmyk_to_rgb, ColorFormat, ColorFormatConverterError};
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
/// 
/// // Out of range: eg. CMYK with a channel value > 100 (returns `Err(ColorFormatConverterError::OutOfRange)`)
/// let range_err = from_cmyk_to_rgb_checked(0.0, 101.0, 10.0, 0.0);
/// assert_eq!(range_err, Err(ColorFormatConverterError::OutOfRange)); 
/// ```
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum ColorFormatConverterError {
    /// Error signifying that the input data length is not a multiple of the source format's channel count.
    InvalidInputLength,
    /// Error signifying that the source and destination formats are the same.
    SameFormat,
    /// Error signifying that the input channel value is out of range for the given color format.
    OutOfRange,
}

impl std::fmt::Display for ColorFormatConverterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColorFormatConverterError::InvalidInputLength => write!(f, "Input data length is not a multiple of the source format's channel count."),
            ColorFormatConverterError::SameFormat => write!(f, "Source and destination formats are the same."),
            ColorFormatConverterError::OutOfRange => write!(f, "Input channel value is out of range for the given color format."),
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

/// Convert raw pixel bytes from RGB color model to CMYK.
/// 
/// This function is implemented using integer arithmetic to avoid floating-point math.
/// I *assume* it is faster, although it hasn't really been benchmarked.
/// However, it is a bit inaccurate due to rounding errors. This means that some resulting values
/// may be off by + or - 1 compared to a floating-point implementation. If you don't mind that,
/// this function should be fine for most use cases.
/// For an accurate conversion, use `from_rgb_to_cmyk()`.
///
/// ## Parameters
/// - `r`: Red channel value (0-255)
/// - `g`: Green channel value (0-255)
/// - `b`: Blue channel value (0-255)
///
/// ## Returns
/// A tuple of `u8`'s containing the CMYK channel values as integers in the range 0-255.
/// CMYK is often represented as percentages (0-100), but here we map the result to 0-255 for consistency with RGB.
/// For example, if we have C=100%, M=10%, Y=50%, K=0%, the returned values will be (255, 25, 127, 0), or close
/// to that, due to rounding errors.
/// 
/// For a percentage-based input (0-100), use `from_cmyk_to_rgb_integer_percentile()`.
///
/// ## Examples
///
/// ```rust
/// # use luna::color_format_converter::from_rgb_to_cmyk_integer;
/// // Rudimentary helper function for approximate equality check
/// fn approx_eq(left: (u8,u8,u8,u8), right: (u8,u8,u8,u8), eps: f32) -> bool {
///     let (lc, lm, ly, lk) = left;
///     let (rc, rm, ry, rk) = right;
/// 
///     if (lc as f32 - rc as f32).abs() > eps { return false; }
///     if (lm as f32 - rm as f32).abs() > eps { return false; }
///     if (ly as f32 - ry as f32).abs() > eps { return false; }
///     if (lk as f32 - rk as f32).abs() > eps { return false; }
///     return true;
/// }
/// 
/// let (c, m, y, k) = from_rgb_to_cmyk_integer(255, 0, 0); // Pure red
/// assert!(approx_eq((c, m, y, k), (0, 255, 255, 0), 1_f32)); 
/// 
/// let (c, m, y, k) = from_rgb_to_cmyk_integer(58, 31, 156); // Purplish blue
/// assert!(approx_eq((c, m, y, k), (160, 204, 0, 99), 1_f32)); 
/// 
/// let (c, m, y, k) = from_rgb_to_cmyk_integer(127, 127, 127); // Mid gray
/// assert!(approx_eq((c, m, y, k), (0, 0, 0, 127), 1_f32)); // This may be off by + or - 1 due to rounding errors
/// ```
pub fn from_rgb_to_cmyk_integer(r: u8, g: u8, b: u8) -> (u8, u8, u8, u8) {

    let r = r as u32;
    let g = g as u32;
    let b = b as u32;

    let max = r.max(g).max(b);
    if max == 0 { return (0, 0, 0, 255); } // Pure black

    let k = 255 - max;    

    let c = (((max - r) * 255 + max/2 ) / max) as u8;
    let m = (((max - g) * 255 + max/2 ) / max) as u8;
    let y = (((max - b) * 255 + max/2 ) / max) as u8;

    return (c, m, y, k as u8);
}

/// Convert raw pixel bytes from RGB color model to CMYK.
/// 
/// This function is implemented using floating-point arithmetic, which makes it *probably*, slower than
/// an integer-math implementation. I haven't really benchmarked it though.
/// However, it is more accurate than an integer-math implementation.
/// For an integer-math, slightly inaccurate conversion, use `from_rgb_to_cmyk_integer()`.
///
/// ## Parameters
/// - `r`: Red channel value (0-255)
/// - `g`: Green channel value (0-255)
/// - `b`: Blue channel value (0-255)
///
/// ## Returns
/// A tuple of `f32`'scontaining the CMYK channel values in the range 0-255.
/// CMYK is often represented as percentages (0-100), but here we map the result to 0-255 as integers for consistency with RGB.
/// For example, if we have C=100%, M=10%, Y=50%, K=0%, the returned values will be (255, 25.5, 127.5, 0).
/// 
/// For a percentage-based input (0-100), use `from_rgb_to_cmyk_percentile()`.
///
/// ## Examples
///
/// ```rust
/// # use luna::color_format_converter::from_rgb_to_cmyk;
/// let (c, m, y, k) = from_rgb_to_cmyk(255, 0, 0); // Pure red
/// assert_eq!((c, m, y, k), (0.0, 255.0, 255.0, 0.0)); 
/// 
/// let (c, m, y, k) = from_rgb_to_cmyk(127, 127, 127); // Mid gray
/// assert_eq!((c, m, y, k), (0.0, 0.0, 0.0, 128.0));
/// 
/// let (c, m, y, k) = from_rgb_to_cmyk(58, 31, 156); // Purplish blue
/// assert_eq!((c, m, y, k), (160.0, 204.0, 0.0, 99.0)); 
/// ```
pub fn from_rgb_to_cmyk(r: u8, g: u8, b: u8) -> (f32, f32, f32, f32) {

    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;

    let k = 1.0 - r.max(g).max(b);
    if k == 1.0 { return (0.0, 0.0, 0.0, 255.0); } // Pure black

    let c = (1.0 - r - k) / (1.0 - k);
    let m = (1.0 - g - k) / (1.0 - k);
    let y = (1.0 - b - k) / (1.0 - k);

    return ((c * 255.0).round(), (m * 255.0).round(), (y * 255.0).round(), (k * 255.0).round());
}

/// Convert raw pixel bytes from RGB color model to CMYK.
/// 
/// This function is implemented using integer arithmetic to avoid floating-point math.
/// I *assume* it is faster, although it hasn't really been benchmarked.
/// However, it is a bit inaccurate due to rounding errors. This means that some resulting values
/// may be off by + or - 1 compared to a floating-point implementation. If you don't mind that,
/// this function should be fine for most use cases.
/// For an accurate conversion, use `from_rgb_to_cmyk_percentile()`.
///
/// ## Parameters
/// - `r`: Red channel value (0-255)
/// - `g`: Green channel value (0-255)
/// - `b`: Blue channel value (0-255)
///
/// ## Returns
/// A tuple of `u8`'s containing the CMYK channel values as integers in the range 0-100.
/// 
/// For a conversion with results in range 0-255, use `from_rgb_to_cmyk_integer()`.
///
/// ## Examples
///
/// ```rust
/// # use luna::color_format_converter::from_rgb_to_cmyk_integer_percentile;
/// // Rudimentary helper function for approximate equality check
/// fn approx_eq(left: (u8,u8,u8,u8), right: (u8,u8,u8,u8), eps: f32) -> bool {
///     let (lc, lm, ly, lk) = left;
///     let (rc, rm, ry, rk) = right;
/// 
///     if (lc as f32 - rc as f32).abs() > eps { return false; }
///     if (lm as f32 - rm as f32).abs() > eps { return false; }
///     if (ly as f32 - ry as f32).abs() > eps { return false; }
///     if (lk as f32 - rk as f32).abs() > eps { return false; }
///     return true;
/// }
/// 
/// let (c, m, y, k) = from_rgb_to_cmyk_integer_percentile(255, 0, 0); // Pure red
/// assert!(approx_eq((c, m, y, k), (0, 100, 100, 0), 1_f32)); 
/// 
/// let (c, m, y, k) = from_rgb_to_cmyk_integer_percentile(58, 31, 156); // Purplish blue
/// assert!(approx_eq((c, m, y, k), (63, 80, 0, 39), 1_f32)); 
/// 
/// let (c, m, y, k) = from_rgb_to_cmyk_integer_percentile(127, 127, 127); // Mid gray
/// assert!(approx_eq((c, m, y, k), (0, 0, 0, 50), 1_f32)); // This may be off by + or - 1 due to rounding errors
/// ```
pub fn from_rgb_to_cmyk_integer_percentile(r: u8, g: u8, b: u8) -> (u8, u8, u8, u8) {
    let max = r.max(g).max(b);
    if max == 0 {
        return (0, 0, 0, 100);
    }

    let max_u32 = max as u32;

    // C
    let num_c = (max - r) as u32 * 100;
    let add_c = max_u32 / 2;
    let mut c = (num_c + add_c) / max_u32;
    if c > 100 { c = 100; }

    // M
    let num_m = (max - g) as u32 * 100;
    let add_m = max_u32 / 2;
    let mut m = (num_m + add_m) / max_u32;
    if m > 100 { m = 100; }

    // Y
    let num_y = (max - b) as u32 * 100;
    let add_y = max_u32 / 2;
    let mut y = (num_y + add_y) / max_u32;
    if y > 100 { y = 100; }

    // K (denominator 255)
    let num_k = (255 - max_u32) * 100;
    let mut k = (num_k + 127) / 255;
    if k > 100 { k = 100; }

    return (c as u8, m as u8, y as u8, k as u8);
}

/// Convert raw pixel bytes from RGB color model to CMYK.
/// 
/// This function is implemented using floating-point arithmetic, which makes it *probably*, slower than
/// an integer-math implementation. I haven't really benchmarked it though.
/// However, it is more accurate than an integer-math implementation.
/// For an integer-math, slightly inaccurate conversion, use `from_rgb_to_cmyk_integer()`.
///
/// ## Parameters
/// - `r`: Red channel value (0-255)
/// - `g`: Green channel value (0-255)
/// - `b`: Blue channel value (0-255)
///
/// ## Returns
/// A tuple of `f32`'scontaining the CMYK channel values in the range 0-100.
/// 
/// For a conversion with results in range 0-255, use `from_rgb_to_cmyk()`.
///
/// ## Examples
///
/// ```rust
/// # use luna::color_format_converter::from_rgb_to_cmyk_percentile;
/// let (c, m, y, k) = from_rgb_to_cmyk_percentile(255, 0, 0); // Pure red
/// assert_eq!((c, m, y, k), (0.0, 100.0, 100.0, 0.0)); 
/// 
/// let (c, m, y, k) = from_rgb_to_cmyk_percentile(127, 127, 127); // Mid gray
/// assert_eq!((c, m, y, k), (0.0, 0.0, 0.0, 50.0));
/// 
/// let (c, m, y, k) = from_rgb_to_cmyk_percentile(58, 31, 156); // Purplish blue
/// assert_eq!((c, m, y, k), (63.0, 80.0, 0.0, 39.0)); 
/// ```
pub fn from_rgb_to_cmyk_percentile(r: u8, g: u8, b: u8) -> (f32, f32, f32, f32) {
    let r = r as f32 /255.0;
    let g = g as f32 /255.0;
    let b = b as f32 /255.0;

    let k = 1.0 - r.max(g).max(b);
    if k == 1.0 { return (0.0, 0.0, 0.0, 100.0); } // Pure black

    let c = (1.0 - r - k) / (1.0 - k);
    let m = (1.0 - g - k) / (1.0 - k);
    let y = (1.0 - b - k) / (1.0 - k);

    return ((c * 100.0).round(), (m * 100.0).round(), (y * 100.0).round(), (k * 100.0).round());
}

/// Convert raw pixel bytes from CMYK color model to RGB.
/// 
/// This function is implemented using integer arithmetic to avoid floating-point math.
/// I *assume* it is faster, although it hasn't really been benchmarked.
/// However, it is a bit inaccurate due to rounding errors. This means that some resulting values
/// may be off by + or - 1 compared to a floating-point implementation. If you don't mind that,
/// this function should be fine for most use cases.
/// For an accurate conversion, use `from_cmyk_to_rgb()`.
/// 
/// CMYK is often represented as percentages (0-100), but here we require that the input is in the range 
/// 0-255 for consistency with RGB.
/// For example, if we have C=100%, M=10%, Y=50%, K=0%, the input values should be (255, 25, 127, 0).
/// 
/// For a percentage-based input (0-100), use `from_cmyk_to_rgb_integer_percentile()`.
///
/// ## Parameters
/// - `c`: Cyan channel value (0-255)
/// - `m`: Magenta channel value (0-255)
/// - `y`: Yellow channel value (0-255)
/// - `k`: Key (Black) channel value (0-255)
///
/// ## Returns
/// A tuple of `u8`'s containing the RGB channel values as integers in the range 0-255.
///
/// ## Examples
///
/// ```rust
/// # use luna::color_format_converter::from_cmyk_to_rgb_integer;
/// // Rudimentary helper function for approximate equality check
/// fn approx_eq(left: (u8,u8,u8), right: (u8,u8,u8), eps: f32) -> bool {
///     let (lr, lg, lb) = left;
///     let (rr, rg, rb) = right;
/// 
///     if (lr as f32 - rr as f32).abs() > eps { return false; }
///     if (lg as f32 - rg as f32).abs() > eps { return false; }
///     if (lb as f32 - rb as f32).abs() > eps { return false; }
///     return true;
/// }
/// 
/// let (r, g, b) = from_cmyk_to_rgb_integer(0, 255, 255, 0); // Pure red
/// assert!(approx_eq((r, g, b), (255, 0, 0), 1_f32)); 
/// 
/// let (r, g, b) = from_cmyk_to_rgb_integer(160, 204, 0, 99); // Purplish blue
/// assert!(approx_eq((r, g, b), (58, 31, 156), 1_f32)); 
/// 
/// let (r, g, b) = from_cmyk_to_rgb_integer(0, 0, 0, 127); // Mid gray
/// assert!(approx_eq((r, g, b), (127, 127, 127), 1_f32)); // This may be off by + or - 1 due to rounding errors
/// ```
pub fn from_cmyk_to_rgb_integer(c: u8, m: u8, y: u8, k: u8) -> (u8, u8, u8) {
    let c = c as u32;
    let m = m as u32;
    let y = y as u32;
    let k = k as u32;

    let r = 255 - ((c * (255 - k) + k * 255 + 127) / 255);
    let g = 255 - ((m * (255 - k) + k * 255 + 127) / 255);
    let b = 255 - ((y * (255 - k) + k * 255 + 127) / 255);

    return (r as u8, g as u8, b as u8);
}

/// Convert raw pixel bytes from CMYK color model to RGB.
/// 
/// This function is implemented using floating-point arithmetic, which makes it *probably*, slower than
/// an integer-math implementation. I haven't really benchmarked it though.
/// However, it is more accurate than an integer-math implementation.
/// For an integer-math, slightly inaccurate conversion, use `from_cmyk_to_rgb_integer()`.
///
/// CMYK is often represented as percentages (0-100), but here we require that the input is in the range 
/// 0-255 for consistency with RGB.
/// For example, if we have C=100%, M=10%, Y=50%, K=0%, the input values should be (255, 25, 127, 0).
/// 
/// For a percentage-based input (0-100), use `from_cmyk_to_rgb_percentile()`.
///
/// ## Parameters
/// - `c`: Cyan channel value (0-255)
/// - `m`: Magenta channel value (0-255)
/// - `y`: Yellow channel value (0-255)
/// - `k`: Key (Black) channel value (0-255)
///
/// ## Returns
/// A tuple of `f32`'scontaining the RGB channel values in the range 0-255.
///
/// ## Examples
///
/// ```rust
/// # use luna::color_format_converter::from_cmyk_to_rgb;
/// let (r, g, b) = from_cmyk_to_rgb(0.0, 255.0, 255.0, 0.0); // Pure red
/// assert_eq!((r, g, b), (255.0, 0.0, 0.0)); 
/// 
/// let (r, g, b) = from_cmyk_to_rgb(160.0, 204.0, 0.0, 99.0); // Purplish blue
/// assert_eq!((r, g, b), (58.0, 31.0, 156.0)); 
/// 
/// let (r, g, b) = from_cmyk_to_rgb(0.0, 0.0, 0.0, 127.0); // Mid gray
/// assert_eq!((r, g, b), (128.0, 128.0, 128.0));
/// ```
pub fn from_cmyk_to_rgb(c: f32, m: f32, y: f32, k: f32) -> (f32, f32, f32) {
    let c = c / 255.0;
    let m = m / 255.0;
    let y = y / 255.0;
    let k = k / 255.0;

    let precalc = 255.0 * (1.0-k);

    let r = (1.0 - c) * precalc;
    let g = (1.0 - m) * precalc;
    let b = (1.0 - y) * precalc;

    return (r.round(), g.round(), b.round());
}

pub fn from_cmyk_to_rgb_integer_percentile(c: u8, m: u8, y: u8, k: u8) -> (u8, u8, u8) {
    let c = c as f32 / 100.0;
    let m = m as f32 / 100.0;
    let y = y as f32 / 100.0;
    let k = k as f32 / 100.0;

    let precalc = 255.0 * (1.0-k);

    let r = (1.0 - c) * precalc;
    let g = (1.0 - m) * precalc;
    let b = (1.0 - y) * precalc;

    return (r.round() as u8, g.round() as u8, b.round() as u8);
}

pub fn from_cmyk_to_rgb_percentile(c: f32, m: f32, y: f32, k: f32) -> (f32, f32, f32) {
    let c = c / 100.0;
    let m = m / 100.0;
    let y = y / 100.0;
    let k = k / 100.0;

    let precalc = 255.0 * (1.0-k);

    let r = (1.0 - c) * precalc;
    let g = (1.0 - m) * precalc;
    let b = (1.0 - y) * precalc;

    return (r.round(), g.round(), b.round());
}


pub fn from_cmyk_to_rgb_integer_percentile_checked(c: u8, m: u8, y: u8, k: u8) -> Result<(u8, u8, u8), ColorFormatConverterError> {
    if c > 100 || m > 100 || y > 100 || k > 100 {
        return Err(ColorFormatConverterError::OutOfRange);
    }
    return Ok(from_cmyk_to_rgb_integer_percentile(c, m, y, k));
}

pub fn from_cmyk_to_rgb_percentile_checked(c: f32, m: f32, y: f32, k: f32) -> Result<(f32, f32, f32), ColorFormatConverterError> {
    if  c < 0.0 || c > 100.0 || 
        m < 0.0 || m > 100.0 || 
        y < 0.0 || y > 100.0 || 
        k < 0.0 || k > 100.0 
    {
        return Err(ColorFormatConverterError::OutOfRange);
    }
    return Ok(from_cmyk_to_rgb_percentile(c, m, y, k));
}

// test these
pub fn from_rgb_to_hsl(r: u8, g: u8, b: u8) -> (u8, u8, u8) {

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

pub fn from_hsl_to_rgb(h: u8, s: u8, l: u8) -> (u8, u8, u8) {
    // TODO
    return (0, 0, 0);
}