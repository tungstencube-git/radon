mod cli;
mod utils;
mod commands;

use clap::Parser;
use cli::{Cli, Commands};
use commands::{install, remove, search, list, upgrade};

fn main() {
    utils::setup_radon_dirs();
    let cli = Cli::parse();
    match cli.command {
        Commands::Install { package, gitlab, codeberg, local, branch, patches } => {
            let source = if codeberg {
                Some("codeberg")
            } else if gitlab {
                Some("gitlab")
            } else {
                None
            };
            install::install(&package, source, local, branch.as_deref(), patches.as_deref());
        },
        Commands::Remove { package } => remove::remove(&package),
        Commands::Search { query } => {
            search::search(&query);
        },
        Commands::List => list::list(),
        Commands::Upgrade { all, package } => upgrade::upgrade(all, package.as_deref()),
    }
}
