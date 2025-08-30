pub mod common;
pub mod distance;
pub mod prismatic;
pub mod revolute;
pub mod shoulder;
pub mod spherical;
pub mod weld;

pub use distance::{DSTJ, JointDistance};
pub use prismatic::{JointPrismatic, PRS2, PRSJ};
pub use revolute::{JointRevolute, REV2, REVJ};
pub use shoulder::{JointShoulder, SHJ2, SHOJ};
pub use spherical::{JointSpherical, SPHJ};
pub use weld::{JointWeld, WELJ, WLJ2, WLJ3};

use wow_alchemy_data::prelude::*;
use wow_alchemy_data::types::MagicStr;
use wow_alchemy_data::utils::string_to_inverted_magic;
use wow_alchemy_data_derive::{WowEnumFrom, WowHeaderR, WowHeaderW};

pub const JOIN: MagicStr = string_to_inverted_magic("JOIN");

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, WowEnumFrom, WowHeaderR, WowHeaderW)]
#[wow_data(from_type=u16)]
pub enum JointType {
    #[default]
    #[wow_data(expr = 0)]
    Spherical = 0,
    #[wow_data(expr = 1)]
    Shoulder = 1,
    #[wow_data(expr = 2)]
    Weld = 2,
    #[wow_data(expr = 3)]
    Revolute = 3,
    #[wow_data(expr = 4)]
    Prismatic = 4,
    #[wow_data(expr = 5)]
    Distance = 5,
}

#[derive(Debug, Clone, Default, WowHeaderR, WowHeaderW)]
pub struct Joint {
    pub body_a_idx: u32,
    pub body_b_idx: u32,
    pub _unknown: [u8; 4],
    pub joint_type: JointType,
    pub joint_id: i16,
}
