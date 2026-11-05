//! Archives written by `ArchiveBuilder` read back through `Archive`.

use wow_alchemy_mpq::compression::flags;
use wow_alchemy_mpq::{Archive, ArchiveBuilder, BlockFlags, FileOptions, FormatVersion};

fn sample(len: usize) -> Vec<u8> {
    (0..len)
        .map(|i| (i * 7 % 251) as u8 ^ (i / 300) as u8)
        .collect()
}

fn files() -> Vec<(&'static str, Vec<u8>, FileOptions)> {
    let plain = FileOptions::default();
    vec![
        ("empty.txt", Vec::new(), plain),
        ("small.txt", b"hello".to_vec(), plain),
        ("Dir\\sectored.bin", sample(20_000), plain),
        (
            "Dir\\stored.bin",
            sample(9_000),
            FileOptions {
                compression: None,
                ..plain
            },
        ),
        (
            "Dir\\Sub\\encrypted.bin",
            sample(10_001),
            FileOptions {
                encrypt: true,
                ..plain
            },
        ),
        (
            "fixkey.bin",
            sample(5_003),
            FileOptions {
                encrypt: true,
                fix_key: true,
                compression: Some(flags::BZIP2),
                ..plain
            },
        ),
        (
            "single.bin",
            sample(7_777),
            FileOptions {
                single_unit: true,
                encrypt: true,
                ..plain
            },
        ),
        (
            "stored-encrypted.bin",
            sample(6_002),
            FileOptions {
                compression: None,
                encrypt: true,
                ..plain
            },
        ),
    ]
}

fn build(version: FormatVersion) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.mpq");
    let mut builder = ArchiveBuilder::new().version(version).attributes(true);
    for (name, data, options) in files() {
        builder = builder.add_file_data_with_options(data, name, options);
    }
    builder.build(&path).unwrap();
    (dir, path)
}

#[test]
fn every_version_reads_back() {
    for version in [
        FormatVersion::V1,
        FormatVersion::V2,
        FormatVersion::V3,
        FormatVersion::V4,
    ] {
        let (_dir, path) = build(version);
        let mut archive = Archive::open(&path).unwrap();
        assert_eq!(archive.format_version(), version);
        for (name, data, _) in files() {
            assert_eq!(archive.read_file(name).unwrap(), data, "{version:?} {name}");
            assert_eq!(
                archive
                    .read_file(&name.to_lowercase().replace('\\', "/"))
                    .unwrap(),
                data
            );
        }
        let names: Vec<_> = archive
            .list()
            .unwrap()
            .into_iter()
            .map(|e| e.name)
            .collect();
        let expected: Vec<_> = files().iter().map(|f| f.0.to_string()).collect();
        assert_eq!(names, expected);
        assert!(!archive.contains_file("missing.txt"));

        let attributes = archive.attributes().unwrap().unwrap();
        let block = archive.find_file("Dir\\sectored.bin").unwrap().block_index;
        assert_eq!(attributes.crc32[block], crc32fast::hash(&sample(20_000)));

        if version >= FormatVersion::V3 {
            assert!(archive.het_table().is_some() && archive.bet_table().is_some());
        }
    }
}

#[test]
fn flags_follow_the_options() {
    let (_dir, path) = build(FormatVersion::V2);
    let archive = Archive::open(&path).unwrap();
    let flags = |name| archive.find_file(name).unwrap().flags;
    assert_eq!(flags("empty.txt"), BlockFlags::EXISTS);
    assert!(flags("single.bin").contains(BlockFlags::SINGLE_UNIT | BlockFlags::ENCRYPTED));
    assert!(flags("fixkey.bin").contains(BlockFlags::FIX_KEY | BlockFlags::COMPRESS));
    assert!(!flags("Dir\\stored.bin").is_compressed());
}

#[test]
fn v4_header_md5_covers_the_header() {
    use md5::{Digest, Md5};
    let (_dir, path) = build(FormatVersion::V4);
    let bytes = std::fs::read(&path).unwrap();
    let digest: [u8; 16] = Md5::digest(&bytes[..0xC0]).into();
    assert_eq!(&bytes[0xC0..0xD0], &digest);
    let archive = Archive::open(&path).unwrap();
    let v4 = archive.header().v4().unwrap();
    let pos = archive.header().hash_table_pos() as usize;
    let hash: [u8; 16] = Md5::digest(&bytes[pos..pos + v4.hash_table_size_64 as usize]).into();
    assert_eq!(v4.md5_hash_table, hash);
}
