//! Writing new MPQ archives.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use md5::{Digest, Md5};
use wow_alchemy_data::prelude::*;

use crate::compression::{self, flags};
use crate::header::{
    FormatVersion, MPQ_MAGIC, MpqHeader, MpqHeaderExt, MpqHeaderV1, MpqHeaderV2, MpqHeaderV3,
    MpqHeaderV4, SinceV2, SinceV3, SinceV4,
};
use crate::patch::{PatchFile, PatchInfo};
use crate::special_files::{
    ATTRIBUTES, AttributeFlags, Attributes, AttributesHeader, LISTFILE, build_listfile,
};
use crate::tables::{
    BET_MAGIC, BetFileInfo, BetTable, BlockEntry, BlockFlags, BlockTable, HET_MAGIC, HashTable,
    HetTable, HiBlockTable, encrypt_table, write_ext_table,
};
use crate::{BLOCK_TABLE_KEY, Error, HASH_TABLE_KEY, Result, crypto};

/// How one file is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileOptions {
    /// Compression byte to use (one of [`flags`]), or `None` to store it as it is.
    pub compression: Option<u8>,
    /// Encrypt the file.
    pub encrypt: bool,
    /// Adjust the key by the file's position and size.
    pub fix_key: bool,
    /// Store it in one piece instead of sectors.
    pub single_unit: bool,
    /// Windows LANGID, 0 for neutral.
    pub locale: u16,
}

impl Default for FileOptions {
    fn default() -> Self {
        Self {
            compression: Some(flags::ZLIB),
            encrypt: false,
            fix_key: false,
            single_unit: false,
            locale: 0,
        }
    }
}

#[derive(Debug)]
enum Source {
    Path(PathBuf),
    Data(Vec<u8>),
    Patch(Vec<u8>),
    DeleteMarker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Plain,
    Patch,
    DeleteMarker,
}

#[derive(Debug)]
struct Pending {
    name: String,
    source: Source,
    options: FileOptions,
}

/// Builds an archive from files and data.
///
/// ```no_run
/// use wow_alchemy_mpq::ArchiveBuilder;
///
/// ArchiveBuilder::new()
///     .add_file_data(b"hello".to_vec(), "greeting.txt")
///     .build("out.mpq")?;
/// # Ok::<(), wow_alchemy_mpq::Error>(())
/// ```
#[derive(Debug)]
pub struct ArchiveBuilder {
    version: FormatVersion,
    block_size: u16,
    listfile: bool,
    attributes: bool,
    default_options: FileOptions,
    files: Vec<Pending>,
}

impl Default for ArchiveBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ArchiveBuilder {
    /// A v1 archive with 4 KB sectors, zlib compression, a `(listfile)` and no
    /// `(attributes)`.
    pub fn new() -> Self {
        Self {
            version: FormatVersion::V1,
            block_size: 3,
            listfile: true,
            attributes: false,
            default_options: FileOptions::default(),
            files: Vec::new(),
        }
    }

    /// Format version to write. v3 and v4 also get HET and BET tables.
    pub fn version(mut self, version: FormatVersion) -> Self {
        self.version = version;
        self
    }

    /// Sector size as a power of two times 512.
    pub fn block_size(mut self, block_size: u16) -> Self {
        self.block_size = block_size;
        self
    }

    /// Compression of files added without options.
    pub fn default_compression(mut self, compression: Option<u8>) -> Self {
        self.default_options.compression = compression;
        self
    }

    /// Whether to write a `(listfile)` of the added files.
    pub fn listfile(mut self, listfile: bool) -> Self {
        self.listfile = listfile;
        self
    }

    /// Whether to write an `(attributes)` with CRC32s and MD5s.
    pub fn attributes(mut self, attributes: bool) -> Self {
        self.attributes = attributes;
        self
    }

    /// Adds the file at `path`, read when building, as `name`.
    pub fn add_file<P: AsRef<Path>>(mut self, path: P, name: &str) -> Self {
        let options = self.default_options;
        self.files.push(Pending {
            name: name.replace('/', "\\"),
            source: Source::Path(path.as_ref().to_path_buf()),
            options,
        });
        self
    }

