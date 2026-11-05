use super::error::Error;
use super::generate_mipmaps;
use crate::header::BlpHeader;
use ::image::{DynamicImage, RgbaImage, imageops::FilterType};

/// Decodes a level of BGRA pixels.
pub fn raw3_to_image(
    header: &BlpHeader,
    bytes: &[u8],
    level: usize,
) -> Result<DynamicImage, Error> {
    let (width, height) = header.mipmap_size(level);
    let needed = width as usize * height as usize * 4;
    let Some(bgra) = bytes.get(..needed) else {
        return Err(Error::MipmapTooShort {
            level,
            width,
            height,
            needed,
            actual: bytes.len(),
        });
    };
    let rgba = bgra
        .chunks_exact(4)
        .flat_map(|p| [p[2], p[1], p[0], p[3]])
        .collect();
    let image = RgbaImage::from_raw(width, height, rgba).expect("sized for the image");
    Ok(DynamicImage::ImageRgba8(image))
}

/// The image and its mipmaps as BGRA pixels.
pub fn image_to_raw3(
    image: DynamicImage,
    make_mipmaps: bool,
    mipmap_filter: FilterType,
) -> Vec<Vec<u8>> {
    generate_mipmaps(image, make_mipmaps, mipmap_filter)
        .into_iter()
        .map(|level| {
            level
                .into_rgba8()
                .pixels()
                .flat_map(|p| [p[2], p[1], p[0], p[3]])
                .collect()
        })
        .collect()
}
