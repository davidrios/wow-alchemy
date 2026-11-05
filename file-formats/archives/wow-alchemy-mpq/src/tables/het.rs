//! The HET table of v3+ archives: 8-bit name hashes pointing at BET entries.

use std::io::{Cursor, Read, Seek, SeekFrom, Write};

use wow_alchemy_data::prelude::*;
use wow_alchemy_data::types::MagicStr;
use wow_alchemy_data_derive::{WowHeaderR, WowHeaderW};

use super::ExtTableHeader;
use super::bits::{BitArray, bits_needed};
use crate::crypto::het_hash;
use crate::{Error, Result, compression, crypto};

/// Magic of the HET table.
pub const HET_MAGIC: MagicStr = *b"HET\x1A";

/// The HET table's header, after its [`ExtTableHeader`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
pub struct HetHeader {
    /// Size of this header and the arrays after it.
    pub table_size: u32,
    /// Files in the table.
    pub entry_count: u32,
    /// Slots in the hash array.
    pub total_count: u32,
    /// Bits of the name hash, 64 in Storm's archives.
    pub name_hash_bits: u32,
    /// Bits each BET index takes in the index array.
    pub index_size_total: u32,
    /// Extra bits after each index.
    pub index_size_extra: u32,
    /// Bits of each BET index.
    pub index_size: u32,
    /// Size of the index array in bytes.
    pub index_table_size: u32,
}

impl HetHeader {
    /// Size on disk.
    pub const SIZE: usize = 32;
}

/// A decoded HET table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HetTable {
    /// Its header.
    pub header: HetHeader,
    /// The top 8 bits of each slot's name hash; 0xFF marks a free slot.
    pub name_hashes: Vec<u8>,
    /// The BET index of each slot.
    pub indices: BitArray,
}

impl HetTable {
    /// A free slot in [`HetTable::name_hashes`].
    pub const FREE: u8 = 0xFF;

    /// Builds a table for these names, `names[i]` being BET entry `i`. Names that are `None`
    /// get no slot.
    pub fn build(names: &[Option<&str>]) -> Self {
        let entry_count = names.len() as u32;
        let total_count = (entry_count * 4 / 3).max(entry_count + 1);
        let index_size = bits_needed(entry_count.saturating_sub(1) as u64);
        let mut indices = BitArray::new(total_count as usize * index_size as usize);
        let mut name_hashes = vec![Self::FREE; total_count as usize];
        for (file_index, name) in names.iter().enumerate() {
            let Some(name) = name else { continue };
            let hash = het_hash(name, 64);
            let mut slot = (hash % total_count as u64) as usize;
            while name_hashes[slot] != Self::FREE {
                slot = (slot + 1) % total_count as usize;
            }
            name_hashes[slot] = (hash >> 56) as u8;
            indices.set(slot * index_size as usize, index_size, file_index as u64);
        }
        let index_table_size = indices.bytes.len() as u32;
        Self {
            header: HetHeader {
                table_size: HetHeader::SIZE as u32 + total_count + index_table_size,
                entry_count,
                total_count,
                name_hash_bits: 64,
                index_size_total: index_size,
                index_size_extra: 0,
                index_size,
                index_table_size,
            },
            name_hashes,
            indices,
        }
    }

    /// Reads the table from its body, after the [`ExtTableHeader`], decrypted and
    /// decompressed.
    pub fn from_body(body: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(body);
        let header: HetHeader = cursor.wow_read()?;
        let mut name_hashes = vec![0u8; header.total_count as usize];
        cursor.read_exact(&mut name_hashes)?;
        let mut index_bytes = vec![0u8; header.index_table_size as usize];
        cursor.read_exact(&mut index_bytes)?;
        if !(8..=64).contains(&header.name_hash_bits) {
            return Err(Error::invalid_format(format!(
                "HET name hashes of {} bits",
                header.name_hash_bits
            )));
        }
        Ok(Self {
            header,
            name_hashes,
            indices: BitArray { bytes: index_bytes },
        })
    }

