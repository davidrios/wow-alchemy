#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

use std::fmt;

use custom_debug::Debug;
use wow_alchemy_data::error::{Result, WowDataError};
use wow_alchemy_data::prelude::*;
use wow_alchemy_data::types::{WowStructR, WowStructW};
use wow_alchemy_data::utils::{magic_to_inverted_string, string_to_inverted_magic};
use wow_alchemy_data::world::MapObjectPlacement;
use wow_alchemy_data_derive::{WowHeaderR, WowHeaderW};
use wow_alchemy_utils::debug;

pub const MVER: MagicStr = string_to_inverted_magic("MVER");
pub const MWMO: MagicStr = string_to_inverted_magic("MWMO");
pub const MWID: MagicStr = string_to_inverted_magic("MWID");
pub const MODF: MagicStr = string_to_inverted_magic("MODF");
pub const MAOF: MagicStr = string_to_inverted_magic("MAOF");
pub const MARE: MagicStr = string_to_inverted_magic("MARE");
pub const MAHO: MagicStr = string_to_inverted_magic("MAHO");

/// Tiles along each side of a map.
pub const MAP_SIZE: usize = 64;
pub const TILE_COUNT: usize = MAP_SIZE * MAP_SIZE;
/// Heights along each side of a tile's outer grid, at its map chunks' corners.
pub const OUTER_SIZE: usize = 17;
/// Heights along each side of a tile's inner grid, at its map chunks' centres.
pub const INNER_SIZE: usize = 16;

/// A tile's heights, the MARE chunk: the corners of its 16 by 16 map chunks, then their centres,
/// each row by row from the north-west.
#[derive(Clone, PartialEq, WowHeaderR, WowHeaderW)]
pub struct Heights {
    /// `outer[y * 17 + x]`.
    pub outer: [i16; OUTER_SIZE * OUTER_SIZE],
    /// `inner[y * 16 + x]`.
    pub inner: [i16; INNER_SIZE * INNER_SIZE],
}

impl Default for Heights {
    fn default() -> Self {
        Self {
            outer: [0; OUTER_SIZE * OUTER_SIZE],
            inner: [0; INNER_SIZE * INNER_SIZE],
        }
    }
}

impl fmt::Debug for Heights {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Heights")
            .field("min", &self.min())
            .field("max", &self.max())
            .finish_non_exhaustive()
    }
}

impl Heights {
    pub fn outer(&self, x: usize, y: usize) -> Option<i16> {
        (x < OUTER_SIZE && y < OUTER_SIZE).then(|| self.outer[y * OUTER_SIZE + x])
    }

    pub fn inner(&self, x: usize, y: usize) -> Option<i16> {
        (x < INNER_SIZE && y < INNER_SIZE).then(|| self.inner[y * INNER_SIZE + x])
    }

    fn all(&self) -> impl Iterator<Item = i16> + '_ {
        self.outer.iter().chain(self.inner.iter()).copied()
    }

    pub fn min(&self) -> i16 {
        self.all().min().unwrap_or_default()
    }

    pub fn max(&self) -> i16 {
        self.all().max().unwrap_or_default()
    }
}

/// A tile's holes, the MAHO chunk, from Wrath of the Lich King on: one row of 16 bits for each
/// row of map chunks, bit `x` set where map chunk `x` of that row is a hole.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
pub struct Holes {
    pub rows: [u16; INNER_SIZE],
}

impl Holes {
    pub fn has_hole(&self, x: usize, y: usize) -> bool {
        x < INNER_SIZE && y < INNER_SIZE && self.rows[y] & (1 << x) != 0
    }

    pub fn set_hole(&mut self, x: usize, y: usize, hole: bool) {
        if x < INNER_SIZE && y < INNER_SIZE {
            if hole {
                self.rows[y] |= 1 << x;
            } else {
                self.rows[y] &= !(1 << x);
            }
        }
    }

    /// Map chunks that are holes.
    pub fn count(&self) -> usize {
        self.rows.iter().map(|row| row.count_ones() as usize).sum()
    }
}

/// One tile's low-resolution terrain: its MARE chunk and the MAHO chunk that follows it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Tile {
    pub heights: Heights,
    pub holes: Option<Holes>,
}

