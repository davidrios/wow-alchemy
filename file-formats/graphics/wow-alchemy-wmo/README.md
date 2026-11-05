# wow-alchemy-wmo

Reads and writes World of Warcraft WMO (world map object) files of WotLK 3.3.5a, version 17.

- `WmoRoot`: a root file, with every chunk as records (MOHD, MOTX, MOMT, MOGN, MOGI, MOSB, MOPV,
  MOPT, MOPR, MOVV, MOVB, MOLT, MODS, MODN, MODD, MFOG, MCVP).
- `WmoGroup`: a group file, `_NNN.wmo` (MOGP's header, MOPY, MOVI, MOVT, MONR, MOTV, MOBA, MOLR,
  MODR, MOBN, MOBR, MOCV, MLIQ), with helpers for drawing it: collision triangles, vertex
  colors as RGBA, and the liquid's grid and LiquidType.dbc row as the client works them out.

Both implement `WowStructR` and `WowStructW` from `wow-alchemy-data`; a client file read and
written back gives the same bytes. Chunks they don't know are kept as they were.

```rust
use std::io::Cursor;
use wow_alchemy_data::types::WowStructR;
use wow_alchemy_wmo::{WmoGroup, WmoRoot};

fn read(root: &[u8], group: &[u8]) -> wow_alchemy_data::error::Result<()> {
    let root = WmoRoot::wow_read(&mut Cursor::new(root))?;
    let group = WmoGroup::wow_read(&mut Cursor::new(group))?;
    println!("{} groups, {} triangles", root.header.group_count, group.triangles.len());
    Ok(())
}
```

`cargo test -p wow-alchemy-wmo --release -- --ignored` with `WOWVR_CLIENT_PATH` set to a 3.3.5a
client reads and writes back every WMO in it.

## License

This project is licensed under the Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
