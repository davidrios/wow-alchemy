# wow-alchemy-mpq

Reader and writer for the MPQ archives of World of Warcraft (format versions 1 to 4).

- `Archive` opens one archive, finds files through the hash and block tables (or HET and BET
  tables), and reads them: sectored or single unit, encrypted, compressed with zlib, bzip2,
  LZMA, PKWARE, Huffman, ADPCM or sparse.
- `PatchChain` reads several archives as the game layers them, applying Cataclysm's `PTCH`
  patches (COPY and BSD0) and honouring delete markers. `PatchChain::open_wotlk_client`
  opens a 3.3.5a client's archives in the client's order.
- `ArchiveBuilder` writes archives of any version, with a `(listfile)` and, optionally, an
  `(attributes)`.

```rust,no_run
use wow_alchemy_mpq::{ArchiveBuilder, PatchChain};

let mut chain = PatchChain::open_wotlk_client("C:/Games/WoW", "enUS")?;
for entry in chain.list()?.iter().take(10) {
    println!("{} ({} bytes)", entry.name, entry.size);
}
let model = chain.read_file("Creature\\Murloc\\Murloc.m2")?;

ArchiveBuilder::new()
    .add_file_data(model, "Creature\\Murloc\\Murloc.m2")
    .build("murloc.mpq")?;
# Ok::<(), wow_alchemy_mpq::Error>(())
```

Tests that need game files read the client at `WOWVR_CLIENT_PATH` and pass without it;
`cargo test -p wow-alchemy-mpq --release -- --ignored` reads every file of the client.

Licensed under Apache-2.0.
