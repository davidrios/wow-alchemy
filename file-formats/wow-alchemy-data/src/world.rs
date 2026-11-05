//! Records that more than one world format stores.

use wow_alchemy_data_derive::{WowHeaderR, WowHeaderW};

use crate::types::{BoundingBox, C3Vector};

mod wow_alchemy_data {
    pub use crate::*;
}

/// Where a WMO stands in the world: an entry of MODF, in ADTs and in WDTs of maps made of one WMO.
/// Positions run east, up and south from the map's north-west corner, and rotations are degrees.
#[derive(Debug, Clone, Default, PartialEq, WowHeaderR, WowHeaderW)]
pub struct MapObjectPlacement {
    /// Index into the file's MWID offsets, or of its MWMO names.
    pub name_id: u32,
    /// Shared by every tile that lists this placement.
    pub unique_id: u32,
    pub position: C3Vector,
    pub rotation: C3Vector,
    /// World bounds.
    pub extents: BoundingBox,
    pub flags: u16,
    pub doodad_set: u16,
    pub name_set: u16,
    /// 1024 is 1.0 from Legion on; 0 before.
    pub scale: u16,
}
