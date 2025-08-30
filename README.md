# warcraft-rs

A Rust library and CLI toolset for parsing, manipulating, and
creating World of Warcraft binary files.


## Quick Start

### Command-Line Usage

Todo...

### Library Usage

Todo...


## Installation

### Binary Releases

Pre-built binaries are available for Linux, macOS, and Windows:

```bash
# Unix/Linux/macOS
curl -fsSL https://raw.githubusercontent.com/wowemulation-dev/warcraft-rs/main/install.sh | bash

# Windows PowerShell
irm https://raw.githubusercontent.com/wowemulation-dev/warcraft-rs/main/install.ps1 | iex
```

### From crates.io

```bash
# Install the CLI tool
cargo install warcraft-rs

# Add specific crates as dependencies
cargo add wow-mpq wow-blp wow-adt
```

### Build from source

```bash
git clone https://github.com/wowemulation-dev/warcraft-rs
cd warcraft-rs
cargo install --path warcraft-rs
```

## License

This project is licensed under the Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)


### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.

---

*This project represents the collective knowledge of the WoW modding community
and is based on reverse engineering efforts. Blizzard Entertainment has not
officially documented any formats handled by this project.*
