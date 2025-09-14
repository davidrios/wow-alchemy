use custom_debug::Debug;
use wow_alchemy_data::error::Result as WDResult;
use wow_alchemy_data::types::MagicStr;
use wow_alchemy_data::{prelude::*, utils::string_to_inverted_magic};
use wow_alchemy_data_derive::{WowHeaderR, WowHeaderW};

pub const MCIN: MagicStr = string_to_inverted_magic("MCIN");

#[derive(Debug, Clone, Default, WowHeaderR, WowHeaderW)]
pub struct ChunkInfo {
    pub offset: u32,
    pub size: u32,
    pub flags: u32,
    pub async_id: u32,
}

#[derive(Debug, Clone)]
pub struct Mcin {
    pub mcnks: Vec<ChunkInfo>,
}

pub const CHUNK_COUNT: usize = 16 * 16;

impl WowHeaderR for Mcin {
    fn wow_read<R: Read + Seek>(reader: &mut R) -> WDResult<Self> {
        let mut mcnks = Vec::with_capacity(CHUNK_COUNT);
        for _ in 0..CHUNK_COUNT {
            mcnks.push(reader.wow_read()?);
        }
        Ok(Self { mcnks })
    }
}

impl WowHeaderW for Mcin {
    fn wow_write<W: Write>(&self, writer: &mut W) -> WDResult<()> {
        for i in 0..CHUNK_COUNT {
            writer.wow_write(&self.mcnks[i])?;
        }
        Ok(())
    }

    fn wow_size(&self) -> usize {
        CHUNK_COUNT
            * if let Some(item) = self.mcnks.first() {
                item.wow_size()
            } else {
                ChunkInfo::default().wow_size()
            }
    }
}
