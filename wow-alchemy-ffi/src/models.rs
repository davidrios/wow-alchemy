//! M2 and WMO models, as flat buffers ready for mesh upload.

use std::io::Cursor;
use std::ptr;

use wow_alchemy_blp::convert::blp_to_image;
use wow_alchemy_blp::parser::load_blp_from_buf;
use wow_alchemy_blp::types::BlpContent;
use wow_alchemy_data::types::{VWowStructR, WowStructR};
use wow_alchemy_m2::chunks::M2Vertex;
use wow_alchemy_m2::skin::SkinVersion;
use wow_alchemy_m2::{M2Model, Skin};
use wow_alchemy_wmo::render::{RenderGroup, RenderLiquid, RenderRoot};

use crate::{WaBuffer, WaStatus, bytes_arg, guard};

/// Marks a material whose texture the model doesn't name, such as a creature's skin.
pub const WA_NO_TEXTURE: u32 = u32::MAX;

pub const WA_GPU_RGBA8: u32 = 0;
pub const WA_GPU_DXT1: u32 = 1;
pub const WA_GPU_DXT3: u32 = 3;
pub const WA_GPU_DXT5: u32 = 5;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct WaBatch {
    pub index_start: u32,
    pub index_count: u32,
    pub material: u32,
}

/// Pointers are owned by the handle the mesh came from.
#[repr(C)]
pub struct WaMesh {
    pub vertex_count: u32,
    /// 3 floats per vertex, in the file's axes: Z up.
    pub positions: *const f32,
    pub normals: *const f32,
    /// 2 floats per vertex.
    pub tex_coords: *const f32,
    /// 4 bytes per vertex: red, green, blue, alpha. Null when there are none.
    pub colors: *const u8,
    pub index_count: u32,
    pub indices: *const u32,
    pub batch_count: u32,
    pub batches: *const WaBatch,
    /// Triangles to collide with. Empty for M2s, which have wa_m2_collision.
    pub collision_index_count: u32,
    pub collision_indices: *const u32,
}

#[derive(Default)]
struct StoredMesh {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    tex_coords: Vec<[f32; 2]>,
    colors: Vec<[u8; 4]>,
    indices: Vec<u32>,
    batches: Vec<WaBatch>,
    collision_indices: Vec<u32>,
}

impl StoredMesh {
    fn view(&self) -> WaMesh {
        WaMesh {
            vertex_count: self.positions.len() as u32,
            positions: self.positions.as_ptr().cast(),
            normals: self.normals.as_ptr().cast(),
            tex_coords: self.tex_coords.as_ptr().cast(),
            colors: if self.colors.is_empty() {
                ptr::null()
            } else {
                self.colors.as_ptr().cast()
            },
            index_count: self.indices.len() as u32,
            indices: self.indices.as_ptr(),
            batch_count: self.batches.len() as u32,
            batches: self.batches.as_ptr(),
            collision_index_count: self.collision_indices.len() as u32,
            collision_indices: self.collision_indices.as_ptr(),
        }
    }
}

/// Writes a borrowed string to out parameters, or reports it missing.
unsafe fn write_name(name: Option<&String>, out: *mut *const u8, out_len: *mut usize) -> WaStatus {
    match name {
        Some(name) => {
            unsafe {
                out.write(name.as_ptr());
                out_len.write(name.len());
            }
            WaStatus::Ok
        }
        None => WaStatus::NotFound,
    }
}

// ---- M2 ----

#[repr(C)]
#[derive(Clone, Copy)]
pub struct WaM2Material {
    /// For wa_m2_texture_name, or WA_NO_TEXTURE.
    pub texture: u32,
    /// M2 blend mode: 0 opaque, 1 alpha key, 2 alpha, 3 and 4 additive, 5 and 6 modulate.
    pub blend_mode: u32,
    /// M2 render flags: 0x1 unlit, 0x4 two-sided.
    pub flags: u32,
    /// M2 texture flags: 0x1 wraps horizontally, 0x2 vertically.
    pub texture_flags: u32,
}

