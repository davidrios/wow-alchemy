//! Reads every table of a 3.3.5a client, from `WOWVR_CLIENT_PATH`. Skips when it's unset.

use wow_alchemy_cdbc::{DbcVersion, WdbFile};

#[test]
fn every_client_dbc_opens_and_writes_back_the_same() {
    let Ok(client) = std::env::var("WOWVR_CLIENT_PATH") else {
        return;
    };
    let mut chain = wow_alchemy_mpq::PatchChain::open_wotlk_client(&client, "enUS").unwrap();
    let mut names: Vec<String> = chain
        .list()
        .unwrap()
        .into_iter()
        .map(|f| f.name)
        .filter(|n| n.to_ascii_lowercase().ends_with(".dbc"))
        .collect();
    names.sort();
    assert!(!names.is_empty());

    let mut failures = Vec::new();
    let (mut records, mut strings) = (0u64, 0u64);
    for name in &names {
        // patch-enUS.MPQ deletes CharVariations.dbc with an empty delete-marker entry (flag
        // 0x02000000), which the chain must treat as the file being gone, not as an empty file.
        let data = chain.read_file(name).unwrap();
        let file = match WdbFile::from_bytes(&data) {
            Ok(file) => file,
            Err(err) => {
                failures.push(format!("{name}: {err}"));
                continue;
            }
        };
        assert_eq!(file.version, DbcVersion::WDBC, "{name}");
        assert_eq!(
            file.to_bytes().unwrap(),
            data,
            "{name} writes back different bytes"
        );
        records += file.record_count() as u64;
        strings += file.header.string_block_size as u64;
    }
    println!(
        "{} tables, {} failed, {records} records, {strings} bytes of strings",
        names.len(),
        failures.len()
    );
    assert!(failures.is_empty(), "{failures:#?}");
}
