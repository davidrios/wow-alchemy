use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "wow-alchemy")]
#[command(about = "Command-line tools for World of Warcraft file formats", long_about = None)]
#[command(version)]
#[command(author)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[arg(short, long, global = true)]
    pub quiet: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    #[cfg(feature = "dbc")]
    Dbc {
        #[command(subcommand)]
        command: crate::commands::dbc::DbcCommands,
    },

    #[cfg(feature = "blp")]
    Blp {
        #[command(subcommand)]
        command: crate::commands::blp::BlpCommands,
    },

    #[cfg(feature = "m2")]
    M2 {
        #[command(subcommand)]
        command: crate::commands::m2::M2Commands,
    },

    #[cfg(feature = "wmo")]
    Wmo {
        #[command(subcommand)]
        command: crate::commands::wmo::WmoCommands,
    },

    #[cfg(feature = "adt")]
    Adt {
        #[command(subcommand)]
        command: crate::commands::adt::AdtCommands,
    },

    #[cfg(feature = "wdt")]
    Wdt {
        #[command(subcommand)]
        command: crate::commands::wdt::WdtCommands,
    },

    #[cfg(feature = "wdl")]
    Wdl {
        #[command(subcommand)]
        command: crate::commands::wdl::WdlCommands,
    },

    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}
