//! WDT and ADT terrain access.

use std::io::Cursor;
use std::ptr;

use wow_alchemy_adt::{
    ALPHA_MAP_BYTES, Adt, AlphaMapFormat, LiquidLayer, MCLY_USE_ALPHA_MAP, McnkChunk,
    decode_alpha_map, read_liquid_layers,
};
use wow_alchemy_wdt::WdtReader;
use wow_alchemy_wdt::version::WowVersion;

use crate::{WaStatus, bytes_arg, guard};

const CHUNKS_PER_TILE: usize = 256;
const TILES_PER_MAP: usize = 64 * 64;
const VERTICES_PER_CHUNK: usize = 145;
const MAX_LAYERS: usize = 4;

#[repr(C)]
pub struct WaTerrainChunk {
    pub ix: u32,
    pub iy: u32,
    pub flags: u32,
    pub area_id: u32,
    pub holes: u32,
    /// World position of the chunk's corner: north, west, base height.
    pub position: [f32; 3],
    pub heights: *const f32,
    pub normals: *const i8,
    pub layer_count: u32,
    pub texture_ids: [u32; MAX_LAYERS],
    pub layer_flags: [u32; MAX_LAYERS],
    /// Each layer's GroundEffectTexture row.
    pub effect_ids: [u32; MAX_LAYERS],
    pub alpha: *const u8,
    /// 145 colors of 4 bytes as stored in the file: blue, green, red, alpha, with 127 neutral.
    pub vertex_colors: *const u8,
}

struct StoredChunk {
    ix: u32,
    iy: u32,
    flags: u32,
    area_id: u32,
    holes: u32,
    position: [f32; 3],
    heights: Vec<f32>,
    normals: Vec<i8>,
    layer_count: u32,
    texture_ids: [u32; MAX_LAYERS],
    layer_flags: [u32; MAX_LAYERS],
    effect_ids: [u32; MAX_LAYERS],
    alpha: Vec<u8>,
    vertex_colors: Vec<u8>,
}

/// A model placed on a tile. Positions run east, up and south from the map's
/// north-west corner, and rotations are degrees.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct WaPlacement {
    /// For wa_adt_model_name or wa_adt_wmo_name.
    pub name: u32,
    /// Shared by every tile that lists this placement.
    pub unique_id: u32,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    /// Always 1 for WMOs.
    pub scale: f32,
    /// The doodad set to show. Always 0 for M2s.
    pub doodad_set: u32,
    /// The world bounds, from the file. All zero for M2s, which don't store them.
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
}

/// One layer of liquid in one chunk, on the chunk's 9x9 grid.
#[repr(C)]
pub struct WaLiquidLayer {
    /// The chunk's Y index * 16 + X index.
    pub chunk: u32,
    /// Row in LiquidType.dbc.
    pub liquid_type: u32,
    /// Bit row * 8 + column is set when that cell has liquid.
    pub cells: u64,
    /// 81 world heights, row by row.
    pub heights: *const f32,
    /// 81 depths from 0 to 255, row by row.
    pub depths: *const u8,
}

pub struct WaAdt {
    liquids: Vec<LiquidLayer>,
    textures: Vec<String>,
    chunks: Vec<StoredChunk>,
    models: Vec<String>,
    wmos: Vec<String>,
    doodads: Vec<WaPlacement>,
    wmo_placements: Vec<WaPlacement>,
}

/// Placements name a slot in MMID or MWID, which holds a string's offset. The
/// parser lists the strings in order, so this finds the slot's string by counting
/// the offsets, which works unless a file's offsets skip strings.
fn name_for_slot(names: &[String], offsets: &[u32], slot: u32) -> u32 {
    let Some(&wanted) = offsets.get(slot as usize) else {
        return slot;
    };
    let mut offset = 0u32;
    for (index, name) in names.iter().enumerate() {
        if offset == wanted {
            return index as u32;
        }
        offset += name.len() as u32 + 1;
    }
    slot
}

