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
        #[command(subcommand)]
        target: RemoveTarget,
    },
    Search {
        query: String,
    },
    List,
    Upgrade {
        #[command(subcommand)]
        target: UpgradeTarget,
    },
}

#[derive(Debug, clap::Subcommand)]
pub enum RemoveTarget {
    Package { package: String },
    Cache,
}

#[derive(Debug, clap::Subcommand)]
pub enum UpgradeTarget {
    All,
    Package { package: String },
    SelfUpgrade,
}