    /// Adds `data` as `name`.
    pub fn add_file_data(self, data: Vec<u8>, name: &str) -> Self {
        let options = self.default_options;
        self.add_file_data_with_options(data, name, options)
    }

    /// Adds `data` as `name`, stored as `options` say.
    pub fn add_file_data_with_options(
        mut self,
        data: Vec<u8>,
        name: &str,
        options: FileOptions,
    ) -> Self {
        self.files.push(Pending {
            name: name.replace('/', "\\"),
            source: Source::Data(data),
            options,
        });
        self
    }

    /// Adds an incremental patch of `name`, stored single unit and uncompressed.
    pub fn add_patch_file(mut self, patch: &PatchFile, name: &str) -> Result<Self> {
        self.files.push(Pending {
            name: name.replace('/', "\\"),
            source: Source::Patch(patch.to_bytes()?),
            options: FileOptions {
                compression: None,
                single_unit: true,
                ..FileOptions::default()
            },
        });
        Ok(self)
    }

    /// Adds a delete marker for `name`: in a [`crate::PatchChain`], the file of lower
    /// archives is gone.
    pub fn add_delete_marker(mut self, name: &str) -> Self {
        self.files.push(Pending {
            name: name.replace('/', "\\"),
            source: Source::DeleteMarker,
            options: FileOptions::default(),
        });
        self
    }

    /// Writes the archive to `path`.
    pub fn build<P: AsRef<Path>>(self, path: P) -> Result<()> {
        let bytes = self.build_bytes()?;
        fs::write(path, bytes)?;
        Ok(())
    }

