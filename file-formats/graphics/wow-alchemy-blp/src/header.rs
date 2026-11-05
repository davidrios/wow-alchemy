//! The header every BLP starts with.

use wow_alchemy_data::error::{Result, WowDataError};
use wow_alchemy_data::prelude::*;
use wow_alchemy_data::utils::magic_to_string;
use wow_alchemy_data_derive::{WowEnumFrom, WowHeaderR, WowHeaderW};

pub const BLP0_MAGIC: MagicStr = *b"BLP0";
pub const BLP1_MAGIC: MagicStr = *b"BLP1";
pub const BLP2_MAGIC: MagicStr = *b"BLP2";

/// The most mipmaps a file with internal mipmaps has room for.
pub const MAX_MIPMAPS: usize = 16;

/// The file's version, which is its magic.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, WowEnumFrom, WowHeaderR, WowHeaderW,
)]
#[wow_data(from_type = MagicStr)]
pub enum BlpVersion {
    /// Warcraft III beta builds. Mipmaps are kept in files of their own, `name.b00` and on.
    #[wow_data(expr = BLP0_MAGIC)]
    Blp0,
    /// Warcraft III.
    #[wow_data(expr = BLP1_MAGIC)]
    Blp1,
    /// World of Warcraft.
    #[wow_data(expr = BLP2_MAGIC)]
    Blp2,
}

impl DataVersion for BlpVersion {}

/// Whether the image is JPEG or pixels to read directly.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, WowEnumFrom, WowHeaderR, WowHeaderW,
)]
#[wow_data(from_type = u32)]
pub enum BlpContentTag {
    /// Each mipmap is a JPEG body, after a header they share. Its colors are BGRA rather than
    /// Y'CbCr.
    #[wow_data(expr = 0)]
    Jpeg,
    /// A palette follows the header, and the mipmaps are palette indices, raw pixels or DXT
    /// blocks.
    #[wow_data(expr = 1)]
    Direct,
}

/// How a BLP2 with direct content stores its pixels.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, WowEnumFrom, WowHeaderR, WowHeaderW,
)]
#[wow_data(from_type = u8)]
pub enum Compression {
    /// Never seen with direct content.
    #[wow_data(expr = 0)]
    Jpeg,
    /// A byte per pixel indexing the palette, then the alpha at `alpha_bits` per pixel.
    #[wow_data(expr = 1)]
    Raw1,
    /// DXT blocks, of the kind `alpha_type` says.
    #[wow_data(expr = 2)]
    Dxtc,
    /// BGRA, a byte each.
    #[wow_data(expr = 3)]
    Raw3,
}

/// BLP2's alpha type, which chooses the kind of DXT blocks.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, WowEnumFrom, WowHeaderR, WowHeaderW,
)]
#[wow_data(from_type = u8)]
pub enum AlphaType {
    /// DXT1, with no alpha or 1-bit alpha.
    #[wow_data(expr = 0)]
    Dxt1,
    /// DXT3, with explicit alpha.
    #[wow_data(expr = 1)]
    Dxt3,
    /// DXT5, with interpolated alpha.
    #[wow_data(expr = 7)]
    Dxt5,
    /// Any other value. 3.3.5a has 2, 4 and 8 on palettized and BGRA textures, where it
    /// decides nothing.
    #[wow_data(
        default,
        from_arm = "_ => Self::Other(value)",
        to_arm = "AlphaType::Other(value) => value"
    )]
    Other(u8),
}

/// The fields after the content tag, which differ between versions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, WowHeaderR, WowHeaderW)]
#[wow_data(version = BlpVersion)]
pub enum BlpFlags {
    #[wow_data(read_if = version >= BlpVersion::Blp2)]
    Blp2 {
        compression: Compression,
        /// 0, 1, 4 or 8 for palettized images, how many alpha bits each pixel has.
        alpha_bits: u8,
        alpha_type: AlphaType,
        /// Not 0 when there are mipmaps. Files have 0, 1, 2 and 17.
        has_mipmaps: u8,
    },
    Old {
        alpha_bits: u32,
    },
}

