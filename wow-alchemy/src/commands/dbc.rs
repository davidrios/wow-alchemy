use anyhow::Result;
use clap::Subcommand;
#[cfg(feature = "sqlite")]
use std::path::{Path, PathBuf};

#[cfg(feature = "sqlite")]
use wow_alchemy_cdbc::sqlite_converter::convert_to_sqlite;

#[derive(Subcommand)]
pub enum DbcCommands {
    /// Convert all dbc files in a folder to a SQLite database
    #[cfg(feature = "sqlite")]
    Convert {
        /// A build in the format x.x.x.xxxx, for example 3.3.5.12340
        game_build: String,
        source_dir: PathBuf,
        output_sqlite: PathBuf,
    },
}

pub fn execute(command: DbcCommands) -> Result<()> {
    match command {
        #[cfg(feature = "sqlite")]
        DbcCommands::Convert {
            game_build,
            source_dir,
            output_sqlite,
        } => convert_command(&game_build, &source_dir, &output_sqlite),
    }
}

#[cfg(feature = "sqlite")]
fn convert_command(game_build: &str, source_dir: &Path, output_sqlite: &Path) -> Result<()> {
    convert_to_sqlite(game_build.try_into()?, source_dir, output_sqlite)?;
    Ok(())
}
