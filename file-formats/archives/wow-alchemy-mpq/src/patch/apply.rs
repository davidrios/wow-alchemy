//! Applying COPY and BSD0 patches, as Storm does.

use std::io::Cursor;

use wow_alchemy_data::prelude::*;
use wow_alchemy_data_derive::{WowHeaderR, WowHeaderW};

use super::{PatchFile, PatchHeader, PatchType, check_md5};
use crate::{Error, Result};

/// Header of the bsdiff inside a BSD0 patch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, WowHeaderR, WowHeaderW)]
pub struct Bsdiff40Header {
    /// `BSDIFF40`.
    pub magic: [u8; 8],
    /// Size of the control block.
    pub ctrl_block_size: u64,
    /// Size of the data block.
    pub data_block_size: u64,
    /// Size of the patched file.
    pub new_file_size: u64,
}

/// One step of a bsdiff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, WowHeaderR, WowHeaderW)]
pub struct BsdiffCtrl {
    /// Bytes to add from the data block to the old file.
    pub add_data_length: u32,
    /// Bytes to copy from the extra block.
    pub mov_data_length: u32,
    /// How far to move in the old file; negative when the top bit is set.
    pub old_move_length: u32,
}

/// Applies `patch` to `base`, checking the MD5s before and after.
pub fn apply_patch(patch: &PatchFile, base: &[u8]) -> Result<Vec<u8>> {
    check_md5(base, &patch.header.md5.md5_before, "file before patching")?;
    let patched = match patch.header.xfrm.patch_type {
        PatchType::Copy => patch.data.clone(),
        PatchType::Bsd0 => apply_bsd0(patch, base)?,
    };
    if patched.len() != patch.header.ptch.size_after as usize {
        return Err(Error::invalid_format(format!(
            "Patched file is {} bytes, the patch says {}",
            patched.len(),
            patch.header.ptch.size_after
        )));
    }
    check_md5(&patched, &patch.header.md5.md5_after, "patched file")?;
    Ok(patched)
}

/// Storm's run-length coding of BSD0 data: a size dword, then runs of `0x80 | (n - 1)`
/// followed by n bytes, or `n - 1` for n zeros.
fn decompress_rle(data: &[u8], size: usize) -> Vec<u8> {
    let mut out = vec![0u8; size];
    let mut src = data.get(4..).unwrap_or_default().iter();
    let mut pos = 0;
    while pos < size {
        let Some(&byte) = src.next() else { break };
        if byte & 0x80 != 0 {
            for _ in 0..(byte & 0x7F) as usize + 1 {
                let Some(&value) = src.next() else { break };
                if pos == size {
                    break;
                }
                out[pos] = value;
                pos += 1;
            }
        } else {
            pos += byte as usize + 1;
        }
    }
    out
}

