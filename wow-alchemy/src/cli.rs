//! Root CLI structure for wow-alchemy

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "wow-alchemy")]
#[command(about = "Command-line tools for World of Warcraft file formats", long_about = None)]
#[command(version)]
#[command(author)]
pub struct Cli {
    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Commands,

    /// Verbosity level (can be repeated for more detail)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Suppress all output except errors
    #[arg(short, long, global = true)]
    pub quiet: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// DBC database operations
    #[cfg(feature = "dbc")]
    Dbc {
        #[command(subcommand)]
        command: crate::commands::dbc::DbcCommands,
    },

    /// DBD (Database Definition) operations
    #[cfg(feature = "dbc")]
    Dbd {
        #[command(subcommand)]
        command: crate::commands::dbd::DbdCommand,
    },

    /// BLP texture operations
    #[cfg(feature = "blp")]
    Blp {
        #[command(subcommand)]
        command: crate::commands::blp::BlpCommands,
    },

    /// M2 model operations
    #[cfg(feature = "m2")]
    M2 {
        #[command(subcommand)]
        command: crate::commands::m2::M2Commands,
    },

    /// WMO object operations
    #[cfg(feature = "wmo")]
    Wmo {
        #[command(subcommand)]
        command: crate::commands::wmo::WmoCommands,
    },

    /// ADT terrain operations
    #[cfg(feature = "adt")]
    Adt {
        #[command(subcommand)]
        command: crate::commands::adt::AdtCommands,
    },

    /// WDT map operations
    #[cfg(feature = "wdt")]
    Wdt {
        #[command(subcommand)]
        command: crate::commands::wdt::WdtCommands,
    },

    /// WDL world operations
    #[cfg(feature = "wdl")]
    Wdl {
        #[command(subcommand)]
        command: crate::commands::wdl::WdlCommands,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}
