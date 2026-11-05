//! MPQ hashing and encryption.
//!
//! Classic tables are found by [`hash_string`] and encrypted with the same table of
//! constants; HET tables use Jenkins' `hashlittle2` through [`het_hash`].

/// Which of the four hashes [`hash_string`] computes.
pub mod hash_type {
    /// Hash table index.
    pub const TABLE_OFFSET: u32 = 0x000;
    /// First check hash, stored in the hash entry.
    pub const NAME_A: u32 = 0x100;
    /// Second check hash, stored in the hash entry.
    pub const NAME_B: u32 = 0x200;
    /// Encryption key of a file or table.
    pub const FILE_KEY: u32 = 0x300;
    /// Part of the table used while encrypting.
    pub const KEY2_MIX: u32 = 0x400;
}

const fn encryption_table() -> [u32; 0x500] {
    let mut table = [0u32; 0x500];
    let mut seed: u32 = 0x0010_0001;
    let mut index1 = 0;
    while index1 < 0x100 {
        let mut index2 = 0;
        while index2 < 5 {
            seed = (seed * 125 + 3) % 0x2A_AAAB;
            let high = (seed & 0xFFFF) << 16;
            seed = (seed * 125 + 3) % 0x2A_AAAB;
            table[index1 + index2 * 0x100] = high | (seed & 0xFFFF);
            index2 += 1;
        }
        index1 += 1;
    }
    table
}

/// The constants every MPQ hash and cipher is built from.
pub const ENCRYPTION_TABLE: [u32; 0x500] = encryption_table();

/// Key of the classic hash table.
pub const HASH_TABLE_KEY: u32 = 0xC3AF_3770;
/// Key of the classic block table.
pub const BLOCK_TABLE_KEY: u32 = 0xEC83_B3A3;

fn normalize_upper(byte: u8) -> u8 {
    if byte == b'/' {
        b'\\'
    } else {
        byte.to_ascii_uppercase()
    }
}

fn normalize_lower(byte: u8) -> u8 {
    if byte == b'/' {
        b'\\'
    } else {
        byte.to_ascii_lowercase()
    }
}

/// The classic MPQ string hash. Case-insensitive, and `/` hashes like `\`.
pub fn hash_string(name: &str, hash_type: u32) -> u32 {
    let mut seed1: u32 = 0x7FED_7FED;
    let mut seed2: u32 = 0xEEEE_EEEE;
    for &byte in name.as_bytes() {
        let ch = normalize_upper(byte) as u32;
        seed1 = ENCRYPTION_TABLE[(hash_type + ch) as usize] ^ seed1.wrapping_add(seed2);
        seed2 = ch
            .wrapping_add(seed1)
            .wrapping_add(seed2)
            .wrapping_add(seed2 << 5)
            .wrapping_add(3);
    }
    seed1
}

/// The encryption key of a file: the hash of its plain name (after the last `\`), adjusted
/// by its position and size when the block has the `FIX_KEY` flag.
pub fn file_key(name: &str, file_pos: u64, file_size: u32, fix_key: bool) -> u32 {
    let normalized = name.replace('/', "\\");
    let plain = normalized.rsplit('\\').next().unwrap_or(&normalized);
    let key = hash_string(plain, hash_type::FILE_KEY);
    if fix_key {
        key.wrapping_add(file_pos as u32) ^ file_size
    } else {
        key
    }
}

/// Encrypts dwords in place. A key of 0 means not encrypted and leaves them as they are.
pub fn encrypt_block(data: &mut [u32], mut key: u32) {
    if key == 0 {
        return;
    }
    let mut seed: u32 = 0xEEEE_EEEE;
    for value in data.iter_mut() {
        seed = seed.wrapping_add(ENCRYPTION_TABLE[(hash_type::KEY2_MIX + (key & 0xFF)) as usize]);
        let plain = *value;
        *value = plain ^ key.wrapping_add(seed);
        key = ((!key << 0x15).wrapping_add(0x1111_1111)) | (key >> 0x0B);
        seed = plain
            .wrapping_add(seed)
            .wrapping_add(seed << 5)
            .wrapping_add(3);
    }
}

