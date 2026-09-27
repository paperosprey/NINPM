use anyhow::Result;
use std::io::Write;
use std::process::{Command, Stdio};

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
