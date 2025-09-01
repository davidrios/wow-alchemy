use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Request error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("Rusqlite error: {0}")]
    Rusqlite(#[from] rusqlite::Error),

    #[error("Invalid DBC header: {0}")]
    InvalidHeader(String),

    #[error("Invalid DBC record: {0}")]
    InvalidRecord(String),

    #[error("Invalid string block: {0}")]
    InvalidStringBlock(String),

    #[error("Schema validation error: {0}")]
    SchemaValidation(String),

    #[error("Out of bounds: {0}")]
    OutOfBounds(String),

    #[error("Type conversion error: {0}")]
    TypeConversion(String),

    #[error("Generic error: {0}")]
    GenericError(String),
}
