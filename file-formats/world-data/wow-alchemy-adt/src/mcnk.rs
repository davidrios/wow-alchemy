//! MCNK: one of a tile's 16 by 16 map chunks, with its heights, normals, texture layers and
//! the rest, as 3.3.5a stores them.
//!
//! The chunk starts with a 128-byte [`McnkHeader`], followed by sub-chunks, each with its own
//! magic and size, that the header finds by their offsets from the MCNK chunk's start. Files
//! from 3.3.5a always order them MCVT, MCCV, MCNR, MCLY, MCRF, MCSH, MCAL, MCLQ, MCSE, and
//! three of them break the usual chunk rules:
//!
//! - MCNR's size counts its 145 normals, but 13 more bytes follow before the next sub-chunk.
//! - MCAL's own size is often wrong, even 0; the header's `alpha_size`, which counts MCAL's
//!   8-byte header too, is the real one.
//! - MCLQ's own size is always 0; the header's `liquid_size`, also counting the 8-byte header,
//!   is the real one.
//!
//! See <https://wowdev.wiki/ADT/v18#MCNK_chunk>.

use std::io::Cursor;

use custom_debug::Debug;
use wow_alchemy_data::error::{Result, WowDataError};
use wow_alchemy_data::prelude::*;
use wow_alchemy_data::types::C3Vector;
use wow_alchemy_data::utils::{magic_to_inverted_string, string_to_inverted_magic};
use wow_alchemy_data_derive::{WowHeaderR, WowHeaderW};
use wow_alchemy_utils::debug;

use crate::alpha_map::{AlphaMapFormat, decode_alpha_map};

pub const MCVT: MagicStr = string_to_inverted_magic("MCVT");
pub const MCCV: MagicStr = string_to_inverted_magic("MCCV");
pub const MCNR: MagicStr = string_to_inverted_magic("MCNR");
pub const MCLY: MagicStr = string_to_inverted_magic("MCLY");
pub const MCRF: MagicStr = string_to_inverted_magic("MCRF");
pub const MCSH: MagicStr = string_to_inverted_magic("MCSH");
pub const MCAL: MagicStr = string_to_inverted_magic("MCAL");
pub const MCLQ: MagicStr = string_to_inverted_magic("MCLQ");
pub const MCSE: MagicStr = string_to_inverted_magic("MCSE");

/// Heights, normals and vertex colors in a chunk: 9 by 9 outer vertices interleaved with 8 by 8
/// inner ones, row by row.
pub const VERTICES_PER_CHUNK: usize = 145;

/// Bytes after MCNR's normals that its size doesn't count.
pub const MCNR_PADDING: usize = 13;

const HEADER_BYTES: usize = 128;
const CHUNK_HEADER_BYTES: usize = 8;

bitflags::bitflags! {
    /// MCNK header flags.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
    #[wow_data(bitflags = u32)]
    pub struct McnkFlags: u32 {
        /// Has a shadow map. Files don't always agree with whether MCSH is there.
        const HAS_MCSH = 0x1;
        const IMPASS = 0x2;
        /// MCLQ liquid layers, in this order.
        const LQ_RIVER = 0x4;
        const LQ_OCEAN = 0x8;
        const LQ_MAGMA = 0x10;
        const LQ_SLIME = 0x20;
        /// Has vertex colors, MCCV.
        const HAS_MCCV = 0x40;
        const UNKNOWN_0X80 = 0x80;
        /// 4-bit alpha maps use all 64 rows and columns as stored, rather than repeating the
        /// 63rd.
        const DO_NOT_FIX_ALPHA_MAP = 0x8000;
        /// `holes_low_res` and what follows it are 64 bits of holes, from Mists of Pandaria on.
        const HIGH_RES_HOLES = 0x10000;
    }
}

