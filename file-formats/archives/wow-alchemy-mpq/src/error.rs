//! Errors of the MPQ reader and writer.

use std::io;
use thiserror::Error;
use wow_alchemy_data::error::WowDataError;

/// Result of MPQ operations.
pub type Result<T> = std::result::Result<T, Error>;

/// What can go wrong reading or writing an MPQ archive.
#[derive(Error, Debug)]
pub enum Error {
    /// Reading or writing the underlying file failed.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// A binary record couldn't be read or written.
    #[error("{0}")]
    Data(#[from] WowDataError),

    /// The archive, or a structure in it, is malformed.
    #[error("Invalid MPQ format: {0}")]
    InvalidFormat(String),

    /// The archive's header names a format version this crate doesn't know.
    #[error("Unsupported MPQ version: {0}")]
    UnsupportedVersion(u16),

    /// The file isn't in the archive or chain.
    #[error("File not found: {0}")]
    FileNotFound(String),

    /// Decompressing or compressing data failed.
    #[error("Compression error: {0}")]
    Compression(String),

    /// A stored checksum doesn't match the data.
    #[error("Checksum mismatch for {file}: expected {expected:08x}, got {actual:08x}")]
    ChecksumMismatch {
        /// File whose data failed the check.
        file: String,
        /// Checksum stored in the archive.
        expected: u32,
        /// Checksum of the data read.
        actual: u32,
    },
}

impl Error {
    /// An [`Error::InvalidFormat`] with this message.
    pub fn invalid_format<S: Into<String>>(msg: S) -> Self {
        Error::InvalidFormat(msg.into())
    }

    /// An [`Error::Compression`] with this message.
    pub fn compression<S: Into<String>>(msg: S) -> Self {
        Error::Compression(msg.into())
    }
}
