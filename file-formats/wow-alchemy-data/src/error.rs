use std::io;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum WowDataError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Unsupported numeric version: {0}")]
    UnsupportedNumericVersion(u32),

    #[error("Conversion error: cannot convert from version {from} to {to}: {reason}")]
    ConversionError { from: u32, to: u32, reason: String },

    #[error("Generic error: {0}")]
    GenericError(String),

    #[error("Invalid value when trying to parse WowEnumFrom: {0} to {1}")]
    InvalidEnumParsedValue(String, String),

    #[error("Invalid magic: expected {expected}, got {actual}")]
    InvalidMagic { expected: String, actual: String },

    #[error("Missing required chunk {0}")]
    MissingChunk(String),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Chunk {magic} says it has {bytes} bytes, but {available} are there to read")]
    ChunkOverrun {
        magic: String,
        bytes: u32,
        available: u64,
    },
}

pub type Result<T> = std::result::Result<T, WowDataError>;
