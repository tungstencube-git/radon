use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, clap::Subcommand)]
pub enum Commands {
    Install {
        package: String,
        #[arg(long)]
        gitlab: bool,
        #[arg(long)]
        codeberg: bool,
        #[arg(long)]
        local: bool,
        #[arg(long)]
        branch: Option<String>,
        #[arg(long)]
        patches: Option<PathBuf>,
    },
    Remove {
        package: String,
    },
    Search {
        query: String,
    },
}
