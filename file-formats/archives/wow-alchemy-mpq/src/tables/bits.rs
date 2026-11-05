//! Bit-packed arrays, as HET and BET tables store their fields.

/// Little-endian bit array: bit `n` is bit `n % 8` of byte `n / 8`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BitArray {
    /// The bytes holding the bits.
    pub bytes: Vec<u8>,
}

impl BitArray {
    /// An array of `bit_count` zero bits.
    pub fn new(bit_count: usize) -> Self {
        Self {
            bytes: vec![0; bit_count.div_ceil(8)],
        }
    }

    /// Reads `count` (up to 64) bits starting at bit `position`, or `None` past the end.
    pub fn get(&self, position: usize, count: u32) -> Option<u64> {
        if count == 0 {
            return Some(0);
        }
        if count > 64 || (position + count as usize).div_ceil(8) > self.bytes.len() {
            return None;
        }
        let mut value = 0u64;
        for i in 0..count as usize {
            let bit = position + i;
            if self.bytes[bit / 8] & (1 << (bit % 8)) != 0 {
                value |= 1 << i;
            }
        }
        Some(value)
    }

    /// Writes the low `count` bits of `value` starting at bit `position`.
    pub fn set(&mut self, position: usize, count: u32, value: u64) {
        for i in 0..count as usize {
            let bit = position + i;
            let mask = 1u8 << (bit % 8);
            if value >> i & 1 != 0 {
                self.bytes[bit / 8] |= mask;
            } else {
                self.bytes[bit / 8] &= !mask;
            }
        }
    }
}

/// Bits needed to store values up to `max`.
pub(crate) fn bits_needed(max: u64) -> u32 {
    (64 - max.leading_zeros()).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_cross_byte_boundaries() {
        let mut bits = BitArray::new(64);
        bits.set(5, 13, 0x1ABC);
        bits.set(18, 3, 0b101);
        assert_eq!(bits.get(5, 13), Some(0x1ABC));
        assert_eq!(bits.get(18, 3), Some(0b101));
        assert_eq!(bits.get(60, 8), None);
        assert_eq!(bits_needed(0), 1);
        assert_eq!(bits_needed(255), 8);
        assert_eq!(bits_needed(256), 9);
    }
}
