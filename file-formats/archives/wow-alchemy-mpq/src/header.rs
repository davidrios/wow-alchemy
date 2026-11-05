//! The user data header (`MPQ\x1B`) and the archive header (`MPQ\x1A`), versions 1 to 4.

use std::io::{Read, Seek, SeekFrom, Write};

use wow_alchemy_data::error::Result as WDResult;
use wow_alchemy_data::prelude::*;
use wow_alchemy_data::types::MagicStr;
use wow_alchemy_data_derive::{WowEnumFrom, WowHeaderR, WowHeaderW};

use crate::{Error, Result};

/// Magic of the archive header.
pub const MPQ_MAGIC: MagicStr = *b"MPQ\x1A";
/// Magic of the user data header.
pub const USER_DATA_MAGIC: MagicStr = *b"MPQ\x1B";

/// Archive headers start at multiples of this many bytes.
pub const HEADER_ALIGNMENT: u64 = 0x200;

/// Format version of an archive.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, WowEnumFrom, WowHeaderR, WowHeaderW,
)]
#[wow_data(from_type = u16)]
pub enum FormatVersion {
    /// The original format, with a 32-byte header.
    #[default]
    #[wow_data(expr = 0)]
    V1,
    /// The Burning Crusade's: 44 bytes, adding the hi-block table and 64-bit table offsets.
    #[wow_data(expr = 1)]
    V2,
    /// Cataclysm beta's: 68 bytes, adding a 64-bit archive size and the HET and BET tables.
    #[wow_data(expr = 2)]
    V3,
    /// Cataclysm's: 208 bytes, adding stored table sizes and MD5s.
    #[wow_data(expr = 3)]
    V4,
}

impl DataVersion for FormatVersion {}

impl FormatVersion {
    /// Size of this version's header.
    pub fn header_size(self) -> u32 {
        match self {
            FormatVersion::V1 => 0x20,
            FormatVersion::V2 => 0x2C,
            FormatVersion::V3 => 0x44,
            FormatVersion::V4 => 0xD0,
        }
    }

    fn previous(self) -> Self {
        match self {
            FormatVersion::V4 => FormatVersion::V3,
            FormatVersion::V3 => FormatVersion::V2,
            _ => FormatVersion::V1,
        }
    }
}

/// The header that may come before the archive, as in Starcraft II maps.
#[derive(Debug, Clone, PartialEq, Eq, WowHeaderR, WowHeaderW)]
pub struct UserDataHeader {
    /// `MPQ\x1B`.
    pub magic: MagicStr,
    /// Bytes reserved for user data.
    pub user_data_size: u32,
    /// Offset of the archive header from this header.
    pub header_offset: u32,
    /// Bytes of user data that follow this header.
    pub user_data_header_size: u32,
}

/// Fields of the header every version has.
#[derive(Debug, Clone, PartialEq, Eq, WowHeaderR, WowHeaderW)]
pub struct MpqHeaderV1 {
    /// `MPQ\x1A`.
    pub magic: MagicStr,
    /// Size of the header in bytes.
    pub header_size: u32,
    /// Size of the archive (superseded by the 64-bit size in v3).
    pub archive_size: u32,
    /// Format version.
    pub format_version: FormatVersion,
    /// Sector size as a power of two times 512.
    pub block_size: u16,
    /// Offset of the hash table from the archive start (low 32 bits).
    pub hash_table_pos: u32,
    /// Offset of the block table from the archive start (low 32 bits).
    pub block_table_pos: u32,
    /// Entries in the hash table.
    pub hash_table_size: u32,
    /// Entries in the block table.
    pub block_table_size: u32,
}

/// Fields added in version 2.
#[derive(Debug, Clone, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
pub struct MpqHeaderV2 {
    /// Offset of the hi-block table, or 0.
    pub hi_block_table_pos: u64,
    /// High 16 bits of the hash table offset.
    pub hash_table_pos_hi: u16,
    /// High 16 bits of the block table offset.
    pub block_table_pos_hi: u16,
}

/// Fields added in version 3.
#[derive(Debug, Clone, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
pub struct MpqHeaderV3 {
    /// 64-bit size of the archive.
    pub archive_size_64: u64,
    /// Offset of the BET table, or 0.
    pub bet_table_pos: u64,
    /// Offset of the HET table, or 0.
    pub het_table_pos: u64,
}

