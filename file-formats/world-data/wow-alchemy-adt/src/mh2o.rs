//! MH2O: the liquids of a WotLK tile.
//!
//! Each of a tile's 256 chunks has a 12-byte [`Mh2oHeader`], and offsets count from the start of
//! MH2O's data. A header points at its chunk's [`Mh2oInstance`]s, one per layer, and each
//! instance at a bitmap of the cells it covers and its vertex data. That covers
//! `(width + 1) * (height + 1)` vertices, as arrays that depend on the vertex format:
//!
//! ```text
//! 0: f32 heights, u8 depths
//! 1: f32 heights, u16 x 2 texture coordinates
//! 2: u8 depths
//! 3: f32 heights, u16 x 2 texture coordinates, u8 depths
//! ```
//!
//! Layers with no vertex data sit flat at their minimum height. The chunk's data is kept as
//! stored, since files share and order these pieces freely; this module reads it.
//!
//! See <https://wowdev.wiki/ADT/v18#MH2O_chunk_(WotLK+)>.

use std::io::Cursor;

use custom_debug::Debug;
use wow_alchemy_data::error::{Result, WowDataError};
use wow_alchemy_data::prelude::*;
use wow_alchemy_data_derive::{WowHeaderR, WowHeaderW};
use wow_alchemy_utils::debug;

/// Cells along one side of a chunk's liquid grid.
pub const LIQUID_CELLS_PER_SIDE: usize = 8;
/// Vertices along one side of a chunk's liquid grid.
pub const LIQUID_VERTICES_PER_SIDE: usize = LIQUID_CELLS_PER_SIDE + 1;
/// Vertices in a chunk's liquid grid.
pub const LIQUID_VERTICES: usize = LIQUID_VERTICES_PER_SIDE * LIQUID_VERTICES_PER_SIDE;

const CHUNKS_PER_TILE: usize = 256;
const FORMAT_HEIGHT_AND_TEXTURE: u16 = 1;
const FORMAT_DEPTH_ONLY: u16 = 2;
const FORMAT_ALL: u16 = 3;
const TEXTURE_COORDINATE_BYTES: usize = 4;
/// How deep a layer with no depth data counts as, as a depth byte.
const FULL_DEPTH: u8 = 255;

/// One chunk's entry at the start of MH2O.
#[derive(Debug, Clone, Default, PartialEq, WowHeaderR, WowHeaderW)]
pub struct Mh2oHeader {
    /// 0 when the chunk has no liquid.
    pub instances_offset: u32,
    pub layer_count: u32,
    /// 0 when the chunk has no [`Mh2oAttributes`].
    pub attributes_offset: u32,
}

/// One layer of liquid in a chunk.
#[derive(Debug, Clone, Default, PartialEq, WowHeaderR, WowHeaderW)]
pub struct Mh2oInstance {
    /// Row in LiquidType.dbc.
    pub liquid_type: u16,
    /// How the vertex data is stored, 0 to 3; see the module.
    pub vertex_format: u16,
    pub min_height: f32,
    pub max_height: f32,
    pub x_offset: u8,
    pub y_offset: u8,
    pub width: u8,
    pub height: u8,
    /// A bit per cell of the `width` by `height` area, row by row with no padding; 0 means all.
    pub exists_offset: u32,
    /// 0 when the layer is flat.
    pub vertex_data_offset: u32,
}

/// Which cells of a chunk are fishable and deep, a bit each, row by row.
#[derive(Debug, Clone, Default, PartialEq, WowHeaderR, WowHeaderW)]
pub struct Mh2oAttributes {
    pub fishable: u64,
    pub deep: u64,
}

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
    #[debug(with = debug::trimmed_collection_fmt)]
    pub heights: Vec<f32>,
    /// How deep the liquid looks at each vertex, from 0 to 255, row by row.
    #[debug(with = debug::trimmed_collection_fmt)]
    pub depths: Vec<u8>,
}

/// The MH2O chunk's data, as stored.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mh2o {
    #[debug(with = debug::trimmed_collection_fmt)]
    pub data: Vec<u8>,
}

impl Mh2o {
    fn reader_at(&self, offset: u32) -> Cursor<&[u8]> {
        let mut reader = Cursor::new(self.data.as_slice());
        reader.set_position(offset as u64);
        reader
    }

    /// The 256 chunk headers, by chunk index (Y index * 16 + X index).
    pub fn headers(&self) -> Result<Vec<Mh2oHeader>> {
        let mut reader = self.reader_at(0);
        (0..CHUNKS_PER_TILE).map(|_| reader.wow_read()).collect()
    }

