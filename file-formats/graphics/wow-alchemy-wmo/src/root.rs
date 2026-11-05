//! The root file: MVER, MOHD, then the materials, groups, portals, lights, doodads and fogs.

use std::collections::HashMap;

use wow_alchemy_data::error::Result;
use wow_alchemy_data::prelude::*;
use wow_alchemy_data::types::{BoundingBox, C3Vector, Quaternion, WowStructR, WowStructW};
use wow_alchemy_data_derive::{WowHeaderR, WowHeaderW};

use crate::*;

bitflags::bitflags! {
    /// MOHD flags.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
    #[wow_data(bitflags = u16)]
    pub struct RootFlags: u16 {
        const DO_NOT_ATTENUATE_VERTICES_BASED_ON_DISTANCE_TO_PORTAL = 0x1;
        const USE_UNIFIED_RENDER_PATH = 0x2;
        /// A group's `group_liquid` is a row of LiquidType.dbc. See [`WmoLiquid::liquid_type`].
        const USE_LIQUID_TYPE_DBC_ID = 0x4;
        const DO_NOT_FIX_VERTEX_COLOR_ALPHA = 0x8;
    }
}

/// The MOHD chunk.
#[derive(Debug, Clone, Default, PartialEq, WowHeaderR, WowHeaderW)]
pub struct RootHeader {
    pub material_count: u32,
    pub group_count: u32,
    pub portal_count: u32,
    pub light_count: u32,
    pub doodad_name_count: u32,
    pub doodad_def_count: u32,
    pub doodad_set_count: u32,
    /// As stored: 0xAARRGGBB.
    pub ambient_color: u32,
    /// Row in WMOAreaTable.dbc.
    pub wmo_id: u32,
    pub bounding_box: BoundingBox,
    pub flags: RootFlags,
    pub lod_count: u16,
}

bitflags::bitflags! {
    /// MOMT flags.
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
    #[wow_data(bitflags = u32)]
    pub struct MaterialFlags: u32 {
        const UNLIT = 0x1;
        const UNFOGGED = 0x2;
        /// Two-sided.
        const UNCULLED = 0x4;
        const EXTERIOR_LIGHT = 0x8;
        /// Lit at night: uses `emissive_color`.
        const SIDN = 0x10;
        const WINDOW = 0x20;
        const CLAMP_S = 0x40;
        const CLAMP_T = 0x80;
    }
}

/// One entry of MOMT, 64 bytes.
#[derive(Debug, Clone, Default, PartialEq, WowHeaderR, WowHeaderW)]
pub struct WmoMaterial {
    pub flags: MaterialFlags,
    /// 0 diffuse, 1 specular, 2 metal, 3 environment, 4 opaque, 5 environment metal, and so on.
    pub shader: u32,
    /// 0 opaque, 1 alpha key, 2 alpha, 3 additive, 4 and 5 modulate, ...
    pub blend_mode: u32,
    /// Offset into MOTX.
    pub texture_1: u32,
    /// As stored: 0xAARRGGBB.
    pub emissive_color: u32,
    /// Set by the client at runtime.
    pub emissive_frame_color: u32,
    /// Offset into MOTX.
    pub texture_2: u32,
    /// As stored: 0xAARRGGBB.
    pub diffuse_color: u32,
    /// Row in TerrainType.dbc.
    pub ground_type: u32,
    /// Offset into MOTX.
    pub texture_3: u32,
    pub color_2: u32,
    pub flags_2: u32,
    /// Set by the client at runtime.
    pub runtime: [u32; 4],
}

/// One entry of MOGI, 32 bytes: what the root knows of a group.
#[derive(Debug, Clone, Default, PartialEq, WowHeaderR, WowHeaderW)]
pub struct GroupInfo {
    /// The same flags as the group's header.
    pub flags: GroupFlags,
    pub bounding_box: BoundingBox,
    /// Offset into MOGN, or -1.
    pub name_offset: i32,
}

/// A plane: points `p` with `normal · p == distance`.
#[derive(Debug, Clone, Copy, Default, PartialEq, WowHeaderR, WowHeaderW)]
pub struct Plane {
    pub normal: C3Vector,
    pub distance: f32,
}

