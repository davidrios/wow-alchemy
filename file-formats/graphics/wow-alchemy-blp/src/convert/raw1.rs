use super::error::Error;
use super::generate_mipmaps;
use super::palette::*;
use crate::file::Palette;
use crate::header::BlpHeader;
use ::image::{DynamicImage, RgbaImage, imageops::FilterType};

/// Decodes a palettized level: an index per pixel, then the alpha packed at `alpha_bits` per
/// pixel from the lowest bits of each byte up, scaled to 0..=255.
pub fn raw1_to_image(
    header: &BlpHeader,
    palette: &Palette,
    bytes: &[u8],
    level: usize,
) -> Result<DynamicImage, Error> {
    let alpha_bits = header.alpha_bits();
    if ![0, 1, 4, 8].contains(&alpha_bits) {
        return Err(Error::Raw1InvalidAlphaBits(alpha_bits));
    }
    let (width, height) = header.mipmap_size(level);
    let pixels = width as usize * height as usize;
    let needed = pixels + (pixels * alpha_bits as usize).div_ceil(8);
    if bytes.len() < needed {
        return Err(Error::MipmapTooShort {
            level,
            width,
            height,
            needed,
            actual: bytes.len(),
        });
    }
    let (indices, alpha) = bytes.split_at(pixels);

    let mut image = RgbaImage::new(width, height);
    for (i, pixel) in image.pixels_mut().enumerate() {
        let color = palette[indices[i] as usize];
        let a = match alpha_bits {
            1 => ((alpha[i / 8] >> (i % 8)) & 0x01) * 0xFF,
            4 => ((alpha[i / 2] >> (i % 2 * 4)) & 0x0F) * 17,
            8 => alpha[i],
            _ => 0xFF,
        };
        pixel.0 = [(color >> 16) as u8, (color >> 8) as u8, color as u8, a];
    }
    Ok(if alpha_bits == 0 {
        DynamicImage::ImageRgb8(DynamicImage::ImageRgba8(image).into_rgb8())
    } else {
        DynamicImage::ImageRgba8(image)
    })
}

/// Quantizes the image and its mipmaps to one palette. Each level is the indices followed by
/// the alpha.
pub fn image_to_raw1(
    image: DynamicImage,
    alpha_bits: u32,
    make_mipmaps: bool,
    mipmap_filter: FilterType,
) -> Result<(Palette, Vec<Vec<u8>>), Error> {
    let levels = generate_mipmaps(image, make_mipmaps, mipmap_filter);
    let mut levels = levels.into_iter().map(DynamicImage::into_rgba8);

    let root = levels.next().ok_or(Error::MissingImage(0))?;
    let root_alpha = index_alpha(&root, alpha_bits)?;
    let (mut root_indices, colors, nq) = quantize_rgba(root);
    if colors.len() != 255 {
        return Err(Error::PaletteWrongSize(colors.len()));
    }
    // The last of the 256 entries isn't used.
    let mut palette = [0; 256];
    palette[..255].copy_from_slice(&colors);
    root_indices.extend(root_alpha);

    let mut mipmaps = vec![root_indices];
    for level in levels {
        let alpha = index_alpha(&level, alpha_bits)?;
        let mut indices = quantize_rgba_known(level, &nq);
        indices.extend(alpha);
        mipmaps.push(indices);
    }
    Ok((palette, mipmaps))
}

fn index_alpha(image: &RgbaImage, alpha_bits: u32) -> Result<Vec<u8>, Error> {
    let alphas = image.pixels().map(|pixel| pixel[3]);
    Ok(match alpha_bits {
        0 => vec![],
        8 => alphas.collect(),
        1 | 4 => {
            let bits = alpha_bits as usize;
            let per_byte = 8 / bits;
            let mut packed = vec![0; (image.len() / 4).div_ceil(per_byte)];
            for (i, alpha) in alphas.enumerate() {
                let value = if bits == 1 {
                    (alpha > 0) as u8
                } else {
                    ((alpha as u32 * 15 + 127) / 255) as u8
                };
                packed[i / per_byte] |= value << (i % per_byte * bits);
            }
            packed
        }
        _ => return Err(Error::Raw1InvalidAlphaBits(alpha_bits)),
    })
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::file::{BlpData, BlpImage};
    use ::image::{DynamicImage, Rgba, RgbaImage, imageops::FilterType};

    fn palettized(color: [u8; 4], alpha_bits: AlphaBits) -> BlpImage {
        let pixels = RgbaImage::from_pixel(8, 8, Rgba(color));
        image_to_blp(
            DynamicImage::ImageRgba8(pixels),
            false,
            BlpTarget::Blp2(Blp2Format::Raw1 { alpha_bits }),
            FilterType::Nearest,
        )
        .unwrap()
    }

    fn palette(blp: &BlpImage) -> &[u32] {
        match &blp.data {
            BlpData::Palette(palette) => palette,
            _ => panic!("not palettized"),
        }
    }

    /// The game's files keep palette entries as blue, green, red bytes: red is the third.
    #[test]
    fn palette_entries_are_bgr() {
        let blp = palettized([200, 20, 10, 255], AlphaBits::Bit8);
        let red = palette(&blp)
            .iter()
            .copied()
            .find(|c| c & 0xFFFFFF != 0)
            .unwrap();
        assert!(
            (red >> 16) & 0xFF > 150,
            "red should be the high byte: {red:08X}"
        );
        assert!(red & 0xFF < 50, "blue should be the low byte: {red:08X}");

        let pixel = blp_to_image(&blp, 0)
            .unwrap()
            .into_rgba8()
            .get_pixel(3, 3)
            .0;
        assert!(pixel[0] > 150 && pixel[2] < 50, "decoded {pixel:?}");
    }

    #[test]
    fn palettes_have_256_entries_and_the_last_is_unused() {
        let blp = palettized([90, 90, 90, 255], AlphaBits::NoAlpha);
        assert_eq!(palette(&blp).len(), 256);
        assert_eq!(palette(&blp)[255], 0);
        // 64 indices and no alpha.
        assert_eq!(blp.mipmaps[0].len(), 64);
    }

    #[test]
    fn low_bit_alphas_scale_to_eight_bits() {
        let opaque = blp_to_image(&palettized([90, 90, 90, 255], AlphaBits::Bit1), 0)
            .unwrap()
            .into_rgba8();
        assert_eq!(opaque.get_pixel(0, 0).0[3], 255);

        let half = blp_to_image(&palettized([90, 90, 90, 136], AlphaBits::Bit4), 0)
            .unwrap()
            .into_rgba8();
        assert_eq!(half.get_pixel(0, 0).0[3], 136);
        assert_eq!(half.get_pixel(1, 0).0[3], 136);
    }

    #[test]
    fn short_levels_are_an_error() {
        let mut blp = palettized([90, 90, 90, 255], AlphaBits::Bit8);
        blp.mipmaps[0].pop();
        assert!(matches!(
            blp_to_image(&blp, 0),
            Err(Error::MipmapTooShort { .. })
        ));
    }
}
