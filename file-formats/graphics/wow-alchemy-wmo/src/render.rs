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
}

/// A WMO root file.
#[derive(Debug, Clone, Default)]
pub struct RenderRoot {
    pub group_count: u32,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    pub materials: Vec<RenderMaterial>,
    pub doodad_sets: Vec<RenderDoodadSet>,
    pub doodad_names: Vec<String>,
    pub doodads: Vec<RenderDoodad>,
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
        let (mut textures, mut materials, mut names, mut sets, mut defs): (&[u8], &[u8], &[u8], &[u8], &[u8]) =
            (&[], &[], &[], &[], &[]);

        // Borrowing the chunks first means their order in the file doesn't matter.
        for (magic, body) in chunks(data)? {
            match &magic {
                b"MOHD" => header = records::<WmoHeaderRecord>("MOHD", body)?.pop(),
                b"MOTX" => textures = body,
                b"MOMT" => materials = body,
                b"MODN" => names = body,
                b"MODS" => sets = body,
                b"MODD" => defs = body,
                _ => {}
            }
        }

        let header = header.ok_or_else(|| WmoError::MissingRequiredChunk("MOHD".into()))?;
        let mut root = Self {
            group_count: header.group_count,
            bounds_min: vec3(&header.bounds_min),
            bounds_max: vec3(&header.bounds_max),
            ..Default::default()
        };

        root.materials = records::<WmoMaterialRecord>("MOMT", materials)?
            .iter()
            .map(|m| RenderMaterial {
                flags: m.flags,
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
                return Self::parse_body(flags, &body[GROUP_HEADER_BYTES..]);
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
                b"MOVT" => group.positions = records::<C3Vector>("MOVT", body)?.iter().map(vec3).collect(),
                b"MONR" => group.normals = records::<C3Vector>("MONR", body)?.iter().map(vec3).collect(),
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
                    group.colors = body.chunks_exact(4).map(|c| [c[2], c[1], c[0], c[3]]).collect();
                }
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

        let mut sub = chunk(b"MOPY", &[TRIANGLE_RENDER, 7, TRIANGLE_RENDER | TRIANGLE_DETAIL, 7]);
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