    /// The archive's bytes.
    pub fn build_bytes(self) -> Result<Vec<u8>> {
        let header_size = self.version.header_size() as usize;
        let sector_size = 512usize << self.block_size;

        let mut files: Vec<(String, Vec<u8>, FileOptions, Kind)> = Vec::new();
        for pending in self.files {
            let (data, patch) = match pending.source {
                Source::Path(path) => (fs::read(&path)?, Kind::Plain),
                Source::Data(data) => (data, Kind::Plain),
                Source::Patch(data) => (data, Kind::Patch),
                Source::DeleteMarker => (Vec::new(), Kind::DeleteMarker),
            };
            files.push((pending.name, data, pending.options, patch));
        }
        if self.listfile {
            let listfile = build_listfile(files.iter().map(|f| f.0.as_str()));
            files.push((LISTFILE.into(), listfile, self.default_options, Kind::Plain));
        }
        let attributes_index = self.attributes.then(|| {
            files.push((
                ATTRIBUTES.into(),
                Vec::new(),
                self.default_options,
                Kind::Plain,
            ));
            files.len() - 1
        });
        if let Some(index) = attributes_index {
            let count = files.len();
            let mut attributes = Attributes {
                header: AttributesHeader {
                    version: Attributes::VERSION,
                    flags: AttributeFlags::CRC32 | AttributeFlags::FILETIME | AttributeFlags::MD5,
                },
                crc32: Vec::with_capacity(count),
                filetime: vec![0; count],
                md5: Vec::with_capacity(count),
                patch_bits: Vec::new(),
            };
            for (i, (_, data, _, _)) in files.iter().enumerate() {
                let own = i == index;
                attributes
                    .crc32
                    .push(if own { 0 } else { crc32fast::hash(data) });
                attributes.md5.push(if own {
                    [0; 16]
                } else {
                    Md5::digest(data).into()
                });
            }
            files[index].1 = attributes.to_bytes()?;
        }

        let mut out = vec![0u8; header_size];
        let mut blocks = Vec::with_capacity(files.len());
        let mut hash_table = HashTable::new((files.len() * 4 / 3 + 1).max(4));
        for (index, (name, data, options, kind)) in files.iter().enumerate() {
            let pos = out.len() as u64;
            let (stored, flags) = encode_file(name, data, *options, *kind, pos, sector_size)?;
            blocks.push((pos, stored.len() as u64, data.len() as u64, flags));
            out.extend(stored);
            hash_table.insert(name, options.locale, index as u32)?;
        }

        let needs_hi = blocks.iter().any(|b| b.0 > u32::MAX as u64);
        if needs_hi && self.version == FormatVersion::V1 {
            return Err(Error::invalid_format("v1 archives can't pass 4 GB"));
        }
        let block_table = BlockTable {
            entries: blocks
                .iter()
                .map(|&(pos, stored, size, flags)| BlockEntry {
                    file_pos: pos as u32,
                    compressed_size: stored as u32,
                    file_size: size as u32,
                    flags,
                })
                .collect(),
        };

        let mut v4 = MpqHeaderV4::default();
        let (mut het_pos, mut bet_pos) = (0, 0);
        if self.version >= FormatVersion::V3 {
            let names: Vec<Option<&str>> = files.iter().map(|f| Some(f.0.as_str())).collect();
            let het = write_ext_table(
                HET_MAGIC,
                &HetTable::build(&names).to_body()?,
                HASH_TABLE_KEY,
            )?;
            het_pos = out.len() as u64;
            v4.het_table_size_64 = het.len() as u64;
            v4.md5_het_table = Md5::digest(&het).into();
            out.extend(het);

            let bet_files: Vec<_> = blocks
                .iter()
                .zip(&files)
                .map(|(&(pos, stored, size, flags), file)| {
                    let info = BetFileInfo {
                        file_pos: pos,
                        file_size: size,
                        compressed_size: stored,
                        flags,
                    };
                    (info, Some(file.0.as_str()))
                })
                .collect();
            let bet = write_ext_table(
                BET_MAGIC,
                &BetTable::build(&bet_files).to_body()?,
                BLOCK_TABLE_KEY,
            )?;
            bet_pos = out.len() as u64;
            v4.bet_table_size_64 = bet.len() as u64;
            v4.md5_bet_table = Md5::digest(&bet).into();
            out.extend(bet);
        }

        let hash_pos = out.len() as u64;
        let hash_bytes = encrypt_table(hash_table.to_bytes()?, HASH_TABLE_KEY);
        v4.hash_table_size_64 = hash_bytes.len() as u64;
        v4.md5_hash_table = Md5::digest(&hash_bytes).into();
        out.extend(hash_bytes);

        let block_pos = out.len() as u64;
        let block_bytes = encrypt_table(block_table.to_bytes()?, BLOCK_TABLE_KEY);
        v4.block_table_size_64 = block_bytes.len() as u64;
        v4.md5_block_table = Md5::digest(&block_bytes).into();
        out.extend(block_bytes);

        let mut hi_pos = 0;
        if needs_hi {
            let hi = HiBlockTable {
                entries: blocks.iter().map(|b| (b.0 >> 32) as u16).collect(),
            }
            .to_bytes()?;
            hi_pos = out.len() as u64;
            v4.hi_block_table_size_64 = hi.len() as u64;
            v4.md5_hi_block_table = Md5::digest(&hi).into();
            out.extend(hi);
        }

        let archive_size = out.len() as u64;
        let mut header = MpqHeader {
            base: MpqHeaderV1 {
                magic: MPQ_MAGIC,
                header_size: header_size as u32,
                archive_size: archive_size.min(u32::MAX as u64) as u32,
                format_version: self.version,
                block_size: self.block_size,
                hash_table_pos: hash_pos as u32,
                block_table_pos: block_pos as u32,
                hash_table_size: hash_table.entries.len() as u32,
                block_table_size: block_table.entries.len() as u32,
            },
            ext: MpqHeaderExt {
                v2: SinceV2::None,
                v3: SinceV3::None,
                v4: SinceV4::None,
            },
        };
        if self.version >= FormatVersion::V2 {
            header.ext.v2 = SinceV2::Some(MpqHeaderV2 {
                hi_block_table_pos: hi_pos,
                hash_table_pos_hi: (hash_pos >> 32) as u16,
                block_table_pos_hi: (block_pos >> 32) as u16,
            });
        }
        if self.version >= FormatVersion::V3 {
            header.ext.v3 = SinceV3::Some(MpqHeaderV3 {
                archive_size_64: archive_size,
                bet_table_pos: bet_pos,
                het_table_pos: het_pos,
            });
        }
        if self.version >= FormatVersion::V4 {
            header.ext.v4 = SinceV4::Some(v4);
        }

        let mut header_bytes = Vec::with_capacity(header_size);
        header.wow_write(&mut header_bytes)?;
        if self.version >= FormatVersion::V4 {
            let md5: [u8; 16] = Md5::digest(&header_bytes[..header_size - 16]).into();
            header_bytes[header_size - 16..].copy_from_slice(&md5);
        }
        out[..header_size].copy_from_slice(&header_bytes);
        Ok(out)
    }
}

