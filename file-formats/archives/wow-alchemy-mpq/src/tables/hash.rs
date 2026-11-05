//! The classic hash table: file names hashed to block table indices.

use std::io::Cursor;

use wow_alchemy_data::prelude::*;
use wow_alchemy_data_derive::{WowHeaderR, WowHeaderW};

use crate::crypto::{hash_string, hash_type};
use crate::{Error, Result};

/// An entry of the hash table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, WowHeaderR, WowHeaderW)]
pub struct HashEntry {
    /// [`hash_type::NAME_A`] hash of the name.
    pub name_1: u32,
    /// [`hash_type::NAME_B`] hash of the name.
    pub name_2: u32,
    /// Windows LANGID of the file, 0 for neutral.
    pub locale: u16,
    /// Platform, always 0.
    pub platform: u16,
    /// Index into the block table, or one of the `EMPTY_*` values.
    pub block_index: u32,
}

impl HashEntry {
    /// Size on disk.
    pub const SIZE: usize = 16;
    /// The entry has never been used; lookups stop here.
    pub const EMPTY_NEVER_USED: u32 = 0xFFFF_FFFF;
    /// The entry's file was deleted; lookups go on past it.
    pub const EMPTY_DELETED: u32 = 0xFFFF_FFFE;

    /// An entry that has never been used.
    pub fn empty() -> Self {
        Self {
            name_1: 0xFFFF_FFFF,
            name_2: 0xFFFF_FFFF,
            locale: 0xFFFF,
            platform: 0xFFFF,
            block_index: Self::EMPTY_NEVER_USED,
        }
    }

    /// Whether the entry has never been used.
    pub fn is_empty(&self) -> bool {
        self.block_index == Self::EMPTY_NEVER_USED
    }

    /// Whether the entry points at a block.
    pub fn is_valid(&self) -> bool {
        self.block_index < Self::EMPTY_DELETED
    }
}

/// The hash table, decrypted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashTable {
    /// The entries; their count is a power of two.
    pub entries: Vec<HashEntry>,
}

impl HashTable {
    /// A table of `size` empty entries, rounded up to a power of two.
    pub fn new(size: usize) -> Self {
        Self {
            entries: vec![HashEntry::empty(); size.max(1).next_power_of_two()],
        }
    }

    /// Reads the entries from decrypted bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(data);
        let mut entries = Vec::with_capacity(data.len() / HashEntry::SIZE);
        for _ in 0..data.len() / HashEntry::SIZE {
            entries.push(cursor.wow_read()?);
        }
        Ok(Self { entries })
    }

    /// The entries as bytes, before encryption.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut bytes = Vec::with_capacity(self.entries.len() * HashEntry::SIZE);
        self.entries.as_slice().wow_write(&mut bytes)?;
        Ok(bytes)
    }

    /// Finds the entry of `name`, probing from its hashed index until a never-used entry.
    /// With several locales of a file, the first entry found wins.
    pub fn find(&self, name: &str) -> Option<(usize, &HashEntry)> {
        if self.entries.is_empty() {
            return None;
        }
        let name_a = hash_string(name, hash_type::NAME_A);
        let name_b = hash_string(name, hash_type::NAME_B);
        let mask = self.entries.len() - 1;
        let start = hash_string(name, hash_type::TABLE_OFFSET) as usize & mask;
        let mut index = start;
        loop {
            let entry = &self.entries[index];
            if entry.is_empty() {
                return None;
            }
            if entry.name_1 == name_a && entry.name_2 == name_b && entry.is_valid() {
                return Some((index, entry));
            }
            index = (index + 1) & mask;
            if index == start {
                return None;
            }
        }
    }

    /// Puts `name` in the first free entry from its hashed index.
    pub fn insert(&mut self, name: &str, locale: u16, block_index: u32) -> Result<()> {
        let mask = self.entries.len() - 1;
        let start = hash_string(name, hash_type::TABLE_OFFSET) as usize & mask;
        for step in 0..self.entries.len() {
            let entry = &mut self.entries[(start + step) & mask];
            if !entry.is_valid() {
                *entry = HashEntry {
                    name_1: hash_string(name, hash_type::NAME_A),
                    name_2: hash_string(name, hash_type::NAME_B),
                    locale,
                    platform: 0,
                    block_index,
                };
                return Ok(());
            }
        }
        Err(Error::invalid_format("Hash table is full"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_round_trip_and_are_found() {
        let mut table = HashTable::new(5);
        assert_eq!(table.entries.len(), 8);
        table.insert("a.txt", 0, 0).unwrap();
        table.insert("DIR\\B.TXT", 0, 1).unwrap();
        let read = HashTable::from_bytes(&table.to_bytes().unwrap()).unwrap();
        assert_eq!(read, table);
        assert_eq!(read.find("A.TXT").unwrap().1.block_index, 0);
        assert_eq!(read.find("dir/b.txt").unwrap().1.block_index, 1);
        assert!(read.find("c.txt").is_none());
    }
}
