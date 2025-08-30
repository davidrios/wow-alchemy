# World of Warcraft File Formats

This directory contains all file format parsing and handling crates for World of
Warcraft, organized by category.

## Directory Structure

```text
file-formats/
├── world-data/    # World and terrain data
│   ├── wow-alchemy-adt    # ADT (Terrain) files
│   ├── wow-alchemy-wdl    # WDL (Low-resolution world maps)
│   └── wow-alchemy-wdt    # WDT (World map definitions)
├── graphics/      # Graphics and model formats
│   ├── wow-alchemy-blp    # BLP (Texture) files
│   ├── wow-alchemy-m2     # M2 (Model) files
│   └── wow-alchemy-wmo    # WMO (World Map Object) files
└── database/      # Game data storage
    └── wow-alchemy-cdbc   # cDBC (Database Client) files
```

## Format Categories

### World Data

- **ADT** - Terrain data files containing height maps, textures, and objects
- **WDL** - Low-resolution world maps used for distant terrain rendering
- **WDT** - World definition tables that define which ADT tiles exist

### Graphics

- **BLP** - Blizzard's proprietary texture format
- **M2** - 3D models for characters, creatures, and objects
- **WMO** - Large world objects like buildings and dungeons

### Database

- **cDBC** - Client-side database files containing game data

## Usage

Each crate can be used independently:

```toml
[dependencies]
wow-alchemy-cdbc = { path = "file-formats/database/wow-alchemy-cdbc" }
wow-alchemy-blp = { path = "file-formats/graphics/wow-alchemy-blp" }
```

## Documentation

See the individual README files in each crate for format-specific documentation
and usage examples.
