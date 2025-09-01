use std::io;
use thiserror::Error;
use wow_alchemy_data::error::WowDataError;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Request error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("Rusqlite error: {0}")]
    Rusqlite(#[from] rusqlite::Error),

    #[error("WowData error: {0}")]
    WowData(#[from] wow_alchemy_data::error::WowDataError),

    #[error("GameBuild error: {0}")]
    GameBuild(String),

    #[error("No dbd field definitions were found for the specified build")]
    NoFieldsForBuild,

    #[error("Error generating SQLite table definition: {0}")]
    SqliteTableDefinition(String),

    #[error("Generic error: {0}")]
    GenericError(String),
}

impl From<Error> for WowDataError {
    fn from(value: Error) -> Self {
        WowDataError::GenericError(format!("Error: {}", value))
    }
}
