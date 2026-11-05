//! Turning BLPs into images, and images into BLPs.

mod bc1;
mod dxtn;
mod error;
mod jpeg;
mod palette;
mod raw1;
mod raw3;

use crate::file::{BlpData, BlpFormat, BlpImage};
use crate::header::*;
use ::image::DynamicImage;
pub use ::image::imageops::FilterType;
pub use bc1::{bc1_mipmaps_size, blp_to_bc1_mipmaps, rgba_to_bc1_mipmaps};
use dxtn::*;
pub use error::Error;
use jpeg::*;
use raw1::*;
use raw3::*;
use std::fmt;
pub use texpresso::Algorithm as DxtAlgorithm;
use wow_alchemy_data::error::WowDataError;

/// The largest width and height a BLP can have.
pub const BLP_MAX_SIZE: u32 = 65535;

/// Decodes a level, 0 being the image itself.
pub fn blp_to_image(image: &BlpImage, level: usize) -> Result<DynamicImage, Error> {
    let bytes = image.mipmaps.get(level).ok_or(Error::MissingImage(level))?;
    let format = image.format()?;
    match (format, &image.data) {
        (BlpFormat::Jpeg, _) => {
            jpeg_to_image(&image.jpeg(level).ok_or(Error::MissingImage(level))?)
        }
        (BlpFormat::Raw1, BlpData::Palette(palette)) => {
            raw1_to_image(&image.header, palette, bytes, level)
        }
        (BlpFormat::Raw3, _) => raw3_to_image(&image.header, bytes, level),
        (BlpFormat::Raw1, BlpData::Jpeg(_)) => Err(Error::Format(WowDataError::InvalidData(
            "palettized image without a palette".into(),
        ))),
        (dxt, _) => dxtn_to_image(&image.header, dxt.dxt().expect("DXT format"), bytes, level),
    }
}

/// The image, followed by its mipmaps down to 1 pixel when `make_mipmaps`, at most 16 levels.
fn generate_mipmaps(
    image: DynamicImage,
    make_mipmaps: bool,
    filter: FilterType,
) -> Vec<DynamicImage> {
    let mut mipmaps = vec![image];
    while make_mipmaps && mipmaps.len() < MAX_MIPMAPS {
        let last = mipmaps.last().expect("one level");
        let (width, height) = (last.width(), last.height());
        if width == 1 && height == 1 {
            break;
        }
        let next = last.resize_exact((width >> 1).max(1), (height >> 1).max(1), filter);
        mipmaps.push(next);
    }
    mipmaps
}

/// Which BLP [image_to_blp] makes. JPEG content can be read but not made: BLP JPEGs have four
/// channels, which the `image` crate doesn't write.
#[derive(Clone, PartialEq, Eq)]
pub enum BlpTarget {
    /// Warcraft III beta builds, palettized, with mipmaps in files of their own.
    Blp0 { alpha_bits: AlphaBits },
    /// Warcraft III, palettized.
    Blp1 { alpha_bits: AlphaBits },
    /// World of Warcraft.
    Blp2(Blp2Format),
}

impl Default for BlpTarget {
    fn default() -> Self {
        BlpTarget::Blp2(Default::default())
    }
}

impl fmt::Display for BlpTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlpTarget::Blp0 { alpha_bits } => write!(f, "BLP0 palettized image with {alpha_bits}"),
            BlpTarget::Blp1 { alpha_bits } => write!(f, "BLP1 palettized image with {alpha_bits}"),
            BlpTarget::Blp2(format) => write!(f, "BLP2 {format}"),
        }
    }
}

/// Alpha bits of palettized images.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum AlphaBits {
    NoAlpha,
    Bit1,
    Bit4,
    #[default]
    Bit8,
}

impl fmt::Display for AlphaBits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlphaBits::NoAlpha => write!(f, "no alpha"),
            AlphaBits::Bit1 => write!(f, "1 bit alpha"),
            AlphaBits::Bit4 => write!(f, "4 bits alpha"),
            AlphaBits::Bit8 => write!(f, "8 bits alpha"),
        }
    }
}

impl From<AlphaBits> for u32 {
    fn from(value: AlphaBits) -> u32 {
        match value {
            AlphaBits::NoAlpha => 0,
            AlphaBits::Bit1 => 1,
            AlphaBits::Bit4 => 4,
            AlphaBits::Bit8 => 8,
        }
    }
}

/// Encodings of BLP2.
#[derive(Clone, PartialEq, Eq)]
pub enum Blp2Format {
    /// 256 colors, with or without alpha.
    Raw1 { alpha_bits: AlphaBits },
    /// BGRA pixels.
    Raw3,
    /// DXT1, with 1-bit alpha or none.
    Dxt1 {
        has_alpha: bool,
        compress_algorithm: DxtAlgorithm,
    },
    /// DXT3, explicit alpha.
    Dxt3 {
        has_alpha: bool,
        compress_algorithm: DxtAlgorithm,
    },
    /// DXT5, interpolated alpha.
    Dxt5 {
        has_alpha: bool,
        compress_algorithm: DxtAlgorithm,
    },
}

