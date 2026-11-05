//! ADT files: one 533⅓-yard tile of a map, with its terrain in 16 by 16 chunks, its textures,
//! the models and WMOs placed on it, and its liquids. This reads and writes the single-file
//! ADTs used up to WotLK (version 18), as 3.3.5a stores them: a file read and written back is
//! the same bytes.
//!
//! See <https://wowdev.wiki/ADT/v18>.

use std::io::Cursor;

use custom_debug::Debug;
use wow_alchemy_data::error::{Result, WowDataError};
use wow_alchemy_data::prelude::*;
use wow_alchemy_data::types::{C3Vector, WowStructR, WowStructW};
use wow_alchemy_data::utils::{magic_to_inverted_string, string_to_inverted_magic};
use wow_alchemy_data_derive::{WowHeaderR, WowHeaderW};
use wow_alchemy_utils::debug;

mod alpha_map;
mod mcnk;
mod mh2o;

pub use alpha_map::{ALPHA_MAP_BYTES, ALPHA_MAP_SIZE, AlphaMapFormat, decode_alpha_map};
pub use mcnk::*;
pub use mh2o::*;
pub use wow_alchemy_data::world::MapObjectPlacement;

pub const MVER: MagicStr = string_to_inverted_magic("MVER");
pub const MHDR: MagicStr = string_to_inverted_magic("MHDR");
pub const MCIN: MagicStr = string_to_inverted_magic("MCIN");
pub const MTEX: MagicStr = string_to_inverted_magic("MTEX");
pub const MMDX: MagicStr = string_to_inverted_magic("MMDX");
pub const MMID: MagicStr = string_to_inverted_magic("MMID");
pub const MWMO: MagicStr = string_to_inverted_magic("MWMO");
pub const MWID: MagicStr = string_to_inverted_magic("MWID");
pub const MDDF: MagicStr = string_to_inverted_magic("MDDF");
pub const MODF: MagicStr = string_to_inverted_magic("MODF");
pub const MH2O: MagicStr = string_to_inverted_magic("MH2O");
pub const MCNK: MagicStr = string_to_inverted_magic("MCNK");
pub const MFBO: MagicStr = string_to_inverted_magic("MFBO");
pub const MTXF: MagicStr = string_to_inverted_magic("MTXF");

/// Map chunks along each side of a tile.
pub const CHUNKS_PER_SIDE: usize = 16;
pub const CHUNKS_PER_TILE: usize = CHUNKS_PER_SIDE * CHUNKS_PER_SIDE;

bitflags::bitflags! {
    /// MHDR flags.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
    #[wow_data(bitflags = u32)]
    pub struct MhdrFlags: u32 {
        /// The tile has MFBO.
        const MFBO = 0x1;
        /// Set on some Northrend tiles.
        const NORTHREND = 0x2;
    }
}

/// The MHDR chunk. Offsets count from the start of its data and are 0 for chunks the file
/// doesn't have; they are worked out again on writing.
#[derive(Debug, Clone, Default, PartialEq, WowHeaderR, WowHeaderW)]
pub struct Mhdr {
    pub flags: MhdrFlags,
    pub mcin_offset: u32,
    pub mtex_offset: u32,
    pub mmdx_offset: u32,
    pub mmid_offset: u32,
    pub mwmo_offset: u32,
    pub mwid_offset: u32,
    pub mddf_offset: u32,
    pub modf_offset: u32,
    pub mfbo_offset: u32,
    pub mh2o_offset: u32,
    pub mtxf_offset: u32,
    /// Texture size amplifier, from Cataclysm on.
    pub mamp_value: u8,
    pub padding: [u8; 3],
    pub unused: [u32; 3],
}

/// One entry of MCIN, where each MCNK is. Worked out again on writing.
#[derive(Debug, Clone, Default, PartialEq, WowHeaderR, WowHeaderW)]
pub struct McinEntry {
    /// From the start of the file.
    pub offset: u32,
    /// Including the MCNK's 8-byte header.
    pub size: u32,
    /// Set by the client at runtime; 0 in files.
    pub flags: u32,
    pub async_id: u32,
}

/// Where an M2 stands: an entry of MDDF. Positions run east, up and south from the map's
/// north-west corner, and rotations are degrees.
#[derive(Debug, Clone, Default, PartialEq, WowHeaderR, WowHeaderW)]
pub struct DoodadPlacement {
    /// Index into MMID.
    pub name_id: u32,
    /// Shared by every tile that lists this placement.
    pub unique_id: u32,
    pub position: C3Vector,
    pub rotation: C3Vector,
    /// 1024 is 1.0.
    pub scale: u16,
    pub flags: u16,
}

impl DoodadPlacement {
    pub fn scale_factor(&self) -> f32 {
        self.scale as f32 / 1024.0
    }
}

