//! The BET table of v3+ archives: bit-packed file entries and the rest of each name hash.

use std::io::{Cursor, Read, Write};

use wow_alchemy_data::prelude::*;
use wow_alchemy_data::types::MagicStr;
use wow_alchemy_data_derive::{WowHeaderR, WowHeaderW};

use super::bits::{BitArray, bits_needed};
use super::block::BlockFlags;
use crate::Result;
use crate::crypto::het_hash;

/// Magic of the BET table.
pub const BET_MAGIC: MagicStr = *b"BET\x1A";

/// The BET table's header, after its [`super::ExtTableHeader`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
pub struct BetHeader {
    /// Size of this header and the arrays after it.
    pub table_size: u32,
    /// Files in the table.
    pub entry_count: u32,
    /// Always 0x10.
    pub unknown_08: u32,
    /// Bits of one file entry.
    pub table_entry_size: u32,
    /// Bit offset of the file position in an entry.
    pub bit_index_file_pos: u32,
    /// Bit offset of the file size in an entry.
    pub bit_index_file_size: u32,
    /// Bit offset of the stored size in an entry.
    pub bit_index_cmp_size: u32,
    /// Bit offset of the flag index in an entry.
    pub bit_index_flag_index: u32,
    /// Bit offset of the unknown field in an entry.
    pub bit_index_unknown: u32,
    /// Bits of the file position.
    pub bit_count_file_pos: u32,
    /// Bits of the file size.
    pub bit_count_file_size: u32,
    /// Bits of the stored size.
    pub bit_count_cmp_size: u32,
    /// Bits of the flag index.
    pub bit_count_flag_index: u32,
    /// Bits of the unknown field.
    pub bit_count_unknown: u32,
    /// Bits each name hash takes in the hash array.
    pub bit_total_name_hash2: u32,
    /// Extra bits after each name hash.
    pub bit_extra_name_hash2: u32,
    /// Bits of each name hash: the HET hash size minus 8.
    pub bit_count_name_hash2: u32,
    /// Size of the hash array in bytes.
    pub name_hash_array_size: u32,
    /// Entries in the flag array.
    pub flag_count: u32,
}

impl BetHeader {
    /// Size on disk.
    pub const SIZE: usize = 76;
}

/// One file of the BET table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BetFileInfo {
    /// Offset of the data from the archive start.
    pub file_pos: u64,
    /// Size of the file.
    pub file_size: u64,
    /// Stored size.
    pub compressed_size: u64,
    /// Flags.
    pub flags: BlockFlags,
}

/// A decoded BET table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BetTable {
    /// Its header.
    pub header: BetHeader,
    /// The distinct flags files use.
    pub flags: Vec<BlockFlags>,
    /// The file entries.
    pub file_table: BitArray,
    /// The low bits of each file's name hash.
    pub name_hashes: BitArray,
}

impl BetTable {
    /// Builds a table of these files and names; a file without a name gets a zero hash.
    pub fn build(files: &[(BetFileInfo, Option<&str>)]) -> Self {
        let max_pos = files.iter().map(|(f, _)| f.file_pos).max().unwrap_or(0);
        let max_size = files.iter().map(|(f, _)| f.file_size).max().unwrap_or(0);
        let max_cmp = files
            .iter()
            .map(|(f, _)| f.compressed_size)
            .max()
            .unwrap_or(0);
        let mut flags: Vec<BlockFlags> = Vec::new();
        for (file, _) in files {
            if !flags.contains(&file.flags) {
                flags.push(file.flags);
            }
        }
        let count_pos = bits_needed(max_pos);
        let count_size = bits_needed(max_size);
        let count_cmp = bits_needed(max_cmp);
        let count_flag = if flags.len() > 1 {
            bits_needed(flags.len() as u64 - 1)
        } else {
            0
        };
        let entry_size = count_pos + count_size + count_cmp + count_flag;
        let hash_bits = 56;
        let mut header = BetHeader {
            table_size: 0,
            entry_count: files.len() as u32,
            unknown_08: 0x10,
            table_entry_size: entry_size,
            bit_index_file_pos: 0,
            bit_index_file_size: count_pos,
            bit_index_cmp_size: count_pos + count_size,
            bit_index_flag_index: count_pos + count_size + count_cmp,
            bit_index_unknown: entry_size,
            bit_count_file_pos: count_pos,
            bit_count_file_size: count_size,
            bit_count_cmp_size: count_cmp,
            bit_count_flag_index: count_flag,
            bit_count_unknown: 0,
            bit_total_name_hash2: hash_bits,
            bit_extra_name_hash2: 0,
            bit_count_name_hash2: hash_bits,
            name_hash_array_size: 0,
            flag_count: flags.len() as u32,
        };
        let mut file_table = BitArray::new(files.len() * entry_size as usize);
        let mut name_hashes = BitArray::new(files.len() * hash_bits as usize);
        for (i, (file, name)) in files.iter().enumerate() {
            let base = i * entry_size as usize;
            let flag_index = flags.iter().position(|f| *f == file.flags).unwrap_or(0);
            file_table.set(
                base + header.bit_index_file_pos as usize,
                count_pos,
                file.file_pos,
            );
            file_table.set(
                base + header.bit_index_file_size as usize,
                count_size,
                file.file_size,
            );
            file_table.set(
                base + header.bit_index_cmp_size as usize,
                count_cmp,
                file.compressed_size,
            );
            file_table.set(
                base + header.bit_index_flag_index as usize,
                count_flag,
                flag_index as u64,
            );
            let hash = name.map_or(0, |name| het_hash(name, 64));
            name_hashes.set(i * hash_bits as usize, hash_bits, hash);
        }
        header.name_hash_array_size = name_hashes.bytes.len() as u32;
        header.table_size = (BetHeader::SIZE + flags.len() * 4 + file_table.bytes.len()) as u32
            + header.name_hash_array_size;
        Self {
            header,
            flags,
            file_table,
            name_hashes,
        }
    }

