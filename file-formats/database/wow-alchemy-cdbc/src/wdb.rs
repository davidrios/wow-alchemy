use std::collections::HashMap;
use std::io::SeekFrom;

use custom_debug::Debug;
use wow_alchemy_data::error::Result as WDResult;
use wow_alchemy_data::{prelude::*, types::MagicStr};
use wow_alchemy_data_derive::{WowEnumFrom, WowHeaderR, WowHeaderW};
use wow_alchemy_utils::debug;

pub const WDBC: MagicStr = *b"WDBC";
pub const WDB2: MagicStr = *b"WDB2";
pub const WDB3: MagicStr = *b"WDB3";
pub const WDB4: MagicStr = *b"WDB4";
pub const WDB5: MagicStr = *b"WDB5";

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, WowEnumFrom, WowHeaderR, WowHeaderW,
)]
#[wow_data(from_type=MagicStr)]
pub enum DbcVersion {
    /// Original WDBC format, up to Legion or possibly WoD
    #[wow_data(expr = WDBC)]
    WDBC,
    /// V2, briefly used in Legion or maybe in WoD
    #[wow_data(expr = WDB2)]
    WDB2,
    /// V3, briefly used in Legion
    #[wow_data(expr = WDB3)]
    WDB3,
    /// V4, briefly used in Legion
    #[wow_data(expr = WDB4)]
    WDB4,
    /// V5, used in Legion+
    #[wow_data(expr = WDB5)]
    #[default]
    WDB5,
}

impl DataVersion for DbcVersion {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, WowHeaderR, WowHeaderW)]
#[wow_data(version = DbcVersion)]
pub enum Wdb2Fields {
    None,

    #[wow_data(read_if = version >= DbcVersion::WDB2)]
    Fields {
        table_hash: u32,
        build: u32,
        timestamp_last_written: u32,
        min_id: u32,
        max_id: u32,
        locale: u32,
        copy_table_size: u32,
    },
}

impl Default for Wdb2Fields {
    fn default() -> Self {
        Self::Fields {
            table_hash: 0,
            build: 0,
            timestamp_last_written: 0,
            min_id: 0,
            max_id: 0,
            locale: 0,
            copy_table_size: 0,
        }
    }
}

#[derive(Debug, Clone, WowHeaderR, WowHeaderW)]
#[wow_data(version = DbcVersion)]
pub enum VGTE4<T: Default + WowHeaderR + WowHeaderW> {
    None,

    #[wow_data(read_if = version >= DbcVersion::WDB4)]
    Some(T),
}

impl<T: Default + WowHeaderR + WowHeaderW> Default for VGTE4<T> {
    fn default() -> Self {
        Self::Some(T::default())
    }
}

#[derive(Debug, Clone, WowHeaderR, WowHeaderW)]
#[wow_data(version = DbcVersion)]
pub enum VGTE5<T: Default + WowHeaderR + WowHeaderW> {
    None,

    #[wow_data(read_if = version >= DbcVersion::WDB4)]
    Some(T),
}

impl<T: Default + WowHeaderR + WowHeaderW> Default for VGTE5<T> {
    fn default() -> Self {
        Self::Some(T::default())
    }
}

#[derive(Debug, Clone, Default, WowHeaderR, WowHeaderW)]
#[wow_data(version = DbcVersion)]
pub struct WdbHeader {
    pub record_count: u32,
    pub field_count: u32,
    pub record_size: u32,
    pub string_block_size: u32,
    #[wow_data(versioned)]
    pub wdb2: Wdb2Fields,
    #[wow_data(versioned)]
    pub flags: VGTE4<u32>,
    #[wow_data(versioned)]
    pub id_index: VGTE5<u32>,
}

impl WdbHeader {
    pub fn string_offset(&self) -> u64 {
        ((4 + self.wow_size()) as u32 + (self.record_size * self.record_count)) as u64
    }
}

#[derive(Debug)]
pub struct WdbFile {
    pub version: DbcVersion,
    pub header: WdbHeader,
    pub header_size: usize,
    #[debug(with = debug::trimmed_collection_fmt)]
    pub strings: Vec<String>,
    #[debug(skip)]
    pub string_pos: HashMap<usize, usize>,
}

impl WdbFile {
    pub fn wow_read<R: Read + Seek>(reader: &mut R) -> WDResult<Self> {
        reader.seek(SeekFrom::Start(0))?;
        let version: DbcVersion = MagicStr::wow_read(reader)?.try_into()?;

        let header: WdbHeader = reader.wow_read_versioned(version)?;

        let header_size = version.wow_size() + header.wow_size();

        let string_block_offset = header_size as u32 + (header.record_count * header.record_size);

        reader.seek(SeekFrom::Start(string_block_offset as u64))?;

        let mut string_block = vec![0u8; header.string_block_size as usize];
        reader.read_exact(&mut string_block)?;

        let strings: Vec<String> = string_block
            .split(|i| *i == 0)
            .map(|v| String::from_utf8_lossy(v).into())
            .collect();

        let mut string_pos = HashMap::new();
        let mut current_pos = 0;
        for (idx, item) in strings.iter().enumerate() {
            string_pos.insert(current_pos, idx);
            current_pos += item.len() + 1;
        }

        Ok(Self {
            version,
            header,
            header_size,
            strings,
            string_pos,
        })
    }

    pub fn get_string(&self, index: usize) -> Option<&String> {
        self.strings.get(index)
    }

    pub fn get_string_by_offset(&self, offset: usize) -> Option<&String> {
        if let Some(item) = self.string_pos.get(&offset) {
            self.get_string(*item)
        } else {
            None
        }
    }

    pub fn records_start_offset(&self) -> u64 {
        self.header_size as u64
    }
}
