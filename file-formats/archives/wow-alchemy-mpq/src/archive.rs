//! Reading an MPQ archive.

use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::{Path, PathBuf};

use wow_alchemy_data::prelude::*;

use crate::header::{FormatVersion, HeaderLocation, MpqHeader, UserDataHeader, find_header};
use crate::patch::{PatchFile, PatchInfo};
use crate::special_files::{ATTRIBUTES, Attributes, LISTFILE, parse_listfile};
use crate::tables::{
    BET_MAGIC, BetTable, BlockEntry, BlockFlags, BlockTable, HET_MAGIC, HashEntry, HashTable,
    HetTable, HiBlockTable, read_ext_table, read_table,
};
use crate::{BLOCK_TABLE_KEY, Error, HASH_TABLE_KEY, Result, compression, crypto};

/// Where a file is and how it is stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileInfo {
    /// Index of its block (or BET entry).
    pub block_index: usize,
    /// Index of its hash table entry, when found through the hash table.
    pub hash_index: Option<usize>,
    /// Offset of its data in the archive's file.
    pub file_pos: u64,
    /// Offset of its data from the archive start, which encryption keys use.
    pub archive_pos: u64,
    /// Stored size.
    pub compressed_size: u64,
    /// Size of the file.
    pub file_size: u64,
    /// Flags.
    pub flags: BlockFlags,
    /// Locale of the hash table entry.
    pub locale: u16,
}

/// A file named in the archive's `(listfile)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    /// Name as the listfile spells it.
    pub name: String,
    /// Size of the file.
    pub size: u64,
    /// Stored size.
    pub compressed_size: u64,
    /// Flags.
    pub flags: BlockFlags,
}

/// An open MPQ archive. Reading takes `&self`: data is read at its position in the file, so
/// any number of threads can read one archive at once.
#[derive(Debug)]
pub struct Archive {
    path: PathBuf,
    file: File,
    location: HeaderLocation,
    hash_table: Option<HashTable>,
    block_table: Option<BlockTable>,
    hi_block_table: Option<HiBlockTable>,
    het_table: Option<HetTable>,
    bet_table: Option<BetTable>,
}

