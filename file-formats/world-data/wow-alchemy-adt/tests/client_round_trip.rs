//! Reads every ADT of a 3.3.5a client and checks it writes back as the same bytes. The client is
//! the folder holding `Data` in `WOWVR_CLIENT_PATH`; without it the test does nothing. It's slow
//! in debug builds, so run it with `cargo test --release -p wow-alchemy-adt -- --ignored`.

use std::collections::HashSet;
use std::io::Cursor;

use wow_alchemy_adt::{Adt, CHUNKS_PER_TILE};
use wow_alchemy_data::types::{WowStructR, WowStructW};
use wow_alchemy_mpq::PatchChain;

#[test]
#[ignore = "reads the whole client; run with --release -- --ignored"]
fn every_client_adt_writes_back_the_same() {
    let Ok(client) = std::env::var("WOWVR_CLIENT_PATH") else {
        return;
    };
    let mut chain = PatchChain::open_wotlk_client(&client, "enUS").unwrap();
    let mut seen = HashSet::new();
    let names: Vec<String> = chain
        .list()
        .unwrap()
        .into_iter()
        .map(|file| file.name)
        .filter(|name| name.to_ascii_lowercase().ends_with(".adt"))
        // Archives list some files under more than one case.
        .filter(|name| seen.insert(name.to_ascii_lowercase()))
        .collect();

    let (mut empty, mut passed) = (0, 0);
    let mut failures = Vec::new();
    for name in &names {
        let bytes = chain.read_file(name).unwrap();
        let read = Adt::wow_read(&mut Cursor::new(&bytes));
        if bytes.is_empty() {
            // Placeholders for tiles that have none.
            assert!(read.is_err(), "{name} is empty but read");
            empty += 1;
            continue;
        }
        let adt = match read {
            Ok(adt) => adt,
            Err(error) => {
                failures.push(format!("{name}: {error}"));
                continue;
            }
        };
        let mut written = Cursor::new(Vec::new());
        adt.wow_write(&mut written).unwrap();
        if adt.chunks.len() != CHUNKS_PER_TILE {
            failures.push(format!("{name}: {} chunks", adt.chunks.len()));
        } else if written.get_ref() != &bytes {
            failures.push(format!("{name}: written differently"));
        } else {
            passed += 1;
        }
    }

    println!(
        "{} ADTs: {passed} written back the same, {empty} empty, {} failed",
        names.len(),
        failures.len()
    );
    assert!(failures.is_empty(), "{failures:#?}");
    assert!(passed > 0);
}
