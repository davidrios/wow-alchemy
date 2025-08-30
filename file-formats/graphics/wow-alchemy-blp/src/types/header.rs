pub use super::locator::MipmapLocator;
pub use super::version::BlpVersion;
use std::fmt;

/// The content field determines how the image data is stored. CONTENT_JPEG
/// uses non-standard JPEG (JFIF) file compression of BGRA colour component
/// values rather than the usual Y′CbCr color component values.
/// CONTENT_DIRECT refers to a variety of storage formats which can be
/// directly read as pixel values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BlpContentTag {
    /// JPEG compressed image data
    Jpeg,
    /// Direct pixel data (palettized or uncompressed)
    Direct,
}

/// Error type for unknown content tag values
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnknownContent(u32);

impl fmt::Display for UnknownContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unknown content field value: {}", self.0)
    }
}

impl TryFrom<u32> for BlpContentTag {
    type Error = UnknownContent;

    fn try_from(val: u32) -> Result<BlpContentTag, Self::Error> {
        match val {
            0 => Ok(BlpContentTag::Jpeg),
            1 => Ok(BlpContentTag::Direct),
            _ => Err(UnknownContent(val)),
        }
    }
}

impl From<BlpContentTag> for u32 {
    fn from(val: BlpContentTag) -> u32 {
        match val {
            BlpContentTag::Jpeg => 0,
            BlpContentTag::Direct => 1,
        }
    }
}

/// BLP file header structure
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlpHeader {
    /// BLP format version
    pub version: BlpVersion,
    /// Content encoding type
    pub content: BlpContentTag,
    /// Version-specific flags
    pub flags: BlpFlags,
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// Mipmap location information
    pub mipmap_locator: MipmapLocator,
}

impl BlpHeader {
    /// Calculate needed count of mipmaps for the defined size
    pub fn mipmaps_count(&self) -> usize {
        if self.has_mipmaps() {
            let width_n = (self.width as f32).log2() as usize;
            let height_n = (self.height as f32).log2() as usize;
            width_n.max(height_n)
        } else {
            0
        }
    }

    /// Returns 'true' if the header defines that the image has mipmaps
    pub fn has_mipmaps(&self) -> bool {
        self.flags.has_mipmaps()
    }

    /// Return expected size of mipmap for the given mipmap level.
    /// 0 level means original image.
    pub fn mipmap_size(&self, i: usize) -> (u32, u32) {
        if i == 0 {
            (self.width, self.height)
        } else {
            ((self.width >> i).max(1), (self.height >> i).max(1))
        }
    }

    /// Return expected count of pixels in mipmap at the level i.
    /// 0 level means original image.
    pub fn mipmap_pixels(&self, i: usize) -> u32 {
        let (w, h) = self.mipmap_size(i);
        w * h
    }

    /// Return alpha bits count in encoding
    pub fn alpha_bits(&self) -> u32 {
        self.flags.alpha_bits()
    }

    /// Get the alpha type for BLP2 format, if available
    pub fn alpha_type(&self) -> Option<AlphaType> {
        self.flags.alpha_type()
    }

    /// Validate the BLP header for compatibility with a specific WoW version
    pub fn validate_for_wow_version(&self, wow_version: WowVersion) -> Result<(), String> {
        if let Some(alpha_type) = self.alpha_type() {
            if !alpha_type.is_supported_in_version(wow_version) {
                return Err(format!(
                    "Alpha type {alpha_type:?} not supported in WoW version {wow_version:?}"
                ));
            }
        }
        Ok(())
    }

    /// Return offsets and sizes of internal mipmaps. For external returns [None]
    pub fn internal_mipmaps(&self) -> Option<([u32; 16], [u32; 16])> {
        match self.mipmap_locator {
            MipmapLocator::Internal { offsets, sizes } => Some((offsets, sizes)),
            MipmapLocator::External => None,
        }
    }

    /// Get size of header in bytes. Doesn't count jpeg header or color map.
    pub fn size(version: BlpVersion) -> usize {
        4 // magic
        + 4 // content
        + 4 // flags or alpha_bits
        + 4 // width
        + 4 // height
        + if version < BlpVersion::Blp2 {8} else {0} // extra and has_mipmaps
        + if version > BlpVersion::Blp0 {16*4*2} else {0} // mipmap locator
    }
}

