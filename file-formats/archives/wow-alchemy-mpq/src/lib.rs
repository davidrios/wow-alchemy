//! Reader and writer for MPQ archives, the archives of World of Warcraft up to Mists of
//! Pandaria.
//!
//! - [`Archive`] opens one archive (format versions 1 to 4) and reads its files.
//! - [`PatchChain`] reads several archives as one, the way the game layers them.
//! - [`ArchiveBuilder`] writes new archives.
//!
//! Binary records ([`header`], [`tables`], [`special_files`], [`patch`]) derive their
//! readers and writers from `wow-alchemy-data`, so each layout is written down once.
//!
//! ```no_run
//! use wow_alchemy_mpq::PatchChain;
//!
//! let mut chain = PatchChain::open_wotlk_client("C:/Games/WoW", "enUS")?;
//! let names = chain.list()?;
//! let data = chain.read_file(&names[0].name)?;
//! # Ok::<(), wow_alchemy_mpq::Error>(())
//! ```

#![warn(missing_docs, rust_2018_idioms)]

pub mod archive;
pub mod builder;
pub mod compression;
pub mod crypto;
pub mod error;
pub mod header;
pub mod patch;
pub mod patch_chain;
pub mod special_files;
pub mod tables;

pub use archive::{Archive, FileEntry, FileInfo};
pub use builder::{ArchiveBuilder, FileOptions};
pub use crypto::{BLOCK_TABLE_KEY, HASH_TABLE_KEY};
pub use error::{Error, Result};
pub use header::{FormatVersion, MpqHeader};
pub use patch_chain::PatchChain;
pub use tables::BlockFlags;
