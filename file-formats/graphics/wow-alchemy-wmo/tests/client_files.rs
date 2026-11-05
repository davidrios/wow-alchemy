//! Reads every WMO in a 3.3.5a client and writes it back. Runs only with `WOWVR_CLIENT_PATH`
//! set: `cargo test -p wow-alchemy-wmo --release -- --ignored`.

use std::io::Cursor;

use wow_alchemy_data::types::{WowStructR, WowStructW};
use wow_alchemy_wmo::{WmoGroup, WmoRoot};

/// Whether a file name is a group's, `name_NNN.wmo`.
fn is_group(name: &str) -> bool {
    let stem = name.strip_suffix(".wmo").unwrap_or(name).as_bytes();
    stem.len() > 4
        && stem[stem.len() - 4] == b'_'
        && stem[stem.len() - 3..].iter().all(u8::is_ascii_digit)
}

fn round_trip<T: WowStructR + WowStructW>(data: &[u8]) -> Result<bool, String> {
    let file = T::wow_read(&mut Cursor::new(data)).map_err(|e| e.to_string())?;
    let mut written = Cursor::new(Vec::new());
    file.wow_write(&mut written).map_err(|e| e.to_string())?;
    Ok(written.into_inner() == data)
}

#[test]
#[ignore = "reads the whole client; run in release"]
fn every_client_wmo_reads_and_writes_back_the_same() {
    let Ok(path) = std::env::var("WOWVR_CLIENT_PATH") else {
        return;
    };
    let mut chain = wow_alchemy_mpq::PatchChain::open_wotlk_client(path, "enUS").unwrap();
    let names: Vec<String> = chain
        .list()
        .unwrap()
        .into_iter()
        .map(|e| e.name)
        .filter(|n| n.to_ascii_lowercase().ends_with(".wmo"))
        .collect();

    let (mut roots, mut groups, mut placeholders) = (0, 0, 0);
    let mut failures = Vec::new();
    for name in &names {
        let data = chain.read_file(name).unwrap();
        let group = is_group(&name.to_ascii_lowercase());
        let result = if group {
            round_trip::<WmoGroup>(&data)
        } else {
            round_trip::<WmoRoot>(&data)
        };
        match result {
            Ok(true) if group => groups += 1,
            Ok(true) => roots += 1,
            Ok(false) => failures.push(format!("{name}: written back differently")),
            // A few groups are empty placeholders.
            Err(_) if group && data.is_empty() => placeholders += 1,
            Err(e) => failures.push(format!("{name}: {e}")),
        }
    }

    println!("{roots} roots and {groups} groups the same; {placeholders} placeholder groups");
    for failure in &failures {
        println!("{failure}");
    }
    assert!(failures.is_empty(), "{} files failed", failures.len());
    assert!(roots + groups > 0);
}