/// One entry of MOPT, 20 bytes.
#[derive(Debug, Clone, Copy, Default, PartialEq, WowHeaderR, WowHeaderW)]
pub struct WmoPortal {
    /// The portal's corners in MOPV.
    pub start: u16,
    pub count: u16,
    pub plane: Plane,
}

/// One entry of MOPR, 8 bytes: a group's link through a portal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
pub struct PortalRef {
    pub portal: u16,
    /// The group on the far side, from the group this entry belongs to.
    pub group: u16,
    /// The side of the portal's plane it's seen from: 1, -1, or 0 for both.
    pub side: i16,
    pub padding: u16,
}

/// One entry of MOVB, 4 bytes: a run of MOVV.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
pub struct VisibleBlock {
    pub start: u16,
    pub count: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightType {
    Omni,
    Spot,
    Directional,
    Ambient,
}

/// One entry of MOLT, 48 bytes.
#[derive(Debug, Clone, Default, PartialEq, WowHeaderR, WowHeaderW)]
pub struct WmoLight {
    /// See [`WmoLight::kind`].
    pub light_type: u8,
    pub use_attenuation: u8,
    pub padding: [u8; 2],
    /// As stored: 0xAARRGGBB.
    pub color: u32,
    pub position: C3Vector,
    pub intensity: f32,
    pub unknown: [f32; 4],
    pub attenuation_start: f32,
    pub attenuation_end: f32,
}

impl WmoLight {
    pub fn kind(&self) -> Option<LightType> {
        Some(match self.light_type {
            0 => LightType::Omni,
            1 => LightType::Spot,
            2 => LightType::Directional,
            3 => LightType::Ambient,
            _ => return None,
        })
    }
}

/// One entry of MODS, 32 bytes: a range of [`WmoRoot::doodads`].
#[derive(Debug, Clone, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
pub struct WmoDoodadSet {
    /// Null-padded.
    pub name: [u8; 20],
    pub start: u32,
    pub count: u32,
    pub padding: u32,
}

impl WmoDoodadSet {
    pub fn name(&self) -> String {
        let end = self.name.iter().position(|&b| b == 0).unwrap_or(20);
        String::from_utf8_lossy(&self.name[..end]).into_owned()
    }
}

/// One entry of MODD, 40 bytes: a doodad placed in the WMO's own space.
#[derive(Debug, Clone, PartialEq, WowHeaderR, WowHeaderW)]
pub struct WmoDoodad {
    /// An offset into MODN in the low 24 bits, and flags in the top 8.
    pub name_and_flags: u32,
    pub position: C3Vector,
    pub rotation: Quaternion,
    pub scale: f32,
    /// Its baked light, as stored: 0xAARRGGBB.
    pub color: u32,
}

impl WmoDoodad {
    /// Offset into MODN.
    pub fn name_offset(&self) -> u32 {
        self.name_and_flags & 0x00FF_FFFF
    }

    /// The top 8 bits of `name_and_flags`; 0x1 accepts projected textures.
    pub fn flags(&self) -> u8 {
        (self.name_and_flags >> 24) as u8
    }
}

/// The fog's end, start (as a fraction of the end) and color.
#[derive(Debug, Clone, Copy, Default, PartialEq, WowHeaderR, WowHeaderW)]
pub struct FogParams {
    pub end: f32,
    pub start_scalar: f32,
    /// As stored: 0xAARRGGBB.
    pub color: u32,
}

/// One entry of MFOG, 48 bytes.
#[derive(Debug, Clone, Copy, Default, PartialEq, WowHeaderR, WowHeaderW)]
pub struct Fog {
    pub flags: u32,
    pub position: C3Vector,
    pub smaller_radius: f32,
    pub larger_radius: f32,
    pub fog: FogParams,
    pub underwater: FogParams,
}

/// The doodads' file names, each once in the order the doodads first use them, from
/// [`WmoRoot::doodad_names`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DoodadNames {
    pub names: Vec<String>,
    /// For each of [`WmoRoot::doodads`], its index into `names`.
    pub indexes: Vec<u32>,
}

