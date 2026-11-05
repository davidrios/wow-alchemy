//! Every WDL of a 3.3.5a client, from `WOWVR_CLIENT_PATH`, reads and writes back byte for byte.
//! Skipped when the variable isn't set.

use std::io::Cursor;

use wow_alchemy_data::types::{WowStructR, WowStructW};
use wow_alchemy_mpq::PatchChain;
use wow_alchemy_wdl::WdlFile;

#[test]
fn every_client_wdl_round_trips() {
    let Ok(client) = std::env::var("WOWVR_CLIENT_PATH") else {
        return;
    };
    let mut chain = PatchChain::open_wotlk_client(&client, "enUS").expect("open the client");
    let names: Vec<String> = chain
        .list()
        .expect("list the client's files")
        .into_iter()
        .map(|entry| entry.name)
        .filter(|name| name.to_ascii_lowercase().ends_with(".wdl"))
        .collect();
    assert!(!names.is_empty(), "the client has no WDLs");

    let (mut tiles, mut with_holes, mut holes, mut placements, mut unknown) = (0, 0, 0, 0, 0);
    let mut failures = Vec::new();
    for name in &names {
        let bytes = chain.read_file(name).expect("read a WDL");
        let wdl = match WdlFile::wow_read(&mut Cursor::new(&bytes)) {
            Ok(wdl) => wdl,
            Err(error) => {
                failures.push(format!("{name}: {error}"));
                continue;
            }
        };
        let mut written = Cursor::new(Vec::new());
        wdl.wow_write(&mut written).expect("write a WDL");
        if written.get_ref() != &bytes {
            failures.push(format!("{name}: written back differently"));
        }
        tiles += wdl.tile_count();
        with_holes += wdl
            .tiles
            .iter()
            .flatten()
            .filter(|t| t.holes.is_some())
            .count();
        holes += wdl.hole_count();
        placements += wdl.wmo_placements.as_ref().map_or(0, Vec::len);
        unknown += wdl.unknown.len();
    }
    println!(
        "{} WDLs, {tiles} tiles, {with_holes} with MAHO, {holes} holes, {placements} WMO placements, {unknown} unknown chunks",
        names.len()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
