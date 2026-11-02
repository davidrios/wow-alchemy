//! What it takes to draw a WMO, read from WotLK root and group files.
//!
//! The records use the same derives as wow-alchemy-m2, so their layouts are
//! written down once. `parser` and `group_parser` still have older layouts that
//! don't match 3.3.5a files, and should move to these records.

use std::io::Cursor;

use wow_alchemy_data::prelude::*;
use wow_alchemy_data::types::{C2Vector, C3Vector};
use wow_alchemy_data_derive::{WowHeaderR, WowHeaderW};

use crate::error::{Result, WmoError};

const GROUP_HEADER_BYTES: usize = 68;
const NO_MATERIAL: u8 = 0xFF;

/// MOPY flags.
const TRIANGLE_DETAIL: u8 = 0x04;
const TRIANGLE_COLLISION: u8 = 0x08;
const TRIANGLE_RENDER: u8 = 0x20;

/// MOGP flag: the group is lit by its vertex colors rather than the sun.
pub const GROUP_INDOOR: u32 = 0x2000;

/// MOHD flag: a group's liquid field is a row of LiquidType.dbc.
const ROOT_LIQUID_IS_ROW: u32 = 0x4;
/// MOGP flag: the group's plain water is the ocean kind.
const GROUP_OCEAN_WATER: u32 = 0x80000;

const MLIQ_HEADER_BYTES: usize = 30;
const MLIQ_VERTEX_BYTES: usize = 8;
/// An MLIQ cell's low bits say this when the cell has no liquid.
const MLIQ_NO_LIQUID: u8 = 0x0F;
/// Where MOGP's header stores the group's liquid.
const GROUP_LIQUID_OFFSET: usize = 52;

/// The MOHD chunk.
#[derive(Debug, Clone, Default, WowHeaderR, WowHeaderW)]
pub struct WmoHeaderRecord {
    pub material_count: u32,
    pub group_count: u32,
    pub portal_count: u32,
    pub light_count: u32,
    pub doodad_name_count: u32,
    pub doodad_def_count: u32,
    pub doodad_set_count: u32,
    pub ambient_color: u32,
    pub wmo_id: u32,
    pub bounds_min: C3Vector,
    pub bounds_max: C3Vector,
    pub flags: u16,
    pub lod_count: u16,
}

/// One entry of MOMT.
#[derive(Debug, Clone, Default, WowHeaderR, WowHeaderW)]
pub struct WmoMaterialRecord {
    pub flags: u32,
    pub shader: u32,
    pub blend_mode: u32,
    /// Offset into MOTX.
    pub texture_1: u32,
    pub emissive_color: u32,
    pub emissive_frame_color: u32,
    /// Offset into MOTX.
    pub texture_2: u32,
    pub diffuse_color: u32,
    pub ground_type: u32,
    /// Offset into MOTX.
    pub texture_3: u32,
    pub color_2: u32,
    pub flags_2: u32,
    pub runtime: [u32; 4],
}

/// One entry of MODS.
#[derive(Debug, Clone, Default, WowHeaderR, WowHeaderW)]
pub struct WmoDoodadSetRecord {
    pub name: [u8; 20],
    pub start: u32,
    pub count: u32,
    pub padding: u32,
}

/// One entry of MODD.
#[derive(Debug, Clone, Default, WowHeaderR, WowHeaderW)]
pub struct WmoDoodadRecord {
    /// An offset into MODN in the low 24 bits, and flags in the top 8.
    pub name_and_flags: u32,
    pub position: C3Vector,
    /// X, Y, Z, W.
    pub rotation: [f32; 4],
    pub scale: f32,
    pub color: u32,
}

/// One entry of MOPT.
#[derive(Debug, Clone, Copy, Default, WowHeaderR, WowHeaderW)]
pub struct WmoPortalRecord {
    pub start: u16,
    pub count: u16,
    pub normal: C3Vector,
    pub offset: f32,
}

/// One entry of MOPR.
#[derive(Debug, Clone, Copy, Default, WowHeaderR, WowHeaderW)]
pub struct WmoPortalRefRecord {
    pub portal: u16,
    /// The group on the far side, from the group this entry belongs to.
    pub group: u16,
    /// The side of the portal's plane it's seen from: 1, -1, or 0 for both.
    pub side: i16,
    pub padding: u16,
}

