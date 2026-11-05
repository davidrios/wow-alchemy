# wow-alchemy CLI

Command-line tool for World of Warcraft file formats, built on the wow-alchemy crates.

## Installation

```bash
git clone https://github.com/davidrios/wow-alchemy
cd wow-alchemy/wow-alchemy
cargo install --path .
# or only some formats:
cargo install --path . --no-default-features --features "blp,adt"
```

## Usage

```bash
wow-alchemy <format> <command> [options]
```

Every `info` command prints a summary, and `--detailed` prints every field read.

| Command | What it does |
|---|---|
| `adt info <file>` | An ADT terrain tile: chunks, heights, textures, models, liquids |
| `wdt info <file>`, `wdt tiles <file>` | A map's flags and global WMO; a 64x64 drawing of its tiles |
| `wdl info <file>`, `wdl tiles <file>` | A map's low-resolution heights and holes |
| `wmo info <file>` | A WMO root or group file (told apart by its chunks) |
| `m2 info <file>`, `m2 skin-info <file> --version N`, `m2 phys-info <file>` | M2 models, their skins and physics |
| `blp info <file>` | A BLP texture's header and mipmaps |
| `blp convert <in> <out>` | BLP to PNG/TGA/... (`--mipmap-level`), or an image to BLP (`--blp-version`, `--format`, `--alpha-bits`, `--no-mipmaps`) |
| `dbc convert <build> <folder> <out.sqlite>` | Every DBC in a folder to SQLite, using WoWDBDefs definitions for that build, such as 3.3.5.12340 |
| `completions <shell>` | Shell completions |

## License

Apache-2.0
