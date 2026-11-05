use super::error::Error;
use super::generate_mipmaps;
use crate::header::BlpHeader;
use ::image::{DynamicImage, RgbaImage, imageops::FilterType};

/// Decodes a level of DXT blocks.
pub fn dxtn_to_image(
    header: &BlpHeader,
    format: texpresso::Format,
    bytes: &[u8],
    level: usize,
) -> Result<DynamicImage, Error> {
    let (width, height) = header.mipmap_size(level);
    let (width_px, height_px) = (width as usize, height as usize);
    let needed = format.compressed_size(width_px, height_px);
    if bytes.len() < needed {
        return Err(Error::MipmapTooShort {
            level,
            width,
            height,
            needed,
            actual: bytes.len(),
        });
    }
    let mut output = vec![0; width_px * height_px * 4];
    format.decompress(bytes, width_px, height_px, &mut output);
    let image = RgbaImage::from_raw(width, height, output).expect("sized for the image");
    Ok(DynamicImage::ImageRgba8(image))
}

/// The image and its mipmaps as DXT blocks.
pub fn image_to_dxtn(
    image: DynamicImage,
    format: texpresso::Format,
    make_mipmaps: bool,
    mipmap_filter: FilterType,
    algorithm: texpresso::Algorithm,
) -> Vec<Vec<u8>> {
    let params = texpresso::Params {
        algorithm,
        ..Default::default()
    };
    generate_mipmaps(image, make_mipmaps, mipmap_filter)
        .into_iter()
        .map(|level| {
            let rgba = level.into_rgba8();
            let (width, height) = (rgba.width() as usize, rgba.height() as usize);
            let mut output = vec![0; format.compressed_size(width, height)];
            format.compress(rgba.as_raw(), width, height, params, &mut output);
            output
        })
        .collect()
}
