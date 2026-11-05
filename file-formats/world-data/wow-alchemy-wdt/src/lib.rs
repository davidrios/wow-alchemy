//! WDT files: which of a map's 64 by 64 tiles exist, the map's flags, and for maps made of one
//! WMO, that WMO and where it stands.

use custom_debug::Debug;
use wow_alchemy_data::error::{Result, WowDataError};
use wow_alchemy_data::prelude::*;
use wow_alchemy_data::types::{WowStructR, WowStructW};
use wow_alchemy_data::utils::{magic_to_inverted_string, string_to_inverted_magic};
use wow_alchemy_data::world::MapObjectPlacement;
use wow_alchemy_data_derive::{WowHeaderR, WowHeaderW};
use wow_alchemy_utils::debug;

pub const MVER: MagicStr = string_to_inverted_magic("MVER");
pub const MPHD: MagicStr = string_to_inverted_magic("MPHD");
pub const MAIN: MagicStr = string_to_inverted_magic("MAIN");
pub const MAID: MagicStr = string_to_inverted_magic("MAID");
pub const MWMO: MagicStr = string_to_inverted_magic("MWMO");
pub const MODF: MagicStr = string_to_inverted_magic("MODF");

/// Tiles along each side of a map.
pub const MAP_SIZE: usize = 64;
pub const TILE_COUNT: usize = MAP_SIZE * MAP_SIZE;

bitflags::bitflags! {
    /// MPHD flags.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
    #[wow_data(bitflags = u32)]
    pub struct MphdFlags: u32 {
        /// The map is one WMO, named in MWMO and placed by MODF, with no terrain.
        const WDT_USES_GLOBAL_MAP_OBJ = 0x0001;
        /// Map chunks have vertex colors, MCCV.
        const ADT_HAS_MCCV = 0x0002;
        /// Alpha maps have 8 bits per value rather than 4.
        const ADT_HAS_BIG_ALPHA = 0x0004;
        /// Doodad references are sorted by size category.
        const ADT_HAS_DOODADREFS_SORTED_BY_SIZE_CAT = 0x0008;
        /// Map chunks have light vertices, MCLV, from Cataclysm on.
        const ADT_HAS_LIGHTING_VERTICES = 0x0010;
        const ADT_HAS_UPSIDE_DOWN_GROUND = 0x0020;
        const UNK_FIRELANDS = 0x0040;
        /// Textures have height maps, _h.blp, from Mists of Pandaria on.
        const ADT_HAS_HEIGHT_TEXTURING = 0x0080;
        const UNK_LOAD_LOD = 0x0100;
        /// MAID holds the map's file ids, and MPHD's other fields are file ids, from 8.1 on.
        const WDT_HAS_MAID = 0x0200;
        const UNK_0x0400 = 0x0400;
        const UNK_0x0800 = 0x0800;
        const UNK_0x1000 = 0x1000;
        const UNK_0x2000 = 0x2000;
        const UNK_0x4000 = 0x4000;
        const UNK_0x8000 = 0x8000;
    }
}

/// The MPHD chunk.
#[derive(Debug, Clone, Default, PartialEq, WowHeaderR, WowHeaderW)]
pub struct Mphd {
    pub flags: MphdFlags,
    /// With [`MphdFlags::WDT_HAS_MAID`], this and `unused` are the file ids of the map's _lgt,
    /// _occ, _fogs, _mpv and _tex WDTs, its WDL and its _pd4 WDT.
    pub something: u32,
    pub unused: [u32; 6],
}

impl Mphd {
    pub fn is_wmo_only(&self) -> bool {
        self.flags.contains(MphdFlags::WDT_USES_GLOBAL_MAP_OBJ)
    }

