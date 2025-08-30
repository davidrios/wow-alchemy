use wow_alchemy_data::prelude::*;
use wow_alchemy_data::types::Color;
use wow_alchemy_data_derive::{WowDataR, WowHeaderR, WowHeaderW};

use crate::chunks::animation::M2AnimationTrackHeader;
use crate::version::MD20Version;

use super::animation::M2AnimationTrackData;

#[derive(Debug, Clone, WowHeaderR, WowHeaderW)]
#[wow_alchemy_data(version = MD20Version)]
pub struct M2ColorAnimationHeader {
    #[wow_alchemy_data(versioned)]
    pub color: M2AnimationTrackHeader<Color>,

    #[wow_alchemy_data(versioned)]
    pub alpha: M2AnimationTrackHeader<u16>,
}

#[derive(Debug, Clone, WowDataR)]
#[wow_alchemy_data(version = MD20Version, header = M2ColorAnimationHeader)]
pub struct M2ColorAnimationData {
    #[wow_alchemy_data(versioned)]
    pub color: M2AnimationTrackData<Color>,
    #[wow_alchemy_data(versioned)]
    pub alpha: M2AnimationTrackData<u16>,
}

#[derive(Debug, Clone)]
pub struct M2ColorAnimation {
    pub header: M2ColorAnimationHeader,
    pub data: M2ColorAnimationData,
}
