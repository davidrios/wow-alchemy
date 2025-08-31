mod cli;
mod commands;
mod utils;

use anyhow::Result;
use clap::CommandFactory;
use clap::Parser;
use clap_complete::{Generator, generate};
use std::io;

use crate::cli::{Cli, Commands};

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let cli = Cli::parse();

    if cli.verbose > 0 {
        log::set_max_level(match cli.verbose {
            1 => log::LevelFilter::Info,
            2 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        });
    } else if cli.quiet {
        log::set_max_level(log::LevelFilter::Error);
    }

    match cli.command {
        #[cfg(feature = "dbc")]
        Commands::Dbc { command } => commands::dbc::execute(command),

        #[cfg(feature = "blp")]
        Commands::Blp { command } => commands::blp::execute(command),

        #[cfg(feature = "m2")]
        Commands::M2 { command } => commands::m2::execute(command),

        #[cfg(feature = "wmo")]
        Commands::Wmo { command } => commands::wmo::execute(command),

        #[cfg(feature = "adt")]
        Commands::Adt { command } => commands::adt::execute(command),

        #[cfg(feature = "wdt")]
        Commands::Wdt { command } => commands::wdt::execute(command),

        #[cfg(feature = "wdl")]
        Commands::Wdl { command } => commands::wdl::execute(command),

        Commands::Completions { shell } => {
            print_completions(shell, &mut Cli::command());
            Ok(())
        }
    }
}

fn print_completions<G: Generator>(generator: G, cmd: &mut clap::Command) {
    generate(
        generator,
        cmd,
        cmd.get_name().to_string(),
        &mut io::stdout(),
    );
}
