use super::error::Error;
use ::image::{DynamicImage, ImageFormat, ImageReader};
use std::io::Cursor;

/// Decodes a whole JPEG, whose channels are BGRA.
pub fn jpeg_to_image(jpeg: &[u8]) -> Result<DynamicImage, Error> {
    let image = ImageReader::with_format(Cursor::new(jpeg), ImageFormat::Jpeg).decode()?;
    let mut rgba = image.into_rgba8();
    for pixel in rgba.pixels_mut() {
        pixel.0.swap(0, 2);
    }
    Ok(DynamicImage::ImageRgba8(rgba))
}
