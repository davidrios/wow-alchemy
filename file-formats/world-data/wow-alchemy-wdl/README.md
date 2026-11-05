# wow-alchemy-wdl

Reader and writer for World of Warcraft WDL files: a map's low-resolution heights, 17 by 17
outer and 16 by 16 inner per tile, and from Wrath of the Lich King on, each tile's holes. They
draw the distant terrain past the loaded ADTs.

```rust,no_run
use wow_alchemy_data::types::WowStructR;
use wow_alchemy_wdl::WdlFile;

let mut file = std::fs::File::open("Azeroth.wdl")?;
let wdl = WdlFile::wow_read(&mut file)?;
if let Some(tile) = wdl.tile(32, 48) {
    println!("{:?} to {:?}", tile.heights.min(), tile.heights.max());
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

Writing a file back gives the same bytes for every WDL in a 3.3.5a client. MWMO, MWID and MODF
are read too; chunks of later versions, such as Legion's ML* chunks, are kept as they were.

## License

Licensed under the Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>).