/// MFBO: the heights a flying player is kept between, on a 3 by 3 grid over the tile.
#[derive(Debug, Clone, Default, PartialEq, WowHeaderR, WowHeaderW)]
pub struct Mfbo {
    pub max: [i16; 9],
    pub min: [i16; 9],
}

/// An ADT file. Chunks it doesn't know are kept as they were, after the known ones.
#[derive(Debug, Clone, PartialEq)]
pub struct Adt {
    /// 18 in every version.
    pub version: u32,
    pub header: Mhdr,
    /// MTEX: the texture file names that layers index.
    pub textures: StringBlock,
    /// MMDX: M2 file names.
    pub models: StringBlock,
    /// MMID: where each M2 name starts in `models`.
    #[debug(with = debug::trimmed_collection_fmt)]
    pub model_offsets: Vec<u32>,
    /// MWMO: WMO file names.
    pub wmos: StringBlock,
    /// MWID: where each WMO name starts in `wmos`.
    #[debug(with = debug::trimmed_collection_fmt)]
    pub wmo_offsets: Vec<u32>,
    /// MDDF.
    #[debug(with = debug::trimmed_collection_fmt)]
    pub doodads: Vec<DoodadPlacement>,
    /// MODF.
    #[debug(with = debug::trimmed_collection_fmt)]
    pub wmo_placements: Vec<MapObjectPlacement>,
    /// MH2O, from WotLK on.
    pub liquids: Option<Mh2o>,
    /// The 256 MCNKs, in the file's order: row by row.
    #[debug(with = debug::trimmed_collection_fmt)]
    pub chunks: Vec<McnkChunk>,
    pub flight_bounds: Option<Mfbo>,
    /// MTXF: flags for each texture of MTEX.
    pub texture_flags: Option<Vec<u32>>,
    #[debug(with = debug::trimmed_collection_fmt)]
    pub unknown: Vec<(MagicStr, Vec<u8>)>,
}

impl Default for Adt {
    fn default() -> Self {
        Self {
            version: 18,
            header: Mhdr::default(),
            textures: StringBlock::default(),
            models: StringBlock::default(),
            model_offsets: Vec::new(),
            wmos: StringBlock::default(),
            wmo_offsets: Vec::new(),
            doodads: Vec::new(),
            wmo_placements: Vec::new(),
            liquids: None,
            chunks: Vec::new(),
            flight_bounds: None,
            texture_flags: None,
            unknown: Vec::new(),
        }
    }
}

/// The index in `names.strings()` of the name that `offsets[slot]` points at.
fn name_index(names: &StringBlock, offsets: &[u32], slot: u32) -> Option<usize> {
    let offset = *offsets.get(slot as usize)?;
    names
        .strings()
        .iter()
        .position(|(start, _)| *start == offset)
}

fn name_at(names: &StringBlock, offsets: &[u32], slot: u32) -> Option<String> {
    names.get(*offsets.get(slot as usize)?)
}

impl Adt {
    /// The chunk at `x`, `y` of the 16 by 16, by its stored indices.
    pub fn chunk(&self, x: u32, y: u32) -> Option<&McnkChunk> {
        self.chunks
            .iter()
            .find(|c| c.header.index_x == x && c.header.index_y == y)
    }

    /// The M2 file name a doodad placement's `name_id` names.
    pub fn model_name(&self, name_id: u32) -> Option<String> {
        name_at(&self.models, &self.model_offsets, name_id)
    }

    /// Where the M2 a doodad placement's `name_id` names is in `models.names()`.
    pub fn model_index(&self, name_id: u32) -> Option<usize> {
        name_index(&self.models, &self.model_offsets, name_id)
    }

    /// The WMO file name a WMO placement's `name_id` names.
    pub fn wmo_name(&self, name_id: u32) -> Option<String> {
        name_at(&self.wmos, &self.wmo_offsets, name_id)
    }

    /// Where the WMO a WMO placement's `name_id` names is in `wmos.names()`.
    pub fn wmo_index(&self, name_id: u32) -> Option<usize> {
        name_index(&self.wmos, &self.wmo_offsets, name_id)
    }

    /// Every MH2O liquid layer, on each chunk's 9x9 grid. None for tiles without MH2O.
    pub fn liquid_layers(&self) -> Result<Vec<LiquidLayer>> {
        match &self.liquids {
            Some(liquids) => liquids.liquid_layers(),
            None => Ok(Vec::new()),
        }
    }
}

impl WowStructR for Adt {
    fn wow_read<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let (mut version, mut header) = (None, None);
        let mut adt = Self::default();

