//! LZMA, as Storm stores it: a filter byte (0, no filter), then the 5 property bytes, the
//! 64-bit uncompressed size and the LZMA stream.

use crate::compression::error_helpers::{compression_error, decompression_error};
use crate::{Error, Result};
use std::io::Cursor;

/// Decompresses Storm's LZMA format.
pub(crate) fn decompress(data: &[u8], expected_size: usize) -> Result<Vec<u8>> {
    match data.first() {
        Some(0) => {}
        Some(filter) => {
            return Err(Error::compression(format!(
                "LZMA with filter {filter} is not supported"
            )));
        }
        None => return Err(Error::compression("Empty LZMA data")),
    }
    let mut output = Vec::with_capacity(expected_size);
    lzma_rs::lzma_decompress(&mut Cursor::new(&data[1..]), &mut output)
        .map_err(|e| decompression_error("LZMA", format!("{e:?}")))?;
    Ok(output)
}

/// Compresses into Storm's LZMA format.
pub(crate) fn compress(data: &[u8]) -> Result<Vec<u8>> {
    let mut output = vec![0u8];
    lzma_rs::lzma_compress(&mut Cursor::new(data), &mut output)
        .map_err(|e| compression_error("LZMA", format!("{e:?}")))?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_with_the_filter_byte() {
        let original = b"LZMA should give good ratios on repeated text, repeated text.".repeat(8);
        let compressed = compress(&original).unwrap();
        assert_eq!(compressed[0], 0);
        assert_eq!(decompress(&compressed, original.len()).unwrap(), original);
    }
}
