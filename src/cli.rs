use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "ninpm",
    version,
    about = "NINPM Is Not a Package Manager -- a thin, safe helper around environment.systemPackages in configuration.nix",
    disable_help_flag = true,
    disable_help_subcommand = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Show help (custom screen, see help.rs)
    #[arg(short = 'h', long, global = true)]
    pub help: bool,

    /// Path to the Nix config file to edit (default: /etc/nixos/configuration.nix)
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    /// Show nixos-rebuild's real output instead of hiding it
    #[arg(long, global = true)]
    pub give_me_details: bool,

    /// Show what would change, but don't touch any file or rebuild
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Update configuration.nix but skip nixos-rebuild switch
    #[arg(long, global = true)]
    pub no_rebuild: bool,

    /// Skip the confirmation prompt for destructive actions
    #[arg(short = 'y', long, global = true)]
    pub yes: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Show help, optionally for a single command
    Help { topic: Option<String> },
    /// Add package(s) to configuration.nix and rebuild
    Create { packages: Vec<String> },
    /// Remove package(s) from configuration.nix and rebuild
    Explode { packages: Vec<String> },
    /// Search nixpkgs and print the single best match
    Search {
        keyword: Option<String>,
        /// Show the top 10 ranked matches instead of only the best one
        #[arg(long)]
        all: bool,
    },
    /// List packages currently in the systemPackages block
    List,
    /// Restore a previous backup of configuration.nix
    Rollback {
        /// 1 = most recent backup, 2 = the one before that, etc. Omit to see the list.
        index: Option<usize>,
    },
    /// Delete old backups, keeping only the N most recent (default: 5)
    CleanBackups {
        #[arg(long, default_value_t = 5)]
        keep: usize,
    },
}
