//! Incremental patch files (`PTCH`), as Cataclysm and later archives store some files: a
//! COPY patch replaces the file, a BSD0 patch is a bsdiff of it.
//!
//! A patch file's data in the archive starts with a [`PatchInfo`]; what follows it, read
//! like any file's data, is a [`PatchHeader`] and the patch.

mod apply;

pub use apply::apply_patch;

use std::io::{Cursor, Read};

use md5::{Digest, Md5};
use wow_alchemy_data::prelude::*;
use wow_alchemy_data::types::MagicStr;
use wow_alchemy_data_derive::{WowEnumFrom, WowHeaderR, WowHeaderW};

use crate::{Error, Result};

/// The block in front of a patch file's data in the archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, WowHeaderR, WowHeaderW)]
pub struct PatchInfo {
    /// Size of this block, 28 or more.
    pub length: u32,
    /// Flags, 0x80000000.
    pub flags: u32,
    /// Size of the patch data that follows, uncompressed.
    pub data_size: u32,
    /// MD5 of the patch data.
    pub md5: [u8; 16],
}

impl PatchInfo {
    /// Size of the fields above.
    pub const SIZE: u32 = 28;
}

/// `PTCH` block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, WowHeaderR, WowHeaderW)]
pub struct PtchBlock {
    /// `PTCH`.
    pub magic: MagicStr,
    /// Size of the whole patch, uncompressed.
    pub patch_data_size: u32,
    /// Size of the file before patching.
    pub size_before: u32,
    /// Size of the file after patching.
    pub size_after: u32,
}

/// `MD5_` block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, WowHeaderR, WowHeaderW)]
pub struct Md5Block {
    /// `MD5_`.
    pub magic: MagicStr,
    /// Size of this block, 40.
    pub block_size: u32,
    /// MD5 of the file before patching.
    pub md5_before: [u8; 16],
    /// MD5 of the file after patching.
    pub md5_after: [u8; 16],
}

const COPY: MagicStr = *b"COPY";
const BSD0: MagicStr = *b"BSD0";

/// What a patch does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, WowEnumFrom, WowHeaderR, WowHeaderW)]
#[wow_data(from_type = MagicStr)]
pub enum PatchType {
    /// The patch data is the new file.
    #[wow_data(expr = COPY)]
    Copy,
    /// The patch data is a bsdiff, run-length coded.
    #[wow_data(expr = BSD0)]
    Bsd0,
}

/// `XFRM` block header; the patch data follows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, WowHeaderR, WowHeaderW)]
pub struct XfrmBlock {
    /// `XFRM`.
    pub magic: MagicStr,
    /// Size of this header and the patch data after it.
    pub block_size: u32,
    /// What the patch does.
    pub patch_type: PatchType,
}

/// The header of a patch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, WowHeaderR, WowHeaderW)]
pub struct PatchHeader {
    /// `PTCH` block.
    pub ptch: PtchBlock,
    /// `MD5_` block.
    pub md5: Md5Block,
    /// `XFRM` block header.
    pub xfrm: XfrmBlock,
}

impl PatchHeader {
    /// Size on disk.
    pub const SIZE: usize = 68;
    /// Size of the `XFRM` header.
    pub const XFRM_HEADER_SIZE: u32 = 12;
}

/// A patch: its header and data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchFile {
    /// The header.
    pub header: PatchHeader,
    /// The data in the `XFRM` block.
    pub data: Vec<u8>,
}

impl PatchFile {
    /// Parses a patch.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(data);
        let header: PatchHeader = cursor.wow_read()?;
        for (magic, expected) in [
            (header.ptch.magic, b"PTCH"),
            (header.md5.magic, b"MD5_"),
            (header.xfrm.magic, b"XFRM"),
        ] {
            if &magic != expected {
                return Err(Error::invalid_format(format!(
                    "Patch block {:?} where {:?} was expected",
                    String::from_utf8_lossy(&magic),
                    String::from_utf8_lossy(expected)
                )));
            }
        }
        let size = header
            .xfrm
            .block_size
            .saturating_sub(PatchHeader::XFRM_HEADER_SIZE) as usize;
        let mut patch = Vec::new();
        cursor.read_to_end(&mut patch)?;
        if patch.len() < size {
            return Err(Error::invalid_format(format!(
                "Patch data is {} bytes, its header says {size}",
                patch.len()
            )));
        }
        patch.truncate(size);
        Ok(Self {
            header,
            data: patch,
        })
    }

    /// The patch's bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(PatchHeader::SIZE + self.data.len());
        self.header.wow_write(&mut out)?;
        out.extend_from_slice(&self.data);
        Ok(out)
    }
}

pub(crate) fn check_md5(data: &[u8], expected: &[u8; 16], what: &str) -> Result<()> {
    let actual: [u8; 16] = Md5::digest(data).into();
    if &actual == expected {
        Ok(())
    } else {
        Err(Error::invalid_format(format!(
            "MD5 of the {what} doesn't match the patch"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_is_68_bytes_and_round_trips() {
        let patch = PatchFile {
            header: PatchHeader {
                ptch: PtchBlock {
                    magic: *b"PTCH",
                    patch_data_size: 72,
                    size_before: 3,
                    size_after: 4,
                },
                md5: Md5Block {
                    magic: *b"MD5_",
                    block_size: 40,
                    md5_before: [1; 16],
                    md5_after: [2; 16],
                },
                xfrm: XfrmBlock {
                    magic: *b"XFRM",
                    block_size: 16,
                    patch_type: PatchType::Copy,
                },
            },
            data: b"abcd".to_vec(),
        };
        let bytes = patch.to_bytes().unwrap();
        assert_eq!(bytes.len(), PatchHeader::SIZE + 4);
        assert_eq!(PatchFile::parse(&bytes).unwrap(), patch);

        let mut wrong = bytes.clone();
        wrong[0] = b'X';
        assert!(PatchFile::parse(&wrong).is_err());
    }
}
