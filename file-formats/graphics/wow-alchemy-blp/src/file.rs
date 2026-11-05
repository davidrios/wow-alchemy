//! A whole BLP: the header, the palette or JPEG header, and the mipmaps.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use custom_debug::Debug;
use wow_alchemy_data::error::{Result, WowDataError};
use wow_alchemy_data::prelude::*;
use wow_alchemy_data::types::{WowStructR, WowStructW};

use crate::header::*;

/// Palette entries are the bytes blue, green, red and alpha, so as a little-endian `u32` red is
/// bits 16 to 23 and blue the lowest byte.
pub type Palette = [u32; 256];

/// What follows the header, which depends on its content tag.
// A palette is 1 KiB, once per file.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BlpData {
    /// For JPEG content: the bytes that start every mipmap's JPEG, stored after their length.
    Jpeg(#[debug(with = byte_count)] Vec<u8>),
    /// For direct content. Only palettized images use it, but every direct file has one.
    Palette(#[debug(skip)] Palette),
}

/// What the mipmaps hold, from the header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BlpFormat {
    /// JPEG bodies to put after the JPEG header.
    Jpeg,
    /// Palette indices, a byte per pixel, then the alpha at `alpha_bits` per pixel, from the
    /// lowest bits of each byte up.
    Raw1,
    /// BGRA pixels.
    Raw3,
    Dxt1,
    Dxt3,
    Dxt5,
}

impl BlpFormat {
    /// The DXT kind, for DXT formats.
    pub fn dxt(self) -> Option<texpresso::Format> {
        match self {
            BlpFormat::Dxt1 => Some(texpresso::Format::Bc1),
            BlpFormat::Dxt3 => Some(texpresso::Format::Bc2),
            BlpFormat::Dxt5 => Some(texpresso::Format::Bc3),
            _ => None,
        }
    }
}

/// A BLP file as it is stored.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlpImage {
    pub header: BlpHeader,
    pub data: BlpData,
    /// Each level's bytes as stored, largest first: as many as the header's size calls for, or
    /// fewer if the file stops earlier. The header's mipmap offsets and sizes are worked out
    /// from these on writing, one level after another.
    #[debug(with = byte_counts)]
    pub mipmaps: Vec<Vec<u8>>,
}

impl BlpImage {
    /// Reads a file whose BLP0 mipmaps come from `external`, which gives level `i`'s file, or
    /// `None` when there isn't one.
    pub fn read_with_external<R, F>(reader: &mut R, mut external: F) -> Result<Self>
    where
        R: Read + Seek,
        F: FnMut(usize) -> Result<Option<Vec<u8>>>,
    {
        let header: BlpHeader = reader.wow_read()?;
        let data = match header.content {
            BlpContentTag::Jpeg => {
                let size: u32 = reader.wow_read()?;
                BlpData::Jpeg(read_bytes(reader, size)?)
            }
            BlpContentTag::Direct => BlpData::Palette(reader.wow_read()?),
        };

        let mut mipmaps = Vec::new();
        match &header.mipmap_locator {
            MipmapLocator::Internal { offsets, sizes } => {
                let end = reader.seek(SeekFrom::End(0))?;
                let levels = header.mipmap_levels();
                for (level, (&offset, &size)) in offsets.iter().zip(sizes).enumerate().take(levels)
                {
                    if size == 0 {
                        break;
                    }
                    if offset as u64 + size as u64 > end {
                        return Err(WowDataError::InvalidData(format!(
                            "mipmap {level} at {offset}, {size} bytes long, ends past the file's {end} bytes"
                        )));
                    }
                    reader.seek(SeekFrom::Start(offset as u64))?;
                    mipmaps.push(read_bytes(reader, size)?);
                }
            }
            MipmapLocator::External => {
                for level in 0..header.mipmap_levels().min(MAX_MIPMAPS) {
                    let bytes = external(level)?.ok_or_else(|| {
                        WowDataError::InvalidData(format!("BLP0 mipmap file {level} is missing"))
                    })?;
                    mipmaps.push(bytes);
                }
            }
        }

        Ok(Self {
            header,
            data,
            mipmaps,
        })
    }

