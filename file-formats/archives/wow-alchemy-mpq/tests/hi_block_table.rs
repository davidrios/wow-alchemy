//! Archives over 4 GiB depend on the hi-block table, which holds 16 bits per block.

use wow_alchemy_mpq::{Archive, ArchiveBuilder, FormatVersion};

const HI_BLOCK_POS_OFFSET: usize = 32;
const BLOCK_COUNT_OFFSET: usize = 28;

/// The builder only writes the table when it's needed, which takes a huge
/// archive, so this adds one to a small archive, as the last thing in the file.
#[test]
fn hi_block_table_at_the_end_of_the_file_is_loaded() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.mpq");
    ArchiveBuilder::new()
        .version(FormatVersion::V2)
        .add_file_data(b"content".to_vec(), "file.txt")
        .build(&path)
        .unwrap();

    let mut bytes = std::fs::read(&path).unwrap();
    let block_count = u32::from_le_bytes(
        bytes[BLOCK_COUNT_OFFSET..BLOCK_COUNT_OFFSET + 4]
            .try_into()
            .unwrap(),
    );
    let table_position = bytes.len() as u64;
    bytes[HI_BLOCK_POS_OFFSET..HI_BLOCK_POS_OFFSET + 8]
        .copy_from_slice(&table_position.to_le_bytes());
    bytes.resize(bytes.len() + block_count as usize * 2, 0);
    std::fs::write(&path, bytes).unwrap();

    let mut archive = Archive::open(&path).unwrap();

    assert!(archive.hi_block_table().is_some());
    assert_eq!(archive.read_file("file.txt").unwrap(), b"content");
}
