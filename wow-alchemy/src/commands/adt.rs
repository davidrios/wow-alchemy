use anyhow::Result;
use clap::Subcommand;
use std::{fs::File, path::PathBuf};
use wow_alchemy_adt::{adt_file::AdtRootFile, chunks::mcin::CHUNK_COUNT};
use wow_alchemy_data::{types::WowStructR, utils::magic_to_inverted_string};

#[derive(Subcommand)]
pub enum AdtCommands {
    Info {
        file: PathBuf,

        #[arg(short, long)]
        detailed: bool,
    },
}

pub fn execute(command: AdtCommands) -> Result<()> {
    match command {
        AdtCommands::Info { file, detailed } => execute_info(file, detailed),
    }
}

fn execute_info(path: PathBuf, detailed: bool) -> Result<()> {
    println!("Loading ADT file: {}", path.display());

    let mut fp = File::open(path)?;

    // Load the ADT file
    let adt = AdtRootFile::wow_read(&mut fp)?;

    if detailed {
        println!("\n=== Detailed Information ===");
        println!(
            "chunk indexes: {:#?}",
            adt.chunk_index
                .iter()
                .map(|(ct, ci)| format!("{}: {}", magic_to_inverted_string(ct), ci))
                .collect::<Vec<_>>()
        );
        println!("{adt:#?}");

        if adt.has_map_chunks() {
            for idx in 0..CHUNK_COUNT {
                println!("\n=== Chunk {idx} ===");
                println!("{:#?}", &adt.read_map_chunk(&mut fp, idx as u32)?);
            }
        }
    }

    Ok(())
}
