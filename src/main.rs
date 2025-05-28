use clap::Parser;

mod cli;
mod commands;
mod utils;

use cli::{Cli, Commands};
use commands::{install, remove, search};

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Install { package, gitlab, codeberg, local, branch } => {
            let source = if codeberg {
                Some("codeberg")
            } else if gitlab {
                Some("gitlab")
            } else {
                None
            };
            install::install(&package, source, local, branch.as_deref());
        },
        Commands::Remove { package } => remove::remove(&package),
        Commands::Search { query } => search::search(&query),
    }
}