    /// What the mipmaps hold.
    pub fn format(&self) -> Result<BlpFormat> {
        Ok(match (self.header.content, &self.header.flags) {
            (BlpContentTag::Jpeg, _) => BlpFormat::Jpeg,
            (BlpContentTag::Direct, BlpFlags::Old { .. }) => BlpFormat::Raw1,
            (
                BlpContentTag::Direct,
                BlpFlags::Blp2 {
                    compression,
                    alpha_type,
                    ..
                },
            ) => match (compression, alpha_type) {
                (Compression::Raw1, _) => BlpFormat::Raw1,
                (Compression::Raw3, _) => BlpFormat::Raw3,
                (Compression::Dxtc, AlphaType::Dxt1) => BlpFormat::Dxt1,
                (Compression::Dxtc, AlphaType::Dxt3) => BlpFormat::Dxt3,
                (Compression::Dxtc, AlphaType::Dxt5) => BlpFormat::Dxt5,
                (Compression::Dxtc, AlphaType::Other(value)) => {
                    return Err(WowDataError::InvalidData(format!(
                        "DXT compression with alpha type {value}"
                    )));
                }
                (Compression::Jpeg, _) => {
                    return Err(WowDataError::InvalidData(
                        "JPEG compression with direct content".into(),
                    ));
                }
            },
        })
    }

    pub fn image_count(&self) -> usize {
        self.mipmaps.len()
    }

    /// The whole JPEG of a level, for JPEG content.
    pub fn jpeg(&self, level: usize) -> Option<Vec<u8>> {
        let BlpData::Jpeg(header) = &self.data else {
            return None;
        };
        let body = self.mipmaps.get(level)?;
        Some([header.as_slice(), body].concat())
    }

    /// Sets the header's mipmap offsets and sizes for the mipmaps one after another, after the
    /// palette or JPEG header, as the game's files have them.
    pub fn update_locator(&mut self) -> Result<()> {
        self.header = self.header_for_writing()?;
        Ok(())
    }

    /// The header with the locator of [`BlpImage::update_locator`], once checked that it can
    /// be written.
    fn header_for_writing(&self) -> Result<BlpHeader> {
        let mut header = self.header.clone();
        header.check_version()?;
        match (header.content, &self.data) {
            (BlpContentTag::Jpeg, BlpData::Jpeg(_))
            | (BlpContentTag::Direct, BlpData::Palette(_)) => {}
            (content, _) => {
                return Err(WowDataError::InvalidData(format!(
                    "{content:?} content with the other kind's data"
                )));
            }
        }

        let start = header.wow_size() + self.data_size();
        if let MipmapLocator::Internal { offsets, sizes } = &mut header.mipmap_locator {
            if self.mipmaps.len() > MAX_MIPMAPS {
                return Err(WowDataError::InvalidData(format!(
                    "{} mipmaps, but there is room for {MAX_MIPMAPS}",
                    self.mipmaps.len()
                )));
            }
            *offsets = [0; MAX_MIPMAPS];
            *sizes = [0; MAX_MIPMAPS];
            let mut offset = start;
            for (level, mipmap) in self.mipmaps.iter().enumerate() {
                offsets[level] = to_u32(offset)?;
                sizes[level] = to_u32(mipmap.len())?;
                offset += mipmap.len();
            }
        }
        Ok(header)
    }

    fn data_size(&self) -> usize {
        match &self.data {
            BlpData::Jpeg(header) => 4 + header.len(),
            BlpData::Palette(palette) => palette.wow_size(),
        }
    }
}

impl WowStructR for BlpImage {
    /// Reads a file. BLP0 files need [`BlpImage::read_with_external`] or [`load_blp`] instead.
    fn wow_read<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        Self::read_with_external(reader, |_| Ok(None))
    }
}

