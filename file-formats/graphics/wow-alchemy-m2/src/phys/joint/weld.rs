use wow_alchemy_data::prelude::*;
use wow_alchemy_data::types::{MagicStr, Mat3x4};
use wow_alchemy_data::utils::string_to_inverted_magic;
use wow_alchemy_data_derive::{WowEnumFrom, WowHeaderR, WowHeaderW};

use super::common::FrequencyDamping;

pub const WELJ: MagicStr = string_to_inverted_magic("WELJ");
pub const WLJ2: MagicStr = string_to_inverted_magic("WLJ2");
pub const WLJ3: MagicStr = string_to_inverted_magic("WLJ3");

#[derive(Debug, Clone, Default, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, WowEnumFrom)]
#[wow_alchemy_data(from_type=MagicStr)]
pub enum Version {
    #[wow_alchemy_data(expr = WELJ)]
    V1,
    #[wow_alchemy_data(expr = WLJ2)]
    V2,
    #[default]
    #[wow_alchemy_data(expr = WLJ3)]
    V3,
}

impl DataVersion for Version {}

#[derive(Debug, Clone, WowHeaderR, WowHeaderW)]
#[wow_alchemy_data(version = Version)]
pub enum VGTE2<T: Default + WowHeaderR + WowHeaderW> {
    None,

    #[wow_alchemy_data(read_if = version >= Version::V2)]
    Some(T),
}

impl<T: Default + WowHeaderR + WowHeaderW> Default for VGTE2<T> {
    fn default() -> Self {
        Self::Some(T::default())
    }
}

#[derive(Debug, Clone, WowHeaderR, WowHeaderW)]
#[wow_alchemy_data(version = Version)]
pub enum VGTE3<T: Default + WowHeaderR + WowHeaderW> {
    None,

    #[wow_alchemy_data(read_if = version >= Version::V3)]
    Some(T),
}

impl<T: Default + WowHeaderR + WowHeaderW> Default for VGTE3<T> {
    fn default() -> Self {
        Self::Some(T::default())
    }
}

#[derive(Debug, Clone, Default, WowHeaderR, WowHeaderW)]
#[wow_alchemy_data(version = Version)]
pub struct JointWeld {
    pub frame_a: Mat3x4,
    pub frame_b: Mat3x4,
    pub angular: FrequencyDamping,
    #[wow_alchemy_data(versioned)]
    pub linear: VGTE2<FrequencyDamping>,
    #[wow_alchemy_data(versioned)]
    pub _x70: VGTE3<f32>,
}
