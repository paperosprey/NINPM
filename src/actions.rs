use crate::backup;
use crate::cli::Cli;
use crate::progress::Progress;
use crate::nixlist::{self, canonical_path, flatten_for_display, item_matches};
use crate::rebuild;
use crate::util::{self, BOLD, CYAN, GREEN, RED, RESET, YELLOW};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command as OsCommand;
use std::thread;
use std::time::Duration;

fn read_config(path: &Path) -> Result<String> {
    fs::read_to_string(path)
        .with_context(|| format!("reading {} (are you root? does the path exist?)", path.display()))
}

/// Writes `new_content`, but first: takes a backup of the OLD content, then
/// (best-effort) checks the NEW content actually parses as valid Nix before
/// committing it. If validation is unavailable (no `nix-instantiate` on
/// PATH) we proceed anyway with a warning -- we never block someone from
/// using the tool just because `nix` isn't installed in this environment.
fn write_validated(
    path: &Path,
    old_content: &str,
    new_content: &str,
    progress: &mut Progress,
) -> Result<()> {
    let result = write_validated_inner(path, old_content, new_content, progress);
    if result.is_err() {
        progress.abort();
    }
    result
}

fn write_validated_inner(
    path: &Path,
    old_content: &str,
    new_content: &str,
    progress: &mut Progress,
) -> Result<()> {
    progress.set(Some(5), "checking Nix syntax");
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
            progress.say(
                &format!(
                    "{YELLOW}⚠️  Could not run `nix-instantiate --parse` to double-check syntax \
                     (is `nix` on PATH?). Proceeding without that safety net.{RESET}"
                ),
                true,
            );
        }
    }

    progress.set(Some(10), "saving backup");
    if let Ok(backup_path) = backup::create(path, old_content) {
        progress.say(&format!("{CYAN}📦 Backup saved: {}{RESET}", backup_path.display()), false);
    } else {
        progress.say(&format!("{YELLOW}⚠️  Could not write a backup before this change.{RESET}"), true);
    }

    progress.set(Some(15), "writing configuration");
    fs::write(path, new_content).with_context(|| format!("writing {}", path.display()))
}

fn maybe_rebuild(
    cli: &Cli,
    config_path: &Path,
    original_content: &str,
    progress: &mut Progress,
    base: u8,
) -> Result<()> {
    if cli.no_rebuild {
        progress.finish("done (rebuild skipped)");
        println!("{YELLOW}⚡ --no-rebuild specified. Skipping nixos-rebuild switch.{RESET}");
        return Ok(());
    }

    progress.set(Some(base), "starting nixos-rebuild switch");
    let status = rebuild::run_rebuild(cli.give_me_details, progress, base);

    match status {
        Ok(s) if s.success() => {
            progress.finish("done");
            println!("{GREEN}🚀 System rebuild completed successfully!{RESET}");
            Ok(())
        }
        Ok(_) => {
            progress.abort();
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
            progress.abort();
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

/// One candidate from `nix search --json`.
struct Hit {
    attr: String,
    pname: String,
    version: String,
    description: String,
}

/// Higher = better. Exact attr/pname match beats prefix, prefix beats substring,
/// substring in the name beats a mention in the description. Ties prefer
/// top-level attrs (no `python3Packages.` style nesting), then shorter names.
fn rank(hit: &Hit, kw: &str) -> (u32, std::cmp::Reverse<usize>, std::cmp::Reverse<usize>) {
    let attr = hit.attr.to_lowercase();
    let pname = hit.pname.to_lowercase();
    let desc = hit.description.to_lowercase();
    let dots = attr.matches('.').count();
    let score = if attr == kw {
        100
    } else if pname == kw {
        90
    } else if attr.starts_with(kw) {
        70
    } else if pname.starts_with(kw) {
        65
    } else if attr.contains(kw) {
        50
    } else if pname.contains(kw) {
        45
    } else if desc.contains(kw) {
        10
    } else {
        0
    };
    (score, std::cmp::Reverse(dots), std::cmp::Reverse(attr.len()))
}

fn run_nix_search(keyword: &str) -> Result<Vec<Hit>> {
    let kw = keyword.to_string();
    let handle = thread::spawn(move || {
        OsCommand::new("nix")
            .args([
                "--extra-experimental-features",
                "nix-command flakes",
                "search",
                "nixpkgs",
                &kw,
                "--json",
            ])
            .output()
    });

    let mut progress = Progress::new();
    progress.set(None, &format!("searching nixpkgs for '{keyword}'"));
    while !handle.is_finished() {
        thread::sleep(Duration::from_millis(100));
        progress.tick();
    }
    progress.clear();

    let output = handle
        .join()
        .map_err(|_| anyhow::anyhow!("search thread panicked"))?
        .context("could not run `nix` (is it installed and on PATH?)")?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("nix search failed: {}", err.trim());
    }

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("could not parse `nix search` output")?;
    let mut hits = Vec::new();
    if let Some(map) = json.as_object() {
        for (key, v) in map {
            // keys look like "legacyPackages.x86_64-linux.firefox"
            let attr = key.splitn(3, '.').nth(2).unwrap_or(key).to_string();
            let field = |n: &str| v.get(n).and_then(|x| x.as_str()).unwrap_or("").to_string();
            hits.push(Hit {
                attr,
                pname: field("pname"),
                version: field("version"),
                description: field("description"),
            });
        }
    }
    Ok(hits)
}

pub fn search(keyword: &str, all: bool) -> Result<()> {
    let kw = keyword.to_lowercase();
    let mut hits = run_nix_search(keyword)?;
    hits.sort_by(|a, b| rank(b, &kw).cmp(&rank(a, &kw)).then_with(|| a.attr.cmp(&b.attr)));

    if hits.is_empty() {
        println!("{YELLOW}No package found for '{keyword}'.{RESET}");
        return Ok(());
    }

    let show = if all { 10 } else { 1 };
    for hit in hits.iter().take(show) {
        let version = if hit.version.is_empty() { String::new() } else { format!(" {}", hit.version) };
        println!("{BOLD}{GREEN}{}{RESET}{version}", hit.attr);
        if !hit.description.is_empty() {
            println!("  {}", hit.description.lines().next().unwrap_or(""));
        }
    }

    println!("\n{CYAN}Install:{RESET} ninpm create {}", hits[0].attr);
    if !all && hits.len() > 1 {
        println!("{YELLOW}({} other matches -- use --all to see the top 10){RESET}", hits.len() - 1);
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

    let mut progress = Progress::new();
    write_validated(config_path, &content, &new_content, &mut progress)?;
    progress.say(&format!("{CYAN}Configuration updated.{RESET}"), false);
    maybe_rebuild(cli, config_path, &content, &mut progress, 15)
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

    let mut progress = Progress::new();
    write_validated(config_path, &content, &new_content, &mut progress)?;
    progress.say(&format!("{CYAN}Configuration updated.{RESET}"), false);
    maybe_rebuild(cli, config_path, &content, &mut progress, 15)
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
    let mut progress = Progress::new();
    maybe_rebuild(cli, config_path, &current, &mut progress, 10)
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
