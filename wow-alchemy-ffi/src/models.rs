//! M2 and WMO models, as flat buffers ready for mesh upload.

use std::io::Cursor;
use std::ptr;

use wow_alchemy_blp::convert::blp_to_image;
use wow_alchemy_blp::parser::load_blp_from_buf;
use wow_alchemy_blp::types::BlpContent;
use wow_alchemy_data::prelude::{WowHeaderR, WowHeaderW};
use wow_alchemy_data::types::{C3Vector, VWowStructR, WowStructR};
use wow_alchemy_m2::chunks::M2Vertex;
use wow_alchemy_m2::chunks::animation::{
    M2AnimationBlending, M2AnimationFlags, M2AnimationTiming, M2AnimationTrackHeader, TrackArray,
};
use wow_alchemy_m2::chunks::bone::{M2BoneHeader, M2BoneRotationHeader};
use wow_alchemy_m2::chunks::color_animation::M2ColorAnimationHeader;
use wow_alchemy_m2::chunks::texture_transform::M2TextureTransformHeader;
use wow_alchemy_m2::chunks::transparency_animation::M2TransparencyAnimationHeader;
use wow_alchemy_m2::header::{M2ModelFlags, M2TextureCombinerCombos};
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
    /// M2 texture type, even without a name: 0 is the named file, and others are chosen at
    /// runtime, such as 1 for a character's body and 11 to 13 for a creature's skins.
    pub texture_type: u32,
    /// The batch's geoset id, for models that show only some of theirs.
    pub geoset: u32,
    /// The batch as stored, for working out how it combines its textures.
    pub shader_id: u32,
    /// How many textures it uses, from 1.
    pub texture_count: u32,
    pub material_index: u32,
    pub material_layer: u32,
    /// M2 batch flags: 0x10 means its texture doesn't animate.
    pub batch_flags: u32,
    /// For wa_m2_texture_name and wa_m2_texture_type, even when the name is empty, or WA_NO_TEXTURE.
    pub textures: [u32; 2],
    /// Which UV set each texture uses: 0, 1, or -1 for a sphere map.
    pub tex_coords: [i32; 2],
    /// Indexes for wa_m2_effects_track, or -1.
    pub texture_transforms: [i32; 2],
    pub color: i32,
    pub texture_weight: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct WaM2Bone {
    /// A bone index, or -1.
    pub parent: i32,
    /// Names bones games look for, such as the head, or -1.
    pub key_bone: i32,
    pub flags: u32,
    pub pivot: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct WaM2Sequence {
    /// An AnimationData row.
    pub id: u32,
    pub variation: u32,
    pub duration_ms: u32,
    /// Yards per second that its feet cover.
    pub movement_speed: f32,
    /// 0x20 means its keys are in the M2, and otherwise in its .anim file. 0x40 means it
    /// has no keys of its own and uses the alias_next sequence's.
    pub flags: u32,
    /// How often this variation plays, out of 32767 across a sequence's variations.
    pub frequency: i32,
    /// The sequence index of the next variation of this id, or -1.
    pub next_variation: i32,
    pub alias_next: u32,
    pub blend_time_ms: u32,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    pub bounds_radius: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct WaM2Attachment {
    /// Where it goes: 5 and 6 are the shoulders, and 11 the helmet.
    pub id: u32,
    pub bone: u32,
    pub position: [f32; 3],
}

/// One bone's keys for one sequence. Pointers are owned by the clip.
#[repr(C)]
pub struct WaM2Track {
    /// 0 steps between keys, 1 is linear, and 2 and 3 are curves whose tangents aren't given.
    pub interpolation: u32,
    /// A wa_m2_global_sequences index that drives this track instead of the sequence, or -1.
    pub global_sequence: i32,
    pub key_count: u32,
    /// In milliseconds.
    pub times: *const u32,
    /// 3 floats per key, or 4 for rotations: X, Y, Z, W. Rotations are decompressed.
    pub values: *const f32,
}

pub const WA_M2_TRACK_TRANSLATION: u32 = 0;
pub const WA_M2_TRACK_ROTATION: u32 = 1;
pub const WA_M2_TRACK_SCALE: u32 = 2;

#[derive(Default)]
struct StoredTrack {
    interpolation: u32,
    global_sequence: i32,
    times: Vec<u32>,
    values: Vec<f32>,
}

/// One sequence's keys for every bone, and when each of the model's events fires in it.
pub struct WaM2Clip {
    tracks: Vec<[StoredTrack; 3]>,
    event_times: Vec<Vec<u32>>,
}

/// Something a model asks for at moments in its sequences, such as a footstep.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct WaM2Event {
    /// 4 characters, usually $ and 3 more, such as $FD1.
    pub identifier: [u8; 4],
    pub data: u32,
    pub bone: u32,
    pub position: [f32; 3],
}

pub struct WaM2 {
    mesh: StoredMesh,
    collision_positions: Vec<[f32; 3]>,
    collision_indices: Vec<u32>,
    materials: Vec<WaM2Material>,
    textures: Vec<String>,
    bounding_radius: f32,
    bone_weights: Vec<[u8; 4]>,
    bone_indices: Vec<[u8; 4]>,
    bones: Vec<WaM2Bone>,
    bone_headers: Vec<M2BoneHeader>,
    sequences: Vec<WaM2Sequence>,
    global_sequences: Vec<u32>,
    attachments: Vec<WaM2Attachment>,
    tex_coords2: Vec<[f32; 2]>,
    events: Vec<WaM2Event>,
    event_tracks: Vec<TrackArray<u32>>,
    texture_types: Vec<u32>,
    texture_wrap_flags: Vec<u32>,
    combiner_overrides: Vec<u16>,
    uses_combiner_overrides: bool,
    color_headers: Vec<M2ColorAnimationHeader>,
    weight_headers: Vec<M2TransparencyAnimationHeader>,
    transform_headers: Vec<M2TextureTransformHeader>,
    first_sequence_in_file: bool,
    /// Track headers point back into the file, so clips read it again.
    file: Vec<u8>,
}

fn store_m2(m2: &[u8], skin: &[u8]) -> Result<WaM2, String> {
    let model = M2Model::wow_read(&mut Cursor::new(m2)).map_err(|e| e.to_string())?;
    let skin =
        Skin::wow_read(&mut Cursor::new(skin), SkinVersion::V2).map_err(|e| e.to_string())?;
    let md20 = model.md20;

    let mut mesh = StoredMesh {
        positions: Vec::with_capacity(md20.vertices.len()),
        normals: Vec::with_capacity(md20.vertices.len()),
        tex_coords: Vec::with_capacity(md20.vertices.len()),
        ..Default::default()
    };
    let mut bounding_radius = 0f32;
    let mut bone_weights = Vec::with_capacity(md20.vertices.len());
    let mut bone_indices = Vec::with_capacity(md20.vertices.len());
    let mut tex_coords2 = Vec::with_capacity(md20.vertices.len());
    for vertex in &md20.vertices {
        let M2Vertex {
            position,
            normal,
            tex_coords,
            ..
        } = vertex;
        tex_coords2.push([vertex.tex_coords2.x, vertex.tex_coords2.y]);
        bone_weights.push(vertex.bone_weights);
        bone_indices.push(vertex.bone_indices);
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
        let looked_up = md20
            .texture_lookup_table
            .get(unit.texture_combo_index as usize)
            .and_then(|&t| usize::try_from(t).ok())
            .filter(|&t| t < textures.len());
        let texture = looked_up.filter(|&t| !textures[t].is_empty());

        mesh.batches.push(WaBatch {
            index_start,
            index_count: count as u32,
            material: materials.len() as u32,
        });
        materials.push(WaM2Material {
            texture: texture.map_or(WA_NO_TEXTURE, |t| t as u32),
            blend_mode: material.map_or(0, |m| m.blend_mode.bits() as u32),
            flags: material.map_or(0, |m| m.flags.bits() as u32),
            texture_flags: looked_up.map_or(0, |t| md20.textures[t].header.flags.bits()),
            texture_type: looked_up.map_or(0, |t| md20.textures[t].header.texture_type as u32),
            geoset: submesh.id as u32,
            shader_id: unit.shader_id as u32,
            texture_count: unit.texture_count as u32,
            material_index: unit.material_index as u32,
            material_layer: unit.material_layer as u32,
            batch_flags: unit.flags.bits() as u32,
            textures: [0, 1].map(|i| {
                md20.texture_lookup_table
                    .get(unit.texture_combo_index as usize + i)
                    .and_then(|&t| usize::try_from(t).ok())
                    .filter(|&t| t < textures.len())
                    .map_or(WA_NO_TEXTURE, |t| t as u32)
            }),
            tex_coords: [0, 1].map(|i| {
                md20.texture_mapping_lookup_table
                    .get(unit.texture_coord_combo_index as usize + i)
                    .map_or(0, |&c| c as i32)
            }),
            texture_transforms: [0, 1].map(|i| {
                md20.texture_animation_lookup
                    .get(unit.texture_transform_combo_index as usize + i)
                    .map_or(-1, |&t| t as i32)
            }),
            color: unit.color_index as i16 as i32,
            texture_weight: md20
                .transparency_lookup_table
                .get(unit.texture_weight_combo_index as usize)
                .map_or(-1, |&w| w as i32),
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

    let bones = md20
        .bones
        .iter()
        .map(|bone| WaM2Bone {
            parent: bone.header.parent_bone as i32,
            key_bone: bone.header.bone_id,
            flags: bone.header.flags.bits(),
            pivot: [
                bone.header.pivot.x,
                bone.header.pivot.y,
                bone.header.pivot.z,
            ],
        })
        .collect();
    let bone_headers = md20.bones.iter().map(|bone| bone.header.clone()).collect();

    let sequences = md20
        .animations
        .iter()
        .map(|sequence| {
            let duration_ms = match sequence.timing {
                M2AnimationTiming::Duration(duration) => duration,
                M2AnimationTiming::StartEnd(start, end) => end.saturating_sub(start),
            };
            let blend_time_ms = match sequence.blending {
                M2AnimationBlending::Time(time) => time,
                M2AnimationBlending::InOut(blend_in, _) => blend_in as u32,
            };
            let bounds = &sequence.bounding_box;
            WaM2Sequence {
                id: sequence.animation_id as u32,
                variation: sequence.sub_animation_id as u32,
                duration_ms,
                movement_speed: sequence.movement_speed,
                flags: sequence.flags.bits(),
                frequency: sequence.frequency as i32,
                next_variation: sequence.next_animation as i32,
                alias_next: sequence.next_alias as u32,
                blend_time_ms,
                bounds_min: [bounds.min.x, bounds.min.y, bounds.min.z],
                bounds_max: [bounds.max.x, bounds.max.y, bounds.max.z],
                bounds_radius: sequence.bounding_radius,
            }
        })
        .collect();

    let attachments = md20
        .attachments
        .iter()
        .map(|attachment| WaM2Attachment {
            id: attachment.header.id as u32,
            bone: attachment.header.bone_index as u32,
            position: [
                attachment.header.position.x,
                attachment.header.position.y,
                attachment.header.position.z,
            ],
        })
        .collect();

    let global_sequences = md20.global_sequences.clone();
    let texture_types = md20
        .textures
        .iter()
        .map(|t| t.header.texture_type as u32)
        .collect();
    let texture_wrap_flags = md20
        .textures
        .iter()
        .map(|t| t.header.flags.bits())
        .collect();
    let uses_combiner_overrides = md20
        .header
        .flags
        .contains(M2ModelFlags::USE_TEXTURE_COMBINERS);
    let combiner_overrides = match &md20.texture_combiner_combos {
        M2TextureCombinerCombos::Some(values) => values.clone(),
        M2TextureCombinerCombos::None => Vec::new(),
    };
    let color_headers = md20
        .color_animations
        .iter()
        .map(|c| c.header.clone())
        .collect();
    let weight_headers = md20
        .texture_weights
        .iter()
        .map(|w| w.header.clone())
        .collect();
    let transform_headers = md20
        .texture_transforms
        .iter()
        .map(|t| t.header.clone())
        .collect();
    let events = md20
        .events
        .iter()
        .map(|event| WaM2Event {
            identifier: event.header.identifier.into(),
            data: event.header.data,
            bone: event.header.bone_index,
            position: [
                event.header.position.x,
                event.header.position.y,
                event.header.position.z,
            ],
        })
        .collect();
    let event_tracks = md20
        .events
        .iter()
        .map(|event| event.header.enabled.timestamps.clone())
        .collect();
    let first_sequence_in_file = md20
        .animations
        .first()
        .is_some_and(|s| s.flags.contains(M2AnimationFlags::PRIMARY_BONE_SEQUENCE));

    Ok(WaM2 {
        mesh,
        collision_positions,
        collision_indices,
        materials,
        textures,
        bounding_radius,
        bone_weights,
        bone_indices,
        bones,
        bone_headers,
        sequences,
        global_sequences,
        attachments,
        tex_coords2,
        events,
        event_tracks,
        texture_types,
        texture_wrap_flags,
        combiner_overrides,
        uses_combiner_overrides,
        color_headers,
        weight_headers,
        transform_headers,
        first_sequence_in_file,
        file: m2.to_vec(),
    })
}

/// A packed rotation component, as WoWModelViewer and the wiki give it.
fn unpack_rotation(value: i16) -> f32 {
    let value = value as i32;
    (if value < 0 {
        value + 32768
    } else {
        value - 32767
    }) as f32
        / 32767.0
}

fn store_track<T: WowHeaderR + WowHeaderW>(
    header: &M2AnimationTrackHeader<T>,
    sequence: usize,
    file: &[u8],
    keys: &[u8],
    flatten: impl Fn(&T, &mut Vec<f32>),
) -> Result<StoredTrack, String> {
    // Global sequences aren't tied to a sequence, and keep their keys in the first slot.
    let (slot, keys) = if header.global_sequence >= 0 {
        (0, file)
    } else {
        (sequence, keys)
    };
    let (times, values) = header
        .read_sequence_keys(slot, &mut Cursor::new(file), &mut Cursor::new(keys))
        .map_err(|e| e.to_string())?;
    if times.len() != values.len() {
        return Err("a track's times and values differ in length".into());
    }

    let mut flat = Vec::new();
    for value in &values {
        flatten(value, &mut flat);
    }

    Ok(StoredTrack {
        interpolation: header.interpolation_type as u32,
        global_sequence: header.global_sequence as i32,
        times,
        values: flat,
    })
}

fn store_clip(model: &WaM2, sequence: usize, keys: &[u8]) -> Result<WaM2Clip, String> {
    let vector = |v: &C3Vector, out: &mut Vec<f32>| out.extend([v.x, v.y, v.z]);
    let mut tracks = Vec::with_capacity(model.bone_headers.len());
    for bone in &model.bone_headers {
        let rotation = match &bone.rotation {
            M2BoneRotationHeader::Later(track) => {
                store_track(track, sequence, &model.file, keys, |q, out| {
                    out.extend([q.x, q.y, q.z, q.w].map(unpack_rotation))
                })?
            }
            M2BoneRotationHeader::Vanilla(track) => {
                store_track(track, sequence, &model.file, keys, |q, out| {
                    out.extend([q.x, q.y, q.z, q.w])
                })?
            }
        };
        tracks.push([
            store_track(&bone.position, sequence, &model.file, keys, vector)?,
            rotation,
            store_track(&bone.scaling, sequence, &model.file, keys, vector)?,
        ]);
    }

    let mut event_times = Vec::with_capacity(model.event_tracks.len());
    for track in &model.event_tracks {
        let times = match track {
            TrackArray::Multiple(outer) if (sequence as u32) < outer.count => {
                let inner = outer
                    .wow_read_to_vec(&mut Cursor::new(&model.file))
                    .map_err(|e| e.to_string())?;
                inner[sequence]
                    .wow_read_to_vec(&mut Cursor::new(keys))
                    .map_err(|e| e.to_string())?
            }
            _ => Vec::new(),
        };
        event_times.push(times);
    }

    Ok(WaM2Clip {
        tracks,
        event_times,
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

/// One set of tracks that change how a batch looks, as read from the first sequence and from
/// global sequences. Tracks whose keys are outside the M2 come back empty.
pub struct WaM2Effects {
    tracks: Vec<Vec<Vec<StoredTrack>>>,
}

pub const WA_M2_EFFECT_COLOR: u32 = 0;
pub const WA_M2_EFFECT_WEIGHT: u32 = 1;
pub const WA_M2_EFFECT_TRANSFORM: u32 = 2;

fn effect_track<T: WowHeaderR + WowHeaderW>(
    header: &M2AnimationTrackHeader<T>,
    model: &WaM2,
    flatten: impl Fn(&T, &mut Vec<f32>),
) -> StoredTrack {
    if header.global_sequence < 0 && !model.first_sequence_in_file {
        return StoredTrack::default();
    }

    store_track(header, 0, &model.file, &model.file, flatten).unwrap_or_default()
}

fn store_effects(model: &WaM2) -> WaM2Effects {
    let vector = |v: &C3Vector, out: &mut Vec<f32>| out.extend([v.x, v.y, v.z]);
    let fixed = |v: &u16, out: &mut Vec<f32>| out.push(*v as f32 / 32767.0);
    let colors = model
        .color_headers
        .iter()
        .map(|c| {
            vec![
                effect_track(&c.color, model, |v, out| out.extend([v.r, v.g, v.b])),
                effect_track(&c.alpha, model, fixed),
            ]
        })
        .collect();
    let weights = model
        .weight_headers
        .iter()
        .map(|w| vec![effect_track(&w.alpha, model, fixed)])
        .collect();
    let transforms = model
        .transform_headers
        .iter()
        .map(|t| {
            vec![
                effect_track(&t.translation, model, vector),
                effect_track(&t.rotation, model, |q, out| {
                    out.extend([q.x, q.y, q.z, q.w])
                }),
                effect_track(&t.scaling, model, vector),
            ]
        })
        .collect();

    WaM2Effects {
        tracks: vec![colors, weights, transforms],
    }
}

/// 2 floats per vertex, owned by model.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_m2_tex_coords2(model: *const WaM2) -> *const f32 {
    unsafe { &*model }.tex_coords2.as_ptr().cast()
}

/// An M2 texture type and its flags, which replaceable textures have too.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_m2_texture_type(
    model: *const WaM2,
    index: u32,
    texture_type: *mut u32,
    flags: *mut u32,
) -> WaStatus {
    let model = unsafe { &*model };
    match model.texture_types.get(index as usize) {
        Some(&value) => {
            unsafe {
                texture_type.write(value);
                flags.write(model.texture_wrap_flags[index as usize]);
            }
            WaStatus::Ok
        }
        None => WaStatus::NotFound,
    }
}

/// How a model's batches combine their textures, when its flag 0x8 says it has a table:
/// each batch's shader_id indexes it. Returns 0 without one, and otherwise 1. Owned by model.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_m2_combiner_overrides(
    model: *const WaM2,
    out: *mut *const u16,
    count: *mut u32,
) -> u32 {
    let model = unsafe { &*model };
    unsafe {
        out.write(model.combiner_overrides.as_ptr());
        count.write(model.combiner_overrides.len() as u32);
    }
    model.uses_combiner_overrides as u32
}

/// The length of the first sequence in milliseconds, which effect tracks not on a global
/// sequence loop over. 0 for a model without sequences.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_m2_first_sequence_ms(model: *const WaM2) -> u32 {
    unsafe { &*model }
        .sequences
        .first()
        .map_or(0, |s| s.duration_ms)
}

/// Never fails, because a track that can't be read comes back empty.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_m2_effects_open(model: *const WaM2) -> *mut WaM2Effects {
    guard(ptr::null_mut(), || {
        Ok(Box::into_raw(Box::new(store_effects(unsafe { &*model }))))
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_m2_effects_free(effects: *mut WaM2Effects) {
    if !effects.is_null() {
        drop(unsafe { Box::from_raw(effects) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_m2_effects_count(effects: *const WaM2Effects, family: u32) -> u32 {
    unsafe { &*effects }
        .tracks
        .get(family as usize)
        .map_or(0, |f| f.len() as u32)
}

/// family is a WA_M2_EFFECT_ value. kind is, for colors, 0 for RGB floats and 1 for an alpha
/// float; for weights, 0 for a float; and for transforms, 0 for translation, 1 for a rotation
/// of X, Y, Z, W, and 2 for scale. Alphas and weights run from 0 to 1.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_m2_effects_track(
    effects: *const WaM2Effects,
    family: u32,
    index: u32,
    kind: u32,
    out: *mut WaM2Track,
) -> WaStatus {
    let track = unsafe { &*effects }
        .tracks
        .get(family as usize)
        .and_then(|f| f.get(index as usize))
        .and_then(|e| e.get(kind as usize));
    match track {
        Some(track) => {
            unsafe {
                out.write(WaM2Track {
                    interpolation: track.interpolation,
                    global_sequence: track.global_sequence,
                    key_count: track.times.len() as u32,
                    times: track.times.as_ptr(),
                    values: track.values.as_ptr(),
                })
            };
            WaStatus::Ok
        }
        None => WaStatus::NotFound,
    }
}

/// 2 pointers to 4 bytes per vertex, owned by model. Indices are into the model's bones,
/// not through a lookup table. Weights add up to 255.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_m2_vertex_bones(
    model: *const WaM2,
    weights: *mut *const u8,
    indices: *mut *const u8,
) {
    let model = unsafe { &*model };
    unsafe {
        weights.write(model.bone_weights.as_ptr().cast());
        indices.write(model.bone_indices.as_ptr().cast());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_m2_bone_count(model: *const WaM2) -> u32 {
    unsafe { &*model }.bones.len() as u32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_m2_bone(
    model: *const WaM2,
    index: u32,
    out: *mut WaM2Bone,
) -> WaStatus {
    match unsafe { &*model }.bones.get(index as usize) {
        Some(bone) => {
            unsafe { out.write(*bone) };
            WaStatus::Ok
        }
        None => WaStatus::NotFound,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_m2_sequence_count(model: *const WaM2) -> u32 {
    unsafe { &*model }.sequences.len() as u32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_m2_sequence(
    model: *const WaM2,
    index: u32,
    out: *mut WaM2Sequence,
) -> WaStatus {
    match unsafe { &*model }.sequences.get(index as usize) {
        Some(sequence) => {
            unsafe { out.write(*sequence) };
            WaStatus::Ok
        }
        None => WaStatus::NotFound,
    }
}

/// Each global sequence's length in milliseconds. Owned by model.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_m2_global_sequences(
    model: *const WaM2,
    out: *mut *const u32,
    count: *mut u32,
) {
    let model = unsafe { &*model };
    unsafe {
        out.write(model.global_sequences.as_ptr());
        count.write(model.global_sequences.len() as u32);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_m2_attachment_count(model: *const WaM2) -> u32 {
    unsafe { &*model }.attachments.len() as u32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_m2_attachment(
    model: *const WaM2,
    index: u32,
    out: *mut WaM2Attachment,
) -> WaStatus {
    match unsafe { &*model }.attachments.get(index as usize) {
        Some(attachment) => {
            unsafe { out.write(*attachment) };
            WaStatus::Ok
        }
        None => WaStatus::NotFound,
    }
}

/// Reads a sequence's keys. A sequence stored in the M2 passes NULL for anim. Otherwise anim is
/// the file <model><id:04>-<variation:02>.anim. An alias has no keys, so pass its target. Returns
/// NULL on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_m2_clip_open(
    model: *const WaM2,
    sequence: u32,
    anim: *const u8,
    anim_len: usize,
) -> *mut WaM2Clip {
    guard(ptr::null_mut(), || {
        let model = unsafe { &*model };
        if sequence as usize >= model.sequences.len() {
            return Err("no such sequence".to_string());
        }

        let keys = if anim.is_null() {
            model.file.as_slice()
        } else {
            unsafe { bytes_arg(anim, anim_len) }?
        };
        Ok(Box::into_raw(Box::new(store_clip(
            model,
            sequence as usize,
            keys,
        )?)))
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_m2_clip_free(clip: *mut WaM2Clip) {
    if !clip.is_null() {
        drop(unsafe { Box::from_raw(clip) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_m2_event_count(model: *const WaM2) -> u32 {
    unsafe { &*model }.events.len() as u32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_m2_event(
    model: *const WaM2,
    index: u32,
    out: *mut WaM2Event,
) -> WaStatus {
    match unsafe { &*model }.events.get(index as usize) {
        Some(event) => {
            unsafe { out.write(*event) };
            WaStatus::Ok
        }
        None => WaStatus::NotFound,
    }
}

/// When an event fires in the clip's sequence, in milliseconds. Owned by clip.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_m2_clip_event_times(
    clip: *const WaM2Clip,
    event: u32,
    out: *mut *const u32,
    count: *mut u32,
) -> WaStatus {
    match unsafe { &*clip }.event_times.get(event as usize) {
        Some(times) => {
            unsafe {
                out.write(times.as_ptr());
                count.write(times.len() as u32);
            }
            WaStatus::Ok
        }
        None => WaStatus::NotFound,
    }
}

/// kind is a WA_M2_TRACK_ value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_m2_clip_track(
    clip: *const WaM2Clip,
    bone: u32,
    kind: u32,
    out: *mut WaM2Track,
) -> WaStatus {
    let track = unsafe { &*clip }
        .tracks
        .get(bone as usize)
        .and_then(|tracks| tracks.get(kind as usize));
    match track {
        Some(track) => {
            unsafe {
                out.write(WaM2Track {
                    interpolation: track.interpolation,
                    global_sequence: track.global_sequence,
                    key_count: track.times.len() as u32,
                    times: track.times.as_ptr(),
                    values: track.values.as_ptr(),
                })
            };
            WaStatus::Ok
        }
        None => WaStatus::NotFound,
    }
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
    /// Its baked light, as stored: 0xAARRGGBB.
    pub color: u32,
}

pub struct WaWmo {
    root: RenderRoot,
}

pub struct WaWmoGroup {
    mesh: StoredMesh,
    doodad_refs: Vec<u16>,
    bounds: [f32; 6],
    portal_start: u32,
    portal_count: u32,
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
        Some(material) => unsafe {
            out.write(WaWmoMaterial {
                flags: material.flags,
                blend_mode: material.blend_mode,
            });
            write_name(Some(&material.texture), texture, texture_len)
        },
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
                    color: doodad.color,
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
            doodad_refs: group.doodad_refs,
            bounds: group.bounds,
            portal_start: group.portal_start,
            portal_count: group.portal_count,
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

/// The doodads standing in this group, for wa_wmo_doodad. Owned by group.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_wmo_group_doodad_refs(
    group: *const WaWmoGroup,
    out: *mut *const u16,
    count: *mut u32,
) {
    let group = unsafe { &*group };
    unsafe {
        out.write(group.doodad_refs.as_ptr());
        count.write(group.doodad_refs.len() as u32);
    }
}

/// One portal, as corners in wa_wmo_portal_vertices and a plane.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct WaWmoPortal {
    pub start: u32,
    pub count: u32,
    pub normal: [f32; 3],
    pub offset: f32,
}

/// A group's link through a portal.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct WaWmoPortalRef {
    pub portal: u32,
    /// The group on the far side.
    pub group: u32,
    /// The side of the portal's plane it's seen from: 1, -1, or 0 for both.
    pub side: i32,
}

/// 3 floats per corner, in the file's axes. Owned by wmo.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_wmo_portal_vertices(
    wmo: *const WaWmo,
    out: *mut *const f32,
    count: *mut u32,
) {
    let wmo = unsafe { &*wmo };
    unsafe {
        out.write(wmo.root.portal_vertices.as_ptr().cast());
        count.write(wmo.root.portal_vertices.len() as u32);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_wmo_portal_count(wmo: *const WaWmo) -> u32 {
    unsafe { &*wmo }.root.portals.len() as u32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_wmo_portal(
    wmo: *const WaWmo,
    index: u32,
    out: *mut WaWmoPortal,
) -> WaStatus {
    match unsafe { &*wmo }.root.portals.get(index as usize) {
        Some(p) => {
            unsafe {
                out.write(WaWmoPortal {
                    start: p.start as u32,
                    count: p.count as u32,
                    normal: [p.normal.x, p.normal.y, p.normal.z],
                    offset: p.offset,
                })
            };
            WaStatus::Ok
        }
        None => WaStatus::NotFound,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_wmo_portal_ref_count(wmo: *const WaWmo) -> u32 {
    unsafe { &*wmo }.root.portal_refs.len() as u32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_wmo_portal_ref(
    wmo: *const WaWmo,
    index: u32,
    out: *mut WaWmoPortalRef,
) -> WaStatus {
    match unsafe { &*wmo }.root.portal_refs.get(index as usize) {
        Some(r) => {
            unsafe {
                out.write(WaWmoPortalRef {
                    portal: r.portal as u32,
                    group: r.group as u32,
                    side: r.side as i32,
                })
            };
            WaStatus::Ok
        }
        None => WaStatus::NotFound,
    }
}

/// The group's box in the file's axes: smallest corner, then largest. 6 floats.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_wmo_group_bounds(group: *const WaWmoGroup, out: *mut f32) {
    unsafe { ptr::copy_nonoverlapping((&*group).bounds.as_ptr(), out, 6) };
}

/// The group's range of wa_wmo_portal_ref.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_wmo_group_portal_refs(
    group: *const WaWmoGroup,
    start: *mut u32,
    count: *mut u32,
) {
    let group = unsafe { &*group };
    unsafe {
        start.write(group.portal_start);
        count.write(group.portal_count);
    }
}

/// The WMO's ambient color, as stored: 0xAARRGGBB.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_wmo_ambient_color(wmo: *const WaWmo) -> u32 {
    unsafe { &*wmo }.root.ambient_color
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
pub unsafe extern "C" fn wa_blp_to_gpu(
    data: *const u8,
    len: usize,
    out: *mut WaGpuTexture,
) -> WaStatus {
    guard(WaStatus::Error, || {
        let bytes = unsafe { bytes_arg(data, len) }?;
        let texture = match compressed_chain(bytes)? {
            Some(texture) => texture,
            None => {
                let blp = load_blp_from_buf(bytes).map_err(|e| e.to_string())?;
                let image = blp_to_image(&blp, 0)
                    .map_err(|e| e.to_string())?
                    .into_rgba8();
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

        assert!(
            unsafe { wa_m2_open(bytes.as_ptr(), bytes.len(), bytes.as_ptr(), bytes.len()) }
                .is_null()
        );
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
