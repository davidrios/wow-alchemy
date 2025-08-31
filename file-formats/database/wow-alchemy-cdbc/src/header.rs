use crate::{Error, Result};
use std::io::{Read, Seek, SeekFrom};

pub const DBC_MAGIC: [u8; 4] = *b"WDBC";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DbcHeader {
    pub magic: [u8; 4],
    pub record_count: u32,
    pub field_count: u32,
    pub record_size: u32,
    pub string_block_size: u32,
}

impl DbcHeader {
    pub const SIZE: usize = 20;

    pub fn parse<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        reader.seek(SeekFrom::Start(0))?;

        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;

        if magic != DBC_MAGIC {
            return Err(Error::InvalidHeader(format!(
                "Invalid magic signature: {magic:?}, expected: {DBC_MAGIC:?}"
            )));
        }

        let mut buf = [0u8; 4];

        reader.read_exact(&mut buf)?;
        let record_count = u32::from_le_bytes(buf);

        reader.read_exact(&mut buf)?;
        let field_count = u32::from_le_bytes(buf);

        reader.read_exact(&mut buf)?;
        let record_size = u32::from_le_bytes(buf);

        reader.read_exact(&mut buf)?;
        let string_block_size = u32::from_le_bytes(buf);

        // Perform basic validation
        if record_size == 0 && record_count > 0 {
            return Err(Error::InvalidHeader(
                "Record size cannot be 0 if record count is greater than 0".to_string(),
            ));
        }

        if field_count == 0 && record_count > 0 {
            return Err(Error::InvalidHeader(
                "Field count cannot be 0 if record count is greater than 0".to_string(),
            ));
        }

        Ok(Self {
            magic,
            record_count,
            field_count,
            record_size,
            string_block_size,
        })
    }

    pub fn string_block_offset(&self) -> u64 {
        DbcHeader::SIZE as u64 + (self.record_count as u64 * self.record_size as u64)
    }

    pub fn total_size(&self) -> u64 {
        self.string_block_offset() + self.string_block_size as u64
    }
}
