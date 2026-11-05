//! The file layout of DBC and early DB2 tables: a header, fixed-size records, and a block of
//! null-terminated strings that string fields point into by byte offset.
//!
//! Supported are WDBC (every `.dbc` up to Cataclysm, 3.3.5a's included) and WDB2 (Cataclysm
//! and Mists of Pandaria `.db2`). The Legion formats (WDB3, WDB4, WDB5, WDB6 and WDC*) have
//! offset maps, id lists, copy tables and packed fields this crate doesn't read, so they are
//! refused rather than read wrong.

use std::borrow::Cow;
use std::io::SeekFrom;

use custom_debug::Debug;
use wow_alchemy_data::error::{Result, WowDataError};
use wow_alchemy_data::prelude::*;
use wow_alchemy_data::types::{WowStructR, WowStructW};
use wow_alchemy_data_derive::{WowEnumFrom, WowHeaderR, WowHeaderW};

pub const WDBC: MagicStr = *b"WDBC";
pub const WDB2: MagicStr = *b"WDB2";

/// The first WDB2 build whose header has the id range, locale and copy table size.
pub const WDB2_ID_RANGE_MIN_BUILD: u32 = 12881;

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, WowEnumFrom, WowHeaderR, WowHeaderW,
)]
#[wow_data(from_type = MagicStr)]
pub enum DbcVersion {
    /// Every `.dbc` up to Cataclysm.
    #[default]
    #[wow_data(expr = WDBC)]
    WDBC,
    /// Cataclysm and Mists of Pandaria `.db2`.
    #[wow_data(expr = WDB2)]
    WDB2,
}

impl DataVersion for DbcVersion {}

/// What WDB2 adds to the header.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
pub struct Wdb2Header {
    pub table_hash: u32,
    pub build: u32,
    pub timestamp_last_written: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
#[wow_data(version = DbcVersion)]
pub enum Wdb2Fields {
    #[default]
    None,
    #[wow_data(read_if = version >= DbcVersion::WDB2)]
    Some(Wdb2Header),
}

/// Read after the header in WDB2 files from build [`WDB2_ID_RANGE_MIN_BUILD`] on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
pub struct Wdb2IdRange {
    pub min_id: u32,
    /// When not 0, an id index follows: see [`WdbFile::id_index`].
    pub max_id: u32,
    pub locale: u32,
    /// Always 0 in WDB2.
    pub copy_table_size: u32,
}

/// The header, after the magic.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
#[wow_data(version = DbcVersion)]
pub struct WdbHeader {
    pub record_count: u32,
    /// Columns. Usually each is 4 bytes, but some 3.3.5a tables have 1-byte fields
    /// (SpellItemEnchantmentCondition), so `record_size` is what gives the layout.
    pub field_count: u32,
    pub record_size: u32,
    pub string_block_size: u32,
    #[wow_data(versioned)]
    pub wdb2: Wdb2Fields,
}

/// A DBC or WDB2 table, held in memory: the raw records and the string block.
///
/// Fields are read raw with [`WdbFile::field_u32`] and strings by offset with
/// [`WdbFile::string_bytes`], or whole records typed by a WoWDBDefs definition with
/// [`WdbFile::read_record`]. Writing it back gives the same bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct WdbFile {
    pub version: DbcVersion,
    pub header: WdbHeader,
    /// WDB2 from build [`WDB2_ID_RANGE_MIN_BUILD`] on.
    pub id_range: Option<Wdb2IdRange>,
    /// WDB2 with a `max_id`: for each id from `min_id` to `max_id`, the record holding it.
    #[debug(skip)]
    pub id_index: Vec<u32>,
    /// WDB2 with a `max_id`: for each id from `min_id` to `max_id`, the length of its strings.
    #[debug(skip)]
    pub string_lengths: Vec<u16>,
    /// `record_count * record_size` bytes.
    #[debug(skip)]
    pub records: Vec<u8>,
    #[debug(skip)]
    pub strings: StringBlock,
}

impl Default for WdbFile {
    fn default() -> Self {
        Self {
            version: DbcVersion::WDBC,
            header: WdbHeader::default(),
            id_range: None,
            id_index: Vec::new(),
            string_lengths: Vec::new(),
            records: Vec::new(),
            strings: StringBlock::default(),
        }
    }
}

/// Reads `len` bytes, or fails saying how many were there, before allocating them.
fn read_block<R: Read + Seek>(reader: &mut R, len: u64, what: &str) -> Result<Vec<u8>> {
    let pos = reader.stream_position()?;
    let end = reader.seek(SeekFrom::End(0))?;
    reader.seek(SeekFrom::Start(pos))?;
    let available = end.saturating_sub(pos);
    if len > available {
        return Err(WowDataError::InvalidData(format!(
            "the {what} is {len} bytes, but only {available} are left in the file"
        )));
    }
    let mut bytes = vec![0; len as usize];
    reader.read_exact(&mut bytes)?;
    Ok(bytes)
}

