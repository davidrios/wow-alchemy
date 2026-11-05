//! `(attributes)`: per-block CRC32s, file times, MD5s and patch bits.

use std::io::{Cursor, Read};

use bitflags::bitflags;
use wow_alchemy_data::prelude::*;
use wow_alchemy_data_derive::{WowHeaderR, WowHeaderW};

use crate::{Error, Result};

bitflags! {
    /// Which arrays an `(attributes)` file has.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
    #[wow_data(bitflags = u32)]
    pub struct AttributeFlags: u32 {
        /// CRC32 of each file.
        const CRC32 = 0x1;
        /// Windows FILETIME of each file.
        const FILETIME = 0x2;
        /// MD5 of each file.
        const MD5 = 0x4;
        /// One bit per file, set for patch files.
        const PATCH_BIT = 0x8;
    }
}

/// The header of `(attributes)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, WowHeaderR, WowHeaderW)]
pub struct AttributesHeader {
    /// Always [`Attributes::VERSION`].
    pub version: u32,
    /// The arrays that follow.
    pub flags: AttributeFlags,
}

/// A parsed `(attributes)` file. Each present array has one entry per block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attributes {
    /// The header.
    pub header: AttributesHeader,
    /// CRC32s, if present.
    pub crc32: Vec<u32>,
    /// File times, if present.
    pub filetime: Vec<u64>,
    /// MD5s, if present.
    pub md5: Vec<[u8; 16]>,
    /// Patch bits, if present.
    pub patch_bits: Vec<bool>,
}

impl Attributes {
    /// The only version there is.
    pub const VERSION: u32 = 100;

    fn size(flags: AttributeFlags, entries: usize) -> usize {
        let mut size = 8;
        if flags.contains(AttributeFlags::CRC32) {
            size += entries * 4;
        }
        if flags.contains(AttributeFlags::FILETIME) {
            size += entries * 8;
        }
        if flags.contains(AttributeFlags::MD5) {
            size += entries * 16;
        }
        if flags.contains(AttributeFlags::PATCH_BIT) {
            size += entries.div_ceil(8);
        }
        size
    }

    /// Parses `(attributes)` of an archive with `block_count` blocks. Some writers leave
    /// out the entry of `(attributes)` itself, so one entry less is accepted too.
    pub fn parse(data: &[u8], block_count: usize) -> Result<Self> {
        let mut cursor = Cursor::new(data);
        let header: AttributesHeader = cursor.wow_read()?;
        if header.version != Self::VERSION {
            return Err(Error::invalid_format(format!(
                "(attributes) version {}",
                header.version
            )));
        }
        let entries = [block_count, block_count.saturating_sub(1)]
            .into_iter()
            .find(|&n| {
                let size = Self::size(header.flags, n);
                // Storm writes the patch bits one byte short when the count is 8n + 1.
                data.len() == size || data.len() + 1 == size
            })
            .ok_or_else(|| {
                Error::invalid_format(format!(
                    "(attributes) of {} bytes doesn't fit {block_count} blocks",
                    data.len()
                ))
            })?;

        let mut attributes = Self {
            header,
            crc32: Vec::new(),
            filetime: Vec::new(),
            md5: Vec::new(),
            patch_bits: Vec::new(),
        };
        if header.flags.contains(AttributeFlags::CRC32) {
            for _ in 0..entries {
                attributes.crc32.push(cursor.wow_read()?);
            }
        }
        if header.flags.contains(AttributeFlags::FILETIME) {
            for _ in 0..entries {
                attributes.filetime.push(cursor.wow_read()?);
            }
        }
        if header.flags.contains(AttributeFlags::MD5) {
            for _ in 0..entries {
                attributes.md5.push(cursor.wow_read()?);
            }
        }
        if header.flags.contains(AttributeFlags::PATCH_BIT) {
            let mut bits = Vec::new();
            cursor.read_to_end(&mut bits)?;
            attributes.patch_bits = (0..entries)
                .map(|i| {
                    bits.get(i / 8)
                        .is_some_and(|byte| byte & (0x80 >> (i % 8)) != 0)
                })
                .collect();
        }
        Ok(attributes)
    }

    /// The file's bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        self.header.wow_write(&mut out)?;
        self.crc32.as_slice().wow_write(&mut out)?;
        self.filetime.as_slice().wow_write(&mut out)?;
        self.md5.as_slice().wow_write(&mut out)?;
        if self.header.flags.contains(AttributeFlags::PATCH_BIT) {
            let mut bits = vec![0u8; self.patch_bits.len().div_ceil(8)];
            for (i, _) in self.patch_bits.iter().enumerate().filter(|(_, set)| **set) {
                bits[i / 8] |= 0x80 >> (i % 8);
            }
            out.extend(bits);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(entries: usize) -> Attributes {
        Attributes {
            header: AttributesHeader {
                version: Attributes::VERSION,
                flags: AttributeFlags::all(),
            },
            crc32: (0..entries as u32).collect(),
            filetime: (0..entries as u64).map(|t| t * 1000).collect(),
            md5: (0..entries).map(|i| [i as u8; 16]).collect(),
            patch_bits: (0..entries).map(|i| i % 3 == 0).collect(),
        }
    }

    #[test]
    fn round_trips() {
        let attributes = sample(10);
        let bytes = attributes.to_bytes().unwrap();
        assert_eq!(bytes.len(), 8 + 10 * 28 + 2);
        assert_eq!(Attributes::parse(&bytes, 10).unwrap(), attributes);
    }

    #[test]
    fn accepts_one_entry_less() {
        let bytes = sample(4).to_bytes().unwrap();
        assert_eq!(Attributes::parse(&bytes, 5).unwrap().crc32.len(), 4);
        assert!(Attributes::parse(&bytes, 7).is_err());
    }
}
