//! The classic block table, and the hi-block table holding the high 16 bits of file
//! offsets in v2 archives.

use std::io::Cursor;

use bitflags::bitflags;
use wow_alchemy_data::prelude::*;
use wow_alchemy_data_derive::{WowHeaderR, WowHeaderW};

use crate::Result;

bitflags! {
    /// Flags of a file's block.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
    #[wow_data(bitflags = u32)]
    pub struct BlockFlags: u32 {
        /// Compressed with PKWARE implode, without a compression byte.
        const IMPLODE = 0x0000_0100;
        /// Compressed, each sector starting with its compression byte.
        const COMPRESS = 0x0000_0200;
        /// Encrypted.
        const ENCRYPTED = 0x0001_0000;
        /// The key is adjusted by the file's position and size.
        const FIX_KEY = 0x0002_0000;
        /// An incremental patch of a file in an earlier archive.
        const PATCH_FILE = 0x0010_0000;
        /// Stored in one piece instead of sectors.
        const SINGLE_UNIT = 0x0100_0000;
        /// A deletion marker.
        const DELETE_MARKER = 0x0200_0000;
        /// Sectors are followed by their checksums.
        const SECTOR_CRC = 0x0400_0000;
        /// The block holds a file.
        const EXISTS = 0x8000_0000;
    }
}

impl BlockFlags {
    /// Either compression flag.
    pub fn is_compressed(self) -> bool {
        self.intersects(Self::IMPLODE | Self::COMPRESS)
    }
}

/// An entry of the block table.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
pub struct BlockEntry {
    /// Offset of the data from the archive start (low 32 bits).
    pub file_pos: u32,
    /// Stored size.
    pub compressed_size: u32,
    /// Size of the file.
    pub file_size: u32,
    /// Flags.
    pub flags: BlockFlags,
}

impl BlockEntry {
    /// Size on disk.
    pub const SIZE: usize = 16;
}

/// The block table, decrypted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BlockTable {
    /// The entries.
    pub entries: Vec<BlockEntry>,
}

impl BlockTable {
    /// Reads the entries from decrypted bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(data);
        let mut entries = Vec::with_capacity(data.len() / BlockEntry::SIZE);
        for _ in 0..data.len() / BlockEntry::SIZE {
            entries.push(cursor.wow_read()?);
        }
        Ok(Self { entries })
    }

    /// The entries as bytes, before encryption.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut bytes = Vec::with_capacity(self.entries.len() * BlockEntry::SIZE);
        self.entries.as_slice().wow_write(&mut bytes)?;
        Ok(bytes)
    }
}

/// The hi-block table: the high 16 bits of each block's offset. Not encrypted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HiBlockTable {
    /// One per block.
    pub entries: Vec<u16>,
}

impl HiBlockTable {
    /// Reads the entries from bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(data);
        let mut entries = Vec::with_capacity(data.len() / 2);
        for _ in 0..data.len() / 2 {
            entries.push(cursor.wow_read()?);
        }
        Ok(Self { entries })
    }

    /// The entries as bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut bytes = Vec::with_capacity(self.entries.len() * 2);
        self.entries.as_slice().wow_write(&mut bytes)?;
        Ok(bytes)
    }

    /// High bits of block `index`'s offset, shifted into place.
    pub fn high_bits(&self, index: usize) -> u64 {
        (self.entries.get(index).copied().unwrap_or(0) as u64) << 32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_round_trip() {
        let table = BlockTable {
            entries: vec![BlockEntry {
                file_pos: 0x20,
                compressed_size: 10,
                file_size: 20,
                flags: BlockFlags::EXISTS | BlockFlags::COMPRESS,
            }],
        };
        let bytes = table.to_bytes().unwrap();
        assert_eq!(bytes.len(), 16);
        assert_eq!(&bytes[12..], &0x8000_0200u32.to_le_bytes());
        assert_eq!(BlockTable::from_bytes(&bytes).unwrap(), table);

        let hi = HiBlockTable {
            entries: vec![1, 0],
        };
        assert_eq!(
            HiBlockTable::from_bytes(&hi.to_bytes().unwrap()).unwrap(),
            hi
        );
        assert_eq!(hi.high_bits(0), 1 << 32);
    }
}