/// Decrypts dwords in place. A key of 0 leaves them as they are.
pub fn decrypt_block(data: &mut [u32], mut key: u32) {
    if key == 0 {
        return;
    }
    let mut seed: u32 = 0xEEEE_EEEE;
    for value in data.iter_mut() {
        seed = seed.wrapping_add(ENCRYPTION_TABLE[(hash_type::KEY2_MIX + (key & 0xFF)) as usize]);
        let plain = *value ^ key.wrapping_add(seed);
        *value = plain;
        key = ((!key << 0x15).wrapping_add(0x1111_1111)) | (key >> 0x0B);
        seed = plain
            .wrapping_add(seed)
            .wrapping_add(seed << 5)
            .wrapping_add(3);
    }
}

fn transform_bytes(data: &mut [u8], key: u32, transform: fn(&mut [u32], u32)) {
    let mut words: Vec<u32> = data
        .chunks_exact(4)
        .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();
    transform(&mut words, key);
    for (chunk, word) in data.chunks_exact_mut(4).zip(words) {
        chunk.copy_from_slice(&word.to_le_bytes());
    }
}

/// Encrypts little-endian dwords in place. Trailing bytes past the last whole dword are
/// left as they are, as Storm does.
pub fn encrypt_bytes(data: &mut [u8], key: u32) {
    transform_bytes(data, key, encrypt_block);
}

/// Decrypts little-endian dwords in place, leaving trailing bytes as they are.
pub fn decrypt_bytes(data: &mut [u8], key: u32) {
    transform_bytes(data, key, decrypt_block);
}

/// Jenkins' `hashlittle2` with seeds `*pc` and `*pb`, returning the new `(*pc, *pb)`.
fn hashlittle2(key: &[u8], pc: u32, pb: u32) -> (u32, u32) {
    let mut a = 0xDEAD_BEEFu32
        .wrapping_add(key.len() as u32)
        .wrapping_add(pc);
    let mut b = a;
    let mut c = a.wrapping_add(pb);

    let word = |bytes: &[u8]| -> u32 {
        let mut padded = [0u8; 4];
        padded[..bytes.len()].copy_from_slice(bytes);
        u32::from_le_bytes(padded)
    };

    let mut rest = key;
    while rest.len() > 12 {
        a = a.wrapping_add(word(&rest[0..4]));
        b = b.wrapping_add(word(&rest[4..8]));
        c = c.wrapping_add(word(&rest[8..12]));
        a = a.wrapping_sub(c);
        a ^= c.rotate_left(4);
        c = c.wrapping_add(b);
        b = b.wrapping_sub(a);
        b ^= a.rotate_left(6);
        a = a.wrapping_add(c);
        c = c.wrapping_sub(b);
        c ^= b.rotate_left(8);
        b = b.wrapping_add(a);
        a = a.wrapping_sub(c);
        a ^= c.rotate_left(16);
        c = c.wrapping_add(b);
        b = b.wrapping_sub(a);
        b ^= a.rotate_left(19);
        a = a.wrapping_add(c);
        c = c.wrapping_sub(b);
        c ^= b.rotate_left(4);
        b = b.wrapping_add(a);
        rest = &rest[12..];
    }

    if rest.is_empty() {
        return (c, b);
    }
    a = a.wrapping_add(word(&rest[..rest.len().min(4)]));
    if rest.len() > 4 {
        b = b.wrapping_add(word(&rest[4..rest.len().min(8)]));
    }
    if rest.len() > 8 {
        c = c.wrapping_add(word(&rest[8..]));
    }
    c ^= b;
    c = c.wrapping_sub(b.rotate_left(14));
    a ^= c;
    a = a.wrapping_sub(c.rotate_left(11));
    b ^= a;
    b = b.wrapping_sub(a.rotate_left(25));
    c ^= b;
    c = c.wrapping_sub(b.rotate_left(16));
    a ^= c;
    a = a.wrapping_sub(c.rotate_left(4));
    b ^= a;
    b = b.wrapping_sub(a.rotate_left(14));
    c ^= b;
    c = c.wrapping_sub(b.rotate_left(24));
    (c, b)
}

