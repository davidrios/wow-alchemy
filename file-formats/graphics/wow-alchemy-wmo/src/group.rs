//! A group file, `_NNN.wmo`: MVER, then MOGP, whose header is followed by the group's chunks.

use std::io::Cursor;

use wow_alchemy_data::error::{Result, WowDataError};
use wow_alchemy_data::prelude::*;
use wow_alchemy_data::types::{BoundingBox, C2Vector, C3Vector, WowStructR, WowStructW};
use wow_alchemy_data_derive::{WowHeaderR, WowHeaderW};

use crate::liquid::WmoLiquid;
use crate::*;

/// Size of MOGP's header, before its chunks.
pub const GROUP_HEADER_BYTES: usize = 68;
/// A triangle's material when it has none.
pub const NO_MATERIAL: u8 = 0xFF;

bitflags::bitflags! {
    /// MOGP and MOGI flags.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
    #[wow_data(bitflags = u32)]
    pub struct GroupFlags: u32 {
        /// Has MOBN and MOBR.
        const HAS_BSP_TREE = 0x1;
        const HAS_LIGHT_MAP = 0x2;
        /// Has MOCV.
        const HAS_VERTEX_COLORS = 0x4;
        const EXTERIOR = 0x8;
        const EXTERIOR_LIT = 0x40;
        const UNREACHABLE = 0x80;
        /// Has MOLR.
        const HAS_LIGHTS = 0x200;
        /// Has MODR.
        const HAS_DOODADS = 0x800;
        /// Has MLIQ.
        const HAS_WATER = 0x1000;
        /// Lit by its vertex colors rather than the sun.
        const INTERIOR = 0x2000;
        /// The group's plain water is the ocean kind. See [`WmoLiquid::liquid_type`].
        const OCEAN_WATER = 0x80000;
        /// Has a second MOCV.
        const HAS_TWO_MOCV = 0x0100_0000;
        /// Has a second MOTV.
        const HAS_TWO_MOTV = 0x0200_0000;
    }
}

/// MOGP's header, 68 bytes.
#[derive(Debug, Clone, Default, PartialEq, WowHeaderR, WowHeaderW)]
pub struct GroupHeader {
    /// Offset into the root's MOGN.
    pub group_name: u32,
    /// Offset into the root's MOGN.
    pub descriptive_group_name: u32,
    pub flags: GroupFlags,
    pub bounding_box: BoundingBox,
    /// The group's range of [`WmoRoot::portal_refs`].
    pub portal_start: u16,
    pub portal_count: u16,
    /// The batches, from the first, that are transition, interior and exterior ones.
    pub transition_batch_count: u16,
    pub interior_batch_count: u16,
    pub exterior_batch_count: u16,
    pub padding: u16,
    /// Indexes into the root's MFOG.
    pub fog_ids: [u8; 4],
    /// The group's liquid. See [`WmoLiquid::liquid_type`].
    pub group_liquid: u32,
    /// Row in WMOAreaTable.dbc.
    pub unique_id: u32,
    pub flags_2: u32,
    pub unused: u32,
}

bitflags::bitflags! {
    /// MOPY flags.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
    #[wow_data(bitflags = u8)]
    pub struct TriangleFlags: u8 {
        const UNKNOWN_0X01 = 0x01;
        const NO_CAMERA_COLLIDE = 0x02;
        const DETAIL = 0x04;
        const COLLISION = 0x08;
        const HINT = 0x10;
        const RENDER = 0x20;
        const UNKNOWN_0X40 = 0x40;
        const COLLIDE_HIT = 0x80;
    }
}

/// One entry of MOPY, one per triangle of MOVI.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
pub struct TriangleInfo {
    pub flags: TriangleFlags,
    /// Index into the root's MOMT, or [`NO_MATERIAL`].
    pub material: u8,
}

impl TriangleInfo {
    /// Whether to collide with the triangle: it says so, or it's drawn and not a detail.
    pub fn collides(&self) -> bool {
        let renders = self.flags.contains(TriangleFlags::RENDER)
            && !self.flags.contains(TriangleFlags::DETAIL)
            && self.material != NO_MATERIAL;
        self.flags.contains(TriangleFlags::COLLISION) || renders
    }
}

/// One entry of MOBA, 24 bytes: a range of MOVI drawn with one material.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
pub struct WmoBatch {
    /// A box for culling: smallest corner, then largest.
    pub bounding_box: [i16; 6],
    pub index_start: u32,
    pub index_count: u16,
    /// The range of MOVT the batch uses.
    pub vertex_start: u16,
    pub vertex_end: u16,
    pub flags: u8,
    /// Index into the root's MOMT.
    pub material: u8,
}

