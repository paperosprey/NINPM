use anyhow::Result;
use std::io::Write;
use std::process::{Command, Stdio};

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

pub fn run_rebuild(show_details: bool) -> std::io::Result<std::process::ExitStatus> {
    let mut cmd = Command::new("nixos-rebuild");
    cmd.arg("switch");
    if !show_details {
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());
    }
    cmd.status()
}