pub struct WaM2 {
    mesh: StoredMesh,
    collision_positions: Vec<[f32; 3]>,
    collision_indices: Vec<u32>,
    materials: Vec<WaM2Material>,
    textures: Vec<String>,
    bounding_radius: f32,
}

fn store_m2(m2: &[u8], skin: &[u8]) -> Result<WaM2, String> {
    let model = M2Model::wow_read(&mut Cursor::new(m2)).map_err(|e| e.to_string())?;
    let skin = Skin::wow_read(&mut Cursor::new(skin), SkinVersion::V2).map_err(|e| e.to_string())?;
    let md20 = model.md20;

    let mut mesh = StoredMesh {
        positions: Vec::with_capacity(md20.vertices.len()),
        normals: Vec::with_capacity(md20.vertices.len()),
        tex_coords: Vec::with_capacity(md20.vertices.len()),
        ..Default::default()
    };
    let mut bounding_radius = 0f32;
    for vertex in &md20.vertices {
        let M2Vertex {
            position,
            normal,
            tex_coords,
            ..
        } = vertex;
        mesh.positions.push([position.x, position.y, position.z]);
        mesh.normals.push([normal.x, normal.y, normal.z]);
        mesh.tex_coords.push([tex_coords.x, tex_coords.y]);
        let length = position.x * position.x + position.y * position.y + position.z * position.z;
        bounding_radius = bounding_radius.max(length.sqrt());
    }

    let textures = md20
        .textures
        .iter()
        .map(|t| t.data.filename.trim_end_matches('\0').to_string())
        .collect::<Vec<_>>();

    let mut materials = Vec::with_capacity(skin.texture_units.len());
    for unit in &skin.texture_units {
        let submesh = skin
            .submeshes
            .get(unit.skin_section_index as usize)
            .ok_or("a batch names a missing submesh")?;

        // `level` holds the high bits of the start in models too big for 16 bits.
        let start = submesh.triangle_start as usize + ((submesh.level as usize) << 16);
        let count = submesh.triangle_count as usize;
        let triangles = skin
            .triangles
            .get(start..start + count)
            .ok_or("a submesh runs past the skin's triangles")?;

        let index_start = mesh.indices.len() as u32;
        for &triangle in triangles {
            let vertex = *skin
                .indices
                .get(triangle as usize)
                .ok_or("a triangle names a missing vertex")?;
            if vertex as usize >= mesh.positions.len() {
                return Err("a skin names a vertex the model doesn't have".into());
            }
            mesh.indices.push(vertex as u32);
        }

        let material = md20.materials.get(unit.material_index as usize);
        let texture = md20
            .texture_lookup_table
            .get(unit.texture_combo_index as usize)
            .and_then(|&t| usize::try_from(t).ok())
            .filter(|&t| t < textures.len() && !textures[t].is_empty());

        mesh.batches.push(WaBatch {
            index_start,
            index_count: count as u32,
            material: materials.len() as u32,
        });
        materials.push(WaM2Material {
            texture: texture.map_or(WA_NO_TEXTURE, |t| t as u32),
            blend_mode: material.map_or(0, |m| m.blend_mode.bits() as u32),
            flags: material.map_or(0, |m| m.flags.bits() as u32),
            texture_flags: texture.map_or(0, |t| md20.textures[t].header.flags.bits()),
        });
    }

    let collision_positions = md20
        .bounding_vertices
        .iter()
        .map(|v| [v.x, v.y, v.z])
        .collect::<Vec<_>>();
    let collision_indices = md20
        .bounding_triangles
        .iter()
        .map(|&i| i as u32)
        .collect::<Vec<_>>();
    if collision_indices
        .iter()
        .any(|&i| i as usize >= collision_positions.len())
    {
        return Err("a collision triangle names a missing vertex".into());
    }

    Ok(WaM2 {
        mesh,
        collision_positions,
        collision_indices,
        materials,
        textures,
        bounding_radius,
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_m2_open(
    m2: *const u8,
    m2_len: usize,
    skin: *const u8,
    skin_len: usize,
) -> *mut WaM2 {
    guard(ptr::null_mut(), || {
        let m2 = unsafe { bytes_arg(m2, m2_len) }?;
        let skin = unsafe { bytes_arg(skin, skin_len) }?;
        Ok(Box::into_raw(Box::new(store_m2(m2, skin)?)))
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_m2_free(model: *mut WaM2) {
    if !model.is_null() {
        drop(unsafe { Box::from_raw(model) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_m2_mesh(model: *const WaM2, out: *mut WaMesh) {
    unsafe { out.write((*model).mesh.view()) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_m2_bounding_radius(model: *const WaM2) -> f32 {
    unsafe { &*model }.bounding_radius
}

/// The model's collision mesh, which has its own vertices. Owned by model, and empty for
/// models that have none.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_m2_collision(
    model: *const WaM2,
    positions: *mut *const f32,
    position_count: *mut u32,
    indices: *mut *const u32,
    index_count: *mut u32,
) {
    let model = unsafe { &*model };
    unsafe {
        positions.write(model.collision_positions.as_ptr().cast());
        position_count.write(model.collision_positions.len() as u32);
        indices.write(model.collision_indices.as_ptr());
        index_count.write(model.collision_indices.len() as u32);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_m2_material(
    model: *const WaM2,
    index: u32,
    out: *mut WaM2Material,
) -> WaStatus {
    match unsafe { &*model }.materials.get(index as usize) {
        Some(material) => {
            unsafe { out.write(*material) };
            WaStatus::Ok
        }
        None => WaStatus::NotFound,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_m2_texture_name(
    model: *const WaM2,
    index: u32,
    out: *mut *const u8,
    out_len: *mut usize,
) -> WaStatus {
    unsafe { write_name((&*model).textures.get(index as usize), out, out_len) }
}

// ---- WMO ----

#[repr(C)]
#[derive(Clone, Copy)]
pub struct WaWmoMaterial {
    /// 0x1 unlit, 0x4 two-sided.
    pub flags: u32,
    /// 0 opaque, 1 alpha key, 2 alpha, 3 additive, 4 and 5 modulate.
    pub blend_mode: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct WaWmoDoodadSet {
    pub start: u32,
    pub count: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct WaWmoDoodad {
    /// For wa_wmo_doodad_name.
    pub name: u32,
    pub position: [f32; 3],
    /// X, Y, Z, W.
    pub rotation: [f32; 4],
    pub scale: f32,
}

pub struct WaWmo {
    root: RenderRoot,
}

pub struct WaWmoGroup {
    mesh: StoredMesh,
    flags: u32,
    liquid: Option<RenderLiquid>,
    group_liquid: u32,
}

/// A group's liquid surface, on a grid in the WMO's own space.
#[repr(C)]
pub struct WaWmoLiquid {
    pub vertices_x: u32,
    pub vertices_y: u32,
    /// The grid's corner: X, Y, Z.
    pub corner: [f32; 3],
    /// vertices_x * vertices_y heights, row by row.
    pub heights: *const f32,
    /// (vertices_x - 1) * (vertices_y - 1) bytes, row by row: nonzero when the cell has liquid.
    pub cells: *const u8,
    /// Row in LiquidType.dbc.
    pub liquid_type: u32,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_wmo_open(data: *const u8, len: usize) -> *mut WaWmo {
    guard(ptr::null_mut(), || {
        let bytes = unsafe { bytes_arg(data, len) }?;
        let root = RenderRoot::parse(bytes).map_err(|e| e.to_string())?;
        Ok(Box::into_raw(Box::new(WaWmo { root })))
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_wmo_free(wmo: *mut WaWmo) {
    if !wmo.is_null() {
        drop(unsafe { Box::from_raw(wmo) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_wmo_flags(wmo: *const WaWmo) -> u32 {
    unsafe { &*wmo }.root.flags
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_wmo_group_count(wmo: *const WaWmo) -> u32 {
    unsafe { &*wmo }.root.group_count
}

/// Writes 6 floats: the minimum corner, then the maximum, in the WMO's own space.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_wmo_bounds(wmo: *const WaWmo, out: *mut f32) {
    let root = &unsafe { &*wmo }.root;
    let out = unsafe { std::slice::from_raw_parts_mut(out, 6) };
    out[..3].copy_from_slice(&root.bounds_min);
    out[3..].copy_from_slice(&root.bounds_max);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_wmo_material_count(wmo: *const WaWmo) -> u32 {
    unsafe { &*wmo }.root.materials.len() as u32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_wmo_material(
    wmo: *const WaWmo,
    index: u32,
    out: *mut WaWmoMaterial,
    texture: *mut *const u8,
    texture_len: *mut usize,
) -> WaStatus {
    match unsafe { &*wmo }.root.materials.get(index as usize) {
        Some(material) => {
            unsafe {
                out.write(WaWmoMaterial {
                    flags: material.flags,
                    blend_mode: material.blend_mode,
                });
                write_name(Some(&material.texture), texture, texture_len)
            }
        }
        None => WaStatus::NotFound,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_wmo_doodad_set_count(wmo: *const WaWmo) -> u32 {
    unsafe { &*wmo }.root.doodad_sets.len() as u32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_wmo_doodad_set(
    wmo: *const WaWmo,
    index: u32,
    out: *mut WaWmoDoodadSet,
) -> WaStatus {
    match unsafe { &*wmo }.root.doodad_sets.get(index as usize) {
        Some(set) => {
            unsafe {
                out.write(WaWmoDoodadSet {
                    start: set.start,
                    count: set.count,
                })
            };
            WaStatus::Ok
        }
        None => WaStatus::NotFound,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_wmo_doodad_count(wmo: *const WaWmo) -> u32 {
    unsafe { &*wmo }.root.doodads.len() as u32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_wmo_doodad(
    wmo: *const WaWmo,
    index: u32,
    out: *mut WaWmoDoodad,
) -> WaStatus {
    match unsafe { &*wmo }.root.doodads.get(index as usize) {
        Some(doodad) => {
            unsafe {
                out.write(WaWmoDoodad {
                    name: doodad.name,
                    position: doodad.position,
                    rotation: doodad.rotation,
                    scale: doodad.scale,
                })
            };
            WaStatus::Ok
        }
        None => WaStatus::NotFound,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_wmo_doodad_name(
    wmo: *const WaWmo,
    index: u32,
    out: *mut *const u8,
    out_len: *mut usize,
) -> WaStatus {
    unsafe { write_name((&*wmo).root.doodad_names.get(index as usize), out, out_len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_wmo_group_open(data: *const u8, len: usize) -> *mut WaWmoGroup {
    guard(ptr::null_mut(), || {
        let bytes = unsafe { bytes_arg(data, len) }?;
        let mut group = RenderGroup::parse(bytes).map_err(|e| e.to_string())?;
        let liquid = group.liquid.take();
        let widen = |indices: &[u16]| indices.iter().map(|&i| i as u32).collect::<Vec<_>>();
        let mesh = StoredMesh {
            indices: widen(&group.indices),
            collision_indices: widen(&group.collision_indices),
            batches: group
                .batches
                .iter()
                .map(|b| WaBatch {
                    index_start: b.index_start,
                    index_count: b.index_count,
                    material: b.material,
                })
                .collect(),
            positions: group.positions,
            normals: group.normals,
            tex_coords: group.tex_coords,
            colors: group.colors,
        };
        Ok(Box::into_raw(Box::new(WaWmoGroup {
            mesh,
            flags: group.flags,
            liquid,
            group_liquid: group.group_liquid,
        })))
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_wmo_group_free(group: *mut WaWmoGroup) {
    if !group.is_null() {
        drop(unsafe { Box::from_raw(group) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_wmo_group_mesh(group: *const WaWmoGroup, out: *mut WaMesh) {
    unsafe { out.write((*group).mesh.view()) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_wmo_group_flags(group: *const WaWmoGroup) -> u32 {
    unsafe { &*group }.flags
}

/// Whether the group has liquid. `root_flags` comes from wa_wmo_flags, because the root decides
/// how the group's liquid type reads. The pointers stay valid until the group is freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_wmo_group_liquid(
    group: *const WaWmoGroup,
    root_flags: u32,
    out: *mut WaWmoLiquid,
) -> WaStatus {
    let group = unsafe { &*group };
    match &group.liquid {
        Some(liquid) => {
            unsafe {
                out.write(WaWmoLiquid {
                    vertices_x: liquid.vertices_x,
                    vertices_y: liquid.vertices_y,
                    corner: liquid.corner,
                    heights: liquid.heights.as_ptr(),
                    cells: liquid.cells.as_ptr(),
                    liquid_type: liquid.liquid_type(root_flags, group.flags, group.group_liquid),
                })
            };
            WaStatus::Ok
        }
        None => WaStatus::NotFound,
    }
}

// ---- Textures ----

#[repr(C)]
pub struct WaGpuTexture {
    pub pixels: WaBuffer,
    /// WA_GPU_RGBA8 holds one level; the others hold a chain down to 1 pixel.
    pub format: u32,
    pub width: u32,
    pub height: u32,
    pub mip_count: u32,
}

/// A texture's whole compressed chain, if it has one the GPU can use as it is.
fn compressed_chain(data: &[u8]) -> Result<Option<WaGpuTexture>, String> {
    let blp = load_blp_from_buf(data).map_err(|e| e.to_string())?;
    let (images, format, block_bytes) = match &blp.content {
        // One-bit alpha needs decoding, because GPU DXT1 formats may ignore it.
        BlpContent::Dxt1(c) if blp.header.alpha_bits() == 0 => (&c.images, WA_GPU_DXT1, 8),
        BlpContent::Dxt3(c) => (&c.images, WA_GPU_DXT3, 16),
        BlpContent::Dxt5(c) => (&c.images, WA_GPU_DXT5, 16),
        _ => return Ok(None),
    };

    let (width, height) = (blp.header.width, blp.header.height);
    if !width.is_power_of_two() || !height.is_power_of_two() {
        return Ok(None);
    }

    let levels = width.max(height).trailing_zeros() + 1;
    let mut pixels = Vec::new();
    for level in 0..levels {
        let blocks = ((width >> level).max(1) as usize).div_ceil(4)
            * ((height >> level).max(1) as usize).div_ceil(4);
        match images
            .get(level as usize)
            .and_then(|image| image.content.get(..blocks * block_bytes))
        {
            Some(data) => pixels.extend_from_slice(data),
            None => return Ok(None),
        }
    }

    Ok(Some(WaGpuTexture {
        pixels: WaBuffer::from_vec(pixels),
        format,
        width,
        height,
        mip_count: levels,
    }))
}

/// A texture for upload: the file's own compressed mipmaps when possible, or else
/// the first level decoded to RGBA8 for the caller to build mipmaps from.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_blp_to_gpu(data: *const u8, len: usize, out: *mut WaGpuTexture) -> WaStatus {
    guard(WaStatus::Error, || {
        let bytes = unsafe { bytes_arg(data, len) }?;
        let texture = match compressed_chain(bytes)? {
            Some(texture) => texture,
            None => {
                let blp = load_blp_from_buf(bytes).map_err(|e| e.to_string())?;
                let image = blp_to_image(&blp, 0).map_err(|e| e.to_string())?.into_rgba8();
                let (width, height) = image.dimensions();
                WaGpuTexture {
                    pixels: WaBuffer::from_vec(image.into_raw()),
                    format: WA_GPU_RGBA8,
                    width,
                    height,
                    mip_count: 1,
                }
            }
        };
        unsafe { out.write(texture) };
        Ok(WaStatus::Ok)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wa_last_error;

    #[test]
    fn invalid_files_report_errors() {
        let bytes = b"not game data";

        assert!(unsafe { wa_m2_open(bytes.as_ptr(), bytes.len(), bytes.as_ptr(), bytes.len()) }.is_null());
        assert!(!wa_last_error().is_null());
        assert!(unsafe { wa_wmo_open(bytes.as_ptr(), bytes.len()) }.is_null());
        assert!(unsafe { wa_wmo_group_open(bytes.as_ptr(), bytes.len()) }.is_null());
    }

    #[test]
    fn handles_can_move_between_threads() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<WaM2>();
        assert_send_sync::<WaWmo>();
    }
}
