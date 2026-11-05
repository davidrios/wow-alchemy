//! WMO (world map object) files of WotLK 3.3.5a, version 17: a root file, which names the
//! materials, groups, portals, lights and doodads, and one file per group, `_NNN.wmo`, with the
//! group's geometry, BSP tree and liquid.
//!
//! [`WmoRoot`] and [`WmoGroup`] read every chunk of these files into records and write them back
//! as they were. Layouts follow the wowdev wiki's WMO page for version 17.

pub mod group;
pub mod liquid;
pub mod root;

use wow_alchemy_data::error::{Result, WowDataError};
use wow_alchemy_data::prelude::MagicStr;
use wow_alchemy_data::utils::{magic_to_inverted_string, string_to_inverted_magic};

pub use group::{
    BspNode, GroupFlags, GroupHeader, TriangleFlags, TriangleInfo, VertexColor, WmoBatch, WmoGroup,
};
pub use liquid::{LiquidHeader, LiquidVertex, WmoLiquid};
pub use root::{
    DoodadNames, Fog, FogParams, GroupInfo, LightType, MaterialFlags, Plane, PortalRef, RootFlags,
    RootHeader, VisibleBlock, WmoDoodad, WmoDoodadSet, WmoLight, WmoMaterial, WmoPortal, WmoRoot,
};

/// The only version this crate reads, as MVER holds it.
pub const WMO_VERSION: u32 = 17;

pub const MVER: MagicStr = string_to_inverted_magic("MVER");

// Root file chunks.
pub const MOHD: MagicStr = string_to_inverted_magic("MOHD");
pub const MOTX: MagicStr = string_to_inverted_magic("MOTX");
pub const MOMT: MagicStr = string_to_inverted_magic("MOMT");
pub const MOGN: MagicStr = string_to_inverted_magic("MOGN");
pub const MOGI: MagicStr = string_to_inverted_magic("MOGI");
pub const MOSB: MagicStr = string_to_inverted_magic("MOSB");
pub const MOPV: MagicStr = string_to_inverted_magic("MOPV");
pub const MOPT: MagicStr = string_to_inverted_magic("MOPT");
pub const MOPR: MagicStr = string_to_inverted_magic("MOPR");
pub const MOVV: MagicStr = string_to_inverted_magic("MOVV");
pub const MOVB: MagicStr = string_to_inverted_magic("MOVB");
pub const MOLT: MagicStr = string_to_inverted_magic("MOLT");
pub const MODS: MagicStr = string_to_inverted_magic("MODS");
pub const MODN: MagicStr = string_to_inverted_magic("MODN");
pub const MODD: MagicStr = string_to_inverted_magic("MODD");
pub const MFOG: MagicStr = string_to_inverted_magic("MFOG");
pub const MCVP: MagicStr = string_to_inverted_magic("MCVP");

// Group file chunks; all but MVER and MOGP are inside MOGP, after its header.
pub const MOGP: MagicStr = string_to_inverted_magic("MOGP");
pub const MOPY: MagicStr = string_to_inverted_magic("MOPY");
pub const MOVI: MagicStr = string_to_inverted_magic("MOVI");
pub const MOVT: MagicStr = string_to_inverted_magic("MOVT");
pub const MONR: MagicStr = string_to_inverted_magic("MONR");
pub const MOTV: MagicStr = string_to_inverted_magic("MOTV");
pub const MOBA: MagicStr = string_to_inverted_magic("MOBA");
pub const MOLR: MagicStr = string_to_inverted_magic("MOLR");
pub const MODR: MagicStr = string_to_inverted_magic("MODR");
pub const MOBN: MagicStr = string_to_inverted_magic("MOBN");
pub const MOBR: MagicStr = string_to_inverted_magic("MOBR");
pub const MOCV: MagicStr = string_to_inverted_magic("MOCV");
pub const MLIQ: MagicStr = string_to_inverted_magic("MLIQ");

fn missing(magic: MagicStr) -> WowDataError {
    WowDataError::MissingChunk(magic_to_inverted_string(&magic))
}

fn check_version(version: Option<u32>) -> Result<u32> {
    match version {
        Some(WMO_VERSION) => Ok(WMO_VERSION),
        Some(other) => Err(WowDataError::InvalidData(format!(
            "WMO version {other}; only {WMO_VERSION} is supported"
        ))),
        None => Err(missing(MVER)),
    }
}

/// Reads a chunk of whole records, and fails rather than drop bytes left over at its end, so a
/// file read in is always written back the same.
fn read_exact_records<T, R>(
    reader: &mut R,
    chunk: &wow_alchemy_data::prelude::ChunkHeader,
    record_size: usize,
) -> Result<Vec<T>>
where
    T: wow_alchemy_data::prelude::WowHeaderR + wow_alchemy_data::prelude::WowHeaderW,
    R: std::io::Read + std::io::Seek,
{
    use wow_alchemy_data::prelude::WowReaderForChunk;
    if chunk.bytes as usize % record_size != 0 {
        return Err(WowDataError::InvalidData(format!(
            "{} has {} bytes, not a whole number of {record_size}-byte records",
            magic_to_inverted_string(&chunk.magic),
            chunk.bytes
        )));
    }
    reader.wow_read_from_chunk(chunk)
}
