mod error;
#[cfg(any(feature = "serde", feature = "csv_export"))]
mod export;
mod field_parser;
mod header;
mod parser;
mod schema;
mod schema_discovery;
mod schema_loader;
mod stringblock;
mod types;
mod versions;
mod writer;

#[cfg(feature = "mmap")]
mod mmap;

mod lazy;

#[cfg(feature = "parallel")]
mod parallel;

#[cfg(feature = "cli")]
pub mod dbd;

pub use error::Error;
pub use header::DbcHeader;
pub use lazy::{LazyDbcParser, LazyRecordIterator};
pub use parser::{DbcParser, Record, RecordSet, Value};
pub use schema::{FieldType, Schema, SchemaField};
pub use schema_discovery::{Confidence, DiscoveredField, DiscoveredSchema, SchemaDiscoverer};
pub use stringblock::{CachedStringBlock, StringBlock};
pub use types::*;

#[cfg(feature = "yaml")]
pub use schema_loader::{SchemaDefinition, SchemaFieldDefinition};

#[cfg(feature = "serde")]
pub use export::export_to_json;

#[cfg(feature = "csv_export")]
pub use export::export_to_csv;

#[cfg(feature = "mmap")]
pub use mmap::MmapDbcFile;

#[cfg(feature = "parallel")]
pub use parallel::parse_records_parallel;

pub use versions::{DbcVersion, Wdb2Header, Wdb5Header};
pub use writer::DbcWriter;

/// Result type used throughout the library
pub type Result<T> = std::result::Result<T, Error>;