impl Default for BlpHeader {
    fn default() -> Self {
        BlpHeader {
            version: BlpVersion::Blp1,
            content: BlpContentTag::Jpeg,
            flags: Default::default(),
            width: 1,
            height: 1,
            mipmap_locator: Default::default(),
        }
    }
}

/// Alpha channel encoding type for BLP2 format
/// Based on empirical analysis of WoW versions 1.12.1 through 5.4.8
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AlphaType {
    /// No alpha channel
    None = 0,
    /// 1-bit alpha (binary transparency)
    OneBit = 1,
    /// Enhanced alpha blending (introduced in TBC 2.4.3)
    Enhanced = 7,
    /// 8-bit alpha channel
    EightBit = 8,
}

impl std::fmt::Display for AlphaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlphaType::None => write!(f, "None (0)"),
            AlphaType::OneBit => write!(f, "1-bit (1)"),
            AlphaType::Enhanced => write!(f, "Enhanced (7)"),
            AlphaType::EightBit => write!(f, "8-bit (8)"),
        }
    }
}

impl AlphaType {
    /// Returns true if this alpha type was available in the specified WoW version
    pub fn is_supported_in_version(self, wow_version: WowVersion) -> bool {
        match self {
            AlphaType::None | AlphaType::OneBit | AlphaType::EightBit => true,
            AlphaType::Enhanced => wow_version >= WowVersion::TBC,
        }
    }

    /// Get the effective alpha bits for this alpha type
    pub fn alpha_bits(self) -> u8 {
        match self {
            AlphaType::None => 0,
            AlphaType::OneBit => 1,
            AlphaType::Enhanced => 8, // Enhanced uses 8-bit precision
            AlphaType::EightBit => 8,
        }
    }
}

/// Error type for unknown alpha type values
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnknownAlphaType(u8);

impl fmt::Display for UnknownAlphaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unknown alpha_type field value: {}", self.0)
    }
}

impl TryFrom<u8> for AlphaType {
    type Error = UnknownAlphaType;

    fn try_from(val: u8) -> Result<AlphaType, Self::Error> {
        match val {
            0 => Ok(AlphaType::None),
            1 => Ok(AlphaType::OneBit),
            7 => Ok(AlphaType::Enhanced),
            8 => Ok(AlphaType::EightBit),
            _ => Err(UnknownAlphaType(val)),
        }
    }
}

impl From<AlphaType> for u8 {
    fn from(val: AlphaType) -> u8 {
        val as u8
    }
}

/// WoW version for format compatibility checking
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WowVersion {
    /// World of Warcraft 1.12.1 (Vanilla)
    Vanilla,
    /// The Burning Crusade 2.4.3
    TBC,
    /// Wrath of the Lich King 3.3.5a
    WotLK,
    /// Cataclysm 4.3.4
    Cataclysm,
    /// Mists of Pandaria 5.4.8
    MoP,
}

/// Compression type for BLP2 format
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Compression {
    /// JPEG compression (rarely used in BLP2)
    Jpeg, // adhoc compression, never met in BLP2
    /// Palettized 256-color format
    Raw1,
    /// Uncompressed RGBA format
    Raw3,
    /// DXT compression (S3TC)
    Dxtc,
}

/// Error type for unknown compression values
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnknownCompression(u8);

impl fmt::Display for UnknownCompression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unknown compression field value: {}", self.0)
    }
}

impl TryFrom<u8> for Compression {
    type Error = UnknownCompression;

    fn try_from(val: u8) -> Result<Compression, Self::Error> {
        match val {
            0 => Ok(Compression::Jpeg),
            1 => Ok(Compression::Raw1),
            2 => Ok(Compression::Dxtc),
            3 => Ok(Compression::Raw3),
            _ => Err(UnknownCompression(val)),
        }
    }
}

impl From<Compression> for u8 {
    fn from(val: Compression) -> u8 {
        match val {
            Compression::Jpeg => 0,
            Compression::Raw1 => 1,
            Compression::Dxtc => 2,
            Compression::Raw3 => 3,
        }
    }
}

