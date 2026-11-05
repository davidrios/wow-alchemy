//! The archive's file tables: the classic hash and block tables (and the hi-block table of
//! v2), and the HET and BET tables of v3 and later.
//!
//! Tables are encrypted and, in v4, may be compressed. They are decrypted and decompressed
//! into a buffer first and their records read from it with the derived readers.

mod bet;
mod bits;
mod block;
mod hash;
mod het;

pub use bet::{BET_MAGIC, BetFileInfo, BetHeader, BetTable};
pub use bits::BitArray;
pub use block::{BlockEntry, BlockFlags, BlockTable, HiBlockTable};
pub use hash::{HashEntry, HashTable};
pub use het::{HET_MAGIC, HetHeader, HetTable};
pub(crate) use het::{read_ext_table, write_ext_table};

use std::io::{Read, Seek, SeekFrom};

use wow_alchemy_data::types::MagicStr;
use wow_alchemy_data_derive::{WowHeaderR, WowHeaderW};

use crate::{Result, compression, crypto};

/// The header in front of HET and BET tables. Never encrypted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, WowHeaderR, WowHeaderW)]
pub struct ExtTableHeader {
    /// `HET\x1A` or `BET\x1A`.
    pub magic: MagicStr,
    /// Always 1.
    pub version: u32,
    /// Size of the table after this header, uncompressed.
    pub data_size: u32,
}

impl ExtTableHeader {
    /// Size on disk.
    pub const SIZE: usize = 12;
}

/// Reads `stored_size` bytes at `offset`, decrypts them with `key` (if not 0) and, if they
/// are shorter than `table_size`, decompresses them. A stored table running past the end of
/// the file is cut there, as Storm does, and the missing bytes are filled with `fill`.
pub(crate) fn read_table<R: Read + Seek>(
    reader: &mut R,
    offset: u64,
    stored_size: u64,
    table_size: usize,
    key: u32,
    fill: u8,
) -> Result<Vec<u8>> {
    let file_size = reader.seek(SeekFrom::End(0))?;
    let available = file_size.saturating_sub(offset).min(stored_size) as usize;
    let mut data = vec![0u8; available];
    reader.seek(SeekFrom::Start(offset))?;
    reader.read_exact(&mut data)?;
    if key != 0 {
        crypto::decrypt_bytes(&mut data, key);
    }
    if (stored_size as usize) < table_size && available == stored_size as usize {
        data = compression::decompress(&data, table_size)?;
    }
    data.resize(table_size, fill);
    Ok(data)
}

/// Encrypts a table for writing.
pub(crate) fn encrypt_table(mut data: Vec<u8>, key: u32) -> Vec<u8> {
    crypto::encrypt_bytes(&mut data, key);
    data
}