impl WowStructW for BlpImage {
    /// Writes the file, without BLP0's mipmaps, which [`save_blp`] writes to their own files.
    fn wow_write<W: Write + Seek>(&self, writer: &mut W) -> Result<()> {
        let header = self.header_for_writing()?;
        writer.wow_write(&header)?;
        match &self.data {
            BlpData::Jpeg(header) => {
                writer.wow_write(&to_u32(header.len())?)?;
                header.as_slice().wow_write(writer)?;
            }
            BlpData::Palette(palette) => writer.wow_write(palette)?,
        }
        if let MipmapLocator::Internal { .. } = header.mipmap_locator {
            for mipmap in &self.mipmaps {
                writer.write_all(mipmap)?;
            }
        }
        Ok(())
    }
}

/// Reads a BLP from memory. BLP0 files fail, as their mipmaps are in other files.
pub fn load_blp_from_buf(bytes: &[u8]) -> Result<BlpImage> {
    BlpImage::wow_read(&mut Cursor::new(bytes))
}

/// Reads a BLP file, and for BLP0 its mipmap files beside it.
pub fn load_blp<P: AsRef<Path>>(path: P) -> Result<BlpImage> {
    let path = path.as_ref();
    let bytes = std::fs::read(path)?;
    BlpImage::read_with_external(&mut Cursor::new(bytes), |level| {
        let mipmap = mipmap_path(path, level)?;
        if mipmap.is_file() {
            Ok(Some(std::fs::read(mipmap)?))
        } else {
            Ok(None)
        }
    })
}

/// Writes a BLP file, and for BLP0 its mipmap files beside it.
pub fn save_blp<P: AsRef<Path>>(image: &BlpImage, path: P) -> Result<()> {
    let path = path.as_ref();
    let mut bytes = Cursor::new(Vec::new());
    image.wow_write(&mut bytes)?;
    std::fs::write(path, bytes.into_inner())?;
    if image.header.mipmap_locator == MipmapLocator::External {
        for (level, mipmap) in image.mipmaps.iter().enumerate() {
            std::fs::write(mipmap_path(path, level)?, mipmap)?;
        }
    }
    Ok(())
}

/// Where BLP0 keeps a mipmap: `name.b00` and on, beside `name.blp`.
pub fn mipmap_path(path: &Path, level: usize) -> Result<PathBuf> {
    let stem = path
        .file_stem()
        .ok_or_else(|| WowDataError::GenericError(format!("no file name in {}", path.display())))?;
    Ok(path
        .with_file_name(stem)
        .with_extension(format!("b{level:02}")))
}

fn read_bytes<R: Read>(reader: &mut R, size: u32) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(size as u64).read_to_end(&mut bytes)?;
    if bytes.len() != size as usize {
        return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof).into());
    }
    Ok(bytes)
}

#[allow(clippy::ptr_arg)]
fn byte_count(bytes: &Vec<u8>, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f, "{} bytes", bytes.len())
}

#[allow(clippy::ptr_arg)]
fn byte_counts(mipmaps: &Vec<Vec<u8>>, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    f.debug_list()
        .entries(mipmaps.iter().map(|m| m.len()))
        .finish()
}

