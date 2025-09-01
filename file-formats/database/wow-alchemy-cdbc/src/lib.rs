mod error;
mod field_parser;
mod lazy;
mod types;
mod wdb;

pub mod converter;
pub mod dbd;

pub use error::Error;
pub use lazy::LazyRecordIterator;
pub use types::*;
pub use wdb::{DbcVersion, WdbFile};

pub type Result<T> = std::result::Result<T, Error>;