/// The 128 bytes at the start of an MCNK. Counts, offsets and sizes of sub-chunks are worked out
/// again on writing, so only the other fields matter when making or changing a chunk.
#[derive(Debug, Clone, Default, PartialEq, WowHeaderR, WowHeaderW)]
pub struct McnkHeader {
    pub flags: McnkFlags,
    pub index_x: u32,
    pub index_y: u32,
    pub layer_count: u32,
    pub doodad_ref_count: u32,
    pub heights_offset: u32,
    pub normals_offset: u32,
    pub layers_offset: u32,
    pub refs_offset: u32,
    pub alpha_offset: u32,
    /// MCAL's size including its 8-byte header.
    pub alpha_size: u32,
    /// Where MCSH would be, even without one.
    pub shadow_offset: u32,
    /// MCSH's size without its header, 0 without one.
    pub shadow_size: u32,
    pub area_id: u32,
    pub map_object_ref_count: u32,
    /// One bit per 2 by 2 cells, 4 by 4 of them, row by row.
    pub holes_low_res: u16,
    pub unknown_but_used: u16,
    /// 2 bits per cell of the 8 by 8, which layer the lowest detail shows.
    pub low_quality_texture_map: [u8; 16],
    /// One bit per cell of the 8 by 8, row by row: no ground effect doodads there.
    pub no_effect_doodad: u64,
    pub sound_emitters_offset: u32,
    pub sound_emitter_count: u32,
    /// 0 without MCLQ.
    pub liquid_offset: u32,
    /// MCLQ's size including its 8-byte header, 0 without one.
    pub liquid_size: u32,
    /// The chunk's corner as stored: north, west, then the base the heights add to.
    pub position: C3Vector,
    /// 0 without MCCV.
    pub vertex_colors_offset: u32,
    /// MCLV, from Cataclysm on.
    pub vertex_lights_offset: u32,
    pub unused: u32,
}

bitflags::bitflags! {
    /// MCLY layer flags.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
    #[wow_data(bitflags = u32)]
    pub struct MclyFlags: u32 {
        /// Rotation of the animation, in 45 degree steps.
        const ANIMATION_ROTATION = 0x007;
        const ANIMATION_SPEED = 0x038;
        const ANIMATION_ENABLED = 0x040;
        const OVERBRIGHT = 0x080;
        /// The layer has an alpha map in MCAL.
        const USE_ALPHA_MAP = 0x100;
        /// Its alpha map is run-length encoded.
        const ALPHA_MAP_COMPRESSED = 0x200;
        const USE_CUBE_MAP_REFLECTION = 0x400;
    }
}

/// One texture layer, from MCLY.
#[derive(Debug, Clone, Default, PartialEq, WowHeaderR, WowHeaderW)]
pub struct MclyEntry {
    /// Index into MTEX.
    pub texture_id: u32,
    pub flags: MclyFlags,
    /// Where the layer's alpha map starts in [`McnkChunk::alpha`].
    pub alpha_offset: u32,
    /// Row in GroundEffectTexture.dbc.
    pub effect_id: u32,
}

/// One sound emitter, from MCSE.
#[derive(Debug, Clone, Default, PartialEq, WowHeaderR, WowHeaderW)]
pub struct McseEntry {
    /// Row in SoundEntriesAdvanced.dbc.
    pub sound_id: u32,
    pub position: C3Vector,
    pub size: C3Vector,
}

/// One vertex of an MCLQ layer. For water `data` is depth, two flow percentages and a filler
/// byte; for magma and slime it's two u16 texture coordinates.
#[derive(Debug, Clone, Copy, Default, PartialEq, WowHeaderR, WowHeaderW)]
pub struct MclqVertex {
    pub data: [u8; 4],
    pub height: f32,
}

impl MclqVertex {
    pub fn depth(&self) -> u8 {
        self.data[0]
    }

    pub fn texture_coordinates(&self) -> (u16, u16) {
        (
            u16::from_le_bytes([self.data[0], self.data[1]]),
            u16::from_le_bytes([self.data[2], self.data[3]]),
        )
    }
}