        read_chunks(reader, |reader, chunk| {
            match chunk.magic {
                MVER => version = Some(reader.wow_read()?),
                MHDR => header = Some(reader.wow_read()?),
                // Where the MCNKs are, which reading them in order already tells.
                MCIN => {}
                MTEX => adt.textures = StringBlock::read(reader, chunk)?,
                MMDX => adt.models = StringBlock::read(reader, chunk)?,
                MMID => adt.model_offsets = reader.wow_read_from_chunk(chunk)?,
                MWMO => adt.wmos = StringBlock::read(reader, chunk)?,
                MWID => adt.wmo_offsets = reader.wow_read_from_chunk(chunk)?,
                MDDF => adt.doodads = reader.wow_read_from_chunk(chunk)?,
                MODF => adt.wmo_placements = reader.wow_read_from_chunk(chunk)?,
                MH2O => {
                    adt.liquids = Some(Mh2o {
                        data: chunk.read_body(reader)?,
                    })
                }
                MCNK => adt.chunks.push(McnkChunk::read(&chunk.read_body(reader)?)?),
                MFBO => adt.flight_bounds = Some(reader.wow_read()?),
                MTXF => adt.texture_flags = Some(reader.wow_read_from_chunk(chunk)?),
                _ => adt.unknown.push((chunk.magic, chunk.read_body(reader)?)),
            }
            Ok(())
        })?;

        let missing =
            |magic: MagicStr| WowDataError::MissingChunk(magic_to_inverted_string(&magic));
        adt.version = version.ok_or_else(|| missing(MVER))?;
        adt.header = header.ok_or_else(|| missing(MHDR))?;
        Ok(adt)
    }
}

