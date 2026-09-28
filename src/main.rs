mod actions;
mod backup;
mod cli;
mod nixlist;
mod progress;
mod rebuild;
mod util;

use clap::Parser;
use cli::{Cli, Command};
use std::path::PathBuf;

const DEFAULT_CONFIG_PATH: &str = "/etc/nixos/configuration.nix";

fn main() {
    let cli = Cli::parse();
    let config_path: PathBuf = cli
        .config
        .clone()
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_PATH));

    let result = match &cli.command {
        Command::List => actions::list(&config_path),
        Command::Search { keyword, all } => actions::search(keyword, *all),
        Command::Create { packages } => actions::create(&cli, &config_path, packages),
        Command::Explode { packages } => actions::explode(&cli, &config_path, packages),
        Command::Rollback { index } => actions::rollback(&cli, &config_path, *index),
        Command::CleanBackups { keep } => actions::clean_backups(&config_path, *keep),
    };

    if let Err(e) = result {
        eprintln!("{}❌ {e:#}{}", util::RED, util::RESET);
        std::process::exit(1);
    }
}