/// A flow of an MCLQ layer.
#[derive(Debug, Clone, Default, PartialEq, WowHeaderR, WowHeaderW)]
pub struct MclqFlow {
    pub sphere_center: C3Vector,
    pub sphere_radius: f32,
    pub direction: C3Vector,
    pub velocity: f32,
    pub amplitude: f32,
    pub frequency: f32,
}

/// One liquid layer of MCLQ, the liquid format from before MH2O. There is one per liquid flag
/// set on the chunk ([`McnkFlags::LQ_RIVER`] and on), in the flags' order.
#[derive(Debug, Clone, PartialEq, WowHeaderR, WowHeaderW)]
pub struct MclqLayer {
    pub min_height: f32,
    pub max_height: f32,
    /// 9 by 9, row by row.
    #[debug(skip)]
    pub vertices: [MclqVertex; 81],
    /// One per cell of the 8 by 8: the low 4 bits are the liquid type, and 0x0F there means
    /// no liquid.
    pub tiles: [u8; 64],
    pub flow_count: u32,
    pub flows: [MclqFlow; 2],
}

/// Bytes of one [`MclqLayer`].
pub const MCLQ_LAYER_BYTES: usize = 804;

/// One map chunk.
#[derive(Debug, Clone, PartialEq)]
pub struct McnkChunk {
    pub header: McnkHeader,
    /// MCVT: 145 heights over `header.position.z`.
    #[debug(with = debug::trimmed_collection_fmt)]
    pub heights: Vec<f32>,
    /// MCCV: 145 colors as stored, blue, green, red, alpha, with 127 neutral.
    #[debug(with = debug::option_trimmed_collection_fmt)]
    pub vertex_colors: Option<Vec<[u8; 4]>>,
    /// MCNR: 145 normals, x, y and z from -127 to 127.
    #[debug(with = debug::trimmed_collection_fmt)]
    pub normals: Vec<[i8; 3]>,
    /// The bytes after MCNR that its size doesn't count.
    pub normals_padding: [u8; MCNR_PADDING],
    /// MCLY.
    pub layers: Vec<MclyEntry>,
    /// MCRF's first part: indices into MDDF.
    #[debug(with = debug::trimmed_collection_fmt)]
    pub doodad_refs: Vec<u32>,
    /// MCRF's second part: indices into MODF.
    #[debug(with = debug::trimmed_collection_fmt)]
    pub map_object_refs: Vec<u32>,
    /// MCSH: 64 rows of 64 bits, lowest bit first.
    #[debug(with = debug::option_trimmed_collection_fmt)]
    pub shadow_map: Option<Vec<u64>>,
    /// MCAL as stored. Layers find their map at their `alpha_offset`, and
    /// [`McnkChunk::alpha_map`] decodes it.
    #[debug(with = debug::trimmed_collection_fmt)]
    pub alpha: Vec<u8>,
    /// The size MCAL's own header states. The client ignores it and many files have it wrong;
    /// it's kept so the file writes back as it was. Set it to `alpha.len()` for new data.
    pub alpha_stated_size: u32,
    /// MCLQ.
    pub liquid: Option<Vec<MclqLayer>>,
    /// MCSE.
    pub sound_emitters: Vec<McseEntry>,
}

impl Default for McnkChunk {
    fn default() -> Self {
        Self {
            header: McnkHeader::default(),
            heights: vec![0.0; VERTICES_PER_CHUNK],
            vertex_colors: None,
            normals: vec![[0, 0, 127]; VERTICES_PER_CHUNK],
            normals_padding: [0; MCNR_PADDING],
            layers: Vec::new(),
            doodad_refs: Vec::new(),
            map_object_refs: Vec::new(),
            shadow_map: None,
            alpha: Vec::new(),
            alpha_stated_size: 0,
            liquid: None,
            sound_emitters: Vec::new(),
        }
    }
}

