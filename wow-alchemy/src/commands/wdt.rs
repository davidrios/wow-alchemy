//! WDT command implementations

use anyhow::{Context, Result};
use clap::Subcommand;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use wow_alchemy_data::types::WowStructR;
use wow_alchemy_wdt::{MAP_SIZE, WdtFile};

#[derive(Subcommand)]
pub enum WdtCommands {
    /// Show what a WDT holds
    Info {
        /// Path to the WDT file
        file: PathBuf,

        /// Print every field
        #[arg(short, long)]
        detailed: bool,
    },

    /// Draw the map's tiles, # where a tile has an ADT
    Tiles {
        /// Path to the WDT file
        file: PathBuf,
    },
}

pub fn execute(command: WdtCommands) -> Result<()> {
    match command {
        WdtCommands::Info { file, detailed } => info(&file, detailed),
        WdtCommands::Tiles { file } => tiles(&file),
    }
}

fn load(path: &Path) -> Result<WdtFile> {
    let mut reader = BufReader::new(
        File::open(path).with_context(|| format!("Failed to open {}", path.display()))?,
    );
    WdtFile::wow_read(&mut reader).with_context(|| format!("Failed to read {}", path.display()))
}

fn info(path: &Path, detailed: bool) -> Result<()> {
    let wdt = load(path)?;
    println!("WDT: {}", path.display());
    println!("Version: {}", wdt.version);
    println!("Flags: {:?}", wdt.header.flags);
    println!("Tiles with an ADT: {}", wdt.tile_count());
    if let Some((name, placement)) = wdt.global_wmo() {
        println!("Global WMO: {name}");
        println!(
            "  at ({:.2}, {:.2}, {:.2}), doodad set {}",
            placement.position.x, placement.position.y, placement.position.z, placement.doodad_set
        );
    }
    if wdt.file_ids.is_some() {
        println!("Has file ids (MAID)");
    }
    for (magic, body) in &wdt.unknown {
        println!(
            "Unknown chunk {}: {} bytes",
            String::from_utf8_lossy(magic),
            body.len()
        );
    }
    if detailed {
        println!("\n{wdt:#?}");
    }
    Ok(())
}

fn tiles(path: &Path) -> Result<()> {
    let wdt = load(path)?;
    for y in 0..MAP_SIZE {
        let row: String = (0..MAP_SIZE)
            .map(|x| if wdt.has_tile(x, y) { '#' } else { '.' })
            .collect();
        println!("{row}");
    }
    Ok(())
}