fn to_u32(value: usize) -> Result<u32> {
    u32::try_from(value)
        .map_err(|_| WowDataError::InvalidData(format!("{value} doesn't fit a BLP")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dxt1(width: u32, height: u32, levels: usize) -> BlpImage {
        let header = BlpHeader::new(
            BlpVersion::Blp2,
            BlpContentTag::Direct,
            BlpFlags::Blp2 {
                compression: Compression::Dxtc,
                alpha_bits: 0,
                alpha_type: AlphaType::Dxt1,
                has_mipmaps: (levels > 1) as u8,
            },
            BlpExtra::None,
            width,
            height,
        );
        let mut palette = [0; 256];
        palette[3] = 0x1234_5678;
        BlpImage {
            header,
            data: BlpData::Palette(palette),
            mipmaps: (0..levels).map(|l| vec![l as u8; 8]).collect(),
        }
    }

    fn bytes(image: &BlpImage) -> Vec<u8> {
        let mut out = Cursor::new(Vec::new());
        image.wow_write(&mut out).unwrap();
        out.into_inner()
    }

    #[test]
    fn writes_mipmaps_after_the_palette_and_reads_them_back() {
        let image = dxt1(4, 4, 3);
        let written = bytes(&image);
        assert_eq!(written.len(), 148 + 1024 + 24);
        assert_eq!(&written[1172..1180], &[0; 8]);

        let read = load_blp_from_buf(&written).unwrap();
        let MipmapLocator::Internal { offsets, sizes } = read.header.mipmap_locator else {
            panic!()
        };
        assert_eq!(&offsets[..4], &[1172, 1180, 1188, 0]);
        assert_eq!(&sizes[..4], &[8, 8, 8, 0]);
        assert_eq!(read.mipmaps, image.mipmaps);
        assert_eq!(read.data, image.data);
        assert_eq!(read.format().unwrap(), BlpFormat::Dxt1);
        assert_eq!(bytes(&read), written);
    }

    #[test]
    fn mipmaps_past_the_end_fail() {
        let mut written = bytes(&dxt1(4, 4, 2));
        written.truncate(written.len() - 1);
        assert!(load_blp_from_buf(&written).is_err());
        assert!(load_blp_from_buf(&written[..100]).is_err());
        assert!(load_blp_from_buf(&[]).is_err());
    }

    #[test]
    fn dxt_needs_a_known_alpha_type() {
        let mut image = dxt1(4, 4, 1);
        if let BlpFlags::Blp2 { alpha_type, .. } = &mut image.header.flags {
            *alpha_type = AlphaType::Other(4);
        }
        assert!(image.format().is_err());
        if let BlpFlags::Blp2 { compression, .. } = &mut image.header.flags {
            *compression = Compression::Raw1;
        }
        assert_eq!(image.format().unwrap(), BlpFormat::Raw1);
    }

    #[test]
    fn headers_must_match_their_version_and_content() {
        let mut image = dxt1(4, 4, 1);
        image.header.version = BlpVersion::Blp1;
        assert!(image.update_locator().is_err());

        let mut image = dxt1(4, 4, 1);
        image.data = BlpData::Jpeg(vec![]);
        assert!(image.update_locator().is_err());
    }

    #[test]
    fn blp0_keeps_its_mipmaps_in_files_of_their_own() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.blp");
        let header = BlpHeader::new(
            BlpVersion::Blp0,
            BlpContentTag::Jpeg,
            BlpFlags::Old { alpha_bits: 8 },
            BlpExtra::Old {
                extra: 5,
                has_mipmaps: 1,
            },
            2,
            2,
        );
        let image = BlpImage {
            header,
            data: BlpData::Jpeg(vec![0xFF, 0xD8]),
            mipmaps: vec![vec![1, 2, 3], vec![4]],
        };
        save_blp(&image, &path).unwrap();
        assert_eq!(std::fs::read(dir.path().join("test.b01")).unwrap(), [4]);
        assert_eq!(load_blp(&path).unwrap(), image);
        assert_eq!(image.jpeg(0).unwrap(), [0xFF, 0xD8, 1, 2, 3]);

        std::fs::remove_file(dir.path().join("test.b01")).unwrap();
        assert!(load_blp(&path).is_err());
    }

    #[test]
    fn mipmap_paths() {
        assert_eq!(
            mipmap_path(Path::new("test.blp"), 0).unwrap(),
            Path::new("test.b00")
        );
        assert_eq!(
            mipmap_path(Path::new("/root/dir/test.blp"), 14).unwrap(),
            Path::new("/root/dir/test.b14")
        );
    }
}