/// A sub-chunk's header at `offset` from the MCNK chunk's start, with `body` the MCNK's data
/// after its own 8-byte header. Returns a reader at the sub-chunk's data.
fn open_sub(body: &[u8], offset: u32, magic: MagicStr) -> Result<(Cursor<&[u8]>, ChunkHeader)> {
    let start = (offset as usize)
        .checked_sub(CHUNK_HEADER_BYTES)
        .filter(|&start| start + CHUNK_HEADER_BYTES <= body.len())
        .ok_or_else(|| {
            WowDataError::InvalidData(format!(
                "MCNK has {} at offset {offset}, outside its {} bytes",
                magic_to_inverted_string(&magic),
                body.len() + CHUNK_HEADER_BYTES
            ))
        })?;
    let mut reader = Cursor::new(body);
    reader.set_position(start as u64);
    let header: ChunkHeader = reader.wow_read()?;
    if header.magic != magic {
        return Err(WowDataError::InvalidMagic {
            expected: magic_to_inverted_string(&magic),
            actual: magic_to_inverted_string(&header.magic),
        });
    }
    Ok((reader, header))
}

fn read_bytes(reader: &mut Cursor<&[u8]>, count: usize) -> Result<Vec<u8>> {
    let mut bytes = vec![0; count];
    reader.read_exact(&mut bytes)?;
    Ok(bytes)
}

/// Reads the sub-chunk at `offset` as records filling it.
fn read_sub<T: WowHeaderR + WowHeaderW>(
    body: &[u8],
    offset: u32,
    magic: MagicStr,
) -> Result<Vec<T>> {
    let (mut reader, header) = open_sub(body, offset, magic)?;
    reader.wow_read_from_chunk(&header)
}

impl McnkChunk {
    /// Reads an MCNK from its data, after the 8-byte chunk header.
    pub fn read(body: &[u8]) -> Result<Self> {
        let header: McnkHeader = Cursor::new(body).wow_read()?;

        let heights = read_sub(body, header.heights_offset, MCVT)?;
        let vertex_colors = match header.vertex_colors_offset {
            0 => None,
            offset => Some(read_sub(body, offset, MCCV)?),
        };

        let (mut reader, normals_header) = open_sub(body, header.normals_offset, MCNR)?;
        let normals = reader.wow_read_from_chunk(&normals_header)?;
        let mut normals_padding = [0; MCNR_PADDING];
        reader.read_exact(&mut normals_padding)?;

        let layers = read_sub(body, header.layers_offset, MCLY)?;

        let mut doodad_refs: Vec<u32> = read_sub(body, header.refs_offset, MCRF)?;
        let doodad_count = header.doodad_ref_count as usize;
        if doodad_refs.len() != doodad_count + header.map_object_ref_count as usize {
            return Err(WowDataError::InvalidData(format!(
                "MCRF has {} references, but the MCNK header counts {} and {}",
                doodad_refs.len(),
                header.doodad_ref_count,
                header.map_object_ref_count
            )));
        }
        let map_object_refs = doodad_refs.split_off(doodad_count);

        let shadow_map = match (header.shadow_offset, header.shadow_size) {
            (0, _) | (_, 0) => None,
            (offset, _) => Some(read_sub(body, offset, MCSH)?),
        };

        let (alpha, alpha_stated_size) = match header.alpha_offset {
            0 => (Vec::new(), 0),
            offset => {
                let (mut reader, sub) = open_sub(body, offset, MCAL)?;
                let size = (header.alpha_size as usize).saturating_sub(CHUNK_HEADER_BYTES);
                (read_bytes(&mut reader, size)?, sub.bytes)
            }
        };

        let liquid = match header.liquid_offset {
            0 => None,
            offset => {
                let (mut reader, _) = open_sub(body, offset, MCLQ)?;
                let size = (header.liquid_size as usize).saturating_sub(CHUNK_HEADER_BYTES);
                if size % MCLQ_LAYER_BYTES != 0 {
                    return Err(WowDataError::InvalidData(format!(
                        "MCLQ has {size} bytes, not whole layers of {MCLQ_LAYER_BYTES}"
                    )));
                }
                let layers = (0..size / MCLQ_LAYER_BYTES)
                    .map(|_| reader.wow_read())
                    .collect::<Result<_>>()?;
                Some(layers)
            }
        };

        let sound_emitters = match header.sound_emitters_offset {
            0 => Vec::new(),
            offset => read_sub(body, offset, MCSE)?,
        };

        Ok(Self {
            header,
            heights,
            vertex_colors,
            normals,
            normals_padding,
            layers,
            doodad_refs,
            map_object_refs,
            shadow_map,
            alpha,
            alpha_stated_size,
            liquid,
            sound_emitters,
        })
    }