/// One entry of MOBA.
#[derive(Debug, Clone, Default, WowHeaderR, WowHeaderW)]
pub struct WmoBatchRecord {
    pub bounds: [i16; 6],
    pub index_start: u32,
    pub index_count: u16,
    pub vertex_start: u16,
    pub vertex_end: u16,
    pub flags: u8,
    pub material: u8,
}

/// One entry of MOPY.
#[derive(Debug, Clone, Default, WowHeaderR, WowHeaderW)]
pub struct WmoTriangleRecord {
    pub flags: u8,
    pub material: u8,
}

/// One material from MOMT.
#[derive(Debug, Clone)]
pub struct RenderMaterial {
    pub flags: u32,
    pub shader: u32,
    pub blend_mode: u32,
    pub texture: String,
}

/// A range of [`RenderRoot::doodads`], from MODS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderDoodadSet {
    pub start: u32,
    pub count: u32,
}

/// A doodad placed in the WMO's own space, from MODD.
#[derive(Debug, Clone, Copy)]
pub struct RenderDoodad {
    /// Index into [`RenderRoot::doodad_names`].
    pub name: u32,
    pub position: [f32; 3],
    /// X, Y, Z, W.
    pub rotation: [f32; 4],
    pub scale: f32,
    /// Its baked light, as stored: 0xAARRGGBB.
    pub color: u32,
}

/// A WMO root file.
#[derive(Debug, Clone, Default)]
pub struct RenderRoot {
    /// MOHD flags.
    pub flags: u32,
    /// MOHD's ambient color, as stored: 0xAARRGGBB.
    pub ambient_color: u32,
    pub group_count: u32,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    pub materials: Vec<RenderMaterial>,
    pub doodad_sets: Vec<RenderDoodadSet>,
    pub doodad_names: Vec<String>,
    pub doodads: Vec<RenderDoodad>,
    /// MOPV: the corners of every portal.
    pub portal_vertices: Vec<[f32; 3]>,
    /// MOPT.
    pub portals: Vec<WmoPortalRecord>,
    /// MOPR.
    pub portal_refs: Vec<WmoPortalRefRecord>,
}

/// A range of [`RenderGroup::indices`] with one material, from MOBA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderBatch {
    pub index_start: u32,
    pub index_count: u32,
    pub material: u32,
}

/// A WMO group file. Vertices are in the WMO's own space, Z up.
#[derive(Debug, Clone, Default)]
pub struct RenderGroup {
    pub flags: u32,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub tex_coords: Vec<[f32; 2]>,
    /// Red, green, blue, alpha. Empty when the group has none.
    pub colors: Vec<[u8; 4]>,
    pub indices: Vec<u16>,
    pub batches: Vec<RenderBatch>,
    /// Triangles to collide with, as indexes into `positions`.
    pub collision_indices: Vec<u16>,
    /// The doodads standing in this group, as indexes into [`RenderRoot::doodads`], from MODR.
    pub doodad_refs: Vec<u16>,
    /// The group's box, from MOGP's header: smallest corner, then largest.
    pub bounds: [f32; 6],
    /// The group's range of [`RenderRoot::portal_refs`], from MOGP's header.
    pub portal_start: u32,
    pub portal_count: u32,
    /// The liquid field of the group's header. See [`RenderLiquid::liquid_type`].
    pub group_liquid: u32,
    pub liquid: Option<RenderLiquid>,
}

/// A group's liquid surface from MLIQ, on a grid in the WMO's own space.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RenderLiquid {
    pub vertices_x: u32,
    pub vertices_y: u32,
    pub corner: [f32; 3],
    /// `vertices_x * vertices_y` heights, row by row.
    pub heights: Vec<f32>,
    /// One byte per cell, row by row: 1 when the cell has liquid.
    pub cells: Vec<u8>,
    /// The kind the first liquid cell states: 0 water, 1 ocean, 2 magma, 3 slime.
    first_cell_kind: Option<u8>,
}

