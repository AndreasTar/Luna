

use image::{self, DynamicImage, ImageError, imageops, imageops::colorops};

pub enum ImgOpenResult {
    Success(DynamicImage),
    Failure(ImageError),
}

impl ImgOpenResult {
    pub fn is_success(&self) -> bool {
        match self {
            ImgOpenResult::Success(_) => true,
            ImgOpenResult::Failure(_) => false,
        }
    }

    pub fn unwrap(self) -> DynamicImage {
        match self {
            ImgOpenResult::Success(img) => img,
            ImgOpenResult::Failure(e) => panic!("Tried to unwrap a failed image open result: {}", e),
        }
    }
    
}

// TODO add image info like format, dims, bytesize etc
// TODO add docs for all of these
// TODO better error handling

pub fn open_image_from_path(path: String) -> ImgOpenResult {
    let reader = match image::ImageReader::open(path) {
        Ok(reader) => reader,
        Err(e) => return ImgOpenResult::Failure(ImageError::IoError(e)),
    };

    return match reader.decode() { // Return format too?
        Ok(img_buff) => ImgOpenResult::Success(img_buff),
        Err(e) => ImgOpenResult::Failure(e),
    };
}

pub fn open_image_from_buffer() {
    todo!()
}

pub fn save_image() { // NOTE should this be here or some IO saving module? maybe a preprocess before IO module?
    todo!()
}

pub fn into_bytes(img: &DynamicImage) -> Vec<u8> {
    return img.as_bytes().to_owned();
}

pub fn into_rgba8(img: &DynamicImage) -> Vec<u8> {
    return img.to_rgba8().into_vec();
}

pub fn from_bytes(bytes: &[u8]) -> Result<DynamicImage, ImageError> {
    let img = image::load_from_memory(bytes)?;
    return Ok(img)
}

// TODO some of these dont have range, just on off, like invert and grayscale. change them to ranged or implement them as such
pub fn brighten(img: &mut DynamicImage, value: i32) {
    colorops::brighten_in_place(img, value);
}

pub fn contrast(img: &mut DynamicImage, value: f32) {
    colorops::contrast_in_place(img, value);
}

pub fn dither(){
    todo!()
}

pub fn grayscale(img: &mut DynamicImage) {
    *img = DynamicImage::grayscale(&img);
}

pub fn invert(img: &mut DynamicImage) {
    DynamicImage::invert(img);
}

pub fn blur(img: &mut DynamicImage, value: f32) {
    *img = DynamicImage::blur(img, value);
}

pub fn fast_blur(img: &mut DynamicImage, value: f32) {
    *img = DynamicImage::fast_blur(img, value);
}

pub fn unsharpen(img: &mut DynamicImage, value: f32, threshold: i32) {
    *img = DynamicImage::unsharpen(img, value, threshold);
}

pub fn huerotate(img: &mut DynamicImage, degrees: i32) {
    *img = DynamicImage::huerotate(img, degrees);
}

pub mod color_conversion {
    #![allow(dead_code, reason="This module is for color conversion utilities, not all functions are used internally.")]


    // TODO maybe add lume and other formats? needs calculations etc but its fine i think
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

    #[derive(PartialEq, Debug, Clone, Copy)]
    enum Channel {
        R,
        G,
        B,
        A,
    }

    fn channel_order(format: ColorFormat) -> &'static [Channel] { // TODO why is this static?
        match format {
            ColorFormat::RGBA => &[Channel::R, Channel::G, Channel::B, Channel::A],
            ColorFormat::RGB  => &[Channel::R, Channel::G, Channel::B],
            ColorFormat::BGRA => &[Channel::B, Channel::G, Channel::R, Channel::A],
            ColorFormat::BGR  => &[Channel::B, Channel::G, Channel::R],
            ColorFormat::GRBA => &[Channel::G, Channel::R, Channel::B, Channel::A],
            ColorFormat::GRB => &[Channel::G, Channel::R, Channel::B],
            ColorFormat::GBRA => &[Channel::G, Channel::B, Channel::R, Channel::A],
            ColorFormat::GBR => &[Channel::G, Channel::B, Channel::R],
        }
    }

    /// Convert raw pixel bytes from one color model to another.
    ///
    /// ## Parameters
    /// - `data`: input byte-slice, length must be a multiple of `src.channel_count()`
    /// - `src`: source color model (e.g. `ColorModel::RGBA`)
    /// - `dst`: destination color model (e.g. `ColorModel::BGR`)
    ///
    /// ## Returns
    /// A new `Vec<u8>` whose length is `pixel_count * dst.channel_count()`, or an empty vector
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

}
