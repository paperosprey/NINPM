mod actions;
mod backup;
mod cli;
mod help;
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

    // -h / --help anywhere: `ninpm --help`, `ninpm create --help`, ...
    if cli.help {
        let topic = cli.command.as_ref().and_then(command_name);
        if let Err(e) = help::show(topic) {
            eprintln!("{}❌ {e:#}{}", util::RED, util::RESET);
            std::process::exit(1);
        }
        return;
    }

    let Some(command) = &cli.command else {
        let _ = help::show(None);
        return;
    };

    let result = match command {
        Command::Help { topic } => help::show(topic.as_deref()),
        Command::List => actions::list(&config_path),
        Command::Search { keyword: Some(keyword), all } => actions::search(keyword, *all),
        Command::Search { keyword: None, .. } => Err(anyhow::anyhow!(
            "missing <KEYWORD>. Try: ninpm search firefox  (details: ninpm help search)"
        )),
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

fn command_name(c: &Command) -> Option<&'static str> {
    Some(match c {
        Command::Help { .. } => return None,
        Command::Create { .. } => "create",
        Command::Explode { .. } => "explode",
        Command::Search { .. } => "search",
        Command::List => "list",
        Command::Rollback { .. } => "rollback",
        Command::CleanBackups { .. } => "clean-backups",
    })
}
