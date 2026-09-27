use crate::backup;
use crate::cli::Cli;
use crate::nixlist::{self, canonical_path, flatten_for_display, item_matches};
use crate::rebuild;
use crate::util::{self, BOLD, CYAN, GREEN, RED, RESET, YELLOW};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command as OsCommand;

fn read_config(path: &Path) -> Result<String> {
    fs::read_to_string(path)
        .with_context(|| format!("reading {} (are you root? does the path exist?)", path.display()))
}

fn write_validated(path: &Path, old_content: &str, new_content: &str) -> Result<()> {
    match rebuild::validate_syntax(new_content) {
        Ok(true) => {}
        Ok(false) => {
            anyhow::bail!(
                "the resulting file would not parse as valid Nix -- aborting without writing anything.\n\
                 Run with the change reverted and inspect it manually; this is almost always a sign\n\
                 an entry in systemPackages isn't a single simple attribute path."
            );
        }
        Err(_) => {
            eprintln!(
                "{YELLOW}⚠️  Could not run `nix-instantiate --parse` to double-check syntax \
                 (is `nix` on PATH?). Proceeding without that safety net.{RESET}"
            );
        }
    }

    if let Ok(backup_path) = backup::create(path, old_content) {
        println!("{CYAN}📦 Backup saved: {}{RESET}", backup_path.display());
    } else {
        eprintln!("{YELLOW}⚠️  Could not write a backup before this change.{RESET}");
    }

    fs::write(path, new_content).with_context(|| format!("writing {}", path.display()))
}

fn maybe_rebuild(cli: &Cli, config_path: &Path, original_content: &str) -> Result<()> {
    if cli.no_rebuild {
        println!("{YELLOW}⚡ --no-rebuild specified. Skipping nixos-rebuild switch.{RESET}");
        return Ok(());
    }

    println!("{CYAN}⚙️  Triggering nixos-rebuild switch...{RESET}");
    let status = rebuild::run_rebuild(cli.give_me_details);

    match status {
        Ok(s) if s.success() => {
            println!("{GREEN}🚀 System rebuild completed successfully!{RESET}");
            Ok(())
        }
        Ok(_) => {
            eprintln!("{RED}❌ Rebuild failed! Rolling back configuration automatically...{RESET}");
            fs::write(config_path, original_content)
                .context("rolling back configuration.nix after failed rebuild")?;
            println!("{YELLOW}🔄 Previous state restored.{RESET}");
            if !cli.give_me_details {
                eprintln!("{YELLOW}Tip: rerun with --give-me-details to see the exact error.{RESET}");
            }
            anyhow::bail!("nixos-rebuild switch failed; config was rolled back")
        }
        Err(e) => {
            eprintln!("{RED}❌ Failed to execute nixos-rebuild: {e}{RESET}");
            fs::write(config_path, original_content)
                .context("rolling back configuration.nix after failed rebuild invocation")?;
            anyhow::bail!("could not invoke nixos-rebuild; config was rolled back")
        }
    }
}

fn dedup(names: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for n in names {
        if !out.contains(n) {
            out.push(n.clone());
        }
    }
    out
}

pub fn list(config_path: &Path) -> Result<()> {
    let content = read_config(config_path)?;
    let (open_idx, close_idx) = nixlist::find_bracket_range(&content)?;
    let items_text = &content[open_idx + 1..close_idx];
    let items = nixlist::parse_items(items_text);

    println!("{CYAN}📦 Installed packages in systemPackages block:{RESET}");
    if items.is_empty() {
        println!("  {YELLOW}(No packages found in block){RESET}");
    } else {
        for span in items {
            println!(
                "  {CYAN}•{RESET} {}",
                flatten_for_display(&items_text[span.start..span.end])
            );
        }
    }
    Ok(())
}

pub fn search(keyword: &str) -> Result<()> {
    println!("{YELLOW}🔍 Searching nixpkgs for '{keyword}'...{RESET}");
    let status = OsCommand::new("nix")
        .args([
            "--extra-experimental-features",
            "nix-command flakes",
            "search",
            "nixpkgs",
            keyword,
        ])
        .status();

    if !status.map(|s| s.success()).unwrap_or(false) {
        eprintln!("{RED}❌ Search failed or no results found.{RESET}");
    }
    Ok(())
}

