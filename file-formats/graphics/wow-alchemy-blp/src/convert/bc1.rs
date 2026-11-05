use super::blp_to_image;
use super::error::Error;
use crate::file::{BlpFormat, BlpImage};
use ::image::{RgbaImage, imageops, imageops::FilterType};

const BC1_BLOCK_BYTES: usize = 8;
const BC3_BLOCK_BYTES: usize = 16;

/// Bytes in a square BC1 mipmap chain from `size` down to 1 pixel.
pub fn bc1_mipmaps_size(size: u32) -> usize {
    (0..=size.trailing_zeros())
        .map(|level| blocks_in_level(size >> level) * BC1_BLOCK_BYTES)
        .sum()
}

fn blocks_in_level(size: u32) -> usize {
    let side = size.max(1).div_ceil(4) as usize;
    side * side
}

/// Converts a texture to a BC1 mipmap chain for uploading to a GPU, dropping
/// alpha. `size` is the width and height of the first level, and must be a power
/// of two. The levels run down to 1 pixel, largest first.
///
/// Square DXT textures at least `size` wide are converted without any quality
/// loss. Anything else is decoded, resized and encoded again.
pub fn blp_to_bc1_mipmaps(image: &BlpImage, size: u32) -> Result<Vec<u8>, Error> {
    if !size.is_power_of_two() {
        return Err(Error::InvalidBc1Size(size));
    }

    match existing_mipmaps(image, size) {
        Some(mipmaps) => Ok(mipmaps),
        None => encode_mipmaps(image, size),
    }
}

/// Reuses the texture's own compressed mipmaps, if it has the whole chain.
fn existing_mipmaps(image: &BlpImage, size: u32) -> Option<Vec<u8>> {
    let block_bytes = match image.format().ok()? {
        BlpFormat::Dxt1 => BC1_BLOCK_BYTES,
        BlpFormat::Dxt3 | BlpFormat::Dxt5 => BC3_BLOCK_BYTES,
        _ => return None,
    };

    let width = image.header.width;
    if width != image.header.height || !width.is_power_of_two() || width < size {
        return None;
    }

    let first = (width / size).trailing_zeros() as usize;
    let levels = size.trailing_zeros() as usize + 1;
    let mut output = Vec::with_capacity(bc1_mipmaps_size(size));

    for level in 0..levels {
        let blocks = blocks_in_level(size >> level);
        let data = image
            .mipmaps
            .get(first + level)?
            .get(..blocks * block_bytes)?;

        if block_bytes == BC1_BLOCK_BYTES {
            output.extend_from_slice(data);
        } else {
            for block in data.chunks_exact(BC3_BLOCK_BYTES) {
                output.extend_from_slice(&bc3_block_to_bc1(block));
            }
        }
    }

    Some(output)
}

/// A BC2 or BC3 block is 8 bytes of alpha, then a color block that always has 4
/// colors. A BC1 block only has 4 colors when its first endpoint is the larger,
/// so smaller ones swap endpoints, which also swaps indexes 0 with 1 and 2 with 3.
fn bc3_block_to_bc1(block: &[u8]) -> [u8; BC1_BLOCK_BYTES] {
    let first = u16::from_le_bytes([block[8], block[9]]);
    let second = u16::from_le_bytes([block[10], block[11]]);
    let indexes = u32::from_le_bytes([block[12], block[13], block[14], block[15]]);

    let (first, second, indexes) = match first.cmp(&second) {
        std::cmp::Ordering::Greater => (first, second, indexes),
        std::cmp::Ordering::Less => (second, first, indexes ^ 0x5555_5555),
        // One solid color, which index 0 gives in either mode.
        std::cmp::Ordering::Equal => (first, second, 0),
    };

    let mut output = [0; BC1_BLOCK_BYTES];
    output[..2].copy_from_slice(&first.to_le_bytes());
    output[2..4].copy_from_slice(&second.to_le_bytes());
    output[4..].copy_from_slice(&indexes.to_le_bytes());
    output
}

fn encode_mipmaps(image: &BlpImage, size: u32) -> Result<Vec<u8>, Error> {
    let source = blp_to_image(image, 0)?.into_rgba8();
    encode_image(&source, size)
}

/// Converts RGBA pixels of any size, 4 bytes each, row by row, such as an image
/// read from a PNG, to a BC1 mipmap chain, as [`blp_to_bc1_mipmaps`] does for a
/// texture it must decode. `size` is the width and height of the first level,
/// and must be a power of two. `None` when the pixels don't fill the image.
pub fn rgba_to_bc1_mipmaps(pixels: &[u8], width: u32, height: u32, size: u32) -> Result<Option<Vec<u8>>, Error> {
    if !size.is_power_of_two() {
        return Err(Error::InvalidBc1Size(size));
    }

    let expected = (width as usize) * (height as usize) * 4;
    if width == 0 || height == 0 || pixels.len() != expected {
        return Ok(None);
    }

    let source = RgbaImage::from_raw(width, height, pixels.to_vec());
    source.map(|image| encode_image(&image, size)).transpose()
}

fn encode_image(source: &RgbaImage, size: u32) -> Result<Vec<u8>, Error> {
    let mut level = imageops::resize(source, size, size, FilterType::Triangle);
    let mut output = Vec::with_capacity(bc1_mipmaps_size(size));

    loop {
        encode_level(&mut level, &mut output);
        if level.width() == 1 {
            return Ok(output);
        }

        let next = level.width() / 2;
        level = imageops::resize(&level, next, next, FilterType::Triangle);
    }
}