const HEIGHTS_BYTES: u32 = ((OUTER_SIZE * OUTER_SIZE + INNER_SIZE * INNER_SIZE) * 2) as u32;
const HOLES_BYTES: u32 = (INNER_SIZE * 2) as u32;

/// A WDL file. MAOF gives each tile's MARE by its offset in the file, so reading follows the
/// offsets and writing works them out again: tiles go after MAOF, in tile order. Chunks it
/// doesn't know are kept as they were and written before MAOF.
#[derive(Debug, Clone, PartialEq)]
pub struct WdlFile {
    /// 18 in every version.
    pub version: u32,
    /// The WMOs placed in MODF, before Legion. Every 3.3.5a WDL has the three chunks, empty.
    pub wmo_names: Option<StringBlock>,
    /// Offsets into `wmo_names`, which MODF's `name_id` indexes.
    pub wmo_name_offsets: Option<Vec<u32>>,
    pub wmo_placements: Option<Vec<MapObjectPlacement>>,
    /// Tile `y * 64 + x`, `None` where the map has none.
    #[debug(with = debug::trimmed_collection_fmt)]
    pub tiles: Vec<Option<Tile>>,
    #[debug(with = debug::trimmed_collection_fmt)]
    pub unknown: Vec<(MagicStr, Vec<u8>)>,
}

impl Default for WdlFile {
    fn default() -> Self {
        Self {
            version: 18,
            wmo_names: None,
            wmo_name_offsets: None,
            wmo_placements: None,
            tiles: vec![None; TILE_COUNT],
            unknown: Vec::new(),
        }
    }
}

impl WdlFile {
    pub fn tile(&self, x: usize, y: usize) -> Option<&Tile> {
        if x >= MAP_SIZE || y >= MAP_SIZE {
            return None;
        }
        self.tiles.get(y * MAP_SIZE + x)?.as_ref()
    }

    /// Tiles with heights.
    pub fn tile_count(&self) -> usize {
        self.tiles.iter().flatten().count()
    }

    /// The lowest and highest heights of every tile, `None` without tiles.
    pub fn height_range(&self) -> Option<(i16, i16)> {
        self.tiles.iter().flatten().fold(None, |range, tile| {
            let (min, max) = (tile.heights.min(), tile.heights.max());
            Some(range.map_or((min, max), |(lo, hi): (i16, i16)| {
                (lo.min(min), hi.max(max))
            }))
        })
    }

    /// Map chunks that are holes, over every tile.
    pub fn hole_count(&self) -> usize {
        self.tiles
            .iter()
            .flatten()
            .filter_map(|tile| tile.holes.as_ref())
            .map(Holes::count)
            .sum()
    }

    /// The WMO names of MWMO, with the names MWID points at in order.
    pub fn wmo_name_list(&self) -> Vec<String> {
        match (&self.wmo_names, &self.wmo_name_offsets) {
            (Some(names), Some(offsets)) => offsets.iter().filter_map(|&o| names.get(o)).collect(),
            (Some(names), None) => names.names(),
            _ => Vec::new(),
        }
    }
}

fn read_tile<R: Read + Seek>(reader: &mut R, offset: u64, end: u64) -> Result<Tile> {
    reader.seek(SeekFrom::Start(offset))?;
    let header: ChunkHeader = reader.wow_read()?;
    if header.magic != MARE || header.bytes != HEIGHTS_BYTES {
        return Err(WowDataError::InvalidData(format!(
            "MAOF points at {offset}, where there is {} of {} bytes rather than MARE of {HEIGHTS_BYTES}",
            magic_to_inverted_string(&header.magic),
            header.bytes
        )));
    }
    let heights = reader.wow_read()?;

    let mut holes = None;
    if reader.stream_position()? + 8 + HOLES_BYTES as u64 <= end {
        let next: ChunkHeader = reader.wow_read()?;
        if next.magic == MAHO && next.bytes == HOLES_BYTES {
            holes = Some(reader.wow_read()?);
        }
    }
    Ok(Tile { heights, holes })
}

impl WowStructR for WdlFile {
    fn wow_read<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let start = reader.stream_position()?;
        let mut version = None;
        let mut offsets: Option<Box<[u32; TILE_COUNT]>> = None;
        let mut wdl = Self::default();