    /// The header as written: the stored one with counts, offsets and sizes worked out from the
    /// sub-chunks, which follow in the order 3.3.5a uses. Also returns the chunk's data size.
    pub fn layout(&self) -> (McnkHeader, usize) {
        let sub = |bytes: usize| CHUNK_HEADER_BYTES + bytes;
        let mut header = self.header.clone();
        // Offsets count from the MCNK chunk's start, before its own header.
        let mut position = CHUNK_HEADER_BYTES + HEADER_BYTES;

        header.heights_offset = position as u32;
        position += sub(self.heights.len() * 4);

        header.vertex_colors_offset = 0;
        if let Some(colors) = &self.vertex_colors {
            header.vertex_colors_offset = position as u32;
            position += sub(colors.len() * 4);
        }

        header.normals_offset = position as u32;
        position += sub(self.normals.len() * 3) + MCNR_PADDING;

        header.layer_count = self.layers.len() as u32;
        header.layers_offset = position as u32;
        position += sub(self.layers.len() * 16);

        header.doodad_ref_count = self.doodad_refs.len() as u32;
        header.map_object_ref_count = self.map_object_refs.len() as u32;
        header.refs_offset = position as u32;
        position += sub((self.doodad_refs.len() + self.map_object_refs.len()) * 4);

        header.shadow_offset = position as u32;
        header.shadow_size = 0;
        if let Some(shadow) = &self.shadow_map {
            header.shadow_size = (shadow.len() * 8) as u32;
            position += sub(shadow.len() * 8);
        }

        header.alpha_offset = position as u32;
        header.alpha_size = sub(self.alpha.len()) as u32;
        position += sub(self.alpha.len());

        header.liquid_offset = 0;
        header.liquid_size = 0;
        if let Some(liquid) = &self.liquid {
            header.liquid_offset = position as u32;
            header.liquid_size = sub(liquid.len() * MCLQ_LAYER_BYTES) as u32;
            position += sub(liquid.len() * MCLQ_LAYER_BYTES);
        }

        header.sound_emitter_count = self.sound_emitters.len() as u32;
        header.sound_emitters_offset = position as u32;
        position += sub(self.sound_emitters.len() * 28);

        (header, position - CHUNK_HEADER_BYTES)
    }

    /// The decoded 64 by 64 alpha map of layer `index`, or `None` for the first layer and those
    /// without [`MclyFlags::USE_ALPHA_MAP`]. `big_alpha` is the WDT's MPHD flag.
    pub fn alpha_map(&self, index: usize, big_alpha: bool) -> Result<Option<Vec<u8>>> {
        let Some(layer) = self.layers.get(index) else {
            return Ok(None);
        };
        if index == 0 || !layer.flags.contains(MclyFlags::USE_ALPHA_MAP) {
            return Ok(None);
        }
        let data = self
            .alpha
            .get(layer.alpha_offset as usize..)
            .ok_or_else(|| {
                WowDataError::InvalidData(format!(
                    "layer {index} has its alpha map at {}, past MCAL's {} bytes",
                    layer.alpha_offset,
                    self.alpha.len()
                ))
            })?;
        let format = AlphaMapFormat::new(big_alpha, layer.flags, self.header.flags);
        decode_alpha_map(data, format).map(Some)
    }
}