impl Archive {
    /// Opens an archive and loads its tables.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path)?;
        let mut reader = BufReader::new(file.try_clone()?);
        let location = find_header(&mut reader)?;
        let mut archive = Self {
            path,
            file,
            location,
            hash_table: None,
            block_table: None,
            hi_block_table: None,
            het_table: None,
            bet_table: None,
        };
        archive.load_tables(&mut reader)?;
        Ok(archive)
    }

    fn load_tables(&mut self, reader: &mut BufReader<File>) -> Result<()> {
        let offset = self.location.archive_offset;
        let header = self.location.header.clone();
        let v4 = header.v4().cloned();
        let stored =
            |v4_size: Option<u64>, size: usize| v4_size.filter(|&s| s != 0).unwrap_or(size as u64);

        let hash_count = header.base.hash_table_size as usize;
        if hash_count > 0 {
            let size = hash_count * HashEntry::SIZE;
            let data = read_table(
                reader,
                offset + header.hash_table_pos(),
                stored(v4.as_ref().map(|v| v.hash_table_size_64), size),
                size,
                HASH_TABLE_KEY,
                0xFF,
            )?;
            self.hash_table = Some(HashTable::from_bytes(&data)?);
        }

        let block_count = header.base.block_table_size as usize;
        if block_count > 0 {
            let size = block_count * BlockEntry::SIZE;
            let data = read_table(
                reader,
                offset + header.block_table_pos(),
                stored(v4.as_ref().map(|v| v.block_table_size_64), size),
                size,
                BLOCK_TABLE_KEY,
                0,
            )?;
            self.block_table = Some(BlockTable::from_bytes(&data)?);

            if let Some(pos) = header.hi_block_table_pos() {
                let size = block_count * 2;
                let data = read_table(
                    reader,
                    offset + pos,
                    stored(v4.as_ref().map(|v| v.hi_block_table_size_64), size),
                    size,
                    0,
                    0,
                )?;
                self.hi_block_table = Some(HiBlockTable::from_bytes(&data)?);
            }
        }

        if let Some(pos) = header.het_table_pos() {
            let size = self.ext_table_size(pos, v4.as_ref().map(|v| v.het_table_size_64));
            match read_ext_table(
                reader,
                offset + pos,
                size,
                HET_MAGIC,
                HASH_TABLE_KEY,
            )
            .and_then(|body| HetTable::from_body(&body))
            {
                Ok(table) => self.het_table = Some(table),
                Err(e) => log::warn!("{}: HET table not loaded: {e}", self.path.display()),
            }
        }
        if let Some(pos) = header.bet_table_pos() {
            let size = self.ext_table_size(pos, v4.as_ref().map(|v| v.bet_table_size_64));
            match read_ext_table(
                reader,
                offset + pos,
                size,
                BET_MAGIC,
                BLOCK_TABLE_KEY,
            )
            .and_then(|body| BetTable::from_body(&body))
            {
                Ok(table) => self.bet_table = Some(table),
                Err(e) => log::warn!("{}: BET table not loaded: {e}", self.path.display()),
            }
        }
        Ok(())
    }

    /// Stored size of a HET or BET table: the v4 header's, or else the space up to the
    /// next table or the archive's end.
    fn ext_table_size(&self, pos: u64, v4_size: Option<u64>) -> u64 {
        if let Some(size) = v4_size.filter(|&s| s != 0) {
            return size;
        }
        let header = &self.location.header;
        let ends = [
            header.het_table_pos(),
            header.bet_table_pos(),
            Some(header.hash_table_pos()),
            Some(header.block_table_pos()),
            header.hi_block_table_pos(),
            Some(header.archive_size()),
        ];
        ends.into_iter()
            .flatten()
            .filter(|&end| end > pos)
            .min()
            .map_or(u64::MAX, |end| end - pos)
    }

    /// Path the archive was opened from.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The archive header.
    pub fn header(&self) -> &MpqHeader {
        &self.location.header
    }

    /// The user data header in front of the archive, if any.
    pub fn user_data(&self) -> Option<&UserDataHeader> {
        self.location.user_data.as_ref()
    }

    /// Offset of the archive in its file.
    pub fn archive_offset(&self) -> u64 {
        self.location.archive_offset
    }

    /// The hash table, if the archive has one.
    pub fn hash_table(&self) -> Option<&HashTable> {
        self.hash_table.as_ref()
    }

    /// The block table, if the archive has one.
    pub fn block_table(&self) -> Option<&BlockTable> {
        self.block_table.as_ref()
    }

    /// The hi-block table, if the archive has one.
    pub fn hi_block_table(&self) -> Option<&HiBlockTable> {
        self.hi_block_table.as_ref()
    }

    /// The HET table, if the archive has one.
    pub fn het_table(&self) -> Option<&HetTable> {
        self.het_table.as_ref()
    }

    /// The BET table, if the archive has one.
    pub fn bet_table(&self) -> Option<&BetTable> {
        self.bet_table.as_ref()
    }

    /// Finds a file's entry: through the hash and block tables when the archive has them, else
    /// through the HET and BET tables. Names are case-insensitive and `/` matches `\`.
    ///
    /// The entry may be a [`BlockFlags::DELETE_MARKER`], which a patch archive uses to delete
    /// the file of earlier archives; [`Archive::contains_file`] and [`Archive::read_file`]
    /// treat those as missing.
    pub fn find_file(&self, name: &str) -> Option<FileInfo> {
        if let (Some(hashes), Some(blocks)) = (&self.hash_table, &self.block_table) {
            let (hash_index, entry) = hashes.find(name)?;
            let block_index = entry.block_index as usize;
            let block = blocks.entries.get(block_index)?;
            if !block.flags.contains(BlockFlags::EXISTS) {
                return None;
            }
            let high = self
                .hi_block_table
                .as_ref()
                .map_or(0, |hi| hi.high_bits(block_index));
            let archive_pos = high | block.file_pos as u64;
            return Some(FileInfo {
                block_index,
                hash_index: Some(hash_index),
                file_pos: self.location.archive_offset + archive_pos,
                archive_pos,
                compressed_size: block.compressed_size as u64,
                file_size: block.file_size as u64,
                flags: block.flags,
                locale: entry.locale,
            });
        }
        let (het, bet) = (self.het_table.as_ref()?, self.bet_table.as_ref()?);
        let (hash, candidates) = het.candidates(name);
        candidates.into_iter().find_map(|index| {
            let file = bet.file(index).filter(|_| bet.matches(index, hash))?;
            file.flags.contains(BlockFlags::EXISTS).then(|| FileInfo {
                block_index: index as usize,
                hash_index: None,
                file_pos: self.location.archive_offset + file.file_pos,
                archive_pos: file.file_pos,
                compressed_size: file.compressed_size,
                file_size: file.file_size,
                flags: file.flags,
                locale: 0,
            })
        })
    }

    /// Whether the archive has a file of this name (and not a delete marker).
    pub fn contains_file(&self, name: &str) -> bool {
        self.find_file(name)
            .is_some_and(|info| !info.flags.contains(BlockFlags::DELETE_MARKER))
    }

    /// Reads a file. Patch files can't be read alone; see [`Archive::read_patch`] and
    /// [`crate::PatchChain`].
    pub fn read_file(&self, name: &str) -> Result<Vec<u8>> {
        let info = self
            .find_file(name)
            .filter(|info| !info.flags.contains(BlockFlags::DELETE_MARKER))
            .ok_or_else(|| Error::FileNotFound(name.to_string()))?;
        if info.flags.contains(BlockFlags::PATCH_FILE) {
            return Err(Error::invalid_format(format!(
                "{name} is a patch, to be applied to the file in an earlier archive"
            )));
        }
        self.read_data(name, &info)
    }

    /// Reads a patch file.
    pub fn read_patch(&self, name: &str) -> Result<PatchFile> {
        let info = self
            .find_file(name)
            .ok_or_else(|| Error::FileNotFound(name.to_string()))?;
        if !info.flags.contains(BlockFlags::PATCH_FILE) {
            return Err(Error::invalid_format(format!("{name} is not a patch")));
        }
        PatchFile::parse(&self.read_data(name, &info)?)
    }

    /// Reads a file's data, decrypting and decompressing it.
    /// Reads `buf.len()` bytes at `pos` in the file, without moving a shared cursor.
    fn read_at(&self, pos: u64, buf: &mut [u8]) -> Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::FileExt;
            self.file.read_exact_at(buf, pos)?;
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::FileExt;
            let mut done = 0;
            while done < buf.len() {
                match self.file.seek_read(&mut buf[done..], pos + done as u64) {
                    Ok(0) => {
                        return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof).into());
                    }
                    Ok(n) => done += n,
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                    Err(e) => return Err(e.into()),
                }
            }
        }
        Ok(())
    }

    fn read_vec_at(&self, pos: u64, len: usize) -> Result<Vec<u8>> {
        let mut data = vec![0u8; len];
        self.read_at(pos, &mut data)?;
        Ok(data)
    }

    fn read_data(&self, name: &str, info: &FileInfo) -> Result<Vec<u8>> {
        let key = if info.flags.contains(BlockFlags::ENCRYPTED) {
            crypto::file_key(
                name,
                info.archive_pos,
                info.file_size as u32,
                info.flags.contains(BlockFlags::FIX_KEY),
            )
        } else {
            0
        };

        let mut start = info.file_pos;
        let mut stored_size = info.compressed_size;
        let mut size = info.file_size;
        if info.flags.contains(BlockFlags::PATCH_FILE) {
            let bytes = self.read_vec_at(start, (PatchInfo::SIZE as usize).min(stored_size as usize))?;
            let patch_info: PatchInfo = Cursor::new(bytes).wow_read()?;
            start += patch_info.length as u64;
            stored_size = stored_size.saturating_sub(patch_info.length as u64);
            size = patch_info.data_size as u64;
        }

        if info.flags.contains(BlockFlags::SINGLE_UNIT) {
            let mut data = self.read_vec_at(start, stored_size as usize)?;
            crypto::decrypt_bytes(&mut data, key);
            return if info.flags.is_compressed() {
                decompress_block(&data, size as usize, info.flags)
            } else {
                data.truncate(size as usize);
                Ok(data)
            };
        }

        let sector_size = self.location.header.sector_size();
        let sector_count = (size as usize).div_ceil(sector_size);
        let offsets = if info.flags.is_compressed() {
            self.read_sector_offsets(start, sector_count, key)?
        } else {
            (0..=sector_count)
                .map(|i| (i * sector_size).min(size as usize) as u32)
                .collect()
        };

        let mut out = Vec::with_capacity(size as usize);
        if sector_count == 0 {
            return Ok(out);
        }
        let first = offsets[0] as usize;
        let end = offsets[sector_count] as usize;
        if end < first {
            return Err(Error::invalid_format(format!(
                "{name}: sectors end at {end}, before their start {first}"
            )));
        }
        let raw = self.read_vec_at(start + first as u64, end - first)?;
        for i in 0..sector_count {
            let (from, to) = (offsets[i] as usize, offsets[i + 1] as usize);
            if to < from || from < first || to > end {
                return Err(Error::invalid_format(format!(
                    "{name}: sector {i} spans {from}..{to}, outside {first}..{end}"
                )));
            }
            let mut sector = raw[from - first..to - first].to_vec();
            if key != 0 {
                crypto::decrypt_bytes(&mut sector, key.wrapping_add(i as u32));
            }
            let expected = sector_size.min(size as usize - out.len());
            if info.flags.is_compressed() && sector.len() < expected {
                sector = decompress_block(&sector, expected, info.flags)?;
            }
            sector.resize(expected, 0);
            out.extend_from_slice(&sector);
        }
        Ok(out)
    }

    /// The offsets of a compressed file's sectors from its start, one more than there are
    /// sectors; encrypted with the file's key minus one.
    fn read_sector_offsets(
        &self,
        start: u64,
        sector_count: usize,
        key: u32,
    ) -> Result<Vec<u32>> {
        let mut bytes = self.read_vec_at(start, (sector_count + 1) * 4)?;
        if key != 0 {
            crypto::decrypt_bytes(&mut bytes, key.wrapping_sub(1));
        }
        let mut cursor = Cursor::new(bytes);
        let mut offsets = Vec::with_capacity(sector_count + 1);
        for _ in 0..=sector_count {
            offsets.push(cursor.wow_read()?);
        }
        Ok(offsets)
    }

    /// The files the archive's `(listfile)` names that are in the archive, in its order.
    /// An archive without a listfile lists nothing, and delete markers aren't listed.
    pub fn list(&self) -> Result<Vec<FileEntry>> {
        let mut entries = self.list_entries()?;
        entries.retain(|e| !e.flags.contains(BlockFlags::DELETE_MARKER));
        Ok(entries)
    }

    /// The entries the `(listfile)` names, delete markers included.
    pub(crate) fn list_entries(&self) -> Result<Vec<FileEntry>> {
        if !self.contains_file(LISTFILE) {
            return Ok(Vec::new());
        }
        let listfile = self.read_file(LISTFILE)?;
        Ok(parse_listfile(&listfile)
            .into_iter()
            .filter_map(|name| {
                let info = self.find_file(&name)?;
                Some(FileEntry {
                    name,
                    size: info.file_size,
                    compressed_size: info.compressed_size,
                    flags: info.flags,
                })
            })
            .collect())
    }

    /// The archive's `(attributes)`, if it has one.
    pub fn attributes(&self) -> Result<Option<Attributes>> {
        if !self.contains_file(ATTRIBUTES) {
            return Ok(None);
        }
        let data = self.read_file(ATTRIBUTES)?;
        let blocks = match (&self.block_table, &self.bet_table) {
            (Some(blocks), _) => blocks.entries.len(),
            (None, Some(bet)) => bet.header.entry_count as usize,
            (None, None) => 0,
        };
        Attributes::parse(&data, blocks).map(Some)
    }

    /// Format version of the archive.
    pub fn format_version(&self) -> FormatVersion {
        self.location.header.format_version()
    }
}