    pub fn has_big_alpha(&self) -> bool {
        self.flags.contains(MphdFlags::ADT_HAS_BIG_ALPHA)
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
    #[wow_data(bitflags = u32)]
    pub struct MainFlags: u32 {
        /// The tile has an ADT.
        const HAS_ADT = 0x1;
        /// Set by the client once the tile is loaded.
        const LOADED = 0x2;
    }
}

/// One tile's entry in MAIN.
#[derive(Debug, Clone, Copy, Default, PartialEq, WowHeaderR, WowHeaderW)]
pub struct MainEntry {
    pub flags: MainFlags,
    /// Set by the client at runtime; 0 in files.
    pub async_id: u32,
}

impl MainEntry {
    pub fn has_adt(&self) -> bool {
        self.flags.contains(MainFlags::HAS_ADT)
    }
}

/// A WDT file. Chunks it doesn't know are kept as they were, after the known ones.
#[derive(Debug, Clone, PartialEq)]
pub struct WdtFile {
    /// 18 in every version.
    pub version: u32,
    pub header: Mphd,
    /// Tile `y * 64 + x`.
    #[debug(with = debug::trimmed_collection_fmt)]
    pub tiles: Vec<MainEntry>,
    /// From 8.1 on: 4096 file ids for each of the root ADTs, _obj0, _obj1, _tex0, _lod, map
    /// textures, their normals and minimap textures, as the file has them.
    #[debug(skip)]
    pub file_ids: Option<Vec<u32>>,
    /// The WMO a map is made of. Before Cataclysm every map has this chunk, empty for terrain.
    pub wmo_names: Option<StringBlock>,
    /// Where that WMO stands.
    pub wmo_placements: Option<Vec<MapObjectPlacement>>,
    #[debug(with = debug::trimmed_collection_fmt)]
    pub unknown: Vec<(MagicStr, Vec<u8>)>,
}

impl Default for WdtFile {
    fn default() -> Self {
        Self {
            version: 18,
            header: Mphd::default(),
            tiles: vec![MainEntry::default(); TILE_COUNT],
            file_ids: None,
            wmo_names: None,
            wmo_placements: None,
            unknown: Vec::new(),
        }
    }
}

impl WdtFile {
    pub fn tile(&self, x: usize, y: usize) -> Option<&MainEntry> {
        if x >= MAP_SIZE || y >= MAP_SIZE {
            return None;
        }
        self.tiles.get(y * MAP_SIZE + x)
    }

    pub fn has_tile(&self, x: usize, y: usize) -> bool {
        self.tile(x, y).is_some_and(MainEntry::has_adt)
    }

    pub fn tile_count(&self) -> usize {
        self.tiles.iter().filter(|t| t.has_adt()).count()
    }

    /// The WMO the map is made of, and where it stands, when it has no terrain.
    pub fn global_wmo(&self) -> Option<(String, &MapObjectPlacement)> {
        let name = self.wmo_names.as_ref()?.names().into_iter().next()?;
        let placement = self.wmo_placements.as_ref()?.first()?;
        Some((name, placement))
    }
}

impl WowStructR for WdtFile {
    fn wow_read<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let (mut version, mut header, mut tiles) = (None, None, None);
        let mut wdt = Self::default();

        read_chunks(reader, |reader, chunk| {
            match chunk.magic {
                MVER => version = Some(reader.wow_read()?),
                MPHD => header = Some(reader.wow_read()?),
                MAIN => {
                    let entries: Vec<MainEntry> = reader.wow_read_from_chunk(chunk)?;
                    if entries.len() != TILE_COUNT {
                        return Err(WowDataError::InvalidData(format!(
                            "MAIN has {} tiles, not {TILE_COUNT}",
                            entries.len()
                        )));
                    }
                    tiles = Some(entries);
                }
                MAID => wdt.file_ids = Some(reader.wow_read_from_chunk(chunk)?),
                MWMO => wdt.wmo_names = Some(StringBlock::read(reader, chunk)?),
                MODF => wdt.wmo_placements = Some(reader.wow_read_from_chunk(chunk)?),
                _ => wdt.unknown.push((chunk.magic, chunk.read_body(reader)?)),
            }
            Ok(())
        })?;

