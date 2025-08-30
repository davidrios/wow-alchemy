# wow-alchemy CLI

Unified command-line tool for working with World of Warcraft file formats.

<div align="center">

[![Crates.io Version](https://img.shields.io/crates/v/wow-alchemy)](https://crates.io/crates/wow-alchemy)
[![docs.rs](https://img.shields.io/docsrs/wow-alchemy)](https://docs.rs/wow-alchemy)

</div>

## Installation

### From crates.io

```bash
# Install with all features
cargo install wow-alchemy --features full

# Or install with specific features only
cargo install wow-alchemy --features "dbc"
```

### From Source

```bash
# Clone the repository
git clone https://github.com/davidrios/wow-alchemy
cd wow-alchemy/wow-alchemy

# Build with all features
cargo build --release --features full

# Or build with specific features only
cargo build --release --features "dbc"

# The binary will be at target/release/wow-alchemy
```

### Using Cargo Install (Local)

```bash
# Install with all formats
cargo install --path . --features full

# Or with specific formats only
cargo install --path . --features "blp"
```

## Usage

The `wow-alchemy` CLI provides subcommands for each supported file format:

```bash
wow-alchemy <format> <command> [options]
```

### Available Formats

- `dbc`
- `blp`
- `m2`
- `wmo`
- `adt`
- `wdt`
- `wdl`

### Global Options

- `-v, --verbose` - Increase verbosity (can be repeated)
- `-q, --quiet` - Suppress all output except errors
- `--help` - Show help for any command

### Shell Completions

Generate shell completions for your shell:

```bash
# Bash
wow-alchemy completions bash > ~/.local/share/bash-completion/completions/wow-alchemy

# Zsh
wow-alchemy completions zsh > ~/.zfunc/_wow-alchemy

# Fish
wow-alchemy completions fish > ~/.config/fish/completions/wow-alchemy.fish

# PowerShell
wow-alchemy completions powershell > _wow-alchemy.ps1
```

## Features

The CLI can be built with different feature flags to include only the formats you
need:

- `full` - Includes all format support
- `dbc` - DBC database support
- `blp` - BLP texture support
- `m2` - M2 model support
- `wmo` - WMO object support
- `adt` - ADT terrain support
- `wdt` - WDT map support
- `wdl` - WDL world support


## Development

See the main [wow-alchemy](https://github.com/davidrios/wow-alchemy) repository
for development information.