/// Decompresses a single unit or sector by its block's compression, into `size` bytes
/// (padding with zeros a short result, as Storm does).
fn decompress_block(data: &[u8], size: usize, flags: BlockFlags) -> Result<Vec<u8>> {
    let mut out = if flags.contains(BlockFlags::COMPRESS) || data.len() == size {
        compression::decompress(data, size)?
    } else {
        let mut imploded = vec![compression::flags::PKWARE];
        imploded.extend_from_slice(data);
        compression::decompress(&imploded, size)?
    };
    out.resize(size, 0);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ArchiveBuilder;

    #[test]
    fn het_and_bet_find_files_without_the_classic_tables() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v4.mpq");
        ArchiveBuilder::new()
            .version(FormatVersion::V4)
            .add_file_data(b"first".to_vec(), "Dir\\First.txt")
            .add_file_data(vec![7; 9000], "second.bin")
            .build(&path)
            .unwrap();
        let mut archive = Archive::open(&path).unwrap();
        archive.hash_table = None;
        archive.block_table = None;
        assert_eq!(archive.read_file("dir/first.TXT").unwrap(), b"first");
        assert_eq!(archive.read_file("second.bin").unwrap(), vec![7; 9000]);
        assert!(archive.find_file("third.bin").is_none());
        assert_eq!(archive.list().unwrap().len(), 2);
    }
}