/// A root file. Chunks it doesn't know are kept as they were, after the known ones.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WmoRoot {
    pub version: u32,
    pub header: RootHeader,
    /// MOTX: texture file names, named by offset.
    pub textures: StringBlock,
    pub materials: Vec<WmoMaterial>,
    /// MOGN: group names, named by offset.
    pub group_names: StringBlock,
    pub groups: Vec<GroupInfo>,
    /// MOSB: a skybox model's name, or nothing.
    pub skybox: StringBlock,
    /// MOPV: the corners of every portal.
    pub portal_vertices: Vec<C3Vector>,
    pub portals: Vec<WmoPortal>,
    pub portal_refs: Vec<PortalRef>,
    /// MOVV.
    pub visible_vertices: Vec<C3Vector>,
    pub visible_blocks: Vec<VisibleBlock>,
    pub lights: Vec<WmoLight>,
    pub doodad_sets: Vec<WmoDoodadSet>,
    /// MODN: doodad file names, named by offset.
    pub doodad_file_names: StringBlock,
    pub doodads: Vec<WmoDoodad>,
    pub fogs: Vec<Fog>,
    /// MCVP, which only some files have: planes bounding a convex volume.
    pub convex_volume_planes: Option<Vec<Plane>>,
    pub unknown: Vec<(MagicStr, Vec<u8>)>,
}

impl WmoRoot {
    /// The MOTX name at `offset`, or empty past its end.
    pub fn texture(&self, offset: u32) -> String {
        self.textures.get(offset).unwrap_or_default()
    }

    /// The MODN name of a doodad, or empty past its end.
    pub fn doodad_name(&self, doodad: &WmoDoodad) -> String {
        self.doodad_file_names
            .get(doodad.name_offset())
            .unwrap_or_default()
    }

    /// The doodads' names, without repeats: doodads naming the same offset share an index.
    pub fn doodad_names(&self) -> DoodadNames {
        let mut names = DoodadNames::default();
        let mut by_offset = HashMap::new();
        for doodad in &self.doodads {
            let index = *by_offset.entry(doodad.name_offset()).or_insert_with(|| {
                names.names.push(self.doodad_name(doodad));
                names.names.len() as u32 - 1
            });
            names.indexes.push(index);
        }
        names
    }
}

impl WowStructR for WmoRoot {
    fn wow_read<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let (mut version, mut header) = (None, None);
        let mut root = Self::default();

        read_chunks(reader, |reader, chunk| {
            match chunk.magic {
                MVER => version = Some(reader.wow_read()?),
                MOHD => header = Some(reader.wow_read()?),
                MOTX => root.textures = StringBlock::read(reader, chunk)?,
                MOMT => root.materials = read_exact_records(reader, chunk, 64)?,
                MOGN => root.group_names = StringBlock::read(reader, chunk)?,
                MOGI => root.groups = read_exact_records(reader, chunk, 32)?,
                MOSB => root.skybox = StringBlock::read(reader, chunk)?,
                MOPV => root.portal_vertices = read_exact_records(reader, chunk, 12)?,
                MOPT => root.portals = read_exact_records(reader, chunk, 20)?,
                MOPR => root.portal_refs = read_exact_records(reader, chunk, 8)?,
                MOVV => root.visible_vertices = read_exact_records(reader, chunk, 12)?,
                MOVB => root.visible_blocks = read_exact_records(reader, chunk, 4)?,
                MOLT => root.lights = read_exact_records(reader, chunk, 48)?,
                MODS => root.doodad_sets = read_exact_records(reader, chunk, 32)?,
                MODN => root.doodad_file_names = StringBlock::read(reader, chunk)?,
                MODD => root.doodads = read_exact_records(reader, chunk, 40)?,
                MFOG => root.fogs = read_exact_records(reader, chunk, 48)?,
                MCVP => root.convex_volume_planes = Some(read_exact_records(reader, chunk, 16)?),
                _ => root.unknown.push((chunk.magic, chunk.read_body(reader)?)),
            }
            Ok(())
        })?;

        root.version = check_version(version)?;
        root.header = header.ok_or_else(|| missing(MOHD))?;
        Ok(root)
    }
}

