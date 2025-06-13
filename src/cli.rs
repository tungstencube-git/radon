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
        #[arg(long)]
        flags: Vec<String>,
    },
    Remove {
        package: String,
    },
    Search {
        query: String,
    },
    List,
    Upgrade {
        #[arg(long)]
        all: bool,
        #[arg(long)]
        package: Option<String>,
    },
    BulkInstall {
        packages: Vec<String>,
        #[arg(long)]
        flags: Vec<String>,
    },
    ConvertCargo,
}

