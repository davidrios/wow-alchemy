//! Reads every BLP of a 3.3.5a client, from `WOWVR_CLIENT_PATH`, and writes each back. Skipped
//! without it. Slow in debug builds: `cargo test -p wow-alchemy-blp --release -- --ignored`.

use std::collections::BTreeMap;
use std::io::Cursor;

use wow_alchemy_blp::{BlpImage, load_blp_from_buf};
use wow_alchemy_data::types::WowStructW;

#[test]
#[ignore]
fn every_client_blp_reads_and_writes_back_the_same() {
    let Ok(client) = std::env::var("WOWVR_CLIENT_PATH") else {
        return;
    };
    let mut chain = wow_alchemy_mpq::PatchChain::open_wotlk_client(&client, "enUS").unwrap();
    let names: Vec<String> = chain
        .list()
        .unwrap()
        .into_iter()
        .map(|entry| entry.name)
        .filter(|name| name.to_ascii_lowercase().ends_with(".blp"))
        .collect();

    let mut formats = BTreeMap::<String, usize>::new();
    let (mut too_short, mut failed, mut different) = (0, Vec::new(), Vec::new());
    for name in &names {
        let bytes = chain.read_file(name).unwrap();
        let blp = match load_blp_from_buf(&bytes) {
            Ok(blp) => blp,
            // Files too short to have a magic and header are expected to fail.
            Err(_) if bytes.len() < 148 => {
                too_short += 1;
                continue;
            }
            Err(error) => {
                failed.push(format!("{name}: {error}"));
                continue;
            }
        };
        let format = blp
            .format()
            .map_or_else(|e| e.to_string(), |f| format!("{f:?}"));
        *formats.entry(format).or_default() += 1;
        if written(&blp) != bytes {
            different.push(name.clone());
        }
    }

    println!("{} BLPs, {too_short} too short to read", names.len());
    for (format, count) in &formats {
        println!("{count:8} {format}");
    }
    assert!(
        failed.is_empty(),
        "{} failed: {:#?}",
        failed.len(),
        &failed[..failed.len().min(20)]
    );
    assert!(
        different.is_empty(),
        "{} written differently: {:#?}",
        different.len(),
        &different[..different.len().min(20)]
    );
    assert!(names.len() > 100_000, "only {} BLPs", names.len());
}

fn written(blp: &BlpImage) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    blp.wow_write(&mut out).unwrap();
    out.into_inner()
}