impl WowStructW for WmoRoot {
    fn wow_write<W: Write + Seek>(&self, writer: &mut W) -> Result<()> {
        write_chunk(writer, MVER, &self.version)?;
        write_chunk(writer, MOHD, &self.header)?;
        write_chunk(writer, MOTX, &self.textures)?;
        write_chunk(writer, MOMT, self.materials.as_slice())?;
        write_chunk(writer, MOGN, &self.group_names)?;
        write_chunk(writer, MOGI, self.groups.as_slice())?;
        write_chunk(writer, MOSB, &self.skybox)?;
        write_chunk(writer, MOPV, self.portal_vertices.as_slice())?;
        write_chunk(writer, MOPT, self.portals.as_slice())?;
        write_chunk(writer, MOPR, self.portal_refs.as_slice())?;
        write_chunk(writer, MOVV, self.visible_vertices.as_slice())?;
        write_chunk(writer, MOVB, self.visible_blocks.as_slice())?;
        write_chunk(writer, MOLT, self.lights.as_slice())?;
        write_chunk(writer, MODS, self.doodad_sets.as_slice())?;
        write_chunk(writer, MODN, &self.doodad_file_names)?;
        write_chunk(writer, MODD, self.doodads.as_slice())?;
        write_chunk(writer, MFOG, self.fogs.as_slice())?;
        if let Some(planes) = &self.convex_volume_planes {
            write_chunk(writer, MCVP, planes.as_slice())?;
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

    use super::*;

    fn root() -> WmoRoot {
        let doodad = |name_and_flags| WmoDoodad {
            name_and_flags,
            position: C3Vector::new(1.0, 2.0, 3.0),
            rotation: Quaternion {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            },
            scale: 1.0,
            color: 0xFF80_4020,
        };
        WmoRoot {
            version: WMO_VERSION,
            header: RootHeader {
                group_count: 2,
                flags: RootFlags::USE_LIQUID_TYPE_DBC_ID,
                ..Default::default()
            },
            textures: StringBlock::from_strings(&["abc", "tex"]).0,
            materials: vec![WmoMaterial {
                texture_1: 4,
                texture_2: 99,
                ..Default::default()
            }],
            doodad_file_names: StringBlock::from_strings(&["a", "b"]).0,
            doodads: vec![doodad(0x0100_0002), doodad(0), doodad(2)],
            convex_volume_planes: Some(vec![Plane::default()]),
            unknown: vec![(*b"XXXX", vec![1, 2, 3])],
            ..Default::default()
        }
    }

    fn written(root: &WmoRoot) -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        root.wow_write(&mut bytes).unwrap();
        bytes.into_inner()
    }

    #[test]
    fn writes_and_reads_back_the_same_file() {
        let bytes = written(&root());
        assert_eq!(&bytes[0..4], b"REVM");
        let read = WmoRoot::wow_read(&mut Cursor::new(&bytes)).unwrap();
        assert_eq!(read, root());
        assert_eq!(written(&read), bytes);
    }

    #[test]
    fn resolves_names_by_offset() {
        let root = root();
        assert_eq!(root.texture(root.materials[0].texture_1), "tex");
        // Past the end of MOTX is empty.
        assert_eq!(root.texture(root.materials[0].texture_2), "");
        let names = root.doodad_names();
        assert_eq!(names.names, vec!["b", "a"]);
        assert_eq!(names.indexes, vec![0, 1, 0]);
        assert_eq!(root.doodads[0].flags(), 1);
    }

    #[test]
    fn truncated_chunks_are_errors() {
        let mut bytes = written(&root());
        bytes.truncate(40);
        assert!(WmoRoot::wow_read(&mut Cursor::new(&bytes)).is_err());
    }

    #[test]
    fn a_chunk_with_a_partial_record_is_an_error() {
        let mut bytes = Cursor::new(Vec::new());
        write_chunk(&mut bytes, MVER, &WMO_VERSION).unwrap();
        write_chunk(&mut bytes, MOHD, &RootHeader::default()).unwrap();
        write_chunk(&mut bytes, MOPV, &[0u8; 13][..]).unwrap();
        assert!(WmoRoot::wow_read(&mut Cursor::new(bytes.into_inner())).is_err());
    }

    #[test]
    fn other_versions_are_errors() {
        let mut root = root();
        root.version = 14;
        let error = WmoRoot::wow_read(&mut Cursor::new(written(&root))).unwrap_err();
        assert!(error.to_string().contains("14"), "{error}");
    }
}