impl WowHeaderW for McnkChunk {
    fn wow_write<W: Write>(&self, writer: &mut W) -> Result<()> {
        let (header, _) = self.layout();
        writer.wow_write(&header)?;
        write_chunk(writer, MCVT, self.heights.as_slice())?;
        if let Some(colors) = &self.vertex_colors {
            write_chunk(writer, MCCV, colors.as_slice())?;
        }
        write_chunk(writer, MCNR, self.normals.as_slice())?;
        writer.write_all(&self.normals_padding)?;
        write_chunk(writer, MCLY, self.layers.as_slice())?;
        writer.wow_write(&ChunkHeader {
            magic: MCRF,
            bytes: ((self.doodad_refs.len() + self.map_object_refs.len()) * 4) as u32,
        })?;
        WowHeaderW::wow_write(self.doodad_refs.as_slice(), writer)?;
        WowHeaderW::wow_write(self.map_object_refs.as_slice(), writer)?;
        if let Some(shadow) = &self.shadow_map {
            write_chunk(writer, MCSH, shadow.as_slice())?;
        }
        writer.wow_write(&ChunkHeader {
            magic: MCAL,
            bytes: self.alpha_stated_size,
        })?;
        writer.write_all(&self.alpha)?;
        if let Some(liquid) = &self.liquid {
            writer.wow_write(&ChunkHeader {
                magic: MCLQ,
                bytes: 0,
            })?;
            WowHeaderW::wow_write(liquid.as_slice(), writer)?;
        }
        write_chunk(writer, MCSE, self.sound_emitters.as_slice())?;
        Ok(())
    }

