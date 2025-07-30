
use image::{self, DynamicImage, ImageError, imageops, ImageFormat};

pub const VERSION: crate::Version = crate::Version::new(0, 2, 1);

/// An enum which can be returned when attempting to open an image from a path and decode it.
/// It can either be a success with the decoded image, or a failure with an [ImageError].
/// 
/// ## Potential causes of failure
/// `Failure(...)` can be thrown because of an invalid path given to the function, or because
/// the image selected is not supported or is invalid. Check out [image::ImageError] for more details on the possible errors.
pub enum ImgOpenResult {
    Success(DynamicImage),
    Failure(ImageError),
}

impl ImgOpenResult {

    /// Returns `true` if the result is a success type, `false` if it is a failure type.
    /// ## Examples
    /// ```ignore
    /// # use luna_lib::img_manipulator::{open_image_from_path, ImgOpenResult};
    /// 
    /// let image = open_image_from_path("path/to/image.png".to_string());
    /// assert!(image.is_success());
    /// 
    /// let image = open_image_from_path("path/to/invalid_image.png".to_string());
    /// assert!(!image.is_success());
    /// ```
    #[inline]
    pub fn is_success(&self) -> bool {
        match self {
            ImgOpenResult::Success(_) => true,
            ImgOpenResult::Failure(_) => false,
        }
    }

    /// Returns the contained `Success` value, consuming the `self` value.
    /// 
    /// Because this function may panic, its use is generally discouraged.
    /// Panics are meant for unrecoverable errors, and
    /// [may abort the entire program][panic-abort].
    /// 
    /// Instead, prefer to use pattern matching and handle the `Failure`
    /// case explicitly.
    /// 
    /// [panic-abort]: https://doc.rust-lang.org/book/ch09-01-unrecoverable-errors-with-panic.html
    /// 
    /// ## Panics
    ///
    /// Panics if the self value equals [`Failure`].
    #[inline]
    pub fn unwrap(self) -> DynamicImage {
        match self {
            ImgOpenResult::Success(img) => img,
            ImgOpenResult::Failure(e) => panic!("Tried to unwrap a failed image open result: {}", e),
        }
    }
    
}


/// A struct which contains information about an image, such as its format, dimensions, aspect ratio, etc.
#[derive(Clone, Debug)]
pub struct ImageInfo {
    pub format: Option<ImageFormat>,
    pub dimensions: (u32, u32),
    pub aspect_ratio: f32,
    pub bytesize: usize,
}

impl ImageInfo {
    pub fn new(
        format: Option<ImageFormat>,
        dimensions: (u32, u32),
        aspect_ratio: f32,
        bytesize: usize
    ) -> Self {

        return Self {
            format,
            dimensions,
            aspect_ratio,
            bytesize
        };
    }
}

impl Default for ImageInfo {
    fn default() -> Self {
        return Self {
            format: None,
            dimensions: (0, 0),
            aspect_ratio: 0.0,
            bytesize: 0
        };
    }
}

impl ToString for ImageInfo {
    fn to_string(&self) -> String {

        let form = if self.format.is_some() {
            match self.format.unwrap() {
                ImageFormat::Png => "PNG",
                ImageFormat::Jpeg => "JPEG",
                ImageFormat::Gif => "GIF",
                ImageFormat::WebP => "WEBP",
                ImageFormat::Pnm => "PNM",
                ImageFormat::Tiff => "TIFF",
                ImageFormat::Tga => "TGA",
                ImageFormat::Dds => "DDS",
                ImageFormat::Bmp => "BMP",
                ImageFormat::Ico => "ICO",
                ImageFormat::Hdr => "HDR",
                ImageFormat::OpenExr => "OpenEXR",
                ImageFormat::Farbfeld => "Farbfeld",
                ImageFormat::Avif => "AVIF",
                ImageFormat::Qoi => "QOI",
                ImageFormat::Pcx => "PCX",
                _ => "Unknown",
            }
        } else {
            "Unknown"
        };

        return format!(
            "Format: {:?}, Dimensions: {}, Aspect Ratio: {:.3}, Bytesize: {}",
            form, 
            if self.dimensions.0 > 0 && self.dimensions.1 > 0 { format!("{}x{}", self.dimensions.0, self.dimensions.1) } else { "Unknown".to_string() }, 
            if self.aspect_ratio > 0.0 { self.aspect_ratio.to_string() } else { "Unknown".to_string() }, 
            if self.bytesize > 0 { self.bytesize.to_string() } else { "Unknown".to_string() }
        );
    }
}

// TODO implement to string

// TODO add docs for all of these
// TODO better error handling
// TODO some of these dont have range, just on off, like invert and grayscale. change them to ranged or implement them as such


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

pub fn get_image_info(img: &Option<DynamicImage>) -> ImageInfo {
    if img.is_none() {
        return ImageInfo::default();
    }
    let img = img.as_ref().unwrap();

    let format = None; // HACK hardcoded
    let dimensions = (img.width(), img.height());
    let aspect_ratio = dimensions.0 as f32 / dimensions.1 as f32;
    let bytesize = img.as_bytes().len();

    return ImageInfo::new(format, dimensions, aspect_ratio, bytesize);
}

pub fn into_bytes(img: &DynamicImage) -> Vec<u8> {
    return img.as_bytes().to_owned();
}

pub fn into_rgba8(img: &DynamicImage) -> Vec<u8> {
    return img.to_rgba8().into_vec();
}

pub fn from_bytes(bytes: &[u8]) -> Result<DynamicImage, ImageError> {
    let img = image::load_from_memory(bytes)?; // TODO check out ImageReader::new
    return Ok(img)
}

pub fn brighten(img: &mut DynamicImage, value: i32) {
    // imageops::colorops::brighten_in_place(img, value);
    *img = img.brighten(value);
}

pub fn contrast(img: &mut DynamicImage, value: f32) {
    // imageops::colorops::contrast_in_place(img, value);
    *img = img.adjust_contrast(value);
}

pub fn dither() {
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

pub fn flip_vertical(img: &mut DynamicImage) {
    *img = DynamicImage::flipv(img);
}

pub fn flip_horizontal(img: &mut DynamicImage) {
    *img = DynamicImage::fliph(img);
}
