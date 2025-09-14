use thiserror::Error;
use wow_alchemy_data::error::WowDataError;

pub type Result<T> = std::result::Result<T, AdtError>;

#[derive(Debug, Error)]
pub enum AdtError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("wow-alchemy-data error: {0}")]
    WowData(#[from] wow_alchemy_data::error::WowDataError),

    #[error("Invalid version: expected 18, found {0}")]
    InvalidVersion(u32),

    #[error("Invalid magic: expected {expected}, found {found}")]
    InvalidMagic { expected: String, found: String },

    #[error("Expected chunk {0} not found")]
    ExpectedChunkNotFound(String),

    #[error("Map chunk out of range")]
    MapChunkOutOfRange,
}

impl From<AdtError> for WowDataError {
    fn from(value: AdtError) -> Self {
        WowDataError::GenericError(format!("AdtError: {}", value))
    }
}