fn store_chunk(chunk: &McnkChunk, big_alpha: bool) -> Result<StoredChunk, String> {
    let location = format!("chunk ({}, {})", chunk.ix, chunk.iy);

    if chunk.height_map.len() != VERTICES_PER_CHUNK || chunk.normals.len() != VERTICES_PER_CHUNK {
        return Err(format!(
            "{location} has {} heights and {} normals, expected {VERTICES_PER_CHUNK}",
            chunk.height_map.len(),
            chunk.normals.len()
        ));
    }

    let mut texture_ids = [0; MAX_LAYERS];
    let mut layer_flags = [0; MAX_LAYERS];
    let mut effect_ids = [0; MAX_LAYERS];
    let mut alpha = vec![0; (MAX_LAYERS - 1) * ALPHA_MAP_BYTES];
    let layers = &chunk.texture_layers[..chunk.texture_layers.len().min(MAX_LAYERS)];

    for (index, layer) in layers.iter().enumerate() {
        texture_ids[index] = layer.texture_id;
        layer_flags[index] = layer.flags;
        effect_ids[index] = layer.effect_id;
        if index == 0 || layer.flags & MCLY_USE_ALPHA_MAP == 0 {
            continue;
        }

        let data = chunk
            .raw_alpha
            .get(layer.alpha_map_offset as usize..)
            .ok_or_else(|| format!("{location} layer {index} has a bad alpha offset"))?;
        let format = AlphaMapFormat::new(big_alpha, layer.flags, chunk.flags);
        let map =
            decode_alpha_map(data, format).map_err(|e| format!("{location} layer {index}: {e}"))?;
        alpha[(index - 1) * ALPHA_MAP_BYTES..index * ALPHA_MAP_BYTES].copy_from_slice(&map);
    }

    // Chunks without MCCV get the neutral color.
    let vertex_colors = if chunk.raw_vertex_colors.len() == VERTICES_PER_CHUNK * 4 {
        chunk.raw_vertex_colors.clone()
    } else {
        vec![127; VERTICES_PER_CHUNK * 4]
    };

    Ok(StoredChunk {
        ix: chunk.ix,
        iy: chunk.iy,
        flags: chunk.flags,
        area_id: chunk.area_id,
        // Only the low 16 bits hold the hole mask.
        holes: chunk.holes & 0xFFFF,
        // McnkChunk orders this [file 1, file 2, file 0], and the file stores
        // north, west, height.
        position: [chunk.position[2], chunk.position[0], chunk.position[1]],
        heights: chunk.height_map.clone(),
        normals: chunk.normals.iter().flatten().map(|b| *b as i8).collect(),
        layer_count: layers.len() as u32,
        texture_ids,
        layer_flags,
        effect_ids,
        alpha,
        vertex_colors,
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_wdt_read(
    data: *const u8,
    len: usize,
    tiles: *mut u8,
    flags: *mut u32,
) -> WaStatus {
    guard(WaStatus::Error, || {
        let bytes = unsafe { bytes_arg(data, len) }?;
        let wdt = WdtReader::new(Cursor::new(bytes), WowVersion::WotLK)
            .read()
            .map_err(|e| e.to_string())?;

        let output = unsafe { std::slice::from_raw_parts_mut(tiles, TILES_PER_MAP) };
        for (index, slot) in output.iter_mut().enumerate() {
            let tile = wdt.get_tile(index % 64, index / 64);
            *slot = tile.is_some_and(|t| t.has_adt) as u8;
        }

        unsafe { flags.write(wdt.mphd.flags.bits()) };
        Ok(WaStatus::Ok)
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_adt_open(data: *const u8, len: usize, big_alpha: i32) -> *mut WaAdt {
    guard(ptr::null_mut(), || {
        let bytes = unsafe { bytes_arg(data, len) }?;
        let adt = Adt::from_reader(Cursor::new(bytes)).map_err(|e| e.to_string())?;
        let liquids = read_liquid_layers(bytes).map_err(|e| e.to_string())?;

        let mut chunks = adt
            .mcnk_chunks
            .iter()
            .map(|chunk| store_chunk(chunk, big_alpha != 0))
            .collect::<Result<Vec<_>, _>>()?;
        if chunks.len() != CHUNKS_PER_TILE {
            return Err(format!(
                "tile has {} chunks, expected {CHUNKS_PER_TILE}",
                chunks.len()
            ));
        }
        chunks.sort_by_key(|chunk| (chunk.iy, chunk.ix));

        let models = adt.mmdx.map(|c| c.filenames).unwrap_or_default();
        let wmos = adt.mwmo.map(|c| c.filenames).unwrap_or_default();
        let model_slots = adt.mmid.map(|c| c.offsets).unwrap_or_default();
        let wmo_slots = adt.mwid.map(|c| c.offsets).unwrap_or_default();

        let doodads = adt
            .mddf
            .map(|c| c.doodads)
            .unwrap_or_default()
            .iter()
            .map(|d| WaPlacement {
                name: name_for_slot(&models, &model_slots, d.name_id),
                unique_id: d.unique_id,
                position: d.position,
                rotation: d.rotation,
                scale: d.scale,
                doodad_set: 0,
                bounds_min: [0.0; 3],
                bounds_max: [0.0; 3],
            })
            .collect();
        let wmo_placements = adt
            .modf
            .map(|c| c.models)
            .unwrap_or_default()
            .iter()
            .map(|m| WaPlacement {
                name: name_for_slot(&wmos, &wmo_slots, m.name_id),
                unique_id: m.unique_id,
                position: m.position,
                rotation: m.rotation,
                scale: 1.0,
                doodad_set: m.doodad_set as u32,
                bounds_min: m.bounds_min,
                bounds_max: m.bounds_max,
            })
            .collect();

        let textures = adt.mtex.map(|mtex| mtex.filenames).unwrap_or_default();
        Ok(Box::into_raw(Box::new(WaAdt {
            liquids,
            textures,
            chunks,
            models,
            wmos,
            doodads,
            wmo_placements,
        })))
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_adt_free(adt: *mut WaAdt) {
    if !adt.is_null() {
        drop(unsafe { Box::from_raw(adt) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_adt_liquid_count(adt: *const WaAdt) -> u32 {
    unsafe { &*adt }.liquids.len() as u32
}

/// The pointers stay valid until the tile is freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_adt_liquid(
    adt: *const WaAdt,
    index: u32,
    out: *mut WaLiquidLayer,
) -> WaStatus {
    match unsafe { &*adt }.liquids.get(index as usize) {
        Some(layer) => {
            unsafe {
                out.write(WaLiquidLayer {
                    chunk: layer.chunk as u32,
                    liquid_type: layer.liquid_type as u32,
                    cells: layer.cells,
                    heights: layer.heights.as_ptr(),
                    depths: layer.depths.as_ptr(),
                })
            };
            WaStatus::Ok
        }
        None => WaStatus::NotFound,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_adt_texture_count(adt: *const WaAdt) -> u32 {
    unsafe { &*adt }.textures.len() as u32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_adt_texture_name(
    adt: *const WaAdt,
    index: u32,
    out: *mut *const u8,
    out_len: *mut usize,
) -> WaStatus {
    match unsafe { &*adt }.textures.get(index as usize) {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_adt_chunk(
    adt: *const WaAdt,
    index: u32,
    out: *mut WaTerrainChunk,
) -> WaStatus {
    match unsafe { &*adt }.chunks.get(index as usize) {
        Some(chunk) => {
            unsafe {
                out.write(WaTerrainChunk {
                    ix: chunk.ix,
                    iy: chunk.iy,
                    flags: chunk.flags,
                    area_id: chunk.area_id,
                    holes: chunk.holes,
                    position: chunk.position,
                    heights: chunk.heights.as_ptr(),
                    normals: chunk.normals.as_ptr(),
                    layer_count: chunk.layer_count,
                    texture_ids: chunk.texture_ids,
                    layer_flags: chunk.layer_flags,
                    effect_ids: chunk.effect_ids,
                    alpha: chunk.alpha.as_ptr(),
                    vertex_colors: chunk.vertex_colors.as_ptr(),
                })
            };
            WaStatus::Ok
        }
        None => WaStatus::NotFound,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_adt_model_name(
    adt: *const WaAdt,
    index: u32,
    out: *mut *const u8,
    out_len: *mut usize,
) -> WaStatus {
    unsafe { write_name((&*adt).models.get(index as usize), out, out_len) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_adt_wmo_name(
    adt: *const WaAdt,
    index: u32,
    out: *mut *const u8,
    out_len: *mut usize,
) -> WaStatus {
    unsafe { write_name((&*adt).wmos.get(index as usize), out, out_len) }
}

/// The tile's M2 placements, owned by adt.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_adt_doodads(
    adt: *const WaAdt,
    out: *mut *const WaPlacement,
    out_len: *mut usize,
) {
    let doodads = &unsafe { &*adt }.doodads;
    unsafe {
        out.write(doodads.as_ptr());
        out_len.write(doodads.len());
    }
}

/// The tile's WMO placements, owned by adt.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_adt_wmos(
    adt: *const WaAdt,
    out: *mut *const WaPlacement,
    out_len: *mut usize,
) {
    let wmos = &unsafe { &*adt }.wmo_placements;
    unsafe {
        out.write(wmos.as_ptr());
        out_len.write(wmos.len());
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wa_last_error;

    #[test]
    fn invalid_files_report_errors() {
        let bytes = b"not game data";

        let adt = unsafe { wa_adt_open(bytes.as_ptr(), bytes.len(), 0) };
        assert!(adt.is_null());
        assert!(!wa_last_error().is_null());

        let mut tiles = vec![0; TILES_PER_MAP];
        let mut flags = 0;
        let status =
            unsafe { wa_wdt_read(bytes.as_ptr(), bytes.len(), tiles.as_mut_ptr(), &mut flags) };
        assert_eq!(status, WaStatus::Error);
    }

    #[test]
    fn handles_can_move_between_threads() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<WaAdt>();
    }
}