/// Fields added in version 4.
#[derive(Debug, Clone, Default, PartialEq, Eq, WowHeaderR, WowHeaderW)]
pub struct MpqHeaderV4 {
    /// Stored (maybe compressed) size of the hash table.
    pub hash_table_size_64: u64,
    /// Stored size of the block table.
    pub block_table_size_64: u64,
    /// Stored size of the hi-block table.
    pub hi_block_table_size_64: u64,
    /// Stored size of the HET table.
    pub het_table_size_64: u64,
    /// Stored size of the BET table.
    pub bet_table_size_64: u64,
    /// Size of the chunks whose MD5s follow each file's data.
    pub raw_chunk_size: u32,
    /// MD5 of the stored block table.
    pub md5_block_table: [u8; 16],
    /// MD5 of the stored hash table.
    pub md5_hash_table: [u8; 16],
    /// MD5 of the stored hi-block table.
    pub md5_hi_block_table: [u8; 16],
    /// MD5 of the stored BET table.
    pub md5_bet_table: [u8; 16],
    /// MD5 of the stored HET table.
    pub md5_het_table: [u8; 16],
    /// MD5 of the header up to this field.
    pub md5_mpq_header: [u8; 16],
}

/// The version 2 fields, when the header has them.
#[derive(Debug, Clone, PartialEq, Eq, WowHeaderR, WowHeaderW)]
#[wow_data(version = FormatVersion)]
pub enum SinceV2 {
    /// Not in this header.
    None,
    /// The fields.
    #[wow_data(read_if = version >= FormatVersion::V2)]
    Some(MpqHeaderV2),
}

/// The version 3 fields, when the header has them.
#[derive(Debug, Clone, PartialEq, Eq, WowHeaderR, WowHeaderW)]
#[wow_data(version = FormatVersion)]
pub enum SinceV3 {
    /// Not in this header.
    None,
    /// The fields.
    #[wow_data(read_if = version >= FormatVersion::V3)]
    Some(MpqHeaderV3),
}

/// The version 4 fields, when the header has them.
#[derive(Debug, Clone, PartialEq, Eq, WowHeaderR, WowHeaderW)]
#[wow_data(version = FormatVersion)]
pub enum SinceV4 {
    /// Not in this header.
    None,
    /// The fields.
    #[wow_data(read_if = version >= FormatVersion::V4)]
    Some(MpqHeaderV4),
}

/// The fields that follow [`MpqHeaderV1`], by the layout the header has.
#[derive(Debug, Clone, PartialEq, Eq, WowHeaderR, WowHeaderW)]
#[wow_data(version = FormatVersion)]
pub struct MpqHeaderExt {
    /// Version 2 fields.
    #[wow_data(versioned)]
    pub v2: SinceV2,
    /// Version 3 fields.
    #[wow_data(versioned)]
    pub v3: SinceV3,
    /// Version 4 fields.
    #[wow_data(versioned)]
    pub v4: SinceV4,
}

/// An archive header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MpqHeader {
    /// Fields of every version.
    pub base: MpqHeaderV1,
    /// Fields of later versions.
    pub ext: MpqHeaderExt,
}

impl MpqHeader {
    /// The layout to read, by version and header size: Storm accepts headers shorter than
    /// their version's, and v3 headers long enough to hold the v4 fields have them.
    pub fn layout(base: &MpqHeaderV1) -> FormatVersion {
        let mut layout = base.format_version;
        while layout > FormatVersion::V1 && base.header_size < layout.header_size() {
            layout = layout.previous();
        }
        if layout == FormatVersion::V3 && base.header_size >= FormatVersion::V4.header_size() {
            layout = FormatVersion::V4;
        }
        layout
    }

    /// Version 2 fields, if present.
    pub fn v2(&self) -> Option<&MpqHeaderV2> {
        match &self.ext.v2 {
            SinceV2::Some(v2) => Some(v2),
            SinceV2::None => None,
        }
    }

    /// Version 3 fields, if present.
    pub fn v3(&self) -> Option<&MpqHeaderV3> {
        match &self.ext.v3 {
            SinceV3::Some(v3) => Some(v3),
            SinceV3::None => None,
        }
    }

    /// Version 4 fields, if present.
    pub fn v4(&self) -> Option<&MpqHeaderV4> {
        match &self.ext.v4 {
            SinceV4::Some(v4) => Some(v4),
            SinceV4::None => None,
        }
    }

