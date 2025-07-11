

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
