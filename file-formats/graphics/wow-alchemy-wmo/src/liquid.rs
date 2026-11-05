//! A group's liquid, MLIQ: a grid of heights over cells that each have liquid or not.

use std::io::Cursor;

use wow_alchemy_data::error::{Result, WowDataError};
use wow_alchemy_data::prelude::*;
use wow_alchemy_data::types::C3Vector;
use wow_alchemy_data_derive::{WowHeaderR, WowHeaderW};

use crate::group::GroupFlags;
use crate::root::RootFlags;

/// An MLIQ cell's low bits say this when the cell has no liquid.
pub const CELL_NO_LIQUID: u8 = 0x0F;

/// The start of MLIQ, 30 bytes.
#[derive(Debug, Clone, Default, PartialEq, WowHeaderR, WowHeaderW)]
pub struct LiquidHeader {
    pub vertices_x: u32,
    pub vertices_y: u32,
    pub cells_x: u32,
    pub cells_y: u32,
    /// The grid's corner, in the WMO's own space.
    pub corner: C3Vector,
    /// Index into MOMT.
    pub material: u16,
}

/// One vertex of the grid: 4 bytes of flow (water: two flow values, a percentage and a filler)
/// or texture coordinates (magma and slime: two i16), then its height.
#[derive(Debug, Clone, Copy, Default, PartialEq, WowHeaderR, WowHeaderW)]
pub struct LiquidVertex {
    pub data: [u8; 4],
    pub height: f32,
}

/// The MLIQ chunk. Vertices and cells run row by row, X fastest.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WmoLiquid {
    pub header: LiquidHeader,
    /// `vertices_x * vertices_y` of them.
    pub vertices: Vec<LiquidVertex>,
    /// `cells_x * cells_y` bytes: the low 4 bits are the cell's kind of liquid (0 water, 1 ocean,
    /// 2 magma, 3 slime, [`CELL_NO_LIQUID`] none), and the top ones flags.
    pub cells: Vec<u8>,
}

impl WmoLiquid {
    /// Reads the body of an MLIQ chunk. A grid with no cells has nothing past the header; one with
    /// cells must have one more vertex than cells along each side, and the chunk must hold
    /// exactly the grid.
    pub fn read(body: &[u8]) -> Result<Self> {
        let mut reader = Cursor::new(body);
        let header: LiquidHeader = reader.wow_read()?;
        let (vertices, cells) = if header.cells_x == 0 || header.cells_y == 0 {
            (0, 0)
        } else if header.cells_x + 1 != header.vertices_x || header.cells_y + 1 != header.vertices_y
        {
            return Err(WowDataError::InvalidData(format!(
                "MLIQ has {}x{} cells but {}x{} vertices",
                header.cells_x, header.cells_y, header.vertices_x, header.vertices_y
            )));
        } else {
            (
                header.vertices_x as usize * header.vertices_y as usize,
                header.cells_x as usize * header.cells_y as usize,
            )
        };

        let needed = 30 + vertices * 8 + cells;
        if body.len() != needed {
            return Err(WowDataError::InvalidData(format!(
                "MLIQ has {} bytes, but its grid needs {needed}",
                body.len()
            )));
        }

        let vertices = (0..vertices)
            .map(|_| reader.wow_read())
            .collect::<Result<_>>()?;
        Ok(Self {
            header,
            vertices,
            cells: body[needed - cells..].to_vec(),
        })
    }

    /// Whether the grid has cells. The wiki says a group with an MLIQ without cells can still be
    /// full of liquid, up to the top of its bounds; nothing here handles that.
    pub fn has_grid(&self) -> bool {
        !self.cells.is_empty()
    }

    pub fn heights(&self) -> Vec<f32> {
        self.vertices.iter().map(|v| v.height).collect()
    }

    pub fn cell_has_liquid(cell: u8) -> bool {
        cell & CELL_NO_LIQUID != CELL_NO_LIQUID
    }

    /// One byte per cell, row by row: 1 when the cell has liquid, else 0.
    pub fn liquid_cells(&self) -> Vec<u8> {
        self.cells
            .iter()
            .map(|&c| u8::from(Self::cell_has_liquid(c)))
            .collect()
    }

    /// The kind the first cell with liquid states: 0 water, 1 ocean, 2 magma, 3 slime.
    pub fn first_cell_kind(&self) -> Option<u8> {
        self.cells
            .iter()
            .map(|c| c & CELL_NO_LIQUID)
            .find(|&kind| kind != CELL_NO_LIQUID)
    }