    /// Format version.
    pub fn format_version(&self) -> FormatVersion {
        self.base.format_version
    }

    /// Size of one sector.
    pub fn sector_size(&self) -> usize {
        512usize << self.base.block_size
    }

    /// Size of the archive, from the 64-bit field when there is one.
    pub fn archive_size(&self) -> u64 {
        self.v3()
            .map_or(self.base.archive_size as u64, |v3| v3.archive_size_64)
    }

    /// Offset of the hash table from the archive start.
    pub fn hash_table_pos(&self) -> u64 {
        let hi = self.v2().map_or(0, |v2| v2.hash_table_pos_hi);
        ((hi as u64) << 32) | self.base.hash_table_pos as u64
    }

    /// Offset of the block table from the archive start.
    pub fn block_table_pos(&self) -> u64 {
        let hi = self.v2().map_or(0, |v2| v2.block_table_pos_hi);
        ((hi as u64) << 32) | self.base.block_table_pos as u64
    }

    /// Offset of the hi-block table, if there is one.
    pub fn hi_block_table_pos(&self) -> Option<u64> {
        self.v2()
            .map(|v2| v2.hi_block_table_pos)
            .filter(|&pos| pos != 0)
    }

    /// Offset of the HET table, if there is one.
    pub fn het_table_pos(&self) -> Option<u64> {
        self.v3().map(|v3| v3.het_table_pos).filter(|&pos| pos != 0)
    }

    /// Offset of the BET table, if there is one.
    pub fn bet_table_pos(&self) -> Option<u64> {
        self.v3().map(|v3| v3.bet_table_pos).filter(|&pos| pos != 0)
    }
}

impl WowHeaderR for MpqHeader {
    fn wow_read<R: Read + Seek>(reader: &mut R) -> WDResult<Self> {
        let base: MpqHeaderV1 = reader.wow_read()?;
        let ext = reader.wow_read_versioned(Self::layout(&base))?;
        Ok(Self { base, ext })
    }
}

impl WowHeaderW for MpqHeader {
    fn wow_write<W: Write>(&self, writer: &mut W) -> WDResult<()> {
        self.base.wow_write(writer)?;
        self.ext.wow_write(writer)
    }

    fn wow_size(&self) -> usize {
        self.base.wow_size() + self.ext.wow_size()
    }
}

/// Where an archive was found in its file.
#[derive(Debug, Clone)]
pub struct HeaderLocation {
    /// Offset of the archive header in the file; table and file offsets count from here.
    pub archive_offset: u64,
    /// The user data header in front of it, if any.
    pub user_data: Option<UserDataHeader>,
    /// The archive header.
    pub header: MpqHeader,
}

/// Finds the archive header: at a multiple of 512 bytes, directly or through a user data
/// header.
pub fn find_header<R: Read + Seek>(reader: &mut R) -> Result<HeaderLocation> {
    let file_size = reader.seek(SeekFrom::End(0))?;
    let mut offset = 0u64;
    while offset + 4 <= file_size {
        reader.seek(SeekFrom::Start(offset))?;
        let magic: MagicStr = reader.wow_read()?;
        if magic == MPQ_MAGIC {
            reader.seek(SeekFrom::Start(offset))?;
            return Ok(HeaderLocation {
                archive_offset: offset,
                user_data: None,
                header: read_header(reader)?,
            });
        }
        if magic == USER_DATA_MAGIC && offset + 16 <= file_size {
            reader.seek(SeekFrom::Start(offset))?;
            let user_data: UserDataHeader = reader.wow_read()?;
            let archive_offset = offset + user_data.header_offset as u64;
            if archive_offset + 4 <= file_size {
                reader.seek(SeekFrom::Start(archive_offset))?;
                let magic: MagicStr = reader.wow_read()?;
                if magic == MPQ_MAGIC {
                    reader.seek(SeekFrom::Start(archive_offset))?;
                    return Ok(HeaderLocation {
                        archive_offset,
                        user_data: Some(user_data),
                        header: read_header(reader)?,
                    });
                }
            }
        }
        offset += HEADER_ALIGNMENT;
    }
    Err(Error::invalid_format("No MPQ header found"))
}

