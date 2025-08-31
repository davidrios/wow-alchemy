mod error;
mod field_parser;
mod header;
mod parser;
mod schema;
mod stringblock;
mod types;
mod versions;
mod writer;

#[cfg(feature = "mmap")]
mod mmap;

mod lazy;

#[cfg(feature = "parallel")]
mod parallel;

pub mod converter;

pub mod dbd;

pub use error::Error;
pub use header::DbcHeader;
pub use lazy::{LazyDbcParser, LazyRecordIterator};
pub use parser::{DbcParser, Record, RecordSet, Value};
pub use schema::{FieldType, Schema, SchemaField};
pub use stringblock::{CachedStringBlock, StringBlock};
pub use types::*;

#[cfg(feature = "mmap")]
pub use mmap::MmapDbcFile;

#[cfg(feature = "parallel")]
pub use parallel::parse_records_parallel;

pub use versions::{DbcVersion, Wdb2Header, Wdb5Header};
pub use writer::DbcWriter;

pub type Result<T> = std::result::Result<T, Error>;
