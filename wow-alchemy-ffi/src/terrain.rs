//! WDT and ADT terrain access.

use std::io::Cursor;
use std::ptr;

use wow_alchemy_adt::{
    ALPHA_MAP_BYTES, Adt, CHUNKS_PER_TILE, LiquidLayer, McnkChunk, VERTICES_PER_CHUNK,
};
use wow_alchemy_data::types::WowStructR;
use wow_alchemy_data::world::MapObjectPlacement;
use wow_alchemy_wdt::WdtFile;

use crate::{WaBuffer, WaStatus, bytes_arg, guard};

const TILES_PER_MAP: usize = 64 * 64;
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
    /// One bit per cell of the chunk's 8 by 8, row by row: no ground effect doodads there.
    pub no_effect_doodad: u64,
}

struct StoredChunk {
    ix: u32,
    iy: u32,
    flags: u32,
    area_id: u32,
    holes: u32,
    no_effect_doodad: u64,
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

fn store_chunk(chunk: &McnkChunk, big_alpha: bool) -> Result<StoredChunk, String> {
    let header = &chunk.header;
    let location = format!("chunk ({}, {})", header.index_x, header.index_y);

    if chunk.heights.len() != VERTICES_PER_CHUNK || chunk.normals.len() != VERTICES_PER_CHUNK {
        return Err(format!(
            "{location} has {} heights and {} normals, expected {VERTICES_PER_CHUNK}",
            chunk.heights.len(),
            chunk.normals.len()
        ));
    }

    let mut texture_ids = [0; MAX_LAYERS];
    let mut layer_flags = [0; MAX_LAYERS];
    let mut effect_ids = [0; MAX_LAYERS];
    let mut alpha = vec![0; (MAX_LAYERS - 1) * ALPHA_MAP_BYTES];
    let layers = &chunk.layers[..chunk.layers.len().min(MAX_LAYERS)];

    for (index, layer) in layers.iter().enumerate() {
        texture_ids[index] = layer.texture_id;
        layer_flags[index] = layer.flags.bits();
        effect_ids[index] = layer.effect_id;
        let map = chunk
            .alpha_map(index, big_alpha)
            .map_err(|e| format!("{location} layer {index}: {e}"))?;
        if let Some(map) = map {
            alpha[(index - 1) * ALPHA_MAP_BYTES..index * ALPHA_MAP_BYTES].copy_from_slice(&map);
        }
    }

    // Chunks without MCCV get the neutral color.
    let vertex_colors = match &chunk.vertex_colors {
        Some(colors) if colors.len() == VERTICES_PER_CHUNK => colors.concat(),
        _ => vec![127; VERTICES_PER_CHUNK * 4],
    };

    Ok(StoredChunk {
        ix: header.index_x,
        iy: header.index_y,
        flags: header.flags.bits(),
        area_id: header.area_id,
        holes: header.holes_low_res as u32,
        no_effect_doodad: header.no_effect_doodad,
        // The file stores north, west, height.
        position: [header.position.x, header.position.y, header.position.z],
        heights: chunk.heights.clone(),
        normals: chunk.normals.iter().flatten().copied().collect(),
        layer_count: layers.len() as u32,
        texture_ids,
        layer_flags,
        effect_ids,
        alpha,
        vertex_colors,
    })
}

fn read_wdt(bytes: &[u8]) -> Result<WdtFile, String> {
    WdtFile::wow_read(&mut Cursor::new(bytes)).map_err(|e| e.to_string())
}

/// A WMO's placement as the C ABI has it, from MODF.
pub(crate) fn wmo_placement(entry: &MapObjectPlacement) -> WaPlacement {
    WaPlacement {
        name: entry.name_id,
        unique_id: entry.unique_id,
        position: [entry.position.x, entry.position.y, entry.position.z],
        rotation: [entry.rotation.x, entry.rotation.y, entry.rotation.z],
        scale: 1.0,
        doodad_set: entry.doodad_set as u32,
        bounds_min: [
            entry.extents.min.x,
            entry.extents.min.y,
            entry.extents.min.z,
        ],
        bounds_max: [
            entry.extents.max.x,
            entry.extents.max.y,
            entry.extents.max.z,
        ],
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_wdt_read(
    data: *const u8,
    len: usize,
    tiles: *mut u8,
    flags: *mut u32,
) -> WaStatus {
    guard(WaStatus::Error, || {
        let wdt = read_wdt(unsafe { bytes_arg(data, len) }?)?;
        let output = unsafe { std::slice::from_raw_parts_mut(tiles, TILES_PER_MAP) };
        for (slot, tile) in output.iter_mut().zip(&wdt.tiles) {
            *slot = tile.has_adt() as u8;
        }
        unsafe { flags.write(wdt.header.flags.bits()) };
        Ok(WaStatus::Ok)
    })
}

/// The WMO a map is made of when it has no terrain, as most instances are: its file name, as a
/// buffer to free with `wa_buffer_free`, and its placement, in the same coordinates as an ADT's.
/// Returns `NotFound` for a map without one.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_wdt_global_wmo(
    data: *const u8,
    len: usize,
    name: *mut WaBuffer,
    placement: *mut WaPlacement,
) -> WaStatus {
    guard(WaStatus::Error, || {
        unsafe { name.write(WaBuffer::EMPTY) };
        let wdt = read_wdt(unsafe { bytes_arg(data, len) }?)?;
        let Some((file, entry)) = wdt.global_wmo() else {
            return Ok(WaStatus::NotFound);
        };
        unsafe {
            name.write(WaBuffer::from_vec(file.into_bytes()));
            placement.write(WaPlacement {
                name: 0,
                ..wmo_placement(entry)
            })
        };
        Ok(WaStatus::Ok)
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn wa_adt_open(data: *const u8, len: usize, big_alpha: i32) -> *mut WaAdt {
    guard(ptr::null_mut(), || {
        let bytes = unsafe { bytes_arg(data, len) }?;
        let adt = Adt::wow_read(&mut Cursor::new(bytes)).map_err(|e| e.to_string())?;
        let liquids = adt.liquid_layers().map_err(|e| e.to_string())?;

        let mut chunks = adt
            .chunks
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

        // Placements name a slot in MMID or MWID, which holds a name's offset; one that names
        // none keeps its slot.
        let doodads = adt
            .doodads
            .iter()
            .map(|d| WaPlacement {
                name: adt.model_index(d.name_id).map_or(d.name_id, |i| i as u32),
                unique_id: d.unique_id,
                position: [d.position.x, d.position.y, d.position.z],
                rotation: [d.rotation.x, d.rotation.y, d.rotation.z],
                scale: d.scale_factor(),
                doodad_set: 0,
                bounds_min: [0.0; 3],
                bounds_max: [0.0; 3],
            })
            .collect();
        let wmo_placements = adt
            .wmo_placements
            .iter()
            .map(|m| WaPlacement {
                name: adt.wmo_index(m.name_id).map_or(m.name_id, |i| i as u32),
                ..wmo_placement(m)
            })
            .collect();

        let models = adt.models.names();
        let wmos = adt.wmos.names();
        let textures = adt.textures.names();
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
                    no_effect_doodad: chunk.no_effect_doodad,
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
    use wow_alchemy_data::types::WowStructW;

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

    fn wdt_bytes(wdt: &WdtFile) -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        wdt.wow_write(&mut bytes).unwrap();
        bytes.into_inner()
    }

    #[test]
    fn global_wmo_is_read_from_a_wmo_only_map() {
        use wow_alchemy_data::types::{C3Vector, StringBlock};

        let mut wdt = WdtFile::default();
        wdt.header.flags |= wow_alchemy_wdt::MphdFlags::WDT_USES_GLOBAL_MAP_OBJ;
        wdt.wmo_names = Some(StringBlock::from_strings(&[r"World\wmo\Dungeon\Test\Test.wmo"]).0);
        wdt.wmo_placements = Some(vec![MapObjectPlacement {
            position: C3Vector::new(17066.666, 10.0, 17000.0),
            rotation: C3Vector::new(0.0, 90.0, 0.0),
            doodad_set: 2,
            ..Default::default()
        }]);
        let bytes = wdt_bytes(&wdt);

        let mut name = WaBuffer::EMPTY;
        let mut placement = std::mem::MaybeUninit::<WaPlacement>::uninit();
        let status = unsafe {
            wa_wdt_global_wmo(
                bytes.as_ptr(),
                bytes.len(),
                &mut name,
                placement.as_mut_ptr(),
            )
        };

        assert_eq!(status, WaStatus::Ok, "{:?}", unsafe {
            std::ffi::CStr::from_ptr(wa_last_error())
        });
        let text = unsafe { std::slice::from_raw_parts(name.ptr, name.len) };
        assert_eq!(text, br"World\wmo\Dungeon\Test\Test.wmo");
        let placement = unsafe { placement.assume_init() };
        assert_eq!(placement.position, [17066.666, 10.0, 17000.0]);
        assert_eq!(placement.rotation[1], 90.0);
        assert_eq!(placement.doodad_set, 2);
        unsafe { crate::wa_buffer_free(name) };
    }

    #[test]
    fn terrain_maps_have_no_global_wmo() {
        let bytes = wdt_bytes(&WdtFile::default());
        let mut name = WaBuffer::EMPTY;
        let mut placement = std::mem::MaybeUninit::<WaPlacement>::uninit();

        let status = unsafe {
            wa_wdt_global_wmo(
                bytes.as_ptr(),
                bytes.len(),
                &mut name,
                placement.as_mut_ptr(),
            )
        };

        assert_eq!(status, WaStatus::NotFound);
        assert!(name.ptr.is_null());
    }

    #[test]
    fn handles_can_move_between_threads() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<WaAdt>();
    }
}
