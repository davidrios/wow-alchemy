//! ADT command implementations

use anyhow::{Context, Result};
use clap::Subcommand;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use wow_alchemy_adt::Adt;
use wow_alchemy_data::types::WowStructR;

#[derive(Subcommand)]
pub enum AdtCommands {
    /// Show what an ADT terrain tile holds
    Info {
        /// Path to the ADT file
        file: PathBuf,

        /// Print every field
        #[arg(short, long)]
        detailed: bool,
    },
}

pub fn execute(command: AdtCommands) -> Result<()> {
    match command {
        AdtCommands::Info { file, detailed } => info(&file, detailed),
    }
}

fn info(path: &Path, detailed: bool) -> Result<()> {
    let data = std::fs::read(path).with_context(|| format!("Failed to open {}", path.display()))?;
    let adt = Adt::wow_read(&mut Cursor::new(&data))
        .with_context(|| format!("Failed to read {}", path.display()))?;

    println!("ADT: {}", path.display());
    println!("Version: {}", adt.version);
    println!("Map chunks: {}", adt.chunks.len());
    let (min, max) = adt
        .chunks
        .iter()
        .flat_map(|c| c.heights.iter().map(move |h| h + c.header.position.z))
        .fold((f32::MAX, f32::MIN), |(lo, hi), h| (lo.min(h), hi.max(h)));
    if min <= max {
        println!("Heights: {min:.2} to {max:.2}");
    }
    println!("Textures: {}", adt.textures.names().len());
    println!(
        "Models: {}, placed {} times",
        adt.models.names().len(),
        adt.doodads.len()
    );
    println!(
        "WMOs: {}, placed {} times",
        adt.wmos.names().len(),
        adt.wmo_placements.len()
    );
    let liquids = adt.liquid_layers()?;
    let old_liquids = adt.chunks.iter().filter(|c| c.liquid.is_some()).count();
    println!("Liquid layers (MH2O): {}", liquids.len());
    println!("Chunks with MCLQ liquid: {old_liquids}");
    for name in adt.textures.names() {
        println!("  texture {name}");
    }
    if detailed {
        println!("\n{adt:#?}");
    }
    Ok(())
}
