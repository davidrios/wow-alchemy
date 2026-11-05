//! WDL command implementations

use anyhow::{Context, Result};
use clap::Subcommand;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use wow_alchemy_data::types::WowStructR;
use wow_alchemy_wdl::{MAP_SIZE, WdlFile};

#[derive(Subcommand)]
pub enum WdlCommands {
    /// Show what a WDL holds
    Info {
        /// Path to the WDL file
        file: PathBuf,

        /// Print every field
        #[arg(short, long)]
        detailed: bool,
    },

    /// Draw the map's tiles, # where a tile has low-resolution heights
    Tiles {
        /// Path to the WDL file
        file: PathBuf,
    },
}

pub fn execute(command: WdlCommands) -> Result<()> {
    match command {
        WdlCommands::Info { file, detailed } => info(&file, detailed),
        WdlCommands::Tiles { file } => tiles(&file),
    }
}

fn load(path: &Path) -> Result<WdlFile> {
    let mut reader = BufReader::new(
        File::open(path).with_context(|| format!("Failed to open {}", path.display()))?,
    );
    WdlFile::wow_read(&mut reader).with_context(|| format!("Failed to read {}", path.display()))
}

fn info(path: &Path, detailed: bool) -> Result<()> {
    let wdl = load(path)?;
    println!("WDL: {}", path.display());
    println!("Version: {}", wdl.version);
    println!("Tiles: {}", wdl.tile_count());
    if let Some((min, max)) = wdl.height_range() {
        println!("Heights: {min} to {max}");
    }
    let with_holes = wdl
        .tiles
        .iter()
        .flatten()
        .filter(|t| t.holes.is_some())
        .count();
    println!(
        "Tiles with hole masks: {with_holes}, {} holes",
        wdl.hole_count()
    );
    if let Some(placements) = &wdl.wmo_placements {
        println!("WMO placements: {}", placements.len());
        for name in wdl.wmo_name_list() {
            println!("  {name}");
        }
    }
    for (magic, body) in &wdl.unknown {
        println!(
            "Unknown chunk {}: {} bytes",
            String::from_utf8_lossy(magic),
            body.len()
        );
    }
    if detailed {
        println!("\n{wdl:#?}");
    }
    Ok(())
}

fn tiles(path: &Path) -> Result<()> {
    let wdl = load(path)?;
    for y in 0..MAP_SIZE {
        let row: String = (0..MAP_SIZE)
            .map(|x| if wdl.tile(x, y).is_some() { '#' } else { '.' })
            .collect();
        println!("{row}");
    }
    Ok(())
}