/// One entry of MOBN, 16 bytes.
#[derive(Debug, Clone, Copy, Default, PartialEq, WowHeaderR, WowHeaderW)]
pub struct BspNode {
    /// 0 to 2: the axis of the plane; 4: a leaf.
    pub flags: u16,
    /// Child nodes, or -1.
    pub negative_child: i16,
    pub positive_child: i16,
    /// The leaf's range of MOBR.
    pub face_count: u16,
    pub face_start: u32,
    pub plane_distance: f32,
}

/// One entry of MOCV, as stored: blue, green, red, alpha.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
pub struct VertexColor {
    pub b: u8,
    pub g: u8,
    pub r: u8,
    pub a: u8,
}

impl VertexColor {
    pub fn to_rgba(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

/// A group file. Vertices are in the WMO's own space, Z up. Chunks it doesn't know are kept as
/// they were, after the known ones.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WmoGroup {
    pub version: u32,
    pub header: GroupHeader,
    /// MOPY: one per triangle.
    pub triangles: Vec<TriangleInfo>,
    /// MOVI: three per triangle, into `positions`.
    pub indices: Vec<u16>,
    /// MOVT.
    pub positions: Vec<C3Vector>,
    /// MONR.
    pub normals: Vec<C3Vector>,
    /// Each MOTV, in order: some groups have a second, for a second texture.
    pub tex_coords: Vec<Vec<C2Vector>>,
    pub batches: Vec<WmoBatch>,
    /// MOLR: indexes into the root's MOLT.
    pub light_refs: Option<Vec<u16>>,
    /// MODR: indexes into the root's MODD.
    pub doodad_refs: Option<Vec<u16>>,
    pub bsp_nodes: Option<Vec<BspNode>>,
    /// MOBR: triangle indexes for the BSP leaves.
    pub bsp_faces: Option<Vec<u16>>,
    /// Each MOCV, in order.
    pub colors: Vec<Vec<VertexColor>>,
    pub liquid: Option<WmoLiquid>,
    /// Chunks inside MOGP it doesn't know.
    pub unknown: Vec<(MagicStr, Vec<u8>)>,
    /// Chunks after MOGP it doesn't know.
    pub unknown_outside: Vec<(MagicStr, Vec<u8>)>,
}

impl WmoGroup {
    /// The first set of texture coordinates, or none.
    pub fn first_tex_coords(&self) -> &[C2Vector] {
        self.tex_coords.first().map_or(&[], Vec::as_slice)
    }

    /// The first set of vertex colors, or none.
    pub fn first_colors(&self) -> &[VertexColor] {
        self.colors.first().map_or(&[], Vec::as_slice)
    }

    /// The first set of vertex colors as red, green, blue, alpha; empty unless there's one per
    /// vertex.
    pub fn colors_rgba(&self) -> Vec<[u8; 4]> {
        let colors = self.first_colors();
        if colors.len() != self.positions.len() {
            return Vec::new();
        }
        colors.iter().map(|c| c.to_rgba()).collect()
    }

    pub fn doodad_refs(&self) -> &[u16] {
        self.doodad_refs.as_deref().unwrap_or(&[])
    }

    /// Triangles to collide with, as indexes into `positions`: those that
    /// [`TriangleInfo::collides`].
    pub fn collision_indices(&self) -> Vec<u16> {
        self.triangles
            .iter()
            .zip(self.indices.chunks_exact(3))
            .filter(|(info, _)| info.collides())
            .flat_map(|(_, triangle)| triangle.iter().copied())
            .collect()
    }

    /// Checks that the mesh's parts agree: a MOPY entry per triangle, and a normal and a texture
    /// coordinate (first set) per vertex.
    pub fn check_mesh(&self) -> Result<()> {
        if self.triangles.len() != self.indices.len() / 3 {
            return Err(WowDataError::InvalidData(format!(
                "MOPY has {} triangles but MOVI has {}",
                self.triangles.len(),
                self.indices.len() / 3
            )));
        }
        let tex_coords = self.first_tex_coords().len();
        if self.normals.len() != self.positions.len() || tex_coords != self.positions.len() {
            return Err(WowDataError::InvalidData(format!(
                "group has {} vertices but {} normals and {tex_coords} texture coordinates",
                self.positions.len(),
                self.normals.len(),
            )));
        }
        Ok(())
    }

