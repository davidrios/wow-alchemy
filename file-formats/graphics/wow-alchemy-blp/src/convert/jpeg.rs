use super::error::Error;
use super::mipmap::generate_mipmaps;
use crate::types::jpeg::MAX_JPEG_HEADER;
use crate::types::*;
use ::image::{DynamicImage, ImageFormat, ImageReader, RgbaImage, imageops::FilterType};
use log::*;
use std::io::Cursor;

pub fn jpeg_to_image(image: &BlpJpeg, mipmap_level: usize) -> Result<DynamicImage, Error> {
    let raw_jpeg = image
        .full_jpeg(mipmap_level)
        .ok_or(Error::MissingImage(mipmap_level))?;
    let jpeg = ImageReader::with_format(Cursor::new(raw_jpeg), ImageFormat::Jpeg).decode()?;
    let mut rgba = jpeg.into_rgba8();
    switch_red_blue(&mut rgba);
    Ok(DynamicImage::ImageRgba8(rgba))
}

pub fn image_to_jpeg(
    image: &DynamicImage,
    make_mipmaps: bool,
    mut alpha_bits: u8,
    mipmap_filter: FilterType,
) -> Result<BlpJpeg, Error> {
    if alpha_bits != 0 && alpha_bits != 8 {
        warn!("Invalid alpha bits value for JPEG encoding {alpha_bits}, defaulting to 0");
        alpha_bits = 0;
    }
    let mut rgba = image.to_rgba8();
    switch_red_blue(&mut rgba);
    if alpha_bits == 0 {
        fill_opaque_alpha(&mut rgba);
    }

    let mut images: Vec<Vec<u8>> = if make_mipmaps {
        let images = generate_mipmaps(DynamicImage::ImageRgba8(rgba), mipmap_filter)?;
        let jpeg_images: Result<Vec<Vec<u8>>, Error> = images
            .into_iter()
            .map(|image| {
                let mut image_bytes = vec![];
                image.write_to(&mut Cursor::new(&mut image_bytes), ImageFormat::Jpeg)?;
                Ok(image_bytes)
            })
            .collect();
        jpeg_images?
    } else {
        let mut root_img = vec![];
        rgba.write_to(&mut Cursor::new(&mut root_img), ImageFormat::Jpeg)?;
        vec![root_img]
    };
    let mut header = fetch_common_header(&mut images);
    // Add two padding bytes to the header as it always persists in War3 files
    header.extend(&vec![0; 2]);
    Ok(BlpJpeg { header, images })
}

fn switch_red_blue(image: &mut RgbaImage) {
    for pixel in image.pixels_mut() {
        let blue = pixel.0[0];
        let green = pixel.0[1];
        let red = pixel.0[2];
        let alpha = pixel.0[3];
        pixel.0 = [red, green, blue, alpha];
    }
}

// Warcraft III has inconsistent processing for CONTENT_JPEG with 0 bit
// alpha as UI images are correctly opaque but alpha component values are
// still used when blending model textures. As such when writing
// CONTENT_JPEG with 0 bit alpha it is required that the alpha band be
// assigned opaque component values (0xFF).
fn fill_opaque_alpha(image: &mut RgbaImage) {
    for pixel in image.pixels_mut() {
        pixel.0[3] = 0xFF;
    }
}

// Allows to get common part of all images and consider it as a 'JPEG header'
fn fetch_common_header(images: &mut [Vec<u8>]) -> Vec<u8> {
    let mut header = vec![];
    if images.is_empty() || images[0].is_empty() {
        return header;
    }
    let mut common_bytes = 0;
    'outer: for i in 0..MAX_JPEG_HEADER {
        if images[0].len() <= i {
            break;
        }
        let current_byte = images[0][i];
        for image in images.iter() {
            if image.len() <= i || image[i] != current_byte {
                break 'outer;
            }
        }
        common_bytes += 1;
    }
    trace!("Common bytes in jpegs are {common_bytes}");
    if common_bytes > 0 {
        header.extend(&images[0][0..common_bytes]);
        for image in images.iter_mut() {
            image.drain(0..common_bytes);
        }
    }
    header
}