impl RenderLiquid {
    fn parse(data: &[u8]) -> Result<Self> {
        let too_short = || WmoError::InvalidChunkSize(data.len() as u32);
        let u32_at = |offset: usize| -> Result<u32> {
            let bytes = data.get(offset..offset + 4).ok_or_else(too_short)?;
            Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
        };

        let vertices_x = u32_at(0)?;
        let vertices_y = u32_at(4)?;
        let cells_x = u32_at(8)? as usize;
        let cells_y = u32_at(12)? as usize;

        // A group can have this chunk with no grid. The wiki says such a group can still be full
        // of liquid, up to the top of its bounds, which isn't handled here.
        if cells_x == 0 || cells_y == 0 {
            return Ok(Self::default());
        }

        if cells_x + 1 != vertices_x as usize || cells_y + 1 != vertices_y as usize {
            return Err(WmoError::InvalidFormat(format!(
                "MLIQ has {cells_x}x{cells_y} cells but {vertices_x}x{vertices_y} vertices"
            )));
        }

        let corner = [
            f32::from_bits(u32_at(16)?),
            f32::from_bits(u32_at(20)?),
            f32::from_bits(u32_at(24)?),
        ];

        // Each vertex holds 4 bytes of flow or texture data, then its height.
        let vertex_count = vertices_x as usize * vertices_y as usize;
        let vertices = data
            .get(MLIQ_HEADER_BYTES..MLIQ_HEADER_BYTES + vertex_count * MLIQ_VERTEX_BYTES)
            .ok_or_else(too_short)?;
        let heights = vertices
            .chunks_exact(MLIQ_VERTEX_BYTES)
            .map(|v| f32::from_le_bytes(v[4..8].try_into().unwrap()))
            .collect();

        let cells_start = MLIQ_HEADER_BYTES + vertices.len();
        let cells = data
            .get(cells_start..cells_start + cells_x * cells_y)
            .ok_or_else(too_short)?;

        Ok(Self {
            vertices_x,
            vertices_y,
            corner,
            heights,
            cells: cells
                .iter()
                .map(|c| u8::from(c & MLIQ_NO_LIQUID != MLIQ_NO_LIQUID))
                .collect(),
            first_cell_kind: cells
                .iter()
                .map(|c| c & MLIQ_NO_LIQUID)
                .find(|&kind| kind != MLIQ_NO_LIQUID),
        })
    }

    /// The liquid's row in LiquidType.dbc, worked out as the 3.3.5a client does, from the root's
    /// flags and the group's flags and liquid field. 0 when it can't be told.
    pub fn liquid_type(&self, root_flags: u32, group_flags: u32, group_liquid: u32) -> u32 {
        let mut entry = if root_flags & ROOT_LIQUID_IS_ROW != 0 {
            group_liquid
        } else if group_liquid == MLIQ_NO_LIQUID as u32 {
            0
        } else {
            group_liquid + 1
        };

        if entry == 0 {
            entry = self.first_cell_kind.map_or(0, |kind| kind as u32 + 1);
        }

        // Low values name a kind of liquid rather than a row.
        if entry == 0 || entry >= 21 {
            return entry;
        }
        match (entry - 1) & 3 {
            0 if group_flags & GROUP_OCEAN_WATER != 0 => 14,
            0 => 13,
            1 => 14,
            2 => 19,
            _ => 20,
        }
    }
}

/// The chunks of a block, in file order, with their magics as they read in the docs.
fn chunks(data: &[u8]) -> Result<Vec<([u8; 4], &[u8])>> {
    let mut found = Vec::new();
    let mut position = 0;
    while position + 8 <= data.len() {
        let mut magic: [u8; 4] = data[position..position + 4].try_into().unwrap();
        magic.reverse();
        let size = u32::from_le_bytes(data[position + 4..position + 8].try_into().unwrap());
        let end = position + 8 + size as usize;
        if end > data.len() {
            return Err(WmoError::InvalidChunkSize(size));
        }
        found.push((magic, &data[position + 8..end]));
        position = end;
    }
    Ok(found)
}

/// Reads a chunk that is a list of one record. Trailing partial records are an error.
fn records<T: WowHeaderR>(chunk: &str, data: &[u8]) -> Result<Vec<T>> {
    let mut reader = Cursor::new(data);
    let mut items = Vec::new();
    while (reader.position() as usize) < data.len() {
        let item: T = reader
            .wow_read()
            .map_err(|e| WmoError::InvalidFormat(format!("{chunk}: {e}")))?;
        items.push(item);
    }
    Ok(items)
}

