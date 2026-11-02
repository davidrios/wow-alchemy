//! Liquid surfaces of a WotLK ADT, read from its MH2O chunk ready for drawing.
//!
//! This reads the layout the 3.3.5a client uses. Each of a tile's 256 chunks has a 12-byte
//! header, and offsets count from the start of the chunk's data:
//!
//! ```text
//! header:   u32 instances, u32 layer count, u32 attributes
//! instance: u16 liquid type, u16 vertex format, f32 min height, f32 max height,
//!           u8 x offset, u8 y offset, u8 width, u8 height,
//!           u32 exists bitmap offset, u32 vertex data offset
//! ```
//!
//! Vertex data covers `(width + 1) * (height + 1)` vertices, as arrays that depend on the format:
//!
//! ```text
//! 0: f32 heights, u8 depths
//! 1: f32 heights, u16 x 2 texture coordinates
//! 2: u8 depths
//! 3: f32 heights, u16 x 2 texture coordinates, u8 depths
//! ```
//!
//! Layers with no vertex data sit flat at their minimum height.

use crate::error::{AdtError, Result};

/// Cells along one side of a chunk's liquid grid.
pub const LIQUID_CELLS_PER_SIDE: usize = 8;
/// Vertices along one side of a chunk's liquid grid.
pub const LIQUID_VERTICES_PER_SIDE: usize = LIQUID_CELLS_PER_SIDE + 1;
/// Vertices in a chunk's liquid grid.
pub const LIQUID_VERTICES: usize = LIQUID_VERTICES_PER_SIDE * LIQUID_VERTICES_PER_SIDE;

const CHUNKS_PER_TILE: usize = 256;
const HEADER_BYTES: usize = 12;
const INSTANCE_BYTES: usize = 24;
const FORMAT_HEIGHT_AND_TEXTURE: u16 = 1;
const FORMAT_DEPTH_ONLY: u16 = 2;
const FORMAT_ALL: u16 = 3;
const TEXTURE_COORDINATE_BYTES: usize = 4;
/// How deep a layer with no depth data counts as, as a depth byte.
const FULL_DEPTH: u8 = 255;

/// One layer of liquid in one map chunk, on the chunk's full 9x9 grid. Columns run along the
/// chunk's X index and rows along its Y index.
#[derive(Debug, Clone, PartialEq)]
pub struct LiquidLayer {
    /// The chunk's index: Y index * 16 + X index.
    pub chunk: u16,
    /// Row in LiquidType.dbc.
    pub liquid_type: u16,
    /// The format the file stored its vertices in.
    pub vertex_format: u16,
    /// Bit `row * 8 + column` is set when that cell has liquid.
    pub cells: u64,
    /// World heights, row by row. Limited to the layer's stated range, because the files can
    /// hold garbage on vertices no cell uses.
    pub heights: Vec<f32>,
    /// How deep the liquid looks at each vertex, from 0 to 255, row by row.
    pub depths: Vec<u8>,
}

fn u16_at(data: &[u8], offset: usize) -> Result<u16> {
    let bytes = data
        .get(offset..offset + 2)
        .ok_or_else(|| AdtError::ParseError(format!("MH2O ends before offset {offset}")))?;
    Ok(u16::from_le_bytes(bytes.try_into().unwrap()))
}

fn u32_at(data: &[u8], offset: usize) -> Result<u32> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or_else(|| AdtError::ParseError(format!("MH2O ends before offset {offset}")))?;
    Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
}

fn f32_at(data: &[u8], offset: usize) -> Result<f32> {
    Ok(f32::from_bits(u32_at(data, offset)?))
}

fn u8_at(data: &[u8], offset: usize) -> Result<u8> {
    data.get(offset)
        .copied()
        .ok_or_else(|| AdtError::ParseError(format!("MH2O ends before offset {offset}")))
}

/// The data of a whole ADT's MH2O chunk, if it has one.
fn find_mh2o(adt: &[u8]) -> Option<&[u8]> {
    let mut position = 0;
    while position + 8 <= adt.len() {
        let size = u32::from_le_bytes(adt[position + 4..position + 8].try_into().unwrap()) as usize;
        let end = (position + 8).checked_add(size)?;
        if end > adt.len() {
            return None;
        }
        if &adt[position..position + 4] == b"O2HM" {
            return Some(&adt[position + 8..end]);
        }
        position = end;
    }
    None
}

/// Reads every liquid layer of a whole ADT file. A tile with no MH2O chunk has none.
pub fn read_liquid_layers(adt: &[u8]) -> Result<Vec<LiquidLayer>> {
    match find_mh2o(adt) {
        Some(data) => parse_mh2o(data),
        None => Ok(Vec::new()),
    }
}