    fn read_mogp(&mut self, body: &[u8]) -> Result<()> {
        if body.len() < GROUP_HEADER_BYTES {
            return Err(WowDataError::InvalidData(format!(
                "MOGP has {} bytes, too few for its header",
                body.len()
            )));
        }
        let mut reader = Cursor::new(body);
        self.header = reader.wow_read()?;

        read_chunks(&mut reader, |reader, chunk| {
            match chunk.magic {
                MOPY => self.triangles = read_exact_records(reader, chunk, 2)?,
                MOVI => self.indices = read_exact_records(reader, chunk, 2)?,
                MOVT => self.positions = read_exact_records(reader, chunk, 12)?,
                MONR => self.normals = read_exact_records(reader, chunk, 12)?,
                MOTV => self.tex_coords.push(read_exact_records(reader, chunk, 8)?),
                MOBA => self.batches = read_exact_records(reader, chunk, 24)?,
                MOLR => self.light_refs = Some(read_exact_records(reader, chunk, 2)?),
                MODR => self.doodad_refs = Some(read_exact_records(reader, chunk, 2)?),
                MOBN => self.bsp_nodes = Some(read_exact_records(reader, chunk, 16)?),
                MOBR => self.bsp_faces = Some(read_exact_records(reader, chunk, 2)?),
                MOCV => self.colors.push(read_exact_records(reader, chunk, 4)?),
                MLIQ => self.liquid = Some(WmoLiquid::read(&chunk.read_body(reader)?)?),
                _ => self.unknown.push((chunk.magic, chunk.read_body(reader)?)),
            }
            Ok(())
        })
    }

    fn write_mogp(&self) -> Result<Vec<u8>> {
        let mut out = Cursor::new(Vec::new());
        let writer = &mut out;
        writer.wow_write(&self.header)?;
        write_chunk(writer, MOPY, self.triangles.as_slice())?;
        write_chunk(writer, MOVI, self.indices.as_slice())?;
        write_chunk(writer, MOVT, self.positions.as_slice())?;
        write_chunk(writer, MONR, self.normals.as_slice())?;
        if let Some(first) = self.tex_coords.first() {
            write_chunk(writer, MOTV, first.as_slice())?;
        }
        write_chunk(writer, MOBA, self.batches.as_slice())?;
        if let Some(refs) = &self.light_refs {
            write_chunk(writer, MOLR, refs.as_slice())?;
        }
        if let Some(refs) = &self.doodad_refs {
            write_chunk(writer, MODR, refs.as_slice())?;
        }
        if let Some(nodes) = &self.bsp_nodes {
            write_chunk(writer, MOBN, nodes.as_slice())?;
        }
        if let Some(faces) = &self.bsp_faces {
            write_chunk(writer, MOBR, faces.as_slice())?;
        }
        if let Some(first) = self.colors.first() {
            write_chunk(writer, MOCV, first.as_slice())?;
        }
        if let Some(liquid) = &self.liquid {
            write_chunk(writer, MLIQ, liquid)?;
        }
        for tex_coords in self.tex_coords.iter().skip(1) {
            write_chunk(writer, MOTV, tex_coords.as_slice())?;
        }
        for colors in self.colors.iter().skip(1) {
            write_chunk(writer, MOCV, colors.as_slice())?;
        }
        for (magic, body) in &self.unknown {
            write_chunk(writer, *magic, body.as_slice())?;
        }
        Ok(out.into_inner())
    }
}

impl WowStructR for WmoGroup {
    fn wow_read<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let mut version = None;
        let mut found_mogp = false;
        let mut group = Self::default();

        read_chunks(reader, |reader, chunk| {
            match chunk.magic {
                MVER => version = Some(reader.wow_read()?),
                MOGP if !found_mogp => {
                    found_mogp = true;
                    group.read_mogp(&chunk.read_body(reader)?)?;
                }
                _ => group
                    .unknown_outside
                    .push((chunk.magic, chunk.read_body(reader)?)),
            }
            Ok(())
        })?;

        group.version = check_version(version)?;
        if !found_mogp {
            return Err(missing(MOGP));
        }
        Ok(group)
    }
}

