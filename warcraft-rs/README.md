# warcraft-rs CLI

Unified command-line tool for working with World of Warcraft file formats.

<div align="center">

[![Crates.io Version](https://img.shields.io/crates/v/warcraft-rs)](https://crates.io/crates/warcraft-rs)
[![docs.rs](https://img.shields.io/docsrs/warcraft-rs)](https://docs.rs/warcraft-rs)

</div>

## Installation

### From crates.io

```bash
# Install with all features
cargo install warcraft-rs --features full

# Or install with specific features only
cargo install warcraft-rs --features "mpq dbc"
```

### From Source

```bash
# Clone the repository
git clone https://github.com/davidrios/warcraft-rs
cd warcraft-rs/warcraft-rs

# Build with all features
cargo build --release --features full

# Or build with specific features only
cargo build --release --features "mpq dbc"

# The binary will be at target/release/warcraft-rs
```

### Using Cargo Install (Local)

```bash
# Install with all formats
cargo install --path . --features full

# Or with specific formats only
cargo install --path . --features "mpq blp"
```

## Usage

The `warcraft-rs` CLI provides subcommands for each supported file format:

```bash
warcraft-rs <format> <command> [options]
```

### Available Formats

- `mpq` - MPQ archive operations (implemented)
- `dbc` - DBC database operations (implemented)
- `blp` - BLP texture operations (planned)
- `m2` - M2 model operations (planned)
- `wmo` - WMO object operations (implemented)
- `adt` - ADT terrain operations (implemented)
- `wdt` - WDT map operations (implemented)
- `wdl` - WDL world operations (implemented)

### MPQ Commands

```bash
# List files in an archive
warcraft-rs mpq list archive.mpq
warcraft-rs mpq list archive.mpq --long
warcraft-rs mpq list archive.mpq --filter "*.dbc"

# Extract files
warcraft-rs mpq extract archive.mpq
warcraft-rs mpq extract archive.mpq --output ./extracted
warcraft-rs mpq extract archive.mpq file1.txt file2.dat

# Create a new archive
warcraft-rs mpq create new.mpq --add file1.txt --add file2.dat
warcraft-rs mpq create new.mpq --add *.txt --version v2 --compression zlib

# Show archive information
warcraft-rs mpq info archive.mpq
warcraft-rs mpq info archive.mpq --show-hash-table

# Validate archive integrity
warcraft-rs mpq validate archive.mpq
```

### Global Options

- `-v, --verbose` - Increase verbosity (can be repeated)
- `-q, --quiet` - Suppress all output except errors
- `--help` - Show help for any command

### Shell Completions

Generate shell completions for your shell:

```bash
# Bash
warcraft-rs completions bash > ~/.local/share/bash-completion/completions/warcraft-rs

# Zsh
warcraft-rs completions zsh > ~/.zfunc/_warcraft-rs

# Fish
warcraft-rs completions fish > ~/.config/fish/completions/warcraft-rs.fish

# PowerShell
warcraft-rs completions powershell > _warcraft-rs.ps1
```

## Features

The CLI can be built with different feature flags to include only the formats you
need:

- `default` - Includes MPQ support only
- `full` - Includes all format support
- `mpq` - MPQ archive support
- `dbc` - DBC database support
- `blp` - BLP texture support
- `m2` - M2 model support
- `wmo` - WMO object support
- `adt` - ADT terrain support
- `wdt` - WDT map support
- `wdl` - WDL world support

## Examples

### Working with MPQ Archives

```bash
# Extract all DBC files from an MPQ
warcraft-rs mpq list patch.mpq --filter "*.dbc" | \
  xargs warcraft-rs mpq extract patch.mpq --output ./dbc_files

# Create a new MPQ with compressed files
warcraft-rs mpq create my_mod.mpq \
  --add data/*.txt \
  --add scripts/*.lua \
  --compression zlib \
  --with-listfile

# Validate multiple archives
for mpq in *.mpq; do
  echo "Checking $mpq..."
  warcraft-rs mpq validate "$mpq"
done
```

### Future Format Support

Once implemented, other formats will follow similar patterns:

```bash
# Convert BLP textures to PNG
warcraft-rs blp convert texture.blp --to png

# Export DBC to JSON
warcraft-rs dbc export Items.dbc --format json --output items.json

# Get model information
warcraft-rs m2 info character.m2
```

## Development

See the main [warcraft-rs](https://github.com/davidrios/warcraft-rs) repository
for development information.
