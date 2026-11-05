//! DBC and early DB2 client database tables (WDBC and WDB2).
//!
//! [`WdbFile`] reads a table and gives its fields raw and its strings by offset; with a
//! WoWDBDefs definition from [`dbd`], [`WdbFile::read_records`] types whole records, which
//! `sqlite_converter` turns into a SQLite database.

mod error;
mod record;
mod types;
mod wdb;

pub mod dbd;

#[cfg(feature = "sqlite")]
pub mod sqlite_converter;

pub use error::Error;
pub use record::{FieldType, RecordLayout};
pub use types::*;
pub use wdb::*;

/// The result of typed reading, definitions and conversion. Reading a table's file returns
/// [`wow_alchemy_data::error::Result`].
pub type Result<T> = std::result::Result<T, Error>;
