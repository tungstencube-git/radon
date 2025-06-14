mod cli;
mod utils;
mod commands;

use clap::Parser;
use cli::{Cli, Commands};
use commands::{install, remove, search, list, upgrade};
use commands::convert::convert;
use std::path::Path;

fn main() {
    utils::setup_radon_dirs();
    let cli = Cli::parse();
    match cli.command {
        Commands::Install { packages, gitlab, codeberg, local, branch, patches, flags, yes } => {
            install::install(
                &packages, 
                gitlab, 
                codeberg, 
                local, 
                branch.as_deref(), 
                patches.as_deref(), 
                &flags,
                yes
            );
        },
        Commands::Remove { package } => remove::remove(&package),
        Commands::Search { query } => search::search(&query),
        Commands::List => list::list(),
        Commands::Upgrade { package, branch, yes } => 
            upgrade::upgrade(package.as_deref(), branch.as_deref(), yes),
        Commands::Convert { file } => convert(file.as_deref().map(Path::new)),
    }
}