impl WowStructW for Adt {
    fn wow_write<W: Write + Seek>(&self, writer: &mut W) -> Result<()> {
        let mut out = Cursor::new(Vec::new());
        write_chunk(&mut out, MVER, &self.version)?;
        let header_at = out.position();
        write_chunk(&mut out, MHDR, &self.header)?;
        let offsets_from = header_at + 8;

        let mut header = self.header.clone();
        let at = |out: &Cursor<Vec<u8>>| (out.position() - offsets_from) as u32;

        header.mcin_offset = at(&out);
        let mcin_at = out.position();
        let mut entries = vec![McinEntry::default(); self.chunks.len()];
        write_chunk(&mut out, MCIN, entries.as_slice())?;

        header.mtex_offset = at(&out);
        write_chunk(&mut out, MTEX, &self.textures)?;
        header.mmdx_offset = at(&out);
        write_chunk(&mut out, MMDX, &self.models)?;
        header.mmid_offset = at(&out);
        write_chunk(&mut out, MMID, self.model_offsets.as_slice())?;
        header.mwmo_offset = at(&out);
        write_chunk(&mut out, MWMO, &self.wmos)?;
        header.mwid_offset = at(&out);
        write_chunk(&mut out, MWID, self.wmo_offsets.as_slice())?;
        header.mddf_offset = at(&out);
        write_chunk(&mut out, MDDF, self.doodads.as_slice())?;
        header.modf_offset = at(&out);
        write_chunk(&mut out, MODF, self.wmo_placements.as_slice())?;

        header.mh2o_offset = 0;
        if let Some(liquids) = &self.liquids {
            header.mh2o_offset = at(&out);
            write_chunk(&mut out, MH2O, liquids.data.as_slice())?;
        }

        for (entry, chunk) in entries.iter_mut().zip(&self.chunks) {
            entry.offset = out.position() as u32;
            write_chunk(&mut out, MCNK, chunk)?;
            entry.size = out.position() as u32 - entry.offset;
        }

        header.mfbo_offset = 0;
        if let Some(bounds) = &self.flight_bounds {
            header.mfbo_offset = at(&out);
            write_chunk(&mut out, MFBO, bounds)?;
        }
        header.mtxf_offset = 0;
        if let Some(flags) = &self.texture_flags {
            header.mtxf_offset = at(&out);
            write_chunk(&mut out, MTXF, flags.as_slice())?;
        }
        for (magic, body) in &self.unknown {
            write_chunk(&mut out, *magic, body.as_slice())?;
        }

        out.set_position(header_at);
        write_chunk(&mut out, MHDR, &header)?;
        out.set_position(mcin_at);
        write_chunk(&mut out, MCIN, entries.as_slice())?;

        writer.write_all(out.get_ref())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Adt {
        let mut adt = Adt::default();
        adt.header.flags = MhdrFlags::MFBO;
        adt.textures = StringBlock::from_strings(&["a.blp", "b.blp"]).0;
        let (models, offsets) = StringBlock::from_strings(&["x.m2", "yy.m2"]);
        adt.models = models;
        adt.model_offsets = vec![offsets[1], offsets[0]];
        let (wmos, offsets) = StringBlock::from_strings(&["w.wmo"]);
        adt.wmos = wmos;
        adt.wmo_offsets = offsets;
        adt.doodads = vec![DoodadPlacement {
            name_id: 0,
            unique_id: 7,
            scale: 2048,
            ..Default::default()
        }];
        adt.wmo_placements = vec![MapObjectPlacement {
            doodad_set: 1,
            ..Default::default()
        }];
        adt.liquids = Some(Mh2o {
            data: vec![0; 256 * 12],
        });
        for y in 0..16 {
            for x in 0..16 {
                let mut chunk = McnkChunk::default();
                chunk.header.index_x = x;
                chunk.header.index_y = y;
                chunk.alpha_stated_size = 0;
                adt.chunks.push(chunk);
            }
        }
        adt.flight_bounds = Some(Mfbo::default());
        adt.texture_flags = Some(vec![0, 1]);
        adt.unknown.push((*b"XXXX", vec![1, 2, 3]));
        adt
    }

    fn bytes(adt: &Adt) -> Vec<u8> {
        let mut out = Cursor::new(Vec::new());
        adt.wow_write(&mut out).unwrap();
        out.into_inner()
    }

    fn u32_at(bytes: &[u8], at: usize) -> u32 {
        u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
    }

    #[test]
    fn writes_and_reads_back_the_same_bytes() {
        let first = bytes(&sample());
        let read = Adt::wow_read(&mut Cursor::new(&first)).unwrap();
        assert_eq!(read.chunks.len(), CHUNKS_PER_TILE);
        assert_eq!(bytes(&read), first);
        assert_eq!(read.textures.names(), ["a.blp", "b.blp"]);
        assert_eq!(read.texture_flags, Some(vec![0, 1]));
        assert_eq!(read.unknown, vec![(*b"XXXX", vec![1, 2, 3])]);
        assert_eq!(read.chunk(3, 2).unwrap().header.index_y, 2);
    }

    #[test]
    fn mhdr_and_mcin_point_at_their_chunks() {
        let data = bytes(&sample());
        assert_eq!(&data[0..4], b"REVM");
        assert_eq!(&data[12..16], b"RDHM");
        let read = Adt::wow_read(&mut Cursor::new(&data)).unwrap();
        let base = 20;
        let at = |offset: u32| &data[base + offset as usize..base + offset as usize + 4];
        assert_eq!(at(read.header.mcin_offset), b"NICM");
        assert_eq!(at(read.header.mtex_offset), b"XETM");
        assert_eq!(at(read.header.modf_offset), b"FDOM");
        assert_eq!(at(read.header.mh2o_offset), b"O2HM");
        assert_eq!(at(read.header.mfbo_offset), b"OBFM");
        assert_eq!(at(read.header.mtxf_offset), b"FXTM");

        let mcin = base + read.header.mcin_offset as usize + 8;
        for i in 0..CHUNKS_PER_TILE {
            let offset = u32_at(&data, mcin + i * 16) as usize;
            let size = u32_at(&data, mcin + i * 16 + 4) as usize;
            assert_eq!(&data[offset..offset + 4], b"KNCM");
            assert_eq!(u32_at(&data, offset + 4) as usize + 8, size);
        }
    }

    #[test]
    fn missing_optional_chunks_have_zero_offsets() {
        let mut adt = sample();
        adt.liquids = None;
        adt.flight_bounds = None;
        adt.texture_flags = None;
        let read = Adt::wow_read(&mut Cursor::new(bytes(&adt))).unwrap();
        assert_eq!(read.header.mh2o_offset, 0);
        assert_eq!(read.header.mfbo_offset, 0);
        assert_eq!(read.header.mtxf_offset, 0);
        assert_eq!(read.liquids, None);
    }

    #[test]
    fn placements_resolve_their_names_through_the_offsets() {
        let adt = sample();
        assert_eq!(adt.model_name(0).as_deref(), Some("yy.m2"));
        assert_eq!(adt.model_index(0), Some(1));
        assert_eq!(adt.model_index(1), Some(0));
        assert_eq!(adt.model_index(2), None);
        assert_eq!(adt.wmo_name(0).as_deref(), Some("w.wmo"));
        assert_eq!(adt.wmo_index(0), Some(0));
        assert_eq!(adt.doodads[0].scale_factor(), 2.0);
    }

    #[test]
    fn empty_and_headerless_files_are_errors() {
        assert!(Adt::wow_read(&mut Cursor::new(Vec::<u8>::new())).is_err());
        let mut data = Cursor::new(Vec::new());
        write_chunk(&mut data, MVER, &18u32).unwrap();
        let error = Adt::wow_read(&mut Cursor::new(data.into_inner())).unwrap_err();
        assert!(error.to_string().contains("MHDR"), "{error}");
    }
}