    /// Reads the table from its body, after the [`super::ExtTableHeader`], decrypted and
    /// decompressed.
    pub fn from_body(body: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(body);
        let header: BetHeader = cursor.wow_read()?;
        let mut flags = Vec::with_capacity(header.flag_count as usize);
        for _ in 0..header.flag_count {
            flags.push(cursor.wow_read()?);
        }
        let table_bits = header.entry_count as usize * header.table_entry_size as usize;
        let mut file_table = vec![0u8; table_bits.div_ceil(8)];
        cursor.read_exact(&mut file_table)?;
        let mut name_hashes = vec![0u8; header.name_hash_array_size as usize];
        cursor.read_exact(&mut name_hashes)?;
        Ok(Self {
            header,
            flags,
            file_table: BitArray { bytes: file_table },
            name_hashes: BitArray { bytes: name_hashes },
        })
    }

    /// The table's body, before encryption.
    pub fn to_body(&self) -> Result<Vec<u8>> {
        let mut body = Vec::with_capacity(self.header.table_size as usize);
        self.header.wow_write(&mut body)?;
        self.flags.as_slice().wow_write(&mut body)?;
        body.write_all(&self.file_table.bytes)?;
        body.write_all(&self.name_hashes.bytes)?;
        Ok(body)
    }

    /// File entry `index`.
    pub fn file(&self, index: u32) -> Option<BetFileInfo> {
        if index >= self.header.entry_count {
            return None;
        }
        let h = &self.header;
        let base = index as usize * h.table_entry_size as usize;
        let field = |offset: u32, count: u32| self.file_table.get(base + offset as usize, count);
        let flag_index = field(h.bit_index_flag_index, h.bit_count_flag_index)? as usize;
        Some(BetFileInfo {
            file_pos: field(h.bit_index_file_pos, h.bit_count_file_pos)?,
            file_size: field(h.bit_index_file_size, h.bit_count_file_size)?,
            compressed_size: field(h.bit_index_cmp_size, h.bit_count_cmp_size)?,
            flags: self.flags.get(flag_index).copied().unwrap_or_default(),
        })
    }

    /// Whether file `index`'s stored name hash matches the full HET hash of a name.
    pub fn matches(&self, index: u32, het_hash: u64) -> bool {
        let bits = self.header.bit_count_name_hash2;
        let position = index as usize * self.header.bit_total_name_hash2 as usize;
        let mask = if bits >= 64 {
            u64::MAX
        } else {
            (1u64 << bits) - 1
        };
        self.name_hashes.get(position, bits) == Some(het_hash & mask)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_and_round_trips() {
        let file = |pos, size, flags| BetFileInfo {
            file_pos: pos,
            file_size: size,
            compressed_size: size / 2,
            flags,
        };
        let files = [
            (file(0x2C, 100, BlockFlags::EXISTS), Some("a.txt")),
            (
                file(
                    0x1_0000_0000,
                    5000,
                    BlockFlags::EXISTS | BlockFlags::COMPRESS,
                ),
                Some("b.txt"),
            ),
        ];
        let table = BetTable::build(&files);
        let read = BetTable::from_body(&table.to_body().unwrap()).unwrap();
        assert_eq!(read, table);
        assert_eq!(read.file(0), Some(files[0].0));
        assert_eq!(read.file(1), Some(files[1].0));
        assert_eq!(read.file(2), None);
        assert!(read.matches(1, het_hash("B.TXT", 64)));
        assert!(!read.matches(0, het_hash("b.txt", 64)));
    }
}
