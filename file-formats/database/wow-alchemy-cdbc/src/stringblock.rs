use crate::{Error, Result, StringRef};
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct StringBlock {
    data: Vec<u8>,
}

impl StringBlock {
    pub fn parse<R: Read + Seek>(reader: &mut R, offset: u64, size: u32) -> Result<Self> {
        reader.seek(SeekFrom::Start(offset))?;

        let mut data = vec![0u8; size as usize];
        reader.read_exact(&mut data)?;

        Ok(Self { data })
    }

    pub fn get_string(&self, string_ref: StringRef) -> Result<&str> {
        let offset = string_ref.offset() as usize;
        if offset >= self.data.len() {
            return Err(Error::OutOfBounds(format!(
                "String reference offset out of bounds: {} (max: {})",
                offset,
                self.data.len()
            )));
        }

        // Find the end of the string (null terminator)
        let mut end = offset;
        while end < self.data.len() && self.data[end] != 0 {
            end += 1;
        }

        std::str::from_utf8(&self.data[offset..end])
            .map_err(|e| Error::TypeConversion(format!("Invalid UTF-8 string: {e}")))
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }
}

#[derive(Debug, Clone)]
pub struct CachedStringBlock {
    data: Arc<Vec<u8>>,
    cache: HashMap<u32, (usize, usize)>,
}

impl CachedStringBlock {
    pub fn from_string_block(string_block: &StringBlock) -> Self {
        let data = Arc::new(string_block.data().to_vec());
        let mut cache = HashMap::new();

        let mut offset = 0;
        while offset < data.len() {
            let start_offset = offset;

            // Find the end of the string (null terminator)
            while offset < data.len() && data[offset] != 0 {
                offset += 1;
            }

            // Cache the string position
            cache.insert(start_offset as u32, (start_offset, offset));

            // Skip the null terminator
            offset += 1;
        }

        Self { data, cache }
    }

    pub fn get_string(&self, string_ref: StringRef) -> Result<&str> {
        let offset = string_ref.offset() as usize;

        if let Some((start, end)) = self.cache.get(&string_ref.offset()) {
            // Convert the bytes to a string
            std::str::from_utf8(&self.data[*start..*end])
                .map_err(|e| Error::TypeConversion(format!("Invalid UTF-8 string: {e}")))
        } else {
            // If not cached, find the end of the string
            if offset >= self.data.len() {
                return Err(Error::OutOfBounds(format!(
                    "String reference offset out of bounds: {} (max: {})",
                    offset,
                    self.data.len()
                )));
            }

            let mut end = offset;
            while end < self.data.len() && self.data[end] != 0 {
                end += 1;
            }

            std::str::from_utf8(&self.data[offset..end])
                .map_err(|e| Error::TypeConversion(format!("Invalid UTF-8 string: {e}")))
        }
    }
}