    fn wow_size(&self) -> usize {
        self.layout().1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> McnkChunk {
        let mut chunk = McnkChunk::default();
        chunk.header.flags = McnkFlags::HAS_MCCV | McnkFlags::LQ_OCEAN;
        chunk.header.index_x = 3;
        chunk.header.index_y = 7;
        chunk.header.area_id = 12;
        chunk.header.holes_low_res = 0x8001;
        chunk.header.no_effect_doodad = 0x0102_0304_0506_0708;
        chunk.header.position = C3Vector::new(1.0, 2.0, 3.0);
        chunk.heights[5] = 4.5;
        chunk.vertex_colors = Some(vec![[127, 127, 127, 255]; VERTICES_PER_CHUNK]);
        chunk.normals[1] = [-1, 2, 120];
        chunk.layers = vec![
            MclyEntry {
                texture_id: 0,
                ..Default::default()
            },
            MclyEntry {
                texture_id: 1,
                flags: MclyFlags::USE_ALPHA_MAP,
                alpha_offset: 0,
                effect_id: 9,
            },
        ];
        chunk.doodad_refs = vec![1, 2];
        chunk.map_object_refs = vec![3];
        chunk.shadow_map = Some(vec![u64::MAX; 64]);
        chunk.alpha = vec![0x11; 2048];
        chunk.alpha_stated_size = 0;
        chunk.liquid = Some(vec![MclqLayer {
            min_height: 1.0,
            max_height: 2.0,
            vertices: [MclqVertex::default(); 81],
            tiles: [0x0F; 64],
            flow_count: 0,
            flows: Default::default(),
        }]);
        chunk.sound_emitters = vec![McseEntry {
            sound_id: 4,
            ..Default::default()
        }];
        chunk
    }

    fn bytes(chunk: &McnkChunk) -> Vec<u8> {
        let mut out = Vec::new();
        chunk.wow_write(&mut out).unwrap();
        assert_eq!(out.len(), chunk.wow_size());
        out
    }

    #[test]
    fn writes_and_reads_back_the_same_chunk() {
        let chunk = sample();
        let body = bytes(&chunk);
        let read = McnkChunk::read(&body).unwrap();
        let (header, _) = chunk.layout();
        assert_eq!(read.header, header);
        assert_eq!(McnkChunk { header, ..chunk }, read);
        assert_eq!(bytes(&read), body);
    }

    #[test]
    fn header_is_128_bytes_and_offsets_count_from_the_chunk_start() {
        assert_eq!(McnkHeader::default().wow_size(), HEADER_BYTES);
        let chunk = sample();
        let body = bytes(&chunk);
        let (header, size) = chunk.layout();
        assert_eq!(size, body.len());
        assert_eq!(header.heights_offset, 136);
        let at = |offset: u32| &body[offset as usize - 8..offset as usize - 4];
        assert_eq!(at(header.heights_offset), b"TVCM");
        assert_eq!(at(header.vertex_colors_offset), b"VCCM");
        assert_eq!(at(header.normals_offset), b"RNCM");
        assert_eq!(at(header.layers_offset), b"YLCM");
        assert_eq!(at(header.refs_offset), b"FRCM");
        assert_eq!(at(header.shadow_offset), b"HSCM");
        assert_eq!(at(header.alpha_offset), b"LACM");
        assert_eq!(at(header.liquid_offset), b"QLCM");
        assert_eq!(at(header.sound_emitters_offset), b"ESCM");
        // MCNR's 13 bytes of padding come after the size it states.
        assert_eq!(
            header.layers_offset - header.normals_offset,
            8 + 435 + MCNR_PADDING as u32
        );
        assert_eq!(header.shadow_size, 512);
        assert_eq!(header.alpha_size, 8 + 2048);
        assert_eq!(header.liquid_size, 8 + MCLQ_LAYER_BYTES as u32);
        assert_eq!(
            (header.doodad_ref_count, header.map_object_ref_count),
            (2, 1)
        );
    }

    #[test]
    fn missing_shadow_map_points_at_the_alpha_maps() {
        let mut chunk = sample();
        chunk.shadow_map = None;
        chunk.liquid = None;
        chunk.vertex_colors = None;
        let (header, _) = chunk.layout();
        assert_eq!(header.shadow_offset, header.alpha_offset);
        assert_eq!(header.shadow_size, 0);
        assert_eq!((header.liquid_offset, header.liquid_size), (0, 0));
        assert_eq!(header.vertex_colors_offset, 0);
        let read = McnkChunk::read(&bytes(&chunk)).unwrap();
        assert_eq!(read.shadow_map, None);
        assert_eq!(read.liquid, None);
        assert_eq!(read.vertex_colors, None);
    }

    #[test]
    fn alpha_size_comes_from_the_mcnk_header_not_mcal() {
        let mut chunk = sample();
        chunk.alpha_stated_size = 7;
        let body = bytes(&chunk);
        let read = McnkChunk::read(&body).unwrap();
        assert_eq!(read.alpha.len(), 2048);
        assert_eq!(read.alpha_stated_size, 7);
        assert_eq!(bytes(&read), body);
    }

    #[test]
    fn alpha_maps_decode_per_layer() {
        let chunk = sample();
        assert_eq!(chunk.alpha_map(0, false).unwrap(), None);
        let map = chunk.alpha_map(1, false).unwrap().unwrap();
        assert_eq!(map[0], 17);
        assert_eq!(chunk.alpha_map(2, false).unwrap(), None);

        let mut chunk = chunk;
        chunk.layers[1].alpha_offset = 5000;
        assert!(chunk.alpha_map(1, false).is_err());
    }

    #[test]
    fn a_wrong_sub_chunk_magic_is_an_error() {
        let mut body = bytes(&sample());
        let (header, _) = sample().layout();
        let at = header.layers_offset as usize - 8;
        body[at..at + 4].copy_from_slice(b"XXXX");
        let error = McnkChunk::read(&body).unwrap_err();
        assert!(error.to_string().contains("MCLY"), "{error}");
    }

    #[test]
    fn mclq_layers_are_804_bytes() {
        let layer = &sample().liquid.unwrap()[0];
        assert_eq!(layer.wow_size(), MCLQ_LAYER_BYTES);
    }
}
