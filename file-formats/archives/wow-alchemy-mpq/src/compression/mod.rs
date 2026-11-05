//! The compressions MPQ sectors use.
//!
//! A compressed sector starts with a byte naming the compressions applied, as a mask of
//! [`flags`] (or [`flags::LZMA`] alone). [`decompress`] undoes them in Storm's order and
//! [`compress`] applies one.

mod adpcm;
mod bzip2;
mod huffman;
mod lzma;
mod pkware;
mod sparse;
mod zlib;

use crate::{Error, Result};

/// Values of the compression byte.
pub mod flags {
    /// Huffman coding (WAVE files, together with ADPCM).
    pub const HUFFMAN: u8 = 0x01;
    /// Deflate with a zlib header.
    pub const ZLIB: u8 = 0x02;
    /// PKWARE Data Compression Library.
    pub const PKWARE: u8 = 0x08;
    /// BZip2.
    pub const BZIP2: u8 = 0x10;
    /// Storm's sparse run-length coding.
    pub const SPARSE: u8 = 0x20;
    /// IMA ADPCM, one channel.
    pub const ADPCM_MONO: u8 = 0x40;
    /// IMA ADPCM, two channels.
    pub const ADPCM_STEREO: u8 = 0x80;
    /// LZMA. A value of its own, not combined with the others.
    pub const LZMA: u8 = 0x12;
}

pub(crate) mod error_helpers {
    use crate::Error;

    pub(crate) fn compression_error(algorithm: &str, err: impl std::fmt::Display) -> Error {
        Error::compression(format!("{algorithm} compression failed: {err}"))
    }

    pub(crate) fn decompression_error(algorithm: &str, err: impl std::fmt::Display) -> Error {
        Error::compression(format!("{algorithm} decompression failed: {err}"))
    }
}

type Decompressor = fn(&[u8], usize) -> Result<Vec<u8>>;

/// The decompressions in the order Storm applies them.
const DECOMPRESSION_ORDER: [(u8, Decompressor); 7] = [
    (flags::BZIP2, bzip2::decompress),
    (flags::PKWARE, pkware::decompress),
    (flags::ZLIB, zlib::decompress),
    (flags::HUFFMAN, huffman::decompress),
    (flags::ADPCM_STEREO, adpcm::decompress_stereo),
    (flags::ADPCM_MONO, adpcm::decompress_mono),
    (flags::SPARSE, sparse::decompress),
];

/// Decompresses a block that starts with its compression byte into `output_size` bytes.
///
/// A block as long as its output is stored as it is, without a compression byte.
pub fn decompress(data: &[u8], output_size: usize) -> Result<Vec<u8>> {
    if data.len() == output_size {
        return Ok(data.to_vec());
    }
    let (&mask, mut payload) = data
        .split_first()
        .ok_or_else(|| Error::compression("Empty compressed data"))?;
    if mask == flags::LZMA {
        return lzma::decompress(payload, output_size);
    }
    let known = DECOMPRESSION_ORDER
        .iter()
        .fold(0, |all, (bit, _)| all | bit);
    if mask == 0 || mask & !known != 0 {
        return Err(Error::compression(format!(
            "Unknown compression 0x{mask:02X}"
        )));
    }

    let mut stage: Option<Vec<u8>> = None;
    for (bit, decompress) in DECOMPRESSION_ORDER {
        if mask & bit != 0 {
            let out = decompress(payload_of(&stage, payload), output_size)?;
            if out.is_empty() {
                return Err(Error::compression(format!(
                    "Compression 0x{bit:02X} produced no data"
                )));
            }
            stage = Some(out);
            payload = &[];
        }
    }
    Ok(stage.unwrap_or_default())
}

fn payload_of<'a>(stage: &'a Option<Vec<u8>>, first: &'a [u8]) -> &'a [u8] {
    stage.as_deref().unwrap_or(first)
}

/// Compresses `data` with one compression of [`flags`], returning the compression byte
/// followed by the compressed data.
pub fn compress(data: &[u8], method: u8) -> Result<Vec<u8>> {
    let compressed = match method {
        flags::ZLIB => zlib::compress(data)?,
        flags::BZIP2 => bzip2::compress(data)?,
        flags::LZMA => lzma::compress(data)?,
        flags::PKWARE => pkware::compress(data)?,
        flags::SPARSE => sparse::compress(data)?,
        flags::ADPCM_MONO => adpcm::compress_mono(data, 5)?,
        flags::ADPCM_STEREO => adpcm::compress_stereo(data, 5)?,
        _ => {
            return Err(Error::compression(format!(
                "Compressing with 0x{method:02X} is not supported"
            )));
        }
    };
    let mut out = Vec::with_capacity(compressed.len() + 1);
    out.push(method);
    out.extend(compressed);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        b"Elwynn Forest, Westfall, Duskwood and Redridge. ".repeat(30)
    }

    #[test]
    fn each_compression_round_trips() {
        let data = sample();
        for method in [
            flags::ZLIB,
            flags::BZIP2,
            flags::LZMA,
            flags::PKWARE,
            flags::SPARSE,
        ] {
            let packed = compress(&data, method).unwrap();
            assert_eq!(packed[0], method);
            assert_eq!(
                decompress(&packed, data.len()).unwrap(),
                data,
                "0x{method:02X}"
            );
        }
    }

    #[test]
    fn data_as_long_as_its_output_is_stored() {
        assert_eq!(decompress(b"abcd", 4).unwrap(), b"abcd");
    }

    #[test]
    fn unknown_masks_are_rejected() {
        assert!(decompress(&[0x04, 1, 2, 3], 10).is_err());
        assert!(decompress(&[0x00, 1, 2, 3], 10).is_err());
    }

    #[test]
    fn sparse_then_zlib_undoes_both() {
        let data = [vec![0u8; 300], sample(), vec![0u8; 300]].concat();
        let sparse = compress(&data, flags::SPARSE).unwrap();
        let mut both = compress(&sparse[1..], flags::ZLIB).unwrap();
        both[0] = flags::ZLIB | flags::SPARSE;
        assert_eq!(decompress(&both, data.len()).unwrap(), data);
    }
}
