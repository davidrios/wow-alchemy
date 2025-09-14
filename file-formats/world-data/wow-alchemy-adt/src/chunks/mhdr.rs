use wow_alchemy_data::types::MagicStr;
use wow_alchemy_data::{prelude::*, utils::string_to_inverted_magic};
use wow_alchemy_data_derive::{WowHeaderR, WowHeaderW};

pub const MHDR: MagicStr = string_to_inverted_magic("MHDR");

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
    #[wow_data(bitflags=u32)]
    pub struct MhdrFlags: u32 {
        const HAS_MFBO = 0x1;
        const NORTHREND = 0x2;
    }
}

#[derive(Debug, Clone, Default, WowHeaderR, WowHeaderW)]
pub struct Mhdr {
    pub flags: MhdrFlags,
    pub mcin: u32,
    pub mtex: u32,
    pub mmdx: u32,
    pub mmid: u32,
    pub mwmo: u32,
    pub mwid: u32,
    pub mddf: u32,
    pub modf: u32,
    pub mfbo: u32,
    pub mh2o: u32,
    pub mtxf: u32,
    pub mamp_value: u8,
    pub padding: [u8; 3],
    pub unused: [u32; 3],
}