        read_chunks(reader, |reader, chunk| {
            match chunk.magic {
                MVER => version = Some(reader.wow_read()?),
                MWMO => wdl.wmo_names = Some(StringBlock::read(reader, chunk)?),
                MWID => wdl.wmo_name_offsets = Some(reader.wow_read_from_chunk(chunk)?),
                MODF => wdl.wmo_placements = Some(reader.wow_read_from_chunk(chunk)?),
                MAOF => {
                    if chunk.bytes as usize != TILE_COUNT * 4 {
                        return Err(WowDataError::InvalidData(format!(
                            "MAOF has {} bytes, not {}",
                            chunk.bytes,
                            TILE_COUNT * 4
                        )));
                    }
                    offsets = Some(Box::new(reader.wow_read()?));
                }
                // Read through MAOF's offsets below.
                MARE | MAHO => {}
                _ => wdl.unknown.push((chunk.magic, chunk.read_body(reader)?)),
            }
            Ok(())
        })?;

        let missing =
            |magic: MagicStr| WowDataError::MissingChunk(magic_to_inverted_string(&magic));
        wdl.version = version.ok_or_else(|| missing(MVER))?;
        let offsets = offsets.ok_or_else(|| missing(MAOF))?;

        let end = reader.seek(SeekFrom::End(0))?;
        for (tile, &offset) in wdl.tiles.iter_mut().zip(offsets.iter()) {
            if offset != 0 {
                *tile = Some(read_tile(reader, start + offset as u64, end)?);
            }
        }
        reader.seek(SeekFrom::Start(end))?;
        Ok(wdl)
    }
}