impl WowStructR for WdbFile {
    fn wow_read<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let magic: MagicStr = reader.wow_read()?;
        let version = DbcVersion::try_from(magic).map_err(|_| {
            WowDataError::InvalidData(format!(
                "unsupported table format {:?}: only WDBC and WDB2 are read",
                String::from_utf8_lossy(&magic)
            ))
        })?;
        let header: WdbHeader = reader.wow_read_versioned(version)?;

        let mut file = Self {
            version,
            header,
            ..Self::default()
        };

        if let Wdb2Fields::Some(wdb2) = header.wdb2
            && wdb2.build >= WDB2_ID_RANGE_MIN_BUILD
        {
            let range: Wdb2IdRange = reader.wow_read()?;
            if range.copy_table_size != 0 {
                return Err(WowDataError::InvalidData(format!(
                    "WDB2 copy tables aren't read (this one is {} bytes)",
                    range.copy_table_size
                )));
            }
            if range.max_id != 0 {
                let span = range.max_id.checked_sub(range.min_id).ok_or_else(|| {
                    WowDataError::InvalidData(format!(
                        "WDB2 max_id {} is below min_id {}",
                        range.max_id, range.min_id
                    ))
                })?;
                let count = span as u64 + 1;
                file.id_index = read_block(reader, count * 4, "id index")?
                    .chunks_exact(4)
                    .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
                    .collect();
                file.string_lengths = read_block(reader, count * 2, "string length index")?
                    .chunks_exact(2)
                    .map(|b| u16::from_le_bytes(b.try_into().unwrap()))
                    .collect();
            }
            file.id_range = Some(range);
        }

        let records_len = header.record_count as u64 * header.record_size as u64;
        file.records = read_block(reader, records_len, "record block")?;
        file.strings = StringBlock {
            bytes: read_block(reader, header.string_block_size as u64, "string block")?,
        };
        Ok(file)
    }
}

impl WowStructW for WdbFile {
    fn wow_write<W: Write + Seek>(&self, writer: &mut W) -> Result<()> {
        let records_len = self.header.record_count as usize * self.header.record_size as usize;
        if self.records.len() != records_len
            || self.strings.bytes.len() != self.header.string_block_size as usize
        {
            return Err(WowDataError::InvalidData(
                "the header's record and string block sizes don't match the data".into(),
            ));
        }
        writer.wow_write(&self.version)?;
        writer.wow_write(&self.header)?;
        if let Some(range) = &self.id_range {
            writer.wow_write(range)?;
            self.id_index.as_slice().wow_write(writer)?;
            self.string_lengths.as_slice().wow_write(writer)?;
        }
        writer.write_all(&self.records)?;
        writer.wow_write(&self.strings)?;
        Ok(())
    }
}