    /// The table's body, before encryption.
    pub fn to_body(&self) -> Result<Vec<u8>> {
        let mut body = Vec::with_capacity(self.header.table_size as usize);
        self.header.wow_write(&mut body)?;
        body.write_all(&self.name_hashes)?;
        body.write_all(&self.indices.bytes)?;
        Ok(body)
    }

    /// The BET indices of the slots whose 8-bit hash matches `name`, in probing order,
    /// with the name's full hash to check them against.
    pub fn candidates(&self, name: &str) -> (u64, Vec<u32>) {
        let total = self.header.total_count as usize;
        let hash = het_hash(name, self.header.name_hash_bits);
        let mut found = Vec::new();
        if total == 0 || self.header.entry_count == 0 {
            return (hash, found);
        }
        let name_hash1 = (hash >> (self.header.name_hash_bits - 8)) as u8;
        let start = (hash % total as u64) as usize;
        let mut slot = start;
        while self.name_hashes[slot] != Self::FREE {
            if self.name_hashes[slot] == name_hash1 {
                let position = slot * self.header.index_size_total as usize;
                if let Some(index) = self.indices.get(position, self.header.index_size) {
                    found.push(index as u32);
                }
            }
            slot = (slot + 1) % total;
            if slot == start {
                break;
            }
        }
        (hash, found)
    }
}

/// Reads an extended (HET or BET) table at `offset`, `stored_size` bytes long, checking its
/// magic and returning its body decrypted and decompressed.
pub(crate) fn read_ext_table<R: Read + Seek>(
    reader: &mut R,
    offset: u64,
    stored_size: u64,
    magic: MagicStr,
    key: u32,
) -> Result<Vec<u8>> {
    reader.seek(SeekFrom::Start(offset))?;
    let ext: ExtTableHeader = reader.wow_read()?;
    if ext.magic != magic {
        return Err(Error::invalid_format(format!(
            "Expected {:?} table, found {:?}",
            String::from_utf8_lossy(&magic),
            String::from_utf8_lossy(&ext.magic)
        )));
    }
    let stored = stored_size
        .saturating_sub(ExtTableHeader::SIZE as u64)
        .min(ext.data_size as u64) as usize;
    let mut body = vec![0u8; stored];
    reader.read_exact(&mut body)?;
    crypto::decrypt_bytes(&mut body, key);
    if stored < ext.data_size as usize {
        body = compression::decompress(&body, ext.data_size as usize)?;
    }
    Ok(body)
}

/// An extended table as stored: its header, then its body encrypted.
pub(crate) fn write_ext_table(magic: MagicStr, body: &[u8], key: u32) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(ExtTableHeader::SIZE + body.len());
    ExtTableHeader {
        magic,
        version: 1,
        data_size: body.len() as u32,
    }
    .wow_write(&mut out)?;
    let mut body = body.to_vec();
    crypto::encrypt_bytes(&mut body, key);
    out.extend(body);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_finds_and_round_trips() {
        let names = [Some("(listfile)"), Some("a\\b.txt"), None, Some("c.txt")];
        let table = HetTable::build(&names);
        let read = HetTable::from_body(&table.to_body().unwrap()).unwrap();
        assert_eq!(read, table);
        for (index, name) in names.iter().enumerate() {
            if let Some(name) = name {
                let (_, candidates) = read.candidates(name);
                assert!(candidates.contains(&(index as u32)), "{name}");
            }
        }
        let stored = write_ext_table(HET_MAGIC, &table.to_body().unwrap(), 0x1234).unwrap();
        let len = stored.len() as u64;
        let body = read_ext_table(&mut Cursor::new(stored), 0, len, HET_MAGIC, 0x1234).unwrap();
        assert_eq!(HetTable::from_body(&body).unwrap(), table);
    }
}