/// Reads the layers from the data of an MH2O chunk.
pub fn parse_mh2o(data: &[u8]) -> Result<Vec<LiquidLayer>> {
    let mut layers = Vec::new();
    for chunk in 0..CHUNKS_PER_TILE {
        let header = chunk * HEADER_BYTES;
        let instances = u32_at(data, header)? as usize;
        let count = u32_at(data, header + 4)? as usize;
        if instances == 0 {
            continue;
        }
        for layer in 0..count {
            layers.push(parse_layer(data, chunk as u16, instances + layer * INSTANCE_BYTES)?);
        }
    }
    Ok(layers)
}

fn parse_layer(data: &[u8], chunk: u16, instance: usize) -> Result<LiquidLayer> {
    let liquid_type = u16_at(data, instance)?;
    let vertex_format = u16_at(data, instance + 2)?;
    let min_height = f32_at(data, instance + 4)?;
    let max_height = f32_at(data, instance + 8)?;
    let x_offset = u8_at(data, instance + 12)? as usize;
    let y_offset = u8_at(data, instance + 13)? as usize;
    let width = u8_at(data, instance + 14)? as usize;
    let height = u8_at(data, instance + 15)? as usize;
    let exists = u32_at(data, instance + 16)? as usize;
    let vertices = u32_at(data, instance + 20)? as usize;

    if x_offset + width > LIQUID_CELLS_PER_SIDE || y_offset + height > LIQUID_CELLS_PER_SIDE {
        return Err(AdtError::ParseError(format!(
            "liquid in chunk {chunk} covers {width}x{height} cells at {x_offset},{y_offset}"
        )));
    }

    let mut cells = 0u64;
    for row in 0..height {
        for column in 0..width {
            let bit = row * width + column;
            let set = exists == 0 || u8_at(data, exists + bit / 8)? & (1 << (bit % 8)) != 0;
            if set {
                cells |= 1 << ((row + y_offset) * LIQUID_CELLS_PER_SIDE + column + x_offset);
            }
        }
    }

    let mut heights = vec![min_height; LIQUID_VERTICES];
    let mut depths = vec![FULL_DEPTH; LIQUID_VERTICES];
    if vertices != 0 {
        let stored = (width + 1) * (height + 1);
        let has_heights = vertex_format != FORMAT_DEPTH_ONLY;
        let has_depths = vertex_format != FORMAT_HEIGHT_AND_TEXTURE;
        let depth_start = vertices
            + if has_heights { stored * 4 } else { 0 }
            + if vertex_format == FORMAT_ALL {
                stored * TEXTURE_COORDINATE_BYTES
            } else {
                0
            };
        for row in 0..=height {
            for column in 0..=width {
                let source = row * (width + 1) + column;
                let target = (row + y_offset) * LIQUID_VERTICES_PER_SIDE + column + x_offset;
                if has_heights {
                    // A NaN stays at the minimum, as clamp would pass it through.
                    let value = f32_at(data, vertices + source * 4)?;
                    if value.is_finite() && min_height <= max_height {
                        heights[target] = value.clamp(min_height, max_height);
                    }
                }
                if has_depths {
                    depths[target] = u8_at(data, depth_start + source)?;
                }
            }
        }
    }

    Ok(LiquidLayer {
        chunk,
        liquid_type,
        vertex_format,
        cells,
        heights,
        depths,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An MH2O chunk with one layer in chunk 17, whose extra data starts after the headers.
    fn chunk_with(instance: &[u8], extra: &[u8]) -> Vec<u8> {
        let instances = CHUNKS_PER_TILE * HEADER_BYTES;
        let mut data = vec![0u8; instances];
        data[17 * HEADER_BYTES..17 * HEADER_BYTES + 4]
            .copy_from_slice(&(instances as u32).to_le_bytes());
        data[17 * HEADER_BYTES + 4..17 * HEADER_BYTES + 8].copy_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(instance);
        data.extend_from_slice(extra);
        data
    }

    fn instance(format: u16, area: [u8; 4], exists: u32, vertices: u32) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&5u16.to_le_bytes());
        out.extend_from_slice(&format.to_le_bytes());
        out.extend_from_slice(&10f32.to_le_bytes());
        out.extend_from_slice(&20f32.to_le_bytes());
        out.extend_from_slice(&area);
        out.extend_from_slice(&exists.to_le_bytes());
        out.extend_from_slice(&vertices.to_le_bytes());
        out
    }

    const EXTRA: u32 = (CHUNKS_PER_TILE * HEADER_BYTES + INSTANCE_BYTES) as u32;

    #[test]
    fn places_vertices_and_depths_on_the_full_grid() {
        let mut extra = Vec::new();
        for value in [11f32, 12.0, 13.0, 14.0] {
            extra.extend_from_slice(&value.to_le_bytes());
        }
        extra.extend_from_slice(&[1, 2, 3, 4]);

        let data = chunk_with(&instance(0, [2, 3, 1, 1], 0, EXTRA), &extra);
        let layers = parse_mh2o(&data).unwrap();

        assert_eq!(layers.len(), 1);
        let layer = &layers[0];
        assert_eq!((layer.chunk, layer.liquid_type), (17, 5));
        assert_eq!(layer.cells, 1 << (3 * 8 + 2));
        assert_eq!(layer.heights[3 * 9 + 2], 11.0);
        assert_eq!(layer.heights[3 * 9 + 3], 12.0);
        assert_eq!(layer.heights[4 * 9 + 2], 13.0);
        assert_eq!(layer.heights[4 * 9 + 3], 14.0);
        assert_eq!(layer.depths[4 * 9 + 3], 4);
        assert_eq!(layer.heights[0], 10.0);
    }

    #[test]
    fn exists_bits_run_through_the_area_without_padding_rows() {
        // 3 wide and 2 tall: the second row starts at bit 3, not at a new byte.
        let data = chunk_with(&instance(0, [0, 0, 3, 2], EXTRA, 0), &[0b0010_1001]);
        let layer = &parse_mh2o(&data).unwrap()[0];
        assert_eq!(layer.cells, 1 | (1 << 8) | (1 << (8 + 2)));
    }

    #[test]
    fn depth_only_format_stays_at_the_minimum_height() {
        let data = chunk_with(&instance(2, [0, 0, 1, 1], 0, EXTRA), &[9, 8, 7, 6]);
        let layer = &parse_mh2o(&data).unwrap()[0];
        assert_eq!(layer.heights[0], 10.0);
        assert_eq!(layer.depths[0], 9);
        assert_eq!(layer.depths[10], 6);
    }

    #[test]
    fn height_only_format_counts_as_fully_deep() {
        let mut extra = Vec::new();
        for _ in 0..4 {
            extra.extend_from_slice(&15f32.to_le_bytes());
        }
        let data = chunk_with(&instance(1, [0, 0, 1, 1], 0, EXTRA), &extra);
        let layer = &parse_mh2o(&data).unwrap()[0];
        assert_eq!(layer.heights[0], 15.0);
        assert_eq!(layer.depths[0], FULL_DEPTH);
    }

    #[test]
    fn full_format_has_depths_after_texture_coordinates() {
        let mut extra = Vec::new();
        for _ in 0..4 {
            extra.extend_from_slice(&15f32.to_le_bytes());
        }
        extra.extend_from_slice(&[0xAA; 16]);
        extra.extend_from_slice(&[1, 2, 3, 4]);
        let data = chunk_with(&instance(3, [0, 0, 1, 1], 0, EXTRA), &extra);
        let layer = &parse_mh2o(&data).unwrap()[0];
        assert_eq!(layer.heights[0], 15.0);
        assert_eq!(layer.depths[0], 1);
        assert_eq!(layer.depths[10], 4);
    }

    #[test]
    fn heights_outside_the_stated_range_are_limited() {
        let mut extra = Vec::new();
        for value in [1000f32, -1000.0, f32::NAN, 15.0] {
            extra.extend_from_slice(&value.to_le_bytes());
        }
        extra.extend_from_slice(&[0; 4]);
        let data = chunk_with(&instance(0, [0, 0, 1, 1], 0, EXTRA), &extra);
        let layer = &parse_mh2o(&data).unwrap()[0];
        assert_eq!(layer.heights[0], 20.0);
        assert_eq!(layer.heights[1], 10.0);
        assert_eq!(layer.heights[9], 10.0);
        assert_eq!(layer.heights[10], 15.0);
    }

    #[test]
    fn rejects_areas_off_the_grid() {
        let data = chunk_with(&instance(0, [7, 0, 2, 1], 0, 0), &[]);
        assert!(parse_mh2o(&data).is_err());
    }

    #[test]
    fn finds_the_chunk_among_others_and_tolerates_none() {
        let mut adt = b"REVM\x04\0\0\0\x12\0\0\0".to_vec();
        assert!(read_liquid_layers(&adt).unwrap().is_empty());

        let body = vec![0u8; CHUNKS_PER_TILE * HEADER_BYTES];
        adt.extend_from_slice(b"O2HM");
        adt.extend_from_slice(&(body.len() as u32).to_le_bytes());
        adt.extend_from_slice(&body);
        assert!(read_liquid_layers(&adt).unwrap().is_empty());
    }
}
