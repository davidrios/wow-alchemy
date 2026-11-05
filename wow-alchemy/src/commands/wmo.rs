//! WMO command implementations

use anyhow::{Context, Result};
use clap::Subcommand;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use wow_alchemy_data::types::WowStructR;
use wow_alchemy_wmo::{MOGP, WmoGroup, WmoRoot};

#[derive(Subcommand)]
pub enum WmoCommands {
    /// Show what a WMO root or group file holds
    Info {
        /// Path to the WMO file; group files are named <root>_NNN.wmo
        file: PathBuf,

        /// Print every field
        #[arg(short, long)]
        detailed: bool,
    },
}

pub fn execute(command: WmoCommands) -> Result<()> {
    match command {
        WmoCommands::Info { file, detailed } => info(&file, detailed),
    }
}

fn info(path: &Path, detailed: bool) -> Result<()> {
    let data = std::fs::read(path).with_context(|| format!("Failed to open {}", path.display()))?;
    // Both kinds start with MVER; a group's next chunk is MOGP.
    let is_group = data.get(12..16) == Some(&MOGP[..]);
    let mut reader = Cursor::new(&data);
    if is_group {
        let group = WmoGroup::wow_read(&mut reader)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        println!("WMO group: {}", path.display());
        println!("Flags: {:?}", group.header.flags);
        println!("Vertices: {}", group.positions.len());
        println!("Triangles: {}", group.indices.len() / 3);
        println!("Batches: {}", group.batches.len());
        println!("Texture coordinate sets: {}", group.tex_coords.len());
        println!("Vertex color sets: {}", group.colors.len());
        println!("Doodads: {}", group.doodad_refs().len());
        println!(
            "Liquid: {}",
            if group.liquid.is_some() { "yes" } else { "no" }
        );
        if detailed {
            println!("\n{group:#?}");
        }
    } else {
        let root = WmoRoot::wow_read(&mut reader)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        println!("WMO root: {}", path.display());
        println!("Flags: {:?}", root.header.flags);
        println!("Groups: {}", root.header.group_count);
        println!("Materials: {}", root.materials.len());
        println!("Portals: {}", root.portals.len());
        println!("Lights: {}", root.lights.len());
        println!("Doodad sets: {}", root.doodad_sets.len());
        println!("Doodads: {}", root.doodads.len());
        for (_, name) in root.textures.strings() {
            println!("  texture {name}");
        }
        if detailed {
            println!("\n{root:#?}");
        }
    }
    Ok(())
}