fn vec3(v: &C3Vector) -> [f32; 3] {
    [v.x, v.y, v.z]
}

fn string_at(block: &[u8], offset: usize) -> String {
    let rest = block.get(offset..).unwrap_or(&[]);
    let end = rest.iter().position(|&b| b == 0).unwrap_or(rest.len());
    String::from_utf8_lossy(&rest[..end]).into_owned()
}

impl RenderRoot {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut header = None;
        let (mut portal_vertices, mut portals, mut portal_refs) =
            (Vec::new(), Vec::new(), Vec::new());
        let (mut textures, mut materials, mut names, mut sets, mut defs): (
            &[u8],
            &[u8],
            &[u8],
            &[u8],
            &[u8],
        ) = (&[], &[], &[], &[], &[]);

        // Borrowing the chunks first means their order in the file doesn't matter.
        for (magic, body) in chunks(data)? {
            match &magic {
                b"MOHD" => header = records::<WmoHeaderRecord>("MOHD", body)?.pop(),
                b"MOTX" => textures = body,
                b"MOMT" => materials = body,
                b"MODN" => names = body,
                b"MODS" => sets = body,
                b"MODD" => defs = body,
                b"MOPV" => {
                    portal_vertices = records::<C3Vector>("MOPV", body)?
                        .iter()
                        .map(vec3)
                        .collect()
                }
                b"MOPT" => portals = records("MOPT", body)?,
                b"MOPR" => portal_refs = records("MOPR", body)?,
                _ => {}
            }
        }

        let header = header.ok_or_else(|| WmoError::MissingRequiredChunk("MOHD".into()))?;
        let mut root = Self {
            flags: header.flags as u32,
            ambient_color: header.ambient_color,
            portal_vertices,
            portals,
            portal_refs,
            group_count: header.group_count,
            bounds_min: vec3(&header.bounds_min),
            bounds_max: vec3(&header.bounds_max),
            ..Default::default()
        };

        root.materials = records::<WmoMaterialRecord>("MOMT", materials)?
            .iter()
            .map(|m| RenderMaterial {
                flags: m.flags,
                shader: m.shader,
                blend_mode: m.blend_mode,
                texture: string_at(textures, m.texture_1 as usize),
            })
            .collect();

        root.doodad_sets = records::<WmoDoodadSetRecord>("MODS", sets)?
            .iter()
            .map(|s| RenderDoodadSet {
                start: s.start,
                count: s.count,
            })
            .collect();

        let mut name_indexes = std::collections::HashMap::new();
        for d in records::<WmoDoodadRecord>("MODD", defs)? {
            let offset = (d.name_and_flags & 0x00FF_FFFF) as usize;
            let name = *name_indexes.entry(offset).or_insert_with(|| {
                root.doodad_names.push(string_at(names, offset));
                root.doodad_names.len() as u32 - 1
            });
            root.doodads.push(RenderDoodad {
                name,
                position: vec3(&d.position),
                rotation: d.rotation,
                scale: d.scale,
                color: d.color,
            });
        }

        Ok(root)
    }
}

impl RenderGroup {
    pub fn parse(data: &[u8]) -> Result<Self> {
        for (magic, body) in chunks(data)? {
            if &magic == b"MOGP" {
                if body.len() < GROUP_HEADER_BYTES {
                    return Err(WmoError::InvalidChunkSize(body.len() as u32));
                }
                let flags = u32::from_le_bytes(body[8..12].try_into().unwrap());
                let mut group = Self::parse_body(flags, &body[GROUP_HEADER_BYTES..])?;
                for (i, bound) in group.bounds.iter_mut().enumerate() {
                    let at = 12 + i * 4;
                    *bound = f32::from_le_bytes(body[at..at + 4].try_into().unwrap());
                }
                group.portal_start = u16::from_le_bytes(body[36..38].try_into().unwrap()) as u32;
                group.portal_count = u16::from_le_bytes(body[38..40].try_into().unwrap()) as u32;
                group.group_liquid = u32::from_le_bytes(
                    body[GROUP_LIQUID_OFFSET..GROUP_LIQUID_OFFSET + 4]
                        .try_into()
                        .unwrap(),
                );
                return Ok(group);
            }
        }
        Err(WmoError::MissingRequiredChunk("MOGP".into()))
    }

