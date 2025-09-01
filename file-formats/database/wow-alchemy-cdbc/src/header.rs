use wow_alchemy_data::error::Result as WDResult;
use wow_alchemy_data::prelude::*;
use wow_alchemy_data::types::MagicStr;
use wow_alchemy_data_derive::WowHeaderW;

use crate::Error;
use std::io::SeekFrom;

pub const DBC_MAGIC: [u8; 4] = *b"WDBC";

#[derive(Debug, Clone, Copy, PartialEq, Eq, WowHeaderW)]
pub struct DbcHeader {
    pub magic: MagicStr,
    pub record_count: u32,
    pub field_count: u32,
    pub record_size: u32,
    pub string_block_size: u32,
}

impl WowHeaderR for DbcHeader {
    fn wow_read<R: Read + Seek>(reader: &mut R) -> WDResult<Self> {
        reader.seek(SeekFrom::Start(0))?;
        let magic: MagicStr = reader.wow_read()?;
        if magic != DBC_MAGIC {
            return Err(Error::InvalidHeader(format!(
                "Invalid magic signature: {magic:?}, expected: {DBC_MAGIC:?}"
            ))
            .into());
        }

        let inst = Self {
            magic,
            record_count: reader.wow_read()?,
            field_count: reader.wow_read()?,
            record_size: reader.wow_read()?,
            string_block_size: reader.wow_read()?,
        };

        if inst.record_size == 0 && inst.record_count > 0 {
            return Err(Error::InvalidHeader(
                "Record size cannot be 0 if record count is greater than 0".to_string(),
            )
            .into());
        }

        if inst.field_count == 0 && inst.record_count > 0 {
            return Err(Error::InvalidHeader(
                "Field count cannot be 0 if record count is greater than 0".to_string(),
            )
            .into());
        }

        Ok(inst)
    }
}

impl DbcHeader {
    pub fn string_block_offset(&self) -> u64 {
        self.wow_size() as u64 + (self.record_count as u64 * self.record_size as u64)
    }

    pub fn total_size(&self) -> u64 {
        self.string_block_offset() + self.string_block_size as u64
    }
}