fn encode_level(level: &mut RgbaImage, output: &mut Vec<u8>) {
    // Opaque pixels keep the encoder from using its transparent mode.
    for pixel in level.pixels_mut() {
        pixel[3] = 255;
    }

    let (width, height) = (level.width() as usize, level.height() as usize);
    let format = texpresso::Format::Bc1;
    let start = output.len();
    output.resize(start + format.compressed_size(width, height), 0);
    format.compress(
        level.as_raw(),
        width,
        height,
        texpresso::Params::default(),
        &mut output[start..],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::{Blp2Format, BlpTarget, DxtAlgorithm, image_to_blp};
    use ::image::DynamicImage;

    fn gradient_blp(size: u32, format: Blp2Format) -> BlpImage {
        let pixels = RgbaImage::from_fn(size, size, |x, y| {
            ::image::Rgba([
                (x * 7 % 256) as u8,
                (y * 5 % 256) as u8,
                ((x + y) % 256) as u8,
                200,
            ])
        });
        image_to_blp(
            DynamicImage::ImageRgba8(pixels),
            true,
            BlpTarget::Blp2(format),
            FilterType::Triangle,
        )
        .unwrap()
    }

    #[test]
    fn rgba_images_of_any_size_make_a_full_chain() {
        let pixels = RgbaImage::from_fn(300, 200, |x, y| ::image::Rgba([x as u8, y as u8, 0, 255]));
        let mipmaps = rgba_to_bc1_mipmaps(pixels.as_raw(), 300, 200, 64).unwrap().unwrap();
        assert_eq!(mipmaps.len(), bc1_mipmaps_size(64));
        assert!(rgba_to_bc1_mipmaps(pixels.as_raw(), 300, 200, 48).is_err());
        assert!(rgba_to_bc1_mipmaps(pixels.as_raw(), 300, 201, 64).unwrap().is_none());
    }

    fn dxt5() -> Blp2Format {
        Blp2Format::Dxt5 {
            has_alpha: true,
            compress_algorithm: DxtAlgorithm::RangeFit,
        }
    }

    fn decode_bc1(data: &[u8], size: u32) -> Vec<u8> {
        let mut output = vec![0; (size * size * 4) as usize];
        texpresso::Format::Bc1.decompress(data, size as usize, size as usize, &mut output);
        output
    }

    fn rgb(pixels: &[u8]) -> Vec<u8> {
        pixels
            .chunks_exact(4)
            .flat_map(|pixel| [pixel[0], pixel[1], pixel[2]])
            .collect()
    }

    #[test]
    fn chain_size_counts_every_level_down_to_one_pixel() {
        assert_eq!(bc1_mipmaps_size(1), 8);
        // 4x4, 2x2 and 1x1 are one block each.
        assert_eq!(bc1_mipmaps_size(4), 24);
        assert_eq!(bc1_mipmaps_size(8), 4 * 8 + 24);
    }

    #[test]
    fn bc3_colors_survive_without_loss_at_every_starting_mip() {
        let blp = gradient_blp(64, dxt5());

        for (mip, size) in [(0, 64), (1, 32), (2, 16)] {
            let chain = blp_to_bc1_mipmaps(&blp, size).unwrap();
            assert_eq!(chain.len(), bc1_mipmaps_size(size));

            let expected = blp_to_image(&blp, mip).unwrap().into_rgba8();
            let actual = decode_bc1(&chain, size);
            assert_eq!(rgb(&actual), rgb(expected.as_raw()), "mip {mip}");
        }
    }

    #[test]
    fn endpoints_in_the_wrong_order_are_swapped_keeping_the_colors() {
        let mut block = [0u8; 16];
        block[8..10].copy_from_slice(&0x001Fu16.to_le_bytes());
        block[10..12].copy_from_slice(&0xF800u16.to_le_bytes());
        // Pixels use indexes 0, 1, 2, 3 in turn.
        block[12..16].copy_from_slice(&0xE4E4_E4E4u32.to_le_bytes());

        let mut as_bc3 = vec![0; 16 * 4];
        texpresso::Format::Bc3.decompress(&block, 4, 4, &mut as_bc3);

        let converted = bc3_block_to_bc1(&block);
        let as_bc1 = decode_bc1(&converted, 4);

        assert_eq!(rgb(&as_bc1), rgb(&as_bc3));
        assert!(as_bc1.chunks_exact(4).all(|pixel| pixel[3] == 255));
    }

    #[test]
    fn equal_endpoints_stay_a_solid_color() {
        let mut block = [0u8; 16];
        block[8..10].copy_from_slice(&0x07E0u16.to_le_bytes());
        block[10..12].copy_from_slice(&0x07E0u16.to_le_bytes());
        block[12..16].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());

        let decoded = decode_bc1(&bc3_block_to_bc1(&block), 4);

        assert!(
            decoded
                .chunks_exact(4)
                .all(|pixel| pixel[1] > 200 && pixel[3] == 255)
        );
    }

    #[test]
    fn smaller_textures_are_encoded_up_to_the_requested_size() {
        let blp = gradient_blp(16, dxt5());

        let chain = blp_to_bc1_mipmaps(&blp, 64).unwrap();

        assert_eq!(chain.len(), bc1_mipmaps_size(64));
    }

    #[test]
    fn size_must_be_a_power_of_two() {
        let blp = gradient_blp(16, dxt5());

        assert!(matches!(
            blp_to_bc1_mipmaps(&blp, 10),
            Err(Error::InvalidBc1Size(10))
        ));
    }
}
