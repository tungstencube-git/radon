use ansi_term::Colour::{Green, Red, Yellow};
use clap::{Parser, Subcommand};

mod cli;
mod commands;
mod utils;

use cli::{Cli, Commands};
use commands::{install, remove, search};

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Install { package } => install::install(&package),
        Commands::Remove { package } => remove::remove(&package),
        Commands::Search { query } => search::search(&query),
    }
}
