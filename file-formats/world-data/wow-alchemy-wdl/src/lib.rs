#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod conversion;
pub mod error;
pub mod parser;
pub mod types;
pub mod validation;
pub mod version;

// Re-export primary types
pub use error::{Result, WdlError};
pub use types::WdlFile;
pub use version::WdlVersion;