/// Part of header that depends on the version
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BlpFlags {
    /// For version >= 2
    Blp2 {
        /// Compression method used
        compression: Compression,
        /// Alpha channel bit depth (0, 1, 4, or 8)
        alpha_bits: u8,
        /// Alpha encoding type (typed enum for better validation)
        alpha_type: AlphaType,
        /// Whether the image has mipmaps
        has_mipmaps: u8,
    },
    /// For version < 2
    Old {
        /// Alpha channel bit depth
        alpha_bits: u32,
        /// Extra field (usually 4 or 5)
        extra: u32, // no purpose, default is 5
        /// Whether the image has mipmaps (0 or 1)
        has_mipmaps: u32, // boolean
    },
}

impl Default for BlpFlags {
    fn default() -> Self {
        BlpFlags::Old {
            alpha_bits: 8,
            extra: 8,
            has_mipmaps: 1,
        }
    }
}

impl BlpFlags {
    /// Returns 'true' if the header defines that the image has mipmaps
    pub fn has_mipmaps(&self) -> bool {
        match self {
            BlpFlags::Blp2 { has_mipmaps, .. } => *has_mipmaps != 0,
            BlpFlags::Old { has_mipmaps, .. } => *has_mipmaps != 0,
        }
    }

    /// Get count of bits alpha channel is encoded in content
    pub fn alpha_bits(&self) -> u32 {
        match self {
            BlpFlags::Blp2 { compression, .. } if *compression == Compression::Raw3 => 4,
            BlpFlags::Blp2 { alpha_bits, .. } => *alpha_bits as u32,
            BlpFlags::Old { alpha_bits, .. } => *alpha_bits,
        }
    }

    /// Get the alpha type for BLP2 format, returns None for older formats
    pub fn alpha_type(&self) -> Option<AlphaType> {
        match self {
            BlpFlags::Blp2 { alpha_type, .. } => Some(*alpha_type),
            BlpFlags::Old { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alpha_type_conversion() {
        assert_eq!(AlphaType::None as u8, 0);
        assert_eq!(AlphaType::OneBit as u8, 1);
        assert_eq!(AlphaType::Enhanced as u8, 7);
        assert_eq!(AlphaType::EightBit as u8, 8);

        // Test TryFrom conversions
        assert_eq!(AlphaType::try_from(0).unwrap(), AlphaType::None);
        assert_eq!(AlphaType::try_from(1).unwrap(), AlphaType::OneBit);
        assert_eq!(AlphaType::try_from(7).unwrap(), AlphaType::Enhanced);
        assert_eq!(AlphaType::try_from(8).unwrap(), AlphaType::EightBit);

        // Test invalid conversion
        assert!(AlphaType::try_from(9).is_err());
    }

    #[test]
    fn test_alpha_type_wow_version_support() {
        // Vanilla supports all except Enhanced
        assert!(AlphaType::None.is_supported_in_version(WowVersion::Vanilla));
        assert!(AlphaType::OneBit.is_supported_in_version(WowVersion::Vanilla));
        assert!(!AlphaType::Enhanced.is_supported_in_version(WowVersion::Vanilla));
        assert!(AlphaType::EightBit.is_supported_in_version(WowVersion::Vanilla));

        // TBC+ supports all
        assert!(AlphaType::Enhanced.is_supported_in_version(WowVersion::TBC));
        assert!(AlphaType::Enhanced.is_supported_in_version(WowVersion::Cataclysm));
    }

    #[test]
    fn test_mipmap_count() {
        let header = BlpHeader {
            width: 512,
            height: 256,
            version: BlpVersion::Blp0,
            ..Default::default()
        };
        assert_eq!(header.mipmaps_count(), 9);

        let header = BlpHeader {
            width: 512,
            height: 256,
            version: BlpVersion::Blp1,
            ..Default::default()
        };
        assert_eq!(header.mipmaps_count(), 9);

        let header = BlpHeader {
            width: 1,
            height: 4,
            ..Default::default()
        };
        assert_eq!(header.mipmaps_count(), 2);

        let header = BlpHeader {
            width: 4,
            height: 7,
            ..Default::default()
        };
        assert_eq!(header.mipmaps_count(), 2);

        let header = BlpHeader {
            width: 768,
            height: 128,
            ..Default::default()
        };
        assert_eq!(header.mipmaps_count(), 9);
    }
}