impl Default for Blp2Format {
    fn default() -> Self {
        Blp2Format::Dxt5 {
            has_alpha: true,
            compress_algorithm: Default::default(),
        }
    }
}

impl fmt::Display for Blp2Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (name, has_alpha, algorithm) = match self {
            Blp2Format::Raw1 { alpha_bits } => {
                return write!(f, "palettized image with {alpha_bits}");
            }
            Blp2Format::Raw3 => return write!(f, "BGRA image"),
            Blp2Format::Dxt1 {
                has_alpha,
                compress_algorithm,
            } => ("DXT1", has_alpha, compress_algorithm),
            Blp2Format::Dxt3 {
                has_alpha,
                compress_algorithm,
            } => ("DXT3", has_alpha, compress_algorithm),
            Blp2Format::Dxt5 {
                has_alpha,
                compress_algorithm,
            } => ("DXT5", has_alpha, compress_algorithm),
        };
        let alpha = if *has_alpha { "with" } else { "without" };
        let quality = match algorithm {
            DxtAlgorithm::RangeFit => "fast/low quality",
            DxtAlgorithm::ClusterFit => "slow/high quality",
            DxtAlgorithm::IterativeClusterFit => "very slow/best quality",
        };
        write!(f, "{name} image {alpha} alpha and compression {quality}")
    }
}

