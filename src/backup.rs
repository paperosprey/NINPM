use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_KEEP: usize = 10;

fn backup_dir_and_prefix(config_path: &Path) -> (PathBuf, String) {
    let dir = config_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let file_name = config_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("configuration.nix")
        .to_string();
    (dir, file_name)
}

fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Writes a timestamped backup of `content` next to `config_path`, then
/// prunes old backups beyond `DEFAULT_KEEP` so they don't accumulate forever.
/// Returns the backup path on success. Failures here are surfaced but never
/// fatal to the caller's main operation -- caller decides how to handle it.
pub fn create(config_path: &Path, content: &str) -> Result<PathBuf> {
    let (dir, file_name) = backup_dir_and_prefix(config_path);
    let backup_path = dir.join(format!("{file_name}.ninpm-backup-{}", now_ts()));
    fs::write(&backup_path, content)
        .with_context(|| format!("writing backup {}", backup_path.display()))?;
    let _ = prune(config_path, DEFAULT_KEEP); // best-effort, never blocks the main op
    Ok(backup_path)
}

/// Lists backups for `config_path`, newest first.
pub fn list(config_path: &Path) -> Result<Vec<PathBuf>> {
    let (dir, file_name) = backup_dir_and_prefix(config_path);
    let prefix = format!("{file_name}.ninpm-backup-");

    let mut backups: Vec<(u64, PathBuf)> = fs::read_dir(&dir)
        .with_context(|| format!("reading directory {}", dir.display()))?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = name.to_str()?;
            let ts_str = name.strip_prefix(&prefix)?;
            let ts: u64 = ts_str.parse().ok()?;
            Some((ts, entry.path()))
        })
        .collect();

    backups.sort_by(|a, b| b.0.cmp(&a.0)); // newest first
    Ok(backups.into_iter().map(|(_, p)| p).collect())
}

/// Deletes all but the newest `keep` backups. Returns how many were removed.
pub fn prune(config_path: &Path, keep: usize) -> Result<usize> {
    let backups = list(config_path)?;
    let mut removed = 0;
    for old in backups.into_iter().skip(keep) {
        if fs::remove_file(&old).is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}

/// Restores backup at 1-based `index` (1 = most recent) into `config_path`.
/// Returns the path of the backup that was restored.
pub fn restore(config_path: &Path, index: usize) -> Result<PathBuf> {
    let backups = list(config_path)?;
    if backups.is_empty() {
        anyhow::bail!("no backups found next to {}", config_path.display());
    }
    let idx = index.saturating_sub(1);
    let backup = backups
        .get(idx)
        .ok_or_else(|| anyhow::anyhow!("no backup at index {index} (have {})", backups.len()))?;
    let content = fs::read_to_string(backup)
        .with_context(|| format!("reading backup {}", backup.display()))?;
    fs::write(config_path, content)
        .with_context(|| format!("restoring into {}", config_path.display()))?;
    Ok(backup.clone())
}
