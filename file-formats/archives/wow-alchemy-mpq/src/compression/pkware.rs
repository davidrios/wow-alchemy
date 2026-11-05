//! PKWARE Data Compression Library (implode/explode), through `pklib`.

use crate::Result;
use crate::compression::error_helpers::{compression_error, decompression_error};
use pklib::{CompressionMode, DictionarySize, explode_bytes, implode_bytes};

/// Implodes `data` in binary mode with a 4 KB dictionary, as Storm does for MPQ files.
pub(crate) fn compress(data: &[u8]) -> Result<Vec<u8>> {
    implode_bytes(data, CompressionMode::Binary, DictionarySize::Size4K)
        .map_err(|e| compression_error("PKWare", e))
}

/// Explodes `data`, keeping at most `expected_size` bytes.
pub(crate) fn decompress(data: &[u8], expected_size: usize) -> Result<Vec<u8>> {
    let mut output = explode_bytes(data).map_err(|e| decompression_error("PKWare", e))?;
    output.truncate(expected_size);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let data: Vec<u8> = b"pkware pkware pkware data ".repeat(40);
        let packed = compress(&data).unwrap();
        assert!(packed.len() < data.len());
        assert_eq!(decompress(&packed, data.len()).unwrap(), data);
    }
}
