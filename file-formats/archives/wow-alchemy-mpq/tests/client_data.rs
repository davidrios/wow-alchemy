//! Reads the player's own 3.3.5a client, from `WOWVR_CLIENT_PATH`. Skips when it's unset.
//!
//! `cargo test -p wow-alchemy-mpq --release -- --ignored` reads every listed file.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use wow_alchemy_data::prelude::*;
use wow_alchemy_mpq::tables::{BlockEntry, HashEntry};
use wow_alchemy_mpq::{Archive, BLOCK_TABLE_KEY, HASH_TABLE_KEY, PatchChain, crypto};

fn client() -> Option<PathBuf> {
    std::env::var_os("WOWVR_CLIENT_PATH").map(PathBuf::from)
}

fn archives(client: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for dir in [client.join("Data"), client.join("Data").join("enUS")] {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("mpq"))
            {
                paths.push(path);
            }
        }
    }
    paths.sort();
    paths
}

fn read_at(path: &Path, offset: u64, len: usize) -> Vec<u8> {
    let mut file = File::open(path).unwrap();
    file.seek(SeekFrom::Start(offset)).unwrap();
    let mut bytes = vec![0u8; len];
    file.read_exact(&mut bytes).unwrap();
    bytes
}

fn read_through_chain(step: usize) {
    let Some(client) = client() else { return };
    let chain = PatchChain::open_wotlk_client(&client, "enUS").unwrap();
    let names = chain.list().unwrap();
    assert!(names.len() > 100_000, "only {} files listed", names.len());
    let mut bytes = 0usize;
    for entry in names.iter().step_by(step) {
        assert!(chain.contains_file(&entry.name), "{}", entry.name);
        let data = chain
            .read_file(&entry.name)
            .unwrap_or_else(|e| panic!("{}: {e}", entry.name));
        assert_eq!(data.len() as u64, entry.size, "{}", entry.name);
        bytes += data.len();
    }
    assert!(bytes > 0);
    assert!(
        chain
            .read_file("Interface\\Glue\\no-such-file.blp")
            .is_err()
    );
}

#[test]
fn chain_reads_a_sample_of_the_client() {
    read_through_chain(97);
}

#[test]
#[ignore = "reads every file of the client; run with --release"]
fn chain_reads_every_file_of_the_client() {
    read_through_chain(1);
}

#[test]
fn each_archive_reserializes_its_header_and_tables() {
    let Some(client) = client() else { return };
    for path in archives(&client) {
        let shown = path.display();
        let archive = Archive::open(&path).unwrap_or_else(|e| panic!("{shown}: {e}"));
        let offset = archive.archive_offset();
        let header = archive.header().clone();

        let mut written = Vec::new();
        header.wow_write(&mut written).unwrap();
        assert_eq!(
            written,
            read_at(&path, offset, written.len()),
            "{shown}: header"
        );

        let file_size = std::fs::metadata(&path).unwrap().len();
        let hash = archive.hash_table().unwrap();
        let hash_pos = offset + header.hash_table_pos();
        let hash_len = hash.entries.len() * HashEntry::SIZE;
        if hash_pos + hash_len as u64 <= file_size {
            let mut bytes = hash.to_bytes().unwrap();
            crypto::encrypt_bytes(&mut bytes, HASH_TABLE_KEY);
            assert_eq!(
                bytes,
                read_at(&path, hash_pos, hash_len),
                "{shown}: hash table"
            );
        }
        let blocks = archive.block_table().unwrap();
        let block_pos = offset + header.block_table_pos();
        let block_len = blocks.entries.len() * BlockEntry::SIZE;
        if block_pos + block_len as u64 <= file_size {
            let mut bytes = blocks.to_bytes().unwrap();
            crypto::encrypt_bytes(&mut bytes, BLOCK_TABLE_KEY);
            assert_eq!(
                bytes,
                read_at(&path, block_pos, block_len),
                "{shown}: block table"
            );
        }
        if let (Some(hi), Some(pos)) = (archive.hi_block_table(), header.hi_block_table_pos()) {
            let bytes = hi.to_bytes().unwrap();
            assert_eq!(
                bytes,
                read_at(&path, offset + pos, bytes.len()),
                "{shown}: hi-block"
            );
        }

        if let Some(attributes) = archive.attributes().unwrap() {
            let stored = archive.read_file("(attributes)").unwrap();
            assert_eq!(
                attributes.to_bytes().unwrap(),
                stored,
                "{shown}: (attributes)"
            );
        }
        assert!(!archive.list().unwrap().is_empty(), "{shown}: listfile");
    }
}