    fn parse_body(flags: u32, data: &[u8]) -> Result<Self> {
        let mut group = Self {
            flags,
            ..Default::default()
        };
        let mut triangles = Vec::new();
        let (mut read_uv, mut read_colors) = (false, false);

        for (magic, body) in chunks(data)? {
            match &magic {
                b"MOPY" => triangles = records::<WmoTriangleRecord>("MOPY", body)?,
                b"MOVI" => group.indices = records("MOVI", body)?,
                b"MODR" => group.doodad_refs = records("MODR", body)?,
                b"MOVT" => {
                    group.positions = records::<C3Vector>("MOVT", body)?
                        .iter()
                        .map(vec3)
                        .collect()
                }
                b"MONR" => {
                    group.normals = records::<C3Vector>("MONR", body)?
                        .iter()
                        .map(vec3)
                        .collect()
                }
                // Some groups have a second set, for a second texture.
                b"MOTV" if !read_uv => {
                    read_uv = true;
                    group.tex_coords = records::<C2Vector>("MOTV", body)?
                        .iter()
                        .map(|v| [v.x, v.y])
                        .collect();
                }
                b"MOCV" if !read_colors => {
                    read_colors = true;
                    group.colors = body
                        .chunks_exact(4)
                        .map(|c| [c[2], c[1], c[0], c[3]])
                        .collect();
                }
                b"MLIQ" => group.liquid = Some(RenderLiquid::parse(body)?),
                b"MOBA" => {
                    group.batches = records::<WmoBatchRecord>("MOBA", body)?
                        .iter()
                        .map(|b| RenderBatch {
                            index_start: b.index_start,
                            index_count: b.index_count as u32,
                            material: b.material as u32,
                        })
                        .collect();
                }
                _ => {}
            }
        }

        if triangles.len() != group.indices.len() / 3 {
            return Err(WmoError::InvalidFormat(format!(
                "MOPY has {} triangles but MOVI has {}",
                triangles.len(),
                group.indices.len() / 3
            )));
        }

        for (triangle, info) in triangles.iter().enumerate() {
            let renders = info.flags & TRIANGLE_RENDER != 0
                && info.flags & TRIANGLE_DETAIL == 0
                && info.material != NO_MATERIAL;
            if info.flags & TRIANGLE_COLLISION != 0 || renders {
                group
                    .collision_indices
                    .extend_from_slice(&group.indices[triangle * 3..triangle * 3 + 3]);
            }
        }

        if group.colors.len() != group.positions.len() {
            group.colors.clear();
        }
        if group.normals.len() != group.positions.len()
            || group.tex_coords.len() != group.positions.len()
        {
            return Err(WmoError::InvalidFormat(format!(
                "group has {} vertices but {} normals and {} texture coordinates",
                group.positions.len(),
                group.normals.len(),
                group.tex_coords.len()
            )));
        }

        Ok(group)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn liquid_chunk() -> Vec<u8> {
        let mut data = Vec::new();
        for value in [3u32, 2, 2, 1] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&floats(&[1.0, 2.0, 3.0]));
        data.extend_from_slice(&7u16.to_le_bytes());
        for height in 0..6 {
            data.extend_from_slice(&[0; 4]);
            data.extend_from_slice(&(height as f32).to_le_bytes());
        }
        data.extend_from_slice(&[0x4F, 0x42]);
        data
    }