impl WowStructW for WmoGroup {
    fn wow_write<W: Write + Seek>(&self, writer: &mut W) -> Result<()> {
        write_chunk(writer, MVER, &self.version)?;
        write_chunk(writer, MOGP, self.write_mogp()?.as_slice())?;
        for (magic, body) in &self.unknown_outside {
            write_chunk(writer, *magic, body.as_slice())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use wow_alchemy_data::utils::string_to_inverted_magic;

    use super::*;

    fn floats(values: &[f32]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    fn chunk(magic: &str, body: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        write_chunk(&mut out, string_to_inverted_magic(magic), body).unwrap();
        out
    }

    /// A group of three vertices and two triangles: one drawn, one a drawn detail.
    fn group_file() -> Vec<u8> {
        let render = TriangleFlags::RENDER.bits();
        let detail = (TriangleFlags::RENDER | TriangleFlags::DETAIL).bits();

        let mut batch = vec![0u8; 24];
        batch[16..18].copy_from_slice(&3u16.to_le_bytes());
        batch[23] = 7;

        let mut mogp = vec![0u8; GROUP_HEADER_BYTES];
        mogp[8..12].copy_from_slice(&GroupFlags::INTERIOR.bits().to_le_bytes());
        mogp[36..38].copy_from_slice(&2u16.to_le_bytes());
        mogp[38..40].copy_from_slice(&3u16.to_le_bytes());
        mogp[40..42].copy_from_slice(&1u16.to_le_bytes());
        mogp[52..56].copy_from_slice(&15u32.to_le_bytes());
        mogp.extend(chunk("MOPY", &[render, 7, detail, 7]));
        mogp.extend(chunk("MOVI", &[0, 0, 1, 0, 2, 0, 2, 0, 1, 0, 0, 0]));
        mogp.extend(chunk("MOVT", &floats(&[0.0; 9])));
        mogp.extend(chunk("MONR", &floats(&[0.0; 9])));
        mogp.extend(chunk("MOTV", &floats(&[0.0; 6])));
        mogp.extend(chunk("MOBA", &batch));
        mogp.extend(chunk("MODR", &[4, 0]));
        mogp.extend(chunk("MOCV", &[1, 2, 3, 4, 1, 2, 3, 4, 1, 2, 3, 4]));
        mogp.extend(chunk("MOTV", &floats(&[1.0; 6])));
        mogp.extend(chunk("XXXX", &[9]));

        let mut data = chunk("MVER", &17u32.to_le_bytes());
        data.extend(chunk("MOGP", &mogp));
        data
    }

    fn read(data: &[u8]) -> WmoGroup {
        WmoGroup::wow_read(&mut Cursor::new(data)).unwrap()
    }

    #[test]
    fn group_reads_geometry_batches_and_collision() {
        let group = read(&group_file());
        group.check_mesh().unwrap();

        assert_eq!(group.header.flags, GroupFlags::INTERIOR);
        assert_eq!(
            (group.header.portal_start, group.header.portal_count),
            (2, 3)
        );
        assert_eq!(group.header.transition_batch_count, 1);
        assert_eq!(group.header.group_liquid, 15);
        assert_eq!(group.positions.len(), 3);
        assert_eq!(group.tex_coords.len(), 2);
        assert_eq!(group.first_tex_coords()[0], C2Vector::new(0.0, 0.0));
        assert_eq!(group.colors_rgba()[0], [3, 2, 1, 4]);
        assert_eq!(group.batches[0].material, 7);
        assert_eq!(group.batches[0].index_count, 3);
        assert_eq!(group.doodad_refs(), &[4]);
        assert_eq!(
            group.unknown,
            vec![(string_to_inverted_magic("XXXX"), vec![9])]
        );
        // The detail triangle renders but doesn't collide.
        assert_eq!(group.collision_indices(), vec![0, 1, 2]);
    }

    #[test]
    fn group_writes_back_the_same() {
        let data = group_file();
        let mut written = Cursor::new(Vec::new());
        read(&data).wow_write(&mut written).unwrap();
        assert_eq!(written.into_inner(), data);
    }

    #[test]
    fn colors_that_dont_match_the_vertices_are_dropped() {
        let mut group = read(&group_file());
        group.colors[0].pop();
        assert!(group.colors_rgba().is_empty());
    }

    #[test]
    fn a_mesh_with_missing_normals_fails_its_check() {
        let mut group = read(&group_file());
        group.normals.pop();
        assert!(group.check_mesh().is_err());
    }

    #[test]
    fn a_group_without_mogp_is_an_error() {
        let data = chunk("MVER", &17u32.to_le_bytes());
        let error = WmoGroup::wow_read(&mut Cursor::new(data)).unwrap_err();
        assert!(error.to_string().contains("MOGP"), "{error}");
    }
}
