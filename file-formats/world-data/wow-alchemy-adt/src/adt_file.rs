use std::collections::HashMap;
use std::io::{Cursor, SeekFrom};

use custom_debug::Debug;
use wow_alchemy_data::error::Result as WDResult;
use wow_alchemy_data::prelude::*;
use wow_alchemy_data::types::{ChunkHeader, MagicStr, WowStructR};
use wow_alchemy_data::utils::{magic_to_inverted_string, string_to_inverted_magic};

use crate::chunks::mcin::{CHUNK_COUNT, MCIN, Mcin};
use crate::chunks::mcnk::MCNK;
use crate::chunks::mhdr::{MHDR, Mhdr};
use crate::{AdtError, Result};

pub const MVER: MagicStr = string_to_inverted_magic("MVER");

#[derive(Debug, Clone)]
pub enum AdtChunk {
    Unknown(Vec<u8>),
}

#[derive(Debug, Clone, Default)]
pub struct AdtRootFile {
    pub header: ChunkHeader,
    pub version: u32,
    pub mhdr: Mhdr,
    pub mcin: Option<Mcin>,
    pub chunk_index: HashMap<MagicStr, usize>,
    pub chunks: Vec<AdtChunk>,
}

impl WowStructR for AdtRootFile {
    fn wow_read<R: Read + Seek>(reader: &mut R) -> WDResult<Self> {
        let mut chunks = Vec::new();
        let mut chunk_index = HashMap::new();

        let first_chunk = ChunkHeader::wow_read(reader)?;
        if first_chunk.magic != MVER {
            return Err(AdtError::InvalidMagic {
                expected: magic_to_inverted_string(&MVER),
                found: magic_to_inverted_string(&first_chunk.magic),
            }
            .into());
        }

        let version = u32::wow_read(reader)?;
        if version != 18 {
            return Err(AdtError::InvalidVersion(version).into());
        }

        let mhdr_chunk = ChunkHeader::wow_read(reader)?;
        if mhdr_chunk.magic != MHDR {
            return Err(AdtError::InvalidMagic {
                expected: magic_to_inverted_string(&MHDR),
                found: magic_to_inverted_string(&mhdr_chunk.magic),
            }
            .into());
        }
        let mhdr = Mhdr::wow_read(reader)?;

        let mut mcin = None;

        loop {
            let Ok(chunk_header) = ChunkHeader::wow_read(reader) else {
                break;
            };

            let (chunk_magic, chunk_data): (&MagicStr, AdtChunk) = match chunk_header.magic {
                MCIN => {
                    mcin = Some(reader.wow_read_from_chunk_single(&chunk_header)?);
                    continue;
                }
                MCNK => {
                    reader.seek_relative(chunk_header.bytes as i64)?;
                    continue;
                }
                _ => (
                    &chunk_header.magic,
                    AdtChunk::Unknown(reader.wow_read_from_chunk(&chunk_header)?),
                ),
            };
            chunks.push(chunk_data);
            chunk_index.insert(*chunk_magic, chunks.len() - 1);
        }

        Ok(Self {
            header: first_chunk,
            version,
            mhdr,
            mcin,
            chunk_index,
            chunks,
        })
    }
}

impl AdtRootFile {
    pub fn has_map_chunks(&self) -> bool {
        self.mcin.is_some()
    }

    pub fn read_map_chunk<R: Read + Seek>(&self, reader: &mut R, index: u32) -> Result<()> {
        if index as usize >= CHUNK_COUNT {
            return Err(AdtError::MapChunkOutOfRange);
        }

        let Some(mcin) = &self.mcin else {
            return Err(AdtError::ExpectedChunkNotFound(magic_to_inverted_string(
                &MCIN,
            )));
        };

        let chunk_info = &mcin.mcnks[index as usize];

        let mut chunk_data = vec![0_u8; chunk_info.size as usize];
        reader.seek(SeekFrom::Start(chunk_info.offset as u64))?;
        reader.read_exact(&mut chunk_data)?;

        let mut chunk_reader = Cursor::new(chunk_data);

        Ok(())
    }
}
