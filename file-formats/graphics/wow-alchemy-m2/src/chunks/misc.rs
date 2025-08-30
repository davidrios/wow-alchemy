use wow_alchemy_data::prelude::*;
use wow_alchemy_data::types::MagicStr;
use wow_alchemy_data_derive::{WowHeaderR, WowHeaderW};

pub const TXAC: MagicStr = *b"TXAC";

#[derive(Debug, Clone, Default, WowHeaderR, WowHeaderW)]
pub struct TXACData {
    pub _unknown1: u8,
    pub _unknown2: u8,
}
