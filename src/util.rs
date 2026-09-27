use std::io::{self, Write};

pub const RESET: &str = "\x1b[0m";
pub const RED: &str = "\x1b[31m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const CYAN: &str = "\x1b[36m";
pub const BOLD: &str = "\x1b[1m";

pub fn is_valid_package_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/'))
}

/// Interactive y/N confirmation. Always returns true if `skip` is set
/// (i.e. --yes was passed, or we're not attached to a real terminal prompt
/// the user could answer).
pub fn confirm(prompt: &str, skip: bool) -> bool {
    if skip {
        return true;
    }
    print!("{YELLOW}{prompt} [y/N]{RESET} ");
    let _ = io::stdout().flush();
    let mut answer = String::new();
    if io::stdin().read_line(&mut answer).is_err() {
        return false;
    }
    matches!(answer.trim().to_lowercase().as_str(), "y" | "yes")
}