/// A file's stored bytes and block flags.
fn encode_file(
    name: &str,
    data: &[u8],
    options: FileOptions,
    kind: Kind,
    pos: u64,
    sector_size: usize,
) -> Result<(Vec<u8>, BlockFlags)> {
    let mut flags = BlockFlags::EXISTS;
    if kind == Kind::DeleteMarker {
        return Ok((Vec::new(), flags | BlockFlags::DELETE_MARKER));
    }
    if data.is_empty() {
        return Ok((Vec::new(), flags));
    }
    if options.compression.is_some() {
        flags |= BlockFlags::COMPRESS;
    }
    if options.single_unit {
        flags |= BlockFlags::SINGLE_UNIT;
    }
    if options.encrypt {
        flags |= BlockFlags::ENCRYPTED;
        if options.fix_key {
            flags |= BlockFlags::FIX_KEY;
        }
    }
    let key = if options.encrypt {
        crypto::file_key(name, pos, data.len() as u32, options.fix_key)
    } else {
        0
    };

    let compress = |chunk: &[u8]| -> Result<Vec<u8>> {
        if let Some(method) = options.compression {
            let packed = compression::compress(chunk, method)?;
            if packed.len() < chunk.len() {
                return Ok(packed);
            }
        }
        Ok(chunk.to_vec())
    };

    if kind == Kind::Patch {
        flags |= BlockFlags::PATCH_FILE;
        let mut out = Vec::new();
        PatchInfo {
            length: PatchInfo::SIZE,
            flags: 0x8000_0000,
            data_size: data.len() as u32,
            md5: Md5::digest(data).into(),
        }
        .wow_write(&mut out)?;
        let mut body = compress(data)?;
        crypto::encrypt_bytes(&mut body, key);
        out.extend(body);
        return Ok((out, flags));
    }

    if options.single_unit {
        let mut body = compress(data)?;
        crypto::encrypt_bytes(&mut body, key);
        return Ok((body, flags));
    }

    let sectors: Vec<&[u8]> = data.chunks(sector_size).collect();
    let mut body = Vec::new();
    if options.compression.is_some() {
        let mut offsets = Vec::with_capacity(sectors.len() + 1);
        let mut packed = Vec::with_capacity(sectors.len());
        let mut offset = ((sectors.len() + 1) * 4) as u32;
        for sector in &sectors {
            offsets.push(offset);
            let p = compress(sector)?;
            offset += p.len() as u32;
            packed.push(p);
        }
        offsets.push(offset);
        let mut table = Vec::with_capacity(offsets.len() * 4);
        offsets.as_slice().wow_write(&mut table)?;
        if key != 0 {
            crypto::encrypt_bytes(&mut table, key.wrapping_sub(1));
        }
        body.write_all(&table)?;
        for (i, mut p) in packed.into_iter().enumerate() {
            if key != 0 {
                crypto::encrypt_bytes(&mut p, key.wrapping_add(i as u32));
            }
            body.extend(p);
        }
    } else {
        for (i, sector) in sectors.iter().enumerate() {
            let mut s = sector.to_vec();
            if key != 0 {
                crypto::encrypt_bytes(&mut s, key.wrapping_add(i as u32));
            }
            body.extend(s);
        }
    }
    Ok((body, flags))
}
