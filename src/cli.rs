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
        packages: Vec<String>,
        #[arg(long)]
        gitlab: bool,
        #[arg(long)]
        codeberg: bool,
        #[arg(long)]
        local: bool,
        #[arg(short, long)]
        branch: Option<String>,
        #[arg(long)]
        patches: Option<PathBuf>,
        #[arg(long)]
        flags: Vec<String>,
        #[arg(short, long)]
        yes: bool,
    },
    Remove {
        package: String,
    },
    Search {
        query: String,
    },
    List,
    Upgrade {
        package: Option<String>,
        #[arg(short, long)]
        branch: Option<String>,
        #[arg(short, long)]
        yes: bool,
    },
    Convert {
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
}