    #[test]
    fn reads_a_liquid_grid() {
        let liquid = RenderLiquid::parse(&liquid_chunk()).unwrap();
        assert_eq!((liquid.vertices_x, liquid.vertices_y), (3, 2));
        assert_eq!(liquid.corner, [1.0, 2.0, 3.0]);
        assert_eq!(liquid.heights, vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(liquid.cells, vec![0, 1]);
    }

    #[test]
    fn a_liquid_chunk_with_no_cells_is_empty_rather_than_an_error() {
        let mut data = vec![0u8; MLIQ_HEADER_BYTES];
        data[0] = 1;
        data[4] = 1;
        let liquid = RenderLiquid::parse(&data).unwrap();
        assert!(liquid.cells.is_empty());
        assert!(liquid.heights.is_empty());
    }

    #[test]
    fn rejects_a_truncated_liquid_grid() {
        let data = liquid_chunk();
        assert!(RenderLiquid::parse(&data[..data.len() - 1]).is_err());
    }

    #[test]
    fn works_out_liquid_types_as_the_client_does() {
        // The first cell with liquid says kind 2, magma.
        let liquid = RenderLiquid::parse(&liquid_chunk()).unwrap();

        // The root says the field is already a row.
        assert_eq!(liquid.liquid_type(ROOT_LIQUID_IS_ROW, 0, 41), 41);
        // Otherwise it's one less than a kind.
        assert_eq!(liquid.liquid_type(0, 0, 0), 13);
        assert_eq!(liquid.liquid_type(0, GROUP_OCEAN_WATER, 0), 14);
        assert_eq!(liquid.liquid_type(0, 0, 1), 14);
        assert_eq!(liquid.liquid_type(0, 0, 2), 19);
        assert_eq!(liquid.liquid_type(0, 0, 3), 20);
        // 15 means the cells decide.
        assert_eq!(liquid.liquid_type(0, 0, 15), 19);
    }

    fn chunk(magic: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut out = magic.iter().rev().copied().collect::<Vec<_>>();
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(body);
        out
    }

    fn floats(values: &[f32]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    #[test]
    fn group_reads_geometry_batches_and_collision() {
        let mut batch = vec![0u8; 24];
        batch[16..18].copy_from_slice(&3u16.to_le_bytes());
        batch[23] = 7;

        let mut sub = chunk(
            b"MOPY",
            &[TRIANGLE_RENDER, 7, TRIANGLE_RENDER | TRIANGLE_DETAIL, 7],
        );
        sub.extend(chunk(b"MOVI", &[0, 0, 1, 0, 2, 0, 2, 0, 1, 0, 0, 0]));
        sub.extend(chunk(b"MOVT", &floats(&[0.0; 9])));
        sub.extend(chunk(b"MONR", &floats(&[0.0; 9])));
        sub.extend(chunk(b"MOTV", &floats(&[0.0; 6])));
        sub.extend(chunk(b"MOBA", &batch));
        sub.extend(chunk(b"MOCV", &[1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4]));

        let mut mogp = vec![0u8; GROUP_HEADER_BYTES];
        mogp[8..12].copy_from_slice(&GROUP_INDOOR.to_le_bytes());
        mogp.extend(sub);

        let group = RenderGroup::parse(&chunk(b"MOGP", &mogp)).unwrap();

        assert_eq!(group.flags, GROUP_INDOOR);
        assert_eq!(group.positions.len(), 3);
        assert_eq!(group.colors[0], [3, 2, 1, 4]);
        assert_eq!(group.batches[0].material, 7);
        assert_eq!(group.batches[0].index_count, 3);
        // The detail triangle renders but doesn't collide.
        assert_eq!(group.collision_indices, vec![0, 1, 2]);
    }

    #[test]
    fn root_resolves_names_by_offset() {
        let mut mohd = vec![0u8; 64];
        mohd[4..8].copy_from_slice(&2u32.to_le_bytes());

        let mut material = vec![0u8; 64];
        material[12..16].copy_from_slice(&4u32.to_le_bytes());

        let mut def = vec![0u8; 40];
        def[0..4].copy_from_slice(&(0x0100_0002u32).to_le_bytes());

        let mut data = chunk(b"MOHD", &mohd);
        data.extend(chunk(b"MOTX", b"abc\0tex\0"));
        data.extend(chunk(b"MOMT", &material));
        data.extend(chunk(b"MODN", b"a\0b\0"));
        data.extend(chunk(b"MODD", &def));

        let root = RenderRoot::parse(&data).unwrap();

        assert_eq!(root.group_count, 2);
        assert_eq!(root.materials[0].texture, "tex");
        assert_eq!(root.doodad_names, vec!["b"]);
    }

    #[test]
    fn truncated_chunks_are_errors() {
        let mut data = chunk(b"MOHD", &[0u8; 64]);
        data.truncate(20);
        assert!(RenderRoot::parse(&data).is_err());
    }
}