    /// A chunk's layers.
    pub fn instances(&self, header: &Mh2oHeader) -> Result<Vec<Mh2oInstance>> {
        if header.instances_offset == 0 {
            return Ok(Vec::new());
        }
        let mut reader = self.reader_at(header.instances_offset);
        (0..header.layer_count).map(|_| reader.wow_read()).collect()
    }

    pub fn attributes(&self, header: &Mh2oHeader) -> Result<Option<Mh2oAttributes>> {
        if header.attributes_offset == 0 {
            return Ok(None);
        }
        self.reader_at(header.attributes_offset)
            .wow_read()
            .map(Some)
    }

    /// Every liquid layer of the tile, on each chunk's 9x9 grid.
    pub fn liquid_layers(&self) -> Result<Vec<LiquidLayer>> {
        let mut layers = Vec::new();
        for (chunk, header) in self.headers()?.iter().enumerate() {
            for instance in self.instances(header)? {
                layers.push(self.grid_layer(chunk as u16, &instance)?);
            }
        }
        Ok(layers)
    }

    fn grid_layer(&self, chunk: u16, instance: &Mh2oInstance) -> Result<LiquidLayer> {
        let x_offset = instance.x_offset as usize;
        let y_offset = instance.y_offset as usize;
        let width = instance.width as usize;
        let height = instance.height as usize;
        let (min_height, max_height) = (instance.min_height, instance.max_height);

        if x_offset + width > LIQUID_CELLS_PER_SIDE || y_offset + height > LIQUID_CELLS_PER_SIDE {
            return Err(WowDataError::InvalidData(format!(
                "liquid in chunk {chunk} covers {width}x{height} cells at {x_offset},{y_offset}"
            )));
        }

        let mut cells = 0u64;
        let exists = match instance.exists_offset {
            0 => None,
            offset => {
                let mut bitmap = vec![0; (width * height).div_ceil(8)];
                self.reader_at(offset).read_exact(&mut bitmap)?;
                Some(bitmap)
            }
        };
        for row in 0..height {
            for column in 0..width {
                let bit = row * width + column;
                let set = exists
                    .as_ref()
                    .is_none_or(|bitmap| bitmap[bit / 8] & (1 << (bit % 8)) != 0);
                if set {
                    cells |= 1 << ((row + y_offset) * LIQUID_CELLS_PER_SIDE + column + x_offset);
                }
            }
        }

        let mut heights = vec![min_height; LIQUID_VERTICES];
        let mut depths = vec![FULL_DEPTH; LIQUID_VERTICES];
        if instance.vertex_data_offset != 0 {
            let format = instance.vertex_format;
            let stored = (width + 1) * (height + 1);
            let has_heights = format != FORMAT_DEPTH_ONLY;
            let has_depths = format != FORMAT_HEIGHT_AND_TEXTURE;

            let mut reader = self.reader_at(instance.vertex_data_offset);
            let stored_heights: Vec<f32> = if has_heights {
                (0..stored)
                    .map(|_| reader.wow_read())
                    .collect::<Result<_>>()?
            } else {
                Vec::new()
            };
            if format == FORMAT_ALL {
                reader.seek(SeekFrom::Current(
                    (stored * TEXTURE_COORDINATE_BYTES) as i64,
                ))?;
            }
            let mut stored_depths = vec![0; if has_depths { stored } else { 0 }];
            reader.read_exact(&mut stored_depths)?;

            for row in 0..=height {
                for column in 0..=width {
                    let source = row * (width + 1) + column;
                    let target = (row + y_offset) * LIQUID_VERTICES_PER_SIDE + column + x_offset;
                    if has_heights {
                        // A NaN stays at the minimum, as clamp would pass it through.
                        let value = stored_heights[source];
                        if value.is_finite() && min_height <= max_height {
                            heights[target] = value.clamp(min_height, max_height);
                        }
                    }
                    if has_depths {
                        depths[target] = stored_depths[source];
                    }
                }
            }
        }

        Ok(LiquidLayer {
            chunk,
            liquid_type: instance.liquid_type,
            vertex_format: instance.vertex_format,
            cells,
            heights,
            depths,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER_BYTES: usize = 12;
    const INSTANCE_BYTES: usize = 24;
    const EXTRA: u32 = (CHUNKS_PER_TILE * HEADER_BYTES + INSTANCE_BYTES) as u32;

    /// An MH2O chunk with one layer in chunk 17, whose extra data starts after the headers.
    fn chunk_with(instance: Mh2oInstance, extra: &[u8]) -> Mh2o {
        let mut headers = vec![Mh2oHeader::default(); CHUNKS_PER_TILE];
        headers[17] = Mh2oHeader {
            instances_offset: (CHUNKS_PER_TILE * HEADER_BYTES) as u32,
            layer_count: 1,
            attributes_offset: 0,
        };
        let mut data = Vec::new();
        WowHeaderW::wow_write(headers.as_slice(), &mut data).unwrap();
        instance.wow_write(&mut data).unwrap();
        data.extend_from_slice(extra);
        Mh2o { data }
    }

    fn instance(format: u16, area: [u8; 4], exists: u32, vertices: u32) -> Mh2oInstance {
        Mh2oInstance {
            liquid_type: 5,
            vertex_format: format,
            min_height: 10.0,
            max_height: 20.0,
            x_offset: area[0],
            y_offset: area[1],
            width: area[2],
            height: area[3],
            exists_offset: exists,
            vertex_data_offset: vertices,
        }
    }

    fn floats(values: &[f32]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    #[test]
    fn records_have_the_stored_sizes() {
        assert_eq!(Mh2oHeader::default().wow_size(), HEADER_BYTES);
        assert_eq!(Mh2oInstance::default().wow_size(), INSTANCE_BYTES);
    }

    #[test]
    fn places_vertices_and_depths_on_the_full_grid() {
        let mut extra = floats(&[11.0, 12.0, 13.0, 14.0]);
        extra.extend_from_slice(&[1, 2, 3, 4]);

        let layers = chunk_with(instance(0, [2, 3, 1, 1], 0, EXTRA), &extra)
            .liquid_layers()
            .unwrap();

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
        let mh2o = chunk_with(instance(0, [0, 0, 3, 2], EXTRA, 0), &[0b0010_1001]);
        let layer = &mh2o.liquid_layers().unwrap()[0];
        assert_eq!(layer.cells, 1 | (1 << 8) | (1 << (8 + 2)));
    }

    #[test]
    fn depth_only_format_stays_at_the_minimum_height() {
        let mh2o = chunk_with(instance(2, [0, 0, 1, 1], 0, EXTRA), &[9, 8, 7, 6]);
        let layer = &mh2o.liquid_layers().unwrap()[0];
        assert_eq!(layer.heights[0], 10.0);
        assert_eq!(layer.depths[0], 9);
        assert_eq!(layer.depths[10], 6);
    }

    #[test]
    fn height_only_format_counts_as_fully_deep() {
        let mh2o = chunk_with(instance(1, [0, 0, 1, 1], 0, EXTRA), &floats(&[15.0; 4]));
        let layer = &mh2o.liquid_layers().unwrap()[0];
        assert_eq!(layer.heights[0], 15.0);
        assert_eq!(layer.depths[0], FULL_DEPTH);
    }

    #[test]
    fn full_format_has_depths_after_texture_coordinates() {
        let mut extra = floats(&[15.0; 4]);
        extra.extend_from_slice(&[0xAA; 16]);
        extra.extend_from_slice(&[1, 2, 3, 4]);
        let mh2o = chunk_with(instance(3, [0, 0, 1, 1], 0, EXTRA), &extra);
        let layer = &mh2o.liquid_layers().unwrap()[0];
        assert_eq!(layer.heights[0], 15.0);
        assert_eq!(layer.depths[0], 1);
        assert_eq!(layer.depths[10], 4);
    }

    #[test]
    fn heights_outside_the_stated_range_are_limited() {
        let mut extra = floats(&[1000.0, -1000.0, f32::NAN, 15.0]);
        extra.extend_from_slice(&[0; 4]);
        let mh2o = chunk_with(instance(0, [0, 0, 1, 1], 0, EXTRA), &extra);
        let layer = &mh2o.liquid_layers().unwrap()[0];
        assert_eq!(layer.heights[0], 20.0);
        assert_eq!(layer.heights[1], 10.0);
        assert_eq!(layer.heights[9], 10.0);
        assert_eq!(layer.heights[10], 15.0);
    }

    #[test]
    fn rejects_areas_off_the_grid_and_short_data() {
        let mh2o = chunk_with(instance(0, [7, 0, 2, 1], 0, 0), &[]);
        assert!(mh2o.liquid_layers().is_err());
        let mh2o = chunk_with(instance(0, [0, 0, 1, 1], 0, EXTRA), &[0; 3]);
        assert!(mh2o.liquid_layers().is_err());
    }

    #[test]
    fn a_chunk_without_liquid_has_no_layers() {
        let mh2o = Mh2o {
            data: vec![0; CHUNKS_PER_TILE * HEADER_BYTES],
        };
        assert!(mh2o.liquid_layers().unwrap().is_empty());
    }
}
