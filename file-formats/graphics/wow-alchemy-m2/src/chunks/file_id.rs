use wow_alchemy_data::types::{ChunkHeader, MagicStr};
use wow_alchemy_data::{error::Result as WDResult, prelude::*};
use wow_alchemy_data_derive::{WowHeaderR, WowHeaderW};

use crate::{
    M2Error,
    header::{M2SkinProfilesHeader, MD20Header},
};

pub const AFID: MagicStr = *b"AFID";
pub const SFID: MagicStr = *b"SFID";

pub const BFID: MagicStr = *b"BFID";
pub const GPID: MagicStr = *b"GPID";
pub const PFID: MagicStr = *b"PFID";
pub const RPID: MagicStr = *b"RPID";
pub const SKID: MagicStr = *b"SKID";
pub const TXID: MagicStr = *b"TXID";

pub type FileId = u32;

#[derive(Debug, Clone, Default, WowHeaderR, WowHeaderW)]
pub struct AnimationFile {
    pub anim_id: u16,
    pub sub_anim_id: u16,
    pub file_id: u32,
}

#[derive(Debug, Clone, Default)]
pub struct SkinFiles {
    pub file_ids: Vec<u32>,
    pub lod_file_ids: Vec<u32>,
}

impl SkinFiles {
    pub fn wow_read_from_chunk<R: Read + Seek>(
        reader: &mut R,
        chunk_header: &ChunkHeader,
        md20_header: &MD20Header,
    ) -> WDResult<Self> {
        let M2SkinProfilesHeader::Later(nviews) = md20_header.skin_profiles else {
            return Err(
                M2Error::ParseError("Unexpected skin profiles in md20 header".into()).into(),
            );
        };

        // The skin profiles' files come first, then their LODs'.
        let mut file_ids: Vec<FileId> = reader.wow_read_from_chunk(chunk_header)?;
        let lod_file_ids = file_ids.split_off((nviews as usize).min(file_ids.len()));

        Ok(Self {
            file_ids,
            lod_file_ids,
        })
    }
}