/// Encodes an image as a BLP, with mipmaps when `make_mipmaps`.
pub fn image_to_blp(
    image: DynamicImage,
    make_mipmaps: bool,
    target: BlpTarget,
    mipmap_filter: FilterType,
) -> Result<BlpImage, Error> {
    let (width, height) = (image.width(), image.height());
    if width > BLP_MAX_SIZE {
        return Err(Error::WidthTooLarge(width));
    }
    if height > BLP_MAX_SIZE {
        return Err(Error::HeightTooLarge(height));
    }
    let has_mipmaps = make_mipmaps as u8;
    let dxt = |image, format, algorithm| {
        let mipmaps = image_to_dxtn(image, format, make_mipmaps, mipmap_filter, algorithm);
        ([0; 256], mipmaps)
    };

    let (version, flags, extra, (palette, mipmaps)) = match target {
        BlpTarget::Blp0 { alpha_bits } | BlpTarget::Blp1 { alpha_bits } => {
            let version = match target {
                BlpTarget::Blp0 { .. } => BlpVersion::Blp0,
                _ => BlpVersion::Blp1,
            };
            let flags = BlpFlags::Old {
                alpha_bits: alpha_bits.into(),
            };
            let extra = BlpExtra::Old {
                extra: 4,
                has_mipmaps: has_mipmaps as u32,
            };
            let content = image_to_raw1(image, alpha_bits.into(), make_mipmaps, mipmap_filter)?;
            (version, flags, extra, content)
        }
        BlpTarget::Blp2(format) => {
            let flags = |compression, alpha_bits, alpha_type| BlpFlags::Blp2 {
                compression,
                alpha_bits,
                alpha_type,
                has_mipmaps,
            };
            let (flags, content) = match format {
                Blp2Format::Raw1 { alpha_bits } => {
                    let bits: u32 = alpha_bits.into();
                    // The game's palettized textures have alpha type 8.
                    (
                        flags(Compression::Raw1, bits as u8, AlphaType::Other(8)),
                        image_to_raw1(image, bits, make_mipmaps, mipmap_filter)?,
                    )
                }
                // As the game's BGRA textures.
                Blp2Format::Raw3 => (
                    flags(Compression::Raw3, 8, AlphaType::Other(2)),
                    ([0; 256], image_to_raw3(image, make_mipmaps, mipmap_filter)),
                ),
                Blp2Format::Dxt1 {
                    has_alpha,
                    compress_algorithm,
                } => (
                    flags(Compression::Dxtc, has_alpha as u8, AlphaType::Dxt1),
                    dxt(image, texpresso::Format::Bc1, compress_algorithm),
                ),
                Blp2Format::Dxt3 {
                    has_alpha,
                    compress_algorithm,
                } => (
                    flags(Compression::Dxtc, 8 * has_alpha as u8, AlphaType::Dxt3),
                    dxt(image, texpresso::Format::Bc2, compress_algorithm),
                ),
                Blp2Format::Dxt5 {
                    has_alpha,
                    compress_algorithm,
                } => (
                    flags(Compression::Dxtc, 8 * has_alpha as u8, AlphaType::Dxt5),
                    dxt(image, texpresso::Format::Bc3, compress_algorithm),
                ),
            };
            (BlpVersion::Blp2, flags, BlpExtra::None, content)
        }
    };

    let mut blp = BlpImage {
        header: BlpHeader::new(version, BlpContentTag::Direct, flags, extra, width, height),
        data: BlpData::Palette(palette),
        mipmaps,
    };
    blp.update_locator()?;
    Ok(blp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::image::{Rgba, RgbaImage};
    use std::io::Cursor;
    use wow_alchemy_data::types::{WowStructR, WowStructW};

    fn gradient(width: u32, height: u32) -> DynamicImage {
        DynamicImage::ImageRgba8(RgbaImage::from_fn(width, height, |x, y| {
            Rgba([(x * 30) as u8, (y * 50) as u8, 128, 255])
        }))
    }

    fn written_and_read(blp: &BlpImage) -> BlpImage {
        let mut bytes = Cursor::new(Vec::new());
        blp.wow_write(&mut bytes).unwrap();
        BlpImage::wow_read(&mut Cursor::new(bytes.into_inner())).unwrap()
    }

    #[test]
    fn mipmaps_halve_down_to_one_pixel() {
        let levels = generate_mipmaps(gradient(8, 2), true, FilterType::Nearest);
        let sizes: Vec<_> = levels.iter().map(|l| (l.width(), l.height())).collect();
        assert_eq!(sizes, [(8, 2), (4, 1), (2, 1), (1, 1)]);
        assert_eq!(
            generate_mipmaps(gradient(8, 2), false, FilterType::Nearest).len(),
            1
        );
    }

    #[test]
    fn every_target_writes_reads_and_decodes() {
        let dxt = DxtAlgorithm::RangeFit;
        let targets = [
            BlpTarget::Blp1 {
                alpha_bits: AlphaBits::Bit4,
            },
            BlpTarget::Blp2(Blp2Format::Raw1 {
                alpha_bits: AlphaBits::Bit1,
            }),
            BlpTarget::Blp2(Blp2Format::Raw3),
            BlpTarget::Blp2(Blp2Format::Dxt1 {
                has_alpha: false,
                compress_algorithm: dxt,
            }),
            BlpTarget::Blp2(Blp2Format::Dxt3 {
                has_alpha: true,
                compress_algorithm: dxt,
            }),
            BlpTarget::Blp2(Blp2Format::Dxt5 {
                has_alpha: true,
                compress_algorithm: dxt,
            }),
        ];
        for target in targets {
            let blp =
                image_to_blp(gradient(8, 4), true, target.clone(), FilterType::Nearest).unwrap();
            assert_eq!(blp.image_count(), 4, "{target}");
            let read = written_and_read(&blp);
            assert_eq!(read, blp, "{target}");
            for level in 0..4 {
                let image = blp_to_image(&read, level).unwrap();
                assert_eq!(
                    (image.width(), image.height()),
                    blp.header.mipmap_size(level)
                );
            }
        }
    }

    #[test]
    fn raw3_keeps_pixels_exactly() {
        let source = gradient(4, 4).into_rgba8();
        let blp = image_to_blp(
            source.clone().into(),
            false,
            BlpTarget::Blp2(Blp2Format::Raw3),
            FilterType::Nearest,
        )
        .unwrap();
        // Stored BGRA.
        assert_eq!(&blp.mipmaps[0][..4], &[128, 0, 0, 255]);
        assert_eq!(blp_to_image(&blp, 0).unwrap().into_rgba8(), source);
    }

    #[test]
    fn jpeg_levels_are_the_header_and_their_body_with_red_and_blue_swapped() {
        let mut jpeg = Vec::new();
        DynamicImage::ImageRgb8(::image::RgbImage::from_pixel(
            8,
            8,
            ::image::Rgb([200, 0, 30]),
        ))
        .write_to(&mut Cursor::new(&mut jpeg), ::image::ImageFormat::Jpeg)
        .unwrap();
        let body = jpeg.split_off(20);
        let blp = BlpImage {
            header: BlpHeader::new(
                BlpVersion::Blp1,
                BlpContentTag::Jpeg,
                BlpFlags::Old { alpha_bits: 0 },
                BlpExtra::Old {
                    extra: 5,
                    has_mipmaps: 0,
                },
                8,
                8,
            ),
            data: BlpData::Jpeg(jpeg),
            mipmaps: vec![body],
        };
        let read = written_and_read(&blp);
        let pixel = blp_to_image(&read, 0)
            .unwrap()
            .into_rgba8()
            .get_pixel(4, 4)
            .0;
        assert!(pixel[2] > 150 && pixel[0] < 60, "{pixel:?}");
    }

    #[test]
    fn levels_past_the_last_are_missing() {
        let blp = image_to_blp(
            gradient(4, 4),
            false,
            BlpTarget::Blp2(Blp2Format::Raw3),
            FilterType::Nearest,
        )
        .unwrap();
        assert!(matches!(blp_to_image(&blp, 1), Err(Error::MissingImage(1))));
    }

    #[test]
    fn short_dxt_levels_are_an_error_not_a_panic() {
        let mut blp = image_to_blp(
            gradient(8, 8),
            false,
            BlpTarget::Blp2(Blp2Format::Dxt1 {
                has_alpha: false,
                compress_algorithm: DxtAlgorithm::RangeFit,
            }),
            FilterType::Nearest,
        )
        .unwrap();
        blp.mipmaps[0].truncate(24);
        assert!(matches!(
            blp_to_image(&blp, 0),
            Err(Error::MipmapTooShort { needed: 32, .. })
        ));
    }
}
