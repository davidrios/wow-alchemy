use ::image::error::ImageError;
use thiserror::Error;
use wow_alchemy_data::error::WowDataError;

/// Errors turning BLPs into images and back.
#[derive(Debug, Error)]
pub enum Error {
    /// The header doesn't say a format that can be decoded.
    #[error(transparent)]
    Format(#[from] WowDataError),
    /// BC1 mipmap chains need a power of two size.
    #[error("{0} is not a power of two")]
    InvalidBc1Size(u32),
    /// The requested mipmap level does not exist in the file.
    #[error("There is no image in the BLP mipmaps level {0}!")]
    MissingImage(usize),
    #[error("Conversion error: {0}")]
    Convert(#[from] ImageError),
    #[error("Maximum value for width is 65,535")]
    WidthTooLarge(u32),
    #[error("Maximum value for height is 65,535")]
    HeightTooLarge(u32),
    /// A level has fewer bytes than its size needs.
    #[error("Mipmap {level} is {width}x{height}, which needs {needed} bytes, but it has {actual}")]
    MipmapTooShort {
        level: usize,
        width: u32,
        height: u32,
        needed: usize,
        actual: usize,
    },
    /// Palettized images have 0, 1, 4 or 8 alpha bits.
    #[error("There are invalid alpha bits for the Raw1 format. Got {0}, expected: 0, 1, 4, 8.")]
    Raw1InvalidAlphaBits(u32),
    #[error("Expected palette of 255 colors, but got {0}")]
    PaletteWrongSize(usize),
}