    /// The liquid's row in LiquidType.dbc, worked out as the 3.3.5a client does, from the root's
    /// flags and the group's flags and liquid field. 0 when it can't be told.
    pub fn liquid_type(
        &self,
        root_flags: RootFlags,
        group_flags: GroupFlags,
        group_liquid: u32,
    ) -> u32 {
        let mut entry = if root_flags.contains(RootFlags::USE_LIQUID_TYPE_DBC_ID) {
            group_liquid
        } else if group_liquid == CELL_NO_LIQUID as u32 {
            0
        } else {
            group_liquid + 1
        };

        if entry == 0 {
            entry = self.first_cell_kind().map_or(0, |kind| kind as u32 + 1);
        }

        // Low values name a kind of liquid rather than a row.
        if entry == 0 || entry >= 21 {
            return entry;
        }
        match (entry - 1) & 3 {
            0 if group_flags.contains(GroupFlags::OCEAN_WATER) => 14,
            0 => 13,
            1 => 14,
            2 => 19,
            _ => 20,
        }
    }
}

impl WowHeaderW for WmoLiquid {
    fn wow_write<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.header.wow_write(writer)?;
        self.vertices.as_slice().wow_write(writer)?;
        writer.write_all(&self.cells)?;
        Ok(())
    }

    fn wow_size(&self) -> usize {
        self.header.wow_size() + self.vertices.len() * 8 + self.cells.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn floats(values: &[f32]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    /// A 2 by 1 grid whose second cell has magma.
    fn liquid_chunk() -> Vec<u8> {
        let mut data = Vec::new();
        for value in [3u32, 2, 2, 1] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&floats(&[1.0, 2.0, 3.0]));
        data.extend_from_slice(&7u16.to_le_bytes());
        for height in 0..6 {
            data.extend_from_slice(&[0; 4]);
            data.extend_from_slice(&(height as f32).to_le_bytes());
        }
        data.extend_from_slice(&[0x4F, 0x42]);
        data
    }

    #[test]
    fn reads_a_liquid_grid() {
        let liquid = WmoLiquid::read(&liquid_chunk()).unwrap();
        assert_eq!((liquid.header.vertices_x, liquid.header.vertices_y), (3, 2));
        assert_eq!(liquid.header.corner, C3Vector::new(1.0, 2.0, 3.0));
        assert_eq!(liquid.header.material, 7);
        assert_eq!(liquid.heights(), vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(liquid.liquid_cells(), vec![0, 1]);
        assert_eq!(liquid.first_cell_kind(), Some(2));
    }

    #[test]
    fn writes_a_liquid_grid_back_the_same() {
        let data = liquid_chunk();
        let liquid = WmoLiquid::read(&data).unwrap();
        let mut written = Vec::new();
        liquid.wow_write(&mut written).unwrap();
        assert_eq!(written, data);
        assert_eq!(liquid.wow_size(), data.len());
    }

    #[test]
    fn a_liquid_chunk_with_no_cells_has_no_grid_rather_than_an_error() {
        let mut data = vec![0u8; 30];
        data[0] = 1;
        data[4] = 1;
        let liquid = WmoLiquid::read(&data).unwrap();
        assert!(!liquid.has_grid());
        assert!(liquid.heights().is_empty());
        assert_eq!(liquid.first_cell_kind(), None);
    }

    #[test]
    fn rejects_a_truncated_liquid_grid() {
        let data = liquid_chunk();
        assert!(WmoLiquid::read(&data[..data.len() - 1]).is_err());
    }

    #[test]
    fn rejects_bytes_after_the_grid() {
        let mut data = liquid_chunk();
        data.push(0);
        assert!(WmoLiquid::read(&data).is_err());
    }

    #[test]
    fn rejects_cells_that_dont_fit_the_vertices() {
        let mut data = liquid_chunk();
        data[8] = 3;
        assert!(WmoLiquid::read(&data).is_err());
    }

    #[test]
    fn works_out_liquid_types_as_the_client_does() {
        // The first cell with liquid says kind 2, magma.
        let liquid = WmoLiquid::read(&liquid_chunk()).unwrap();
        let none = GroupFlags::empty();
        let plain = RootFlags::empty();

        // The root says the field is already a row.
        assert_eq!(
            liquid.liquid_type(RootFlags::USE_LIQUID_TYPE_DBC_ID, none, 41),
            41
        );
        // Otherwise it's one less than a kind.
        assert_eq!(liquid.liquid_type(plain, none, 0), 13);
        assert_eq!(liquid.liquid_type(plain, GroupFlags::OCEAN_WATER, 0), 14);
        assert_eq!(liquid.liquid_type(plain, none, 1), 14);
        assert_eq!(liquid.liquid_type(plain, none, 2), 19);
        assert_eq!(liquid.liquid_type(plain, none, 3), 20);
        // 15 means the cells decide.
        assert_eq!(liquid.liquid_type(plain, none, 15), 19);
        // With no cells to ask, it can't be told.
        assert_eq!(WmoLiquid::default().liquid_type(plain, none, 15), 0);
    }
}
