use wow_alchemy_data::types::MagicStr;
use wow_alchemy_data::{prelude::*, utils::string_to_inverted_magic};
use wow_alchemy_data_derive::{WowHeaderR, WowHeaderW};

pub const PHYT: MagicStr = string_to_inverted_magic("PHYT");

#[derive(Debug, Clone, Default, WowHeaderR, WowHeaderW)]
pub struct Phyt {
    pub phyt: u32,
}
