mod error;
mod field_parser;
mod lazy;
mod types;
mod wdb;

pub mod dbd;

#[cfg(feature = "sqlite")]
pub mod sqlite_converter;

pub use error::Error;
pub use lazy::LazyRecordIterator;
pub use types::*;
pub use wdb::{DbcVersion, WdbFile};

pub type Result<T> = std::result::Result<T, Error>;
