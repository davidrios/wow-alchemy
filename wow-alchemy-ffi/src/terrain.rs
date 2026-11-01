//! WDT and ADT terrain access.

use std::io::Cursor;
use std::ptr;

use wow_alchemy_adt::{
    ALPHA_MAP_BYTES, Adt, AlphaMapFormat, MCLY_USE_ALPHA_MAP, McnkChunk, decode_alpha_map,
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
    alpha: Vec<u8>,
    vertex_colors: Vec<u8>,
}

pub struct WaAdt {
    textures: Vec<String>,
    chunks: Vec<StoredChunk>,
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
    let mut alpha = vec![0; (MAX_LAYERS - 1) * ALPHA_MAP_BYTES];
    let layers = &chunk.texture_layers[..chunk.texture_layers.len().min(MAX_LAYERS)];

    for (index, layer) in layers.iter().enumerate() {
        texture_ids[index] = layer.texture_id;
        layer_flags[index] = layer.flags;
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

        let textures = adt.mtex.map(|mtex| mtex.filenames).unwrap_or_default();
        Ok(Box::into_raw(Box::new(WaAdt { textures, chunks })))
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_adt_free(adt: *mut WaAdt) {
    if !adt.is_null() {
        drop(unsafe { Box::from_raw(adt) });
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
                    alpha: chunk.alpha.as_ptr(),
                    vertex_colors: chunk.vertex_colors.as_ptr(),
                })
            };
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