impl WdbFile {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        Self::wow_read(&mut std::io::Cursor::new(bytes))
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut out = std::io::Cursor::new(Vec::new());
        self.wow_write(&mut out)?;
        Ok(out.into_inner())
    }

    pub fn record_count(&self) -> u32 {
        self.header.record_count
    }

    pub fn field_count(&self) -> u32 {
        self.header.field_count
    }

    pub fn record_size(&self) -> u32 {
        self.header.record_size
    }

    /// Record `index`'s bytes, or `None` past the last record.
    pub fn record(&self, index: u32) -> Option<&[u8]> {
        if index >= self.header.record_count {
            return None;
        }
        let size = self.header.record_size as usize;
        let start = index as usize * size;
        self.records.get(start..start + size)
    }

    /// Each record's bytes, in order.
    pub fn records(&self) -> impl ExactSizeIterator<Item = &[u8]> {
        (0..self.header.record_count).map(|i| self.record(i).unwrap())
    }

    /// The little-endian 32-bit value at byte `field * 4` of record `record`. `None` when the
    /// record doesn't exist or those 4 bytes go past `record_size`. `field_count` isn't
    /// checked: `record_size` is what bounds the data.
    pub fn field_u32(&self, record: u32, field: u32) -> Option<u32> {
        let offset = field as usize * 4;
        let bytes = self.record(record)?.get(offset..offset + 4)?;
        Some(u32::from_le_bytes(bytes.try_into().unwrap()))
    }

    /// The string at byte `offset` of the string block, up to its null or the block's end,
    /// borrowed from the block. Offsets may point into the middle of a string, where writers
    /// share common suffixes. `None` when `offset` is at or past the block's end.
    pub fn string_bytes(&self, offset: u32) -> Option<&[u8]> {
        self.strings.get_bytes(offset)
    }

    /// [`Self::string_bytes`] as text, with invalid UTF-8 replaced.
    pub fn string(&self, offset: u32) -> Option<Cow<'_, str>> {
        self.string_bytes(offset).map(String::from_utf8_lossy)
    }

    /// The string that field `field` of record `record` points to: [`Self::field_u32`] then
    /// [`Self::string_bytes`].
    pub fn field_string(&self, record: u32, field: u32) -> Option<&[u8]> {
        self.string_bytes(self.field_u32(record, field)?)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Two records of [id, name]; "Forest" shares the tail of "Elwynn Forest".
    pub(crate) fn sample_dbc() -> Vec<u8> {
        let strings = b"\0Elwynn Forest\0";
        let mut data = b"WDBC".to_vec();
        for value in [2u32, 2, 8, strings.len() as u32] {
            data.extend(value.to_le_bytes());
        }
        for value in [12u32, 1, 40, 8] {
            data.extend(value.to_le_bytes());
        }
        data.extend(strings);
        data
    }

    #[test]
    fn reads_header_fields_and_strings() {
        let dbc = WdbFile::from_bytes(&sample_dbc()).unwrap();
        assert_eq!(dbc.version, DbcVersion::WDBC);
        assert_eq!(dbc.header.wdb2, Wdb2Fields::None);
        assert_eq!(
            (dbc.record_count(), dbc.field_count(), dbc.record_size()),
            (2, 2, 8)
        );
        assert_eq!(dbc.field_u32(0, 0), Some(12));
        assert_eq!(dbc.field_u32(1, 0), Some(40));
        assert_eq!(dbc.field_u32(1, 1), Some(8));
        assert_eq!(dbc.field_u32(2, 0), None);
        assert_eq!(dbc.field_u32(0, 2), None);
        assert_eq!(dbc.field_u32(u32::MAX, u32::MAX), None);
        assert_eq!(dbc.field_string(0, 1), Some(&b"Elwynn Forest"[..]));
        assert_eq!(dbc.field_string(1, 1), Some(&b"Forest"[..]));
        assert_eq!(dbc.string(0).as_deref(), Some(""));
        assert_eq!(dbc.string(14).as_deref(), Some(""));
        assert_eq!(dbc.string(15), None);
        assert_eq!(dbc.string(u32::MAX), None);
        assert_eq!(dbc.records().count(), 2);
    }

    #[test]
    fn a_string_without_its_null_ends_at_the_block() {
        let mut bytes = sample_dbc();
        bytes.pop();
        let size = (bytes.len() - 36) as u32;
        bytes[16..20].copy_from_slice(&size.to_le_bytes());
        let dbc = WdbFile::from_bytes(&bytes).unwrap();
        assert_eq!(dbc.field_string(1, 1), Some(&b"Forest"[..]));
    }

    #[test]
    fn writes_back_the_same_bytes() {
        let bytes = sample_dbc();
        assert_eq!(
            WdbFile::from_bytes(&bytes).unwrap().to_bytes().unwrap(),
            bytes
        );
    }

    #[test]
    fn truncated_files_are_errors() {
        let bytes = sample_dbc();
        assert!(WdbFile::from_bytes(&bytes[..10]).is_err());
        assert!(WdbFile::from_bytes(&bytes[..30]).is_err());
        assert!(WdbFile::from_bytes(&bytes[..bytes.len() - 1]).is_err());
    }

    #[test]
    fn later_formats_are_refused() {
        for magic in [b"WDB3", b"WDB4", b"WDB5", b"WDC3"] {
            let mut bytes = sample_dbc();
            bytes[..4].copy_from_slice(magic);
            let err = WdbFile::from_bytes(&bytes).unwrap_err().to_string();
            assert!(err.contains(std::str::from_utf8(magic).unwrap()), "{err}");
        }
    }

    fn wdb2(build: u32, min_id: u32, max_id: u32) -> Vec<u8> {
        let strings = b"\0a\0";
        let mut data = b"WDB2".to_vec();
        for value in [1u32, 1, 4, strings.len() as u32, 0xABCD, build, 7] {
            data.extend(value.to_le_bytes());
        }
        if build >= WDB2_ID_RANGE_MIN_BUILD {
            for value in [min_id, max_id, 0, 0] {
                data.extend(value.to_le_bytes());
            }
            if max_id != 0 {
                for i in min_id..=max_id {
                    data.extend((i - min_id).to_le_bytes());
                }
                for _ in min_id..=max_id {
                    data.extend(1u16.to_le_bytes());
                }
            }
        }
        data.extend(1u32.to_le_bytes());
        data.extend(strings);
        data
    }

    #[test]
    fn reads_wdb2_headers_and_id_index() {
        for (build, min_id, max_id) in [(12340, 0, 0), (15595, 0, 0), (15595, 5, 7)] {
            let bytes = wdb2(build, min_id, max_id);
            let file = WdbFile::from_bytes(&bytes).unwrap();
            assert_eq!(file.version, DbcVersion::WDB2);
            assert_eq!(
                file.header.wdb2,
                Wdb2Fields::Some(Wdb2Header {
                    table_hash: 0xABCD,
                    build,
                    timestamp_last_written: 7
                })
            );
            assert_eq!(file.id_range.is_some(), build >= WDB2_ID_RANGE_MIN_BUILD);
            let ids = if max_id != 0 { max_id - min_id + 1 } else { 0 };
            assert_eq!(file.id_index.len(), ids as usize);
            assert_eq!(file.string_lengths.len(), ids as usize);
            assert_eq!(file.field_u32(0, 0), Some(1));
            assert_eq!(file.field_string(0, 0), Some(&b"a"[..]));
            assert_eq!(file.to_bytes().unwrap(), bytes);
        }
    }
}
