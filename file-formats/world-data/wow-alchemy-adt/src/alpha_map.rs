//! MCAL alpha maps, as stored up to WotLK.

use wow_alchemy_data::error::{Result, WowDataError};

use crate::mcnk::{MclyFlags, McnkFlags};

/// Width and height of a decoded alpha map.
pub const ALPHA_MAP_SIZE: usize = 64;

/// Bytes in a decoded alpha map.
pub const ALPHA_MAP_BYTES: usize = ALPHA_MAP_SIZE * ALPHA_MAP_SIZE;

/// How one layer's alpha map is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlphaMapFormat {
    /// From the WDT's MPHD `ADT_HAS_BIG_ALPHA` flag: uncompressed maps have 8 bits per pixel.
    pub big_alpha: bool,
    /// From [`MclyFlags::ALPHA_MAP_COMPRESSED`] on the layer.
    pub compressed: bool,
    /// From [`McnkFlags::DO_NOT_FIX_ALPHA_MAP`] on the chunk.
    pub do_not_fix: bool,
}

impl AlphaMapFormat {
    /// The format for a layer with `layer_flags` in a chunk with `chunk_flags`.
    pub fn new(big_alpha: bool, layer_flags: MclyFlags, chunk_flags: McnkFlags) -> Self {
        Self {
            big_alpha,
            compressed: layer_flags.contains(MclyFlags::ALPHA_MAP_COMPRESSED),
            do_not_fix: chunk_flags.contains(McnkFlags::DO_NOT_FIX_ALPHA_MAP),
        }
    }
}

/// Decodes one alpha map to 8 bits per pixel, row by row.
///
/// `data` starts at the layer's offset into the MCAL data and may run past the end of the map.
pub fn decode_alpha_map(data: &[u8], format: AlphaMapFormat) -> Result<Vec<u8>> {
    if format.compressed {
        decode_compressed(data)
    } else if format.big_alpha {
        data.get(..ALPHA_MAP_BYTES)
            .map(<[u8]>::to_vec)
            .ok_or_else(|| truncated(data.len()))
    } else {
        decode_4_bit(data, format.do_not_fix)
    }
}

fn truncated(length: usize) -> WowDataError {
    WowDataError::InvalidData(format!("alpha map data is too short: {length} bytes"))
}

/// Run-length encoding: each command byte's low 7 bits are a count. With the high bit set the
/// next byte repeats, otherwise that many literal bytes follow.
fn decode_compressed(data: &[u8]) -> Result<Vec<u8>> {
    let mut output = Vec::with_capacity(ALPHA_MAP_BYTES);
    let mut position = 0;

    while output.len() < ALPHA_MAP_BYTES {
        let command = *data.get(position).ok_or_else(|| truncated(data.len()))?;
        position += 1;

        let count = (command & 0x7F) as usize;
        let count = count.min(ALPHA_MAP_BYTES - output.len());

        if command & 0x80 != 0 {
            let value = *data.get(position).ok_or_else(|| truncated(data.len()))?;
            position += 1;
            output.resize(output.len() + count, value);
        } else {
            let literal = data
                .get(position..position + count)
                .ok_or_else(|| truncated(data.len()))?;
            position += count;
            output.extend_from_slice(literal);
        }
    }

    Ok(output)
}

/// Two pixels per byte, low nibble first. Without `do_not_fix`, the client treats the map as
/// 63 by 63, so the last row and column repeat their neighbours.
fn decode_4_bit(data: &[u8], do_not_fix: bool) -> Result<Vec<u8>> {
    let packed = data
        .get(..ALPHA_MAP_BYTES / 2)
        .ok_or_else(|| truncated(data.len()))?;

    let mut output = Vec::with_capacity(ALPHA_MAP_BYTES);
    for byte in packed {
        output.push((byte & 0x0F) * 17);
        output.push((byte >> 4) * 17);
    }

    if !do_not_fix {
        for row in 0..ALPHA_MAP_SIZE {
            let end = row * ALPHA_MAP_SIZE + ALPHA_MAP_SIZE - 1;
            output[end] = output[end - 1];
        }
        let last_row = ALPHA_MAP_BYTES - ALPHA_MAP_SIZE;
        output.copy_within(last_row - ALPHA_MAP_SIZE..last_row, last_row);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FOUR_BIT: AlphaMapFormat = AlphaMapFormat {
        big_alpha: false,
        compressed: false,
        do_not_fix: true,
    };

    #[test]
    fn format_comes_from_the_flags() {
        let format = AlphaMapFormat::new(
            true,
            MclyFlags::USE_ALPHA_MAP | MclyFlags::ALPHA_MAP_COMPRESSED,
            McnkFlags::DO_NOT_FIX_ALPHA_MAP,
        );
        assert_eq!(
            format,
            AlphaMapFormat {
                big_alpha: true,
                compressed: true,
                do_not_fix: true
            }
        );
        let format = AlphaMapFormat::new(false, MclyFlags::USE_ALPHA_MAP, McnkFlags::empty());
        assert!(!format.compressed && !format.do_not_fix);
    }

    #[test]
    fn four_bit_expands_low_nibble_first() {
        let mut data = vec![0u8; 2048];
        data[0] = 0xF0;

        let map = decode_alpha_map(&data, FOUR_BIT).unwrap();

        assert_eq!(map.len(), ALPHA_MAP_BYTES);
        assert_eq!(&map[..2], &[0, 255]);
    }

    #[test]
    fn four_bit_fix_repeats_the_last_row_and_column() {
        let mut data = vec![0u8; 2048];
        // Pixel (62, 0) is the low nibble of the row's last byte.
        data[31] = 0x0F;
        // Pixel (0, 62) is the first of the second-to-last row.
        data[62 * 32] = 0x0F;

        let format = AlphaMapFormat {
            do_not_fix: false,
            ..FOUR_BIT
        };
        let map = decode_alpha_map(&data, format).unwrap();

        assert_eq!(map[63], 255);
        assert_eq!(map[63 * 64], 255);
    }

    #[test]
    fn big_alpha_is_copied() {
        let data: Vec<u8> = (0..ALPHA_MAP_BYTES + 9).map(|i| i as u8).collect();
        let format = AlphaMapFormat {
            big_alpha: true,
            ..FOUR_BIT
        };

        let map = decode_alpha_map(&data, format).unwrap();

        assert_eq!(map, data[..ALPHA_MAP_BYTES]);
    }

    #[test]
    fn compressed_handles_fills_and_literals_and_stops_at_the_end() {
        let mut data = vec![0x02, 7, 8];
        // 32 fills of 127 reach 4066 pixels, then the last fill overruns and is clamped.
        for _ in 0..33 {
            data.extend([0xFF, 9]);
        }
        let format = AlphaMapFormat {
            compressed: true,
            ..FOUR_BIT
        };

        let map = decode_alpha_map(&data, format).unwrap();

        assert_eq!(map.len(), ALPHA_MAP_BYTES);
        assert_eq!(&map[..3], &[7, 8, 9]);
        assert_eq!(map[ALPHA_MAP_BYTES - 1], 9);
    }

    #[test]
    fn truncated_data_is_an_error() {
        assert!(decode_alpha_map(&[0; 10], FOUR_BIT).is_err());

        let format = AlphaMapFormat {
            compressed: true,
            ..FOUR_BIT
        };
        assert!(decode_alpha_map(&[0xFF], format).is_err());
        // A zero-length literal run must not spin forever on truncated input.
        assert!(decode_alpha_map(&[0x00], format).is_err());
    }
}