impl WowStructW for WdlFile {
    fn wow_write<W: Write + Seek>(&self, writer: &mut W) -> Result<()> {
        let start = writer.stream_position()?;
        write_chunk(writer, MVER, &self.version)?;
        if let Some(names) = &self.wmo_names {
            write_chunk(writer, MWMO, names)?;
        }
        if let Some(offsets) = &self.wmo_name_offsets {
            write_chunk(writer, MWID, offsets.as_slice())?;
        }
        if let Some(placements) = &self.wmo_placements {
            write_chunk(writer, MODF, placements.as_slice())?;
        }
        for (magic, body) in &self.unknown {
            write_chunk(writer, *magic, body.as_slice())?;
        }

        let mut next = writer.stream_position()? - start + 8 + TILE_COUNT as u64 * 4;
        let mut offsets = [0u32; TILE_COUNT];
        for (offset, tile) in offsets.iter_mut().zip(&self.tiles) {
            if let Some(tile) = tile {
                *offset = u32::try_from(next).map_err(|_| {
                    WowDataError::InvalidData("the tiles go past 4 GiB".to_string())
                })?;
                next += 8 + HEIGHTS_BYTES as u64;
                if tile.holes.is_some() {
                    next += 8 + HOLES_BYTES as u64;
                }
            }
        }
        write_chunk(writer, MAOF, &offsets)?;

        for tile in self.tiles.iter().flatten() {
            write_chunk(writer, MARE, &tile.heights)?;
            if let Some(holes) = &tile.holes {
                write_chunk(writer, MAHO, holes)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    fn write(wdl: &WdlFile) -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        wdl.wow_write(&mut bytes).unwrap();
        bytes.into_inner()
    }

    fn sample() -> WdlFile {
        let mut wdl = WdlFile {
            wmo_names: Some(StringBlock::default()),
            wmo_name_offsets: Some(Vec::new()),
            wmo_placements: Some(Vec::new()),
            ..Default::default()
        };
        let mut tile = Tile::default();
        tile.heights.outer[0] = -100;
        tile.heights.inner[255] = 300;
        let mut holes = Holes::default();
        holes.set_hole(3, 2, true);
        tile.holes = Some(holes);
        wdl.tiles[10 * MAP_SIZE + 20] = Some(tile.clone());
        tile.holes = None;
        tile.heights.outer[5] = 500;
        wdl.tiles[40 * MAP_SIZE + 2] = Some(tile);
        wdl
    }

    #[test]
    fn records_have_the_sizes_of_the_file() {
        assert_eq!(Heights::default().wow_size(), 1090);
        assert_eq!(Holes::default().wow_size(), 32);
    }

    #[test]
    fn heights_and_holes_are_indexed_by_row() {
        let mut heights = Heights::default();
        heights.outer[2 * 17 + 1] = 7;
        heights.inner[3 * 16 + 4] = -8;
        assert_eq!(heights.outer(1, 2), Some(7));
        assert_eq!(heights.inner(4, 3), Some(-8));
        assert_eq!(heights.outer(17, 0), None);
        assert_eq!((heights.min(), heights.max()), (-8, 7));

        let holes = Holes {
            rows: [0, 0, 0b1000, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x8000],
        };
        assert!(holes.has_hole(3, 2));
        assert!(!holes.has_hole(2, 3));
        assert!(holes.has_hole(15, 15));
        assert_eq!(holes.count(), 2);
    }

    #[test]
    fn writes_and_reads_back_the_same_file() {
        let mut wdl = sample();
        wdl.unknown.push((*b"XXXX", vec![1, 2, 3]));
        let bytes = write(&wdl);
        let read = WdlFile::wow_read(&mut Cursor::new(&bytes)).unwrap();
        assert_eq!(read, wdl);
        assert_eq!(read.tile_count(), 2);
        assert_eq!(read.height_range(), Some((-100, 500)));
        assert_eq!(read.hole_count(), 1);
        assert!(read.tile(20, 10).unwrap().holes.unwrap().has_hole(3, 2));
        assert!(read.tile(2, 40).unwrap().holes.is_none());
        assert!(read.tile(10, 20).is_none());
    }

    #[test]
    fn maof_points_at_each_mare() {
        let bytes = write(&sample());
        assert_eq!(&bytes[0..4], b"REVM");
        // MVER, then empty MWMO, MWID and MODF, then MAOF.
        let maof = 12 + 3 * 8;
        assert_eq!(&bytes[maof..maof + 4], b"FOAM");
        let offset_of = |index: usize| {
            let at = maof + 8 + index * 4;
            u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize
        };
        let first = offset_of(10 * MAP_SIZE + 20);
        assert_eq!(first, maof + 8 + TILE_COUNT * 4);
        assert_eq!(&bytes[first..first + 4], b"ERAM");
        assert_eq!(&bytes[first + 8 + 1090..first + 8 + 1090 + 4], b"OHAM");
        assert_eq!(offset_of(40 * MAP_SIZE + 2), first + 8 + 1090 + 8 + 32);
        assert_eq!(offset_of(0), 0);
        assert_eq!(bytes.len(), offset_of(40 * MAP_SIZE + 2) + 8 + 1090);
    }

    #[test]
    fn reading_follows_the_offsets_rather_than_the_chunk_order() {
        // A tile whose MARE comes before MAOF, and an offset table pointing back at it.
        let mut bytes = Cursor::new(Vec::new());
        write_chunk(&mut bytes, MVER, &18u32).unwrap();
        let mare_at = bytes.position() as u32;
        let mut heights = Heights::default();
        heights.inner[0] = 42;
        write_chunk(&mut bytes, MARE, &heights).unwrap();
        let mut offsets = [0u32; TILE_COUNT];
        offsets[65] = mare_at;
        write_chunk(&mut bytes, MAOF, &offsets).unwrap();

        let wdl = WdlFile::wow_read(&mut Cursor::new(bytes.get_ref())).unwrap();
        let tile = wdl.tile(1, 1).unwrap();
        assert_eq!(tile.heights.inner(0, 0), Some(42));
        assert_eq!(tile.holes, None);
        assert_eq!(wdl.tile_count(), 1);
    }

    #[test]
    fn an_offset_not_at_a_mare_is_an_error() {
        let mut bytes = Cursor::new(Vec::new());
        write_chunk(&mut bytes, MVER, &18u32).unwrap();
        // Tile 0 points at MVER rather than at a MARE.
        let mut offsets = [0u32; TILE_COUNT];
        offsets[0] = 0;
        offsets[1] = 12;
        write_chunk(&mut bytes, MAOF, &offsets).unwrap();
        let error = WdlFile::wow_read(&mut Cursor::new(bytes.get_ref())).unwrap_err();
        assert!(error.to_string().contains("MAOF points at 12"), "{error}");
    }

    #[test]
    fn a_file_without_maof_is_an_error() {
        let mut bytes = Cursor::new(Vec::new());
        write_chunk(&mut bytes, MVER, &18u32).unwrap();
        let error = WdlFile::wow_read(&mut Cursor::new(bytes.get_ref())).unwrap_err();
        assert!(error.to_string().contains("MAOF"), "{error}");
    }
}