/// The 64-bit Jenkins hash of a file name, lowercased with `/` turned into `\`, as HET and
/// BET tables store it (before masking to the table's hash size).
pub fn jenkins_hash(name: &str) -> u64 {
    let normalized: Vec<u8> = name.bytes().map(normalize_lower).collect();
    // primary_hash = 1 is *pb, secondary_hash = 2 is *pc
    let (secondary, primary) = hashlittle2(&normalized, 2, 1);
    ((primary as u64) << 32) | secondary as u64
}

/// The HET hash of a name for a table storing `hash_bits` bits: the Jenkins hash masked to
/// that many bits with its top bit set.
pub fn het_hash(name: &str, hash_bits: u32) -> u64 {
    let and_mask = if hash_bits >= 64 {
        u64::MAX
    } else {
        (1u64 << hash_bits) - 1
    };
    let or_mask = 1u64 << (hash_bits.clamp(1, 64) - 1);
    (jenkins_hash(name) & and_mask) | or_mask
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_string_matches_known_values() {
        assert_eq!(
            hash_string("(listfile)", hash_type::TABLE_OFFSET),
            0x5F3D_E859
        );
        assert_eq!(
            hash_string("(hash table)", hash_type::FILE_KEY),
            HASH_TABLE_KEY
        );
        assert_eq!(
            hash_string("(block table)", hash_type::FILE_KEY),
            BLOCK_TABLE_KEY
        );
        assert_eq!(
            hash_string("path\\to\\file", hash_type::TABLE_OFFSET),
            0x534C_C8EE
        );
        assert_eq!(
            hash_string("file.txt", hash_type::TABLE_OFFSET),
            0x3EA9_8D7A
        );
    }

    #[test]
    fn hash_string_ignores_case_and_slash_direction() {
        assert_eq!(
            hash_string("Interface/Glue/MainMenu.blp", hash_type::NAME_A),
            hash_string("INTERFACE\\GLUE\\MAINMENU.BLP", hash_type::NAME_A)
        );
    }

    #[test]
    fn encryption_round_trips_and_keeps_trailing_bytes() {
        let original: Vec<u8> = (0..23).collect();
        let mut data = original.clone();
        encrypt_bytes(&mut data, 0x1234_5678);
        assert_ne!(data[..20], original[..20]);
        assert_eq!(data[20..], original[20..]);
        decrypt_bytes(&mut data, 0x1234_5678);
        assert_eq!(data, original);
    }

    #[test]
    fn file_key_uses_the_plain_name() {
        assert_eq!(
            file_key("Interface\\Glue\\file.txt", 0, 0, false),
            hash_string("file.txt", hash_type::FILE_KEY)
        );
        let fixed = file_key("file.txt", 0x100, 0x20, true);
        assert_eq!(
            fixed,
            hash_string("file.txt", hash_type::FILE_KEY).wrapping_add(0x100) ^ 0x20
        );
    }

    #[test]
    fn jenkins_hash_matches_the_reference() {
        // hashlittle2("", pc=0, pb=0) is the reference's documented empty-string result.
        assert_eq!(hashlittle2(b"", 0, 0), (0xDEAD_BEEF, 0xDEAD_BEEF));
        // hashlittle2("Four score and seven years ago", 0, 0) from lookup3.c's self test.
        assert_eq!(
            hashlittle2(b"Four score and seven years ago", 0, 0).0,
            0x1777_0551
        );
        assert_eq!(
            hashlittle2(b"Four score and seven years ago", 1, 0).0,
            0xCD62_8161
        );
        assert_eq!(jenkins_hash("A/B"), jenkins_hash("a\\b"));
    }
}