fn apply_bsd0(patch: &PatchFile, base: &[u8]) -> Result<Vec<u8>> {
    let size = (patch.header.ptch.patch_data_size as usize).saturating_sub(PatchHeader::SIZE);
    let diff = if patch.data.len() < size {
        decompress_rle(&patch.data, size)
    } else {
        patch.data.clone()
    };

    let mut cursor = Cursor::new(&diff);
    let header: Bsdiff40Header = cursor.wow_read()?;
    if &header.magic != b"BSDIFF40" {
        return Err(Error::invalid_format(
            "BSD0 patch without a BSDIFF40 header",
        ));
    }
    let corrupt = || Error::invalid_format("BSD0 patch is corrupt");
    let ctrl_start = cursor.position() as usize;
    let data_start = ctrl_start + header.ctrl_block_size as usize;
    let extra_start = data_start + header.data_block_size as usize;
    if extra_start > diff.len() {
        return Err(corrupt());
    }
    let mut ctrl = Cursor::new(&diff[ctrl_start..data_start]);
    let mut data = &diff[data_start..extra_start];
    let mut extra = &diff[extra_start..];

    let new_size = header.new_file_size as usize;
    let mut new = vec![0u8; new_size];
    let mut new_offset = 0usize;
    let mut old_offset = 0u32;
    while new_offset < new_size {
        let step: BsdiffCtrl = ctrl.wow_read()?;
        let add = step.add_data_length as usize;
        if new_offset + add > new_size || add > data.len() {
            return Err(corrupt());
        }
        new[new_offset..new_offset + add].copy_from_slice(&data[..add]);
        data = &data[add..];
        let old = old_offset as usize;
        let combine = add.min(base.len().saturating_sub(old));
        for i in 0..combine {
            new[new_offset + i] = new[new_offset + i].wrapping_add(base[old + i]);
        }
        new_offset += add;
        old_offset = old_offset.wrapping_add(step.add_data_length);

        let mov = step.mov_data_length as usize;
        if new_offset + mov > new_size || mov > extra.len() {
            return Err(corrupt());
        }
        new[new_offset..new_offset + mov].copy_from_slice(&extra[..mov]);
        extra = &extra[mov..];
        new_offset += mov;

        let mut moved = step.old_move_length;
        if moved & 0x8000_0000 != 0 {
            moved = 0x8000_0000u32.wrapping_sub(moved);
        }
        old_offset = old_offset.wrapping_add(moved);
    }
    Ok(new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patch::{Md5Block, PtchBlock, XfrmBlock};
    use md5::{Digest, Md5};

    fn patch(kind: PatchType, before: &[u8], after: &[u8], data: Vec<u8>) -> PatchFile {
        PatchFile {
            header: PatchHeader {
                ptch: PtchBlock {
                    magic: *b"PTCH",
                    patch_data_size: (PatchHeader::SIZE + data.len()) as u32,
                    size_before: before.len() as u32,
                    size_after: after.len() as u32,
                },
                md5: Md5Block {
                    magic: *b"MD5_",
                    block_size: 40,
                    md5_before: Md5::digest(before).into(),
                    md5_after: Md5::digest(after).into(),
                },
                xfrm: XfrmBlock {
                    magic: *b"XFRM",
                    block_size: 12 + data.len() as u32,
                    patch_type: kind,
                },
            },
            data,
        }
    }

    #[test]
    fn copy_replaces_the_file() {
        let p = patch(PatchType::Copy, b"old", b"new data", b"new data".to_vec());
        assert_eq!(apply_patch(&p, b"old").unwrap(), b"new data");
        assert!(apply_patch(&p, b"other").is_err());
    }

    #[test]
    fn bsd0_adds_copies_and_moves_back() {
        let old = b"ABCDEFGH";
        let new = b"BCDxyzABCD";
        // Move to old offset 1, add "BCD" (differences of 0) and copy "xyz", move back 4
        // to offset 0, then add "ABCD".
        let steps = [
            BsdiffCtrl {
                add_data_length: 0,
                mov_data_length: 0,
                old_move_length: 1,
            },
            BsdiffCtrl {
                add_data_length: 3,
                mov_data_length: 3,
                old_move_length: 0x8000_0000 | 4,
            },
            BsdiffCtrl {
                add_data_length: 4,
                mov_data_length: 0,
                old_move_length: 0,
            },
        ];
        let mut diff = Vec::new();
        Bsdiff40Header {
            magic: *b"BSDIFF40",
            ctrl_block_size: 36,
            data_block_size: 7,
            new_file_size: new.len() as u64,
        }
        .wow_write(&mut diff)
        .unwrap();
        steps.as_slice().wow_write(&mut diff).unwrap();
        diff.extend([0u8; 7]);
        diff.extend(b"xyz");
        let p = patch(PatchType::Bsd0, old, new, diff);
        assert_eq!(apply_patch(&p, old).unwrap(), new);
    }

    #[test]
    fn rle_fills_zero_runs() {
        let rle = [8, 0, 0, 0, 0x82, b'A', b'B', b'C', 0x01, 0x81, b'D', b'E'];
        assert_eq!(decompress_rle(&rle, 8), b"ABC\0\0DE\0");
    }
}
