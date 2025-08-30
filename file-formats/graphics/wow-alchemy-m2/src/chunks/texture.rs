use wow_alchemy_data::prelude::*;
use wow_alchemy_data::types::WowCharArray;
use wow_alchemy_data::{error::Result as WDResult, types::WowString};
use wow_alchemy_data_derive::{WowEnumFrom, WowHeaderR, WowHeaderW};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, WowEnumFrom, WowHeaderR, WowHeaderW)]
#[wow_data(from_type=u32)]
pub enum M2TextureType {
    /// Texture defined in filename
    #[default]
    #[wow_data(expr = 0)]
    Hardcoded = 0,
    /// Body + clothes
    #[wow_data(expr = 1)]
    Body = 1,
    /// Item, capes
    #[wow_data(expr = 2)]
    Item = 2,
    /// Weapon blade
    #[wow_data(expr = 3)]
    WeaponBlade = 3,
    /// Weapon handle
    #[wow_data(expr = 4)]
    WeaponHandle = 4,
    /// Environment
    #[wow_data(expr = 5)]
    Environment = 5,
    /// Hair, beard
    #[wow_data(expr = 6)]
    Hair = 6,
    #[wow_data(expr = 7)]
    FacialHair = 7,
    #[wow_data(expr = 8)]
    SkinExtra = 8,
    #[wow_data(expr = 9)]
    UISkin = 9,
    #[wow_data(expr = 10)]
    TaurenMane = 10,
    #[wow_data(expr = 11)]
    Monster1 = 11,
    #[wow_data(expr = 12)]
    Monster2 = 12,
    #[wow_data(expr = 13)]
    Monster3 = 13,
    #[wow_data(expr = 14)]
    ItemIcon = 14,
    #[wow_data(expr = 15)]
    GuildBgColor = 15,
    #[wow_data(expr = 16)]
    GuildEmblemColor = 16,
    #[wow_data(expr = 17)]
    GuildBorderColor = 17,
    #[wow_data(expr = 18)]
    GuildEmblem = 18,
    #[wow_data(expr = 19)]
    CharacterEyes = 19,
    #[wow_data(expr = 20)]
    CharacterAccessory = 20,
    #[wow_data(expr = 21)]
    CharacterSecondarySkin = 21,
    #[wow_data(expr = 22)]
    CharacterSecondaryHair = 22,
    #[wow_data(expr = 23)]
    CharacterSecondaryArmor = 23,
    #[wow_data(expr = 24)]
    Unknown1 = 24,
    #[wow_data(expr = 25)]
    Unknown2 = 25,
    #[wow_data(expr = 26)]
    Unknown3 = 26,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
    #[wow_data(bitflags=u32)]
    pub struct M2TextureFlags: u32 {
        /// Texture is wrapped horizontally
        const WRAP_X = 0x01;
        /// Texture is wrapped vertically
        const WRAP_Y = 0x02;
        /// Texture will not be replaced by other textures
        /// (character customization texture replacement)
        const NOT_REPLACEABLE = 0x04;
    }
}

#[derive(Debug, Clone, Default, WowHeaderR, WowHeaderW)]
pub struct M2TextureHeader {
    pub texture_type: M2TextureType,
    pub flags: M2TextureFlags,
    pub filename: WowCharArray,
}

impl M2TextureHeader {
    pub fn new(texture_type: M2TextureType, filename: WowCharArray) -> Self {
        Self {
            texture_type,
            flags: M2TextureFlags::empty(),
            filename,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct M2TextureData {
    pub filename: String,
}

impl WowDataR<M2TextureHeader> for M2TextureData {
    fn new_from_header<R: Read + Seek>(reader: &mut R, header: &M2TextureHeader) -> WDResult<Self> {
        Ok(Self {
            filename: String::from_wow_char_array(reader, header.filename.clone())?,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct M2Texture {
    pub header: M2TextureHeader,
    pub data: M2TextureData,
}
