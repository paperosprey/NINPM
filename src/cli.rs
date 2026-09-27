use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "ninpm",
    version,
    about = "NINPM Is Not a Package Manager -- a thin, safe helper around environment.systemPackages in configuration.nix"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    #[arg(long, global = true)]
    pub give_me_details: bool,

    #[arg(long, global = true)]
    pub dry_run: bool,

    #[arg(long, global = true)]
    pub no_rebuild: bool,

    #[arg(short = 'y', long, global = true)]
    pub yes: bool,
}

#[derive(Subcommand)]
pub enum Command {
    Create { packages: Vec<String> },
    Explode { packages: Vec<String> },
    Search { keyword: String },
    List,
    Rollback {
        index: Option<usize>,
    },
    CleanBackups {
        #[arg(long, default_value_t = 5)]
        keep: usize,
    },
}