        let missing =
            |magic: MagicStr| WowDataError::MissingChunk(magic_to_inverted_string(&magic));
        wdt.version = version.ok_or_else(|| missing(MVER))?;
        wdt.header = header.ok_or_else(|| missing(MPHD))?;
        wdt.tiles = tiles.ok_or_else(|| missing(MAIN))?;
        Ok(wdt)
    }
}

impl WowStructW for WdtFile {
    fn wow_write<W: Write + Seek>(&self, writer: &mut W) -> Result<()> {
        write_chunk(writer, MVER, &self.version)?;
        write_chunk(writer, MPHD, &self.header)?;
        write_chunk(writer, MAIN, self.tiles.as_slice())?;
        if let Some(ids) = &self.file_ids {
            write_chunk(writer, MAID, ids.as_slice())?;
        }
        if let Some(names) = &self.wmo_names {
            write_chunk(writer, MWMO, names)?;
        }
        if let Some(placements) = &self.wmo_placements {
            write_chunk(writer, MODF, placements.as_slice())?;
        }
        for (magic, body) in &self.unknown {
            write_chunk(writer, *magic, body.as_slice())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use wow_alchemy_data::types::{BoundingBox, C3Vector};

    use super::*;

    fn global_wmo_map() -> WdtFile {
        let mut wdt = WdtFile::default();
        wdt.header.flags = MphdFlags::WDT_USES_GLOBAL_MAP_OBJ;
        wdt.wmo_names = Some(StringBlock::from_strings(&["World\\wmo\\Dungeon\\a.wmo"]).0);
        wdt.wmo_placements = Some(vec![MapObjectPlacement {
            unique_id: 0xFFFF_FFFF,
            position: C3Vector::new(1.0, 2.0, 3.0),
            extents: BoundingBox::new(C3Vector::origin(), C3Vector::new(4.0, 5.0, 6.0)),
            doodad_set: 1,
            ..Default::default()
        }]);
        wdt
    }

    #[test]
    fn writes_and_reads_back_the_same_file() {
        let mut wdt = global_wmo_map();
        wdt.tiles[3 * MAP_SIZE + 5].flags = MainFlags::HAS_ADT;
        wdt.unknown.push((*b"XXXX", vec![1, 2, 3]));

        let mut bytes = Cursor::new(Vec::new());
        wdt.wow_write(&mut bytes).unwrap();
        let read = WdtFile::wow_read(&mut Cursor::new(bytes.get_ref())).unwrap();

        assert_eq!(read, wdt);
        assert!(read.has_tile(5, 3));
        assert!(!read.has_tile(3, 5));
        assert_eq!(read.tile_count(), 1);
        let (name, placement) = read.global_wmo().unwrap();
        assert_eq!(name, "World\\wmo\\Dungeon\\a.wmo");
        assert_eq!(placement.doodad_set, 1);
    }

    #[test]
    fn chunks_are_stored_with_reversed_magics() {
        let mut bytes = Cursor::new(Vec::new());
        WdtFile::default().wow_write(&mut bytes).unwrap();
        let bytes = bytes.into_inner();
        assert_eq!(&bytes[0..4], b"REVM");
        assert_eq!(&bytes[12..16], b"DHPM");
        // MVER, MPHD of 32 bytes, then MAIN of 4096 tiles.
        assert_eq!(bytes.len(), 12 + 40 + 8 + TILE_COUNT * 8);
    }

    #[test]
    fn a_file_without_main_is_an_error() {
        let mut bytes = Cursor::new(Vec::new());
        write_chunk(&mut bytes, MVER, &18u32).unwrap();
        write_chunk(&mut bytes, MPHD, &Mphd::default()).unwrap();
        let error = WdtFile::wow_read(&mut Cursor::new(bytes.get_ref())).unwrap_err();
        assert!(error.to_string().contains("MAIN"), "{error}");
    }
}