/// The fields BLP0 and BLP1 have after the size.
#[derive(Debug, Clone, PartialEq, Eq, Hash, WowHeaderR, WowHeaderW)]
#[wow_data(version = BlpVersion)]
pub enum BlpExtra {
    #[wow_data(read_if = version < BlpVersion::Blp2)]
    Old {
        /// Has no known use. Tools write 4 for palettized images and 5 for JPEG.
        extra: u32,
        /// 1 when there are mipmaps.
        has_mipmaps: u32,
    },
    None,
}

/// Where the mipmaps are.
#[derive(Debug, Clone, PartialEq, Eq, Hash, WowHeaderR, WowHeaderW)]
#[wow_data(version = BlpVersion)]
pub enum MipmapLocator {
    /// In the file, where `offsets` say, `sizes` bytes long. Unused levels are 0 in both.
    #[wow_data(read_if = version >= BlpVersion::Blp1)]
    Internal {
        offsets: [u32; MAX_MIPMAPS],
        sizes: [u32; MAX_MIPMAPS],
    },
    /// In files of their own, `name.b00`, `name.b01` and on (BLP0).
    External,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, WowHeaderR, WowHeaderW)]
#[wow_data(version = BlpVersion)]
pub struct BlpHeader {
    pub version: BlpVersion,
    pub content: BlpContentTag,
    #[wow_data(versioned)]
    pub flags: BlpFlags,
    pub width: u32,
    pub height: u32,
    #[wow_data(versioned)]
    pub extra: BlpExtra,
    /// Filled in from the mipmaps when the file is written.
    #[wow_data(versioned)]
    pub mipmap_locator: MipmapLocator,
}

impl WowHeaderR for BlpHeader {
    fn wow_read<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let start = reader.stream_position()?;
        let magic: MagicStr = reader.wow_read()?;
        let version = BlpVersion::try_from(magic).map_err(|_| WowDataError::InvalidMagic {
            expected: "BLP0, BLP1 or BLP2".into(),
            actual: magic_to_string(&magic),
        })?;
        // The versioned read reads the magic again, as the version field.
        reader.seek(SeekFrom::Start(start))?;
        reader.wow_read_versioned(version)
    }
}

impl BlpHeader {
    /// A header for `version`, with no mipmaps and the locator to be filled in on writing.
    pub fn new(
        version: BlpVersion,
        content: BlpContentTag,
        flags: BlpFlags,
        extra: BlpExtra,
        width: u32,
        height: u32,
    ) -> Self {
        let mipmap_locator = if version == BlpVersion::Blp0 {
            MipmapLocator::External
        } else {
            MipmapLocator::Internal {
                offsets: [0; MAX_MIPMAPS],
                sizes: [0; MAX_MIPMAPS],
            }
        };
        Self {
            version,
            content,
            flags,
            width,
            height,
            extra,
            mipmap_locator,
        }
    }

    pub fn has_mipmaps(&self) -> bool {
        match (&self.flags, &self.extra) {
            (BlpFlags::Blp2 { has_mipmaps, .. }, _) => *has_mipmaps != 0,
            (_, BlpExtra::Old { has_mipmaps, .. }) => *has_mipmaps != 0,
            _ => false,
        }
    }

    /// Alpha bits per pixel, as the header has them.
    pub fn alpha_bits(&self) -> u32 {
        match self.flags {
            BlpFlags::Blp2 { alpha_bits, .. } => alpha_bits as u32,
            BlpFlags::Old { alpha_bits } => alpha_bits,
        }
    }

    /// How many levels the size calls for: down to 1 pixel on the longer side with mipmaps,
    /// otherwise 1. Files with internal mipmaps can't hold more than [`MAX_MIPMAPS`].
    pub fn mipmap_levels(&self) -> usize {
        if self.has_mipmaps() {
            let longest = self.width.max(self.height);
            longest.checked_ilog2().unwrap_or(0) as usize + 1
        } else {
            1
        }
    }

