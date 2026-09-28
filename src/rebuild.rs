use anyhow::Result;
use crate::progress::Progress;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;

/// Best-effort syntax check via `nix-instantiate --parse -`, fed the
/// candidate file content over stdin so nothing is written to disk yet.
/// Returns:
///   Ok(true)  -> parsed cleanly
///   Ok(false) -> `nix-instantiate` ran and reported a syntax error
///   Err(_)    -> `nix-instantiate` isn't available or failed to run at all;
///                caller should treat this as "couldn't verify", not "bad".
pub fn validate_syntax(content: &str) -> Result<bool> {
    let mut child = Command::new("nix-instantiate")
        .args(["--parse", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(content.as_bytes())?;
    }
    let output = child.wait_with_output()?;
    Ok(output.status.success())
}

/// Tracks nix's own output to estimate how far along a rebuild is.
/// nix announces up front "these N derivations will be built" / "these M paths
/// will be fetched", then prints one "building '...'" / "copying path '...'"
/// line per item, so done/total is a real (not faked) percentage.
#[derive(Default)]
struct RebuildState {
    to_build: u32,
    to_fetch: u32,
    done: u32,
    activating: bool,
}

impl RebuildState {
    fn feed(&mut self, line: &str) {
        let line = line.trim();
        if line.contains("will be built") {
            self.to_build += first_number(line).unwrap_or(1);
        } else if line.contains("will be fetched") {
            self.to_fetch += first_number(line).unwrap_or(1);
        } else if line.starts_with("building '") || line.starts_with("copying path '") {
            self.done += 1;
        } else if line.contains("activating the configuration") {
            self.activating = true;
        }
    }

    fn total(&self) -> u32 {
        self.to_build + self.to_fetch
    }

    /// (percentage, label). Build/fetch fills `base..=95`, activation 96, done 100.
    fn view(&self, base: u8) -> (Option<u8>, String) {
        if self.activating {
            return (Some(96), "activating configuration".into());
        }
        let total = self.total();
        if total == 0 {
            return (None, "rebuilding (waiting for nix)".into());
        }
        let done = self.done.min(total);
        let span = 95u32.saturating_sub(base as u32);
        let pct = base as u32 + span * done / total;
        (Some(pct as u8), format!("rebuilding {done}/{total}"))
    }
}

fn first_number(line: &str) -> Option<u32> {
    line.split_whitespace().find_map(|w| w.parse::<u32>().ok())
}

pub fn run_rebuild(show_details: bool, progress: &mut Progress, base: u8) -> std::io::Result<ExitStatus> {
    let mut cmd = Command::new("nixos-rebuild");
    cmd.arg("switch");

    if show_details {
        // The user wants raw output; a redrawing bar would fight with it.
        progress.clear();
        return cmd.status();
    }

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn()?;
    let (tx, rx) = mpsc::channel::<String>();
    for stream in [
        child.stdout.take().map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
        child.stderr.take().map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
    ]
    .into_iter()
    .flatten()
    {
        let tx = tx.clone();
        thread::spawn(move || {
            for line in BufReader::new(stream).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
    }
    drop(tx);

    let mut state = RebuildState::default();
    let (pct, label) = state.view(base);
    progress.set(pct, &label);
    loop {
        match rx.recv_timeout(Duration::from_millis(120)) {
            Ok(line) => {
                state.feed(&line);
                let (pct, label) = state.view(base);
                progress.set(pct, &label);
            }
            Err(RecvTimeoutError::Timeout) => progress.tick(),
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    child.wait()
}