fn read_header<R: Read + Seek>(reader: &mut R) -> Result<MpqHeader> {
    let header: MpqHeader = reader.wow_read()?;
    if header.base.block_size > 23 {
        return Err(Error::invalid_format(format!(
            "Sector size shift {} is too large",
            header.base.block_size
        )));
    }
    Ok(header)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn base(version: FormatVersion, header_size: u32) -> MpqHeaderV1 {
        MpqHeaderV1 {
            magic: MPQ_MAGIC,
            header_size,
            archive_size: 0x1000,
            format_version: version,
            block_size: 3,
            hash_table_pos: 0x100,
            block_table_pos: 0x200,
            hash_table_size: 16,
            block_table_size: 4,
        }
    }

    fn no_ext() -> MpqHeaderExt {
        MpqHeaderExt {
            v2: SinceV2::None,
            v3: SinceV3::None,
            v4: SinceV4::None,
        }
    }

    #[test]
    fn every_version_round_trips_at_its_size() {
        let v2 = MpqHeaderV2 {
            hi_block_table_pos: 0x300,
            hash_table_pos_hi: 1,
            block_table_pos_hi: 2,
        };
        let v3 = MpqHeaderV3 {
            archive_size_64: 0x1_0000_1000,
            bet_table_pos: 0x400,
            het_table_pos: 0x500,
        };
        let v4 = MpqHeaderV4 {
            raw_chunk_size: 0x4000,
            md5_hash_table: [7; 16],
            ..Default::default()
        };
        let exts = [
            (FormatVersion::V1, no_ext()),
            (
                FormatVersion::V2,
                MpqHeaderExt {
                    v2: SinceV2::Some(v2.clone()),
                    ..no_ext()
                },
            ),
            (
                FormatVersion::V3,
                MpqHeaderExt {
                    v2: SinceV2::Some(v2.clone()),
                    v3: SinceV3::Some(v3.clone()),
                    v4: SinceV4::None,
                },
            ),
            (
                FormatVersion::V4,
                MpqHeaderExt {
                    v2: SinceV2::Some(v2),
                    v3: SinceV3::Some(v3),
                    v4: SinceV4::Some(v4),
                },
            ),
        ];
        for (version, ext) in exts {
            let header = MpqHeader {
                base: base(version, version.header_size()),
                ext,
            };
            assert_eq!(header.wow_size(), version.header_size() as usize);
            let mut bytes = Vec::new();
            header.wow_write(&mut bytes).unwrap();
            assert_eq!(bytes.len(), header.wow_size());
            let read: MpqHeader = Cursor::new(bytes).wow_read().unwrap();
            assert_eq!(read, header);
        }
    }

    #[test]
    fn positions_combine_their_high_bits() {
        let header = MpqHeader {
            base: base(FormatVersion::V2, 0x2C),
            ext: MpqHeaderExt {
                v2: SinceV2::Some(MpqHeaderV2 {
                    hi_block_table_pos: 0,
                    hash_table_pos_hi: 1,
                    block_table_pos_hi: 2,
                }),
                ..no_ext()
            },
        };
        assert_eq!(header.hash_table_pos(), 0x1_0000_0100);
        assert_eq!(header.block_table_pos(), 0x2_0000_0200);
        assert_eq!(header.hi_block_table_pos(), None);
        assert_eq!(header.sector_size(), 4096);
    }

    #[test]
    fn short_v3_headers_read_as_v2() {
        let mut bytes = Vec::new();
        base(FormatVersion::V3, 0x2C).wow_write(&mut bytes).unwrap();
        MpqHeaderV2::default().wow_write(&mut bytes).unwrap();
        let header: MpqHeader = Cursor::new(bytes).wow_read().unwrap();
        assert!(header.v2().is_some());
        assert!(header.v3().is_none());
    }

    #[test]
    fn finds_the_header_after_user_data() {
        let user = UserDataHeader {
            magic: USER_DATA_MAGIC,
            user_data_size: 0x200,
            header_offset: 0x400,
            user_data_header_size: 0x10,
        };
        let mut bytes = Vec::new();
        user.wow_write(&mut bytes).unwrap();
        bytes.resize(0x400, 0);
        let header = MpqHeader {
            base: base(FormatVersion::V1, 0x20),
            ext: no_ext(),
        };
        header.wow_write(&mut bytes).unwrap();
        let found = find_header(&mut Cursor::new(bytes)).unwrap();
        assert_eq!(found.archive_offset, 0x400);
        assert_eq!(found.user_data, Some(user));
        assert_eq!(found.header, header);
    }
}