    /// Width and height of a level. Level 0 is the image itself.
    pub fn mipmap_size(&self, level: usize) -> (u32, u32) {
        let shift = level.min(31) as u32;
        ((self.width >> shift).max(1), (self.height >> shift).max(1))
    }

    /// Whether the version-dependent fields are those of the version.
    pub(crate) fn check_version(&self) -> Result<()> {
        let blp2 = self.version >= BlpVersion::Blp2;
        let internal = self.version >= BlpVersion::Blp1;
        let consistent = matches!(self.flags, BlpFlags::Blp2 { .. }) == blp2
            && matches!(self.extra, BlpExtra::Old { .. }) != blp2
            && matches!(self.mipmap_locator, MipmapLocator::Internal { .. }) == internal;
        if consistent {
            Ok(())
        } else {
            Err(WowDataError::InvalidData(format!(
                "the header's fields aren't those of {:?}",
                self.version
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn round_trip(bytes: &[u8]) -> BlpHeader {
        let header: BlpHeader = Cursor::new(bytes).wow_read().unwrap();
        assert_eq!(header.wow_size(), bytes.len());
        let mut out = Vec::new();
        header.wow_write(&mut out).unwrap();
        assert_eq!(out, bytes);
        header
    }

    #[test]
    fn blp2_header_layout() {
        let mut bytes = b"BLP2".to_vec();
        bytes.extend(1u32.to_le_bytes());
        bytes.extend([2, 8, 7, 17]);
        bytes.extend(256u32.to_le_bytes());
        bytes.extend(128u32.to_le_bytes());
        for i in 0..32u32 {
            bytes.extend(i.to_le_bytes());
        }
        let header = round_trip(&bytes);
        assert_eq!(header.version, BlpVersion::Blp2);
        assert_eq!(
            header.flags,
            BlpFlags::Blp2 {
                compression: Compression::Dxtc,
                alpha_bits: 8,
                alpha_type: AlphaType::Dxt5,
                has_mipmaps: 17
            }
        );
        assert_eq!((header.width, header.height), (256, 128));
        assert_eq!(header.mipmap_levels(), 9);
        assert_eq!(header.mipmap_size(8), (1, 1));
        let MipmapLocator::Internal { offsets, sizes } = header.mipmap_locator else {
            panic!()
        };
        assert_eq!((offsets[1], sizes[15]), (1, 31));
    }

    #[test]
    fn old_headers_have_extra_fields_and_blp0_no_locator() {
        let mut bytes = b"BLP0".to_vec();
        for v in [1u32, 4, 8, 2, 4, 1] {
            bytes.extend(v.to_le_bytes());
        }
        let header = round_trip(&bytes);
        assert_eq!(header.flags, BlpFlags::Old { alpha_bits: 4 });
        assert_eq!(
            header.extra,
            BlpExtra::Old {
                extra: 4,
                has_mipmaps: 1
            }
        );
        assert_eq!(header.mipmap_locator, MipmapLocator::External);
        assert_eq!(header.mipmap_levels(), 4);
    }

    #[test]
    fn unknown_alpha_types_are_kept() {
        assert_eq!(AlphaType::try_from(4).unwrap(), AlphaType::Other(4));
        assert_eq!(u8::from(AlphaType::Other(4)), 4);
    }

    #[test]
    fn other_magics_are_rejected() {
        let error =
            <BlpHeader as WowHeaderR>::wow_read(&mut Cursor::new(b"BLP9\0\0\0\0")).unwrap_err();
        assert!(
            matches!(error, WowDataError::InvalidMagic { .. }),
            "{error}"
        );
        assert!(<BlpHeader as WowHeaderR>::wow_read(&mut Cursor::new(b"")).is_err());
    }
}
