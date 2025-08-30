//! Bounds checking utilities for BLP parsers

use super::error::Error;
use super::types::ParseResult;
use log::error;

/// Check if a given offset and size are within the bounds of input data
///
/// This helper function consolidates the bounds checking logic that was duplicated
/// across multiple BLP parsing functions.
pub fn check_bounds(input: &[u8], offset: u32, size: u32, mipmap_index: usize) -> ParseResult<()> {
    // Check if offset is within input bounds
    if offset as usize >= input.len() {
        error!(
            "Offset of mipmap {} is out of bounds! {} >= {}",
            mipmap_index,
            offset,
            input.len()
        );
        return Err(Error::OutOfBounds {
            offset: offset as usize,
            size: 0,
        });
    }

    // Check if offset + size extends beyond input bounds
    if (offset + size) as usize > input.len() {
        error!(
            "Offset+size of mipmap {} is out of bounds! {} > {}",
            mipmap_index,
            offset + size,
            input.len()
        );
        return Err(Error::OutOfBounds {
            offset: offset as usize,
            size: size as usize,
        });
    }

    Ok(())
}

/// Get a slice from input data after bounds checking
///
/// Convenience function that checks bounds and returns the slice if valid.
pub fn get_bounded_slice(
    input: &[u8],
    offset: u32,
    size: u32,
    mipmap_index: usize,
) -> ParseResult<&[u8]> {
    check_bounds(input, offset, size, mipmap_index)?;
    Ok(&input[offset as usize..(offset + size) as usize])
}
