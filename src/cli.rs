use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "radon", version, author, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Install { package: String },
    Remove { package: String },
    Search { query: String },
}
