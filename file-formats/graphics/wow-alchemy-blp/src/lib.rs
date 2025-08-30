#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]

/// Conversion utilities to/from DynamicImage
pub mod convert;
/// Encoding BLP format into stream of bytes
pub mod encode;
/// Decoding BLP format from raw bytes
pub mod parser;
/// Utilities for mipmaps filename generation
pub mod path;
/// Defines structure of parsed BLP file
pub mod types;

pub use types::*;