pub fn create(cli: &Cli, config_path: &Path, raw_packages: &[String]) -> Result<()> {
    let packages = dedup(raw_packages);
    for p in &packages {
        if !util::is_valid_package_name(p) {
            anyhow::bail!("invalid package name: '{p}'");
        }
    }

    let content = read_config(config_path)?;
    let (open_idx, close_idx) = nixlist::find_bracket_range(&content)?;
    let items_text = &content[open_idx + 1..close_idx];
    let existing: Vec<String> = nixlist::parse_items(items_text)
        .iter()
        .map(|s| canonical_path(&items_text[s.start..s.end]).to_string())
        .collect();

    let mut to_add = Vec::new();
    for package in &packages {
        if existing.iter().any(|e| e == package || nixlist::first_segment(e) == package) {
            println!("{YELLOW}⚠️  '{package}' is already declared, skipping.{RESET}");
            continue;
        }
        to_add.push(package.clone());
    }

    if to_add.is_empty() {
        println!("{YELLOW}Nothing to add.{RESET}");
        return Ok(());
    }

    println!("{BOLD}The following would be added:{RESET}");
    for p in &to_add {
        println!("  {GREEN}+ {p}{RESET}");
    }

    if cli.dry_run {
        println!("{CYAN}[DRY RUN] No files changed, no rebuild triggered.{RESET}");
        return Ok(());
    }

    if !util::confirm("Apply this change?", cli.yes) {
        println!("{YELLOW}Aborted, nothing changed.{RESET}");
        return Ok(());
    }

    let additions: String = to_add.iter().map(|p| format!("    {p}\n")).collect();
    let mut new_content = content.clone();
    new_content.insert_str(open_idx + 1, &format!("\n{additions}"));

    write_validated(config_path, &content, &new_content)?;
    println!("{CYAN}Configuration updated.{RESET}");
    maybe_rebuild(cli, config_path, &content)
}

pub fn explode(cli: &Cli, config_path: &Path, raw_packages: &[String]) -> Result<()> {
    let packages = dedup(raw_packages);

    let content = read_config(config_path)?;
    let (open_idx, close_idx) = nixlist::find_bracket_range(&content)?;
    let items_text = &content[open_idx + 1..close_idx];
    let items = nixlist::parse_items(items_text);

    let mut kept: Vec<&str> = Vec::new();
    let mut removed_texts: Vec<String> = Vec::new();

    for span in &items {
        let raw = &items_text[span.start..span.end];
        let matched = packages.iter().any(|p| item_matches(raw, p));
        if matched {
            removed_texts.push(flatten_for_display(raw));
        } else {
            kept.push(raw);
        }
    }

    for package in &packages {
        let was_present = items
            .iter()
            .any(|span| item_matches(&items_text[span.start..span.end], package));
        if !was_present {
            println!("{YELLOW}⚠️  '{package}' not found in the systemPackages block, skipping.{RESET}");
        }
    }

    if removed_texts.is_empty() {
        println!("{YELLOW}Nothing to remove.{RESET}");
        return Ok(());
    }

    println!("{BOLD}The following would be removed:{RESET}");
    for t in &removed_texts {
        println!("  {RED}- {t}{RESET}");
    }

    if cli.dry_run {
        println!("{CYAN}[DRY RUN] No files changed, no rebuild triggered.{RESET}");
        return Ok(());
    }

    if !util::confirm("Apply this change?", cli.yes) {
        println!("{YELLOW}Aborted, nothing changed.{RESET}");
        return Ok(());
    }

    let new_items_text: String = kept.iter().map(|item| format!("    {}\n", item.trim())).collect();
    let new_content = format!(
        "{}\n{}{}",
        &content[..=open_idx],
        new_items_text,
        &content[close_idx..]
    );

    write_validated(config_path, &content, &new_content)?;
    println!("{CYAN}Configuration updated.{RESET}");
    maybe_rebuild(cli, config_path, &content)
}

pub fn rollback(cli: &Cli, config_path: &Path, index: Option<usize>) -> Result<()> {
    let backups = backup::list(config_path)?;
    if backups.is_empty() {
        println!("{YELLOW}No backups found next to {}.{RESET}", config_path.display());
        return Ok(());
    }

    let Some(index) = index else {
        println!("{BOLD}Available backups (newest first):{RESET}");
        for (i, b) in backups.iter().enumerate() {
            println!("  {CYAN}{}{RESET}  {}", i + 1, b.display());
        }
        println!("\nRun `ninpm rollback <N>` to restore one.");
        return Ok(());
    };

    if index == 0 || index > backups.len() {
        anyhow::bail!("no backup at index {index} (have {})", backups.len());
    }

    let current = read_config(config_path)?;
    println!(
        "{YELLOW}This will overwrite {} with backup #{index} ({}).{RESET}",
        config_path.display(),
        backups[index - 1].display()
    );
    if !util::confirm("Continue?", cli.yes) {
        println!("{YELLOW}Aborted, nothing changed.{RESET}");
        return Ok(());
    }

    let restored_from = backup::restore(config_path, index)?;
    println!("{GREEN}Restored from {}.{RESET}", restored_from.display());
    maybe_rebuild(cli, config_path, &current)
}

pub fn clean_backups(config_path: &Path, keep: usize) -> Result<()> {
    let removed = backup::prune(config_path, keep)?;
    if removed == 0 {
        println!("{CYAN}Nothing to clean up (already at or under {keep} backups).{RESET}");
    } else {
        println!("{GREEN}Removed {removed} old backup(s), keeping the {keep} most recent.{RESET}");
    }
    Ok(())
}
