//! M2 model file command implementations

use anyhow::Result;
use clap::Subcommand;
use std::{fs::File, path::PathBuf};
use wow_alchemy_data::types::{VWowStructR, WowStructR};

use wow_alchemy_m2::{M2Model, PhysFile, Skin, skin::SkinVersion};

#[derive(Subcommand)]
pub enum M2Commands {
    /// Display information about an M2 model file
    Info {
        /// Path to the M2 file
        file: PathBuf,

        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },

    /// Display information about a Skin file
    SkinInfo {
        /// Path to the Skin file
        file: PathBuf,

        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,

        /// Parse version
        #[arg(long)]
        version: u32,
    },

    /// Display information about a physics (.phys) file
    PhysInfo {
        /// Path to the Phys file
        file: PathBuf,

        #[arg(short, long)]
        detailed: bool,
    },
}

pub fn execute(cmd: M2Commands) -> Result<()> {
    match cmd {
        M2Commands::Info { file, detailed } => handle_info(file, detailed),
        M2Commands::SkinInfo {
            file,
            detailed,
            version,
        } => handle_skin_info(file, detailed, version),
        M2Commands::PhysInfo { file, detailed } => handle_phys_info(file, detailed),
    }
}

fn handle_info(path: PathBuf, detailed: bool) -> Result<()> {
    println!("Loading M2 model: {}", path.display());

    let mut fp = File::open(path)?;
    let model = M2Model::wow_read(&mut fp)?;

    println!("\n=== M2 Model Information ===");

    println!("File loaded successfully!");

    if detailed {
        println!("\n=== Detailed Information ===");
        println!("{:#?}", model);
    }

    Ok(())
}

fn handle_skin_info(path: PathBuf, detailed: bool, version: u32) -> Result<()> {
    println!("Loading Skin file: {}", path.display());

    let version: SkinVersion = version.try_into()?;

    let mut fp = File::open(path)?;
    let skin = Skin::wow_read(&mut fp, version)?;

    println!("\n=== Skin Information ===");
    println!("File loaded successfully!");

    if detailed {
        println!("\n=== Detailed Information ===");
        println!("{:#?}", skin);
    }

    Ok(())
}

fn handle_phys_info(path: PathBuf, detailed: bool) -> Result<()> {
    println!("Loading Phys file: {}", path.display());

    let mut fp = File::open(path)?;
    let phys = PhysFile::wow_read(&mut fp)?;

    println!("\n=== Phys Information ===");
    println!("File loaded successfully!");

    if detailed {
        println!("\n=== Detailed Information ===");
        println!("{:#?}", phys);
    }

    Ok(())
}
