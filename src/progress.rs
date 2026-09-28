use std::io::{self, IsTerminal, Write};
use std::time::Instant;

const WIDTH: usize = 28;
const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Minimal single-line progress bar. On a real terminal it redraws in place;
/// otherwise (pipes, CI, logs) it prints one plain line per label change so
/// output stays readable. `pct == None` means "unknown total": spinner only.
pub struct Progress {
    tty: bool,
    start: Instant,
    pct: Option<u8>,
    label: String,
    frame: usize,
    drawn: bool,
}

impl Progress {
    pub fn new() -> Self {
        Self {
            tty: io::stdout().is_terminal(),
            start: Instant::now(),
            pct: Some(0),
            label: String::new(),
            frame: 0,
            drawn: false,
        }
    }

    pub fn set(&mut self, pct: Option<u8>, label: &str) {
        let changed = self.label != label;
        self.pct = pct.map(|p| p.min(100));
        self.label = label.to_string();
        if self.tty {
            self.draw();
        } else if changed {
            match self.pct {
                Some(p) => println!("[{p:>3}%] {label}"),
                None => println!("[ ...] {label}"),
            }
        }
    }

    /// Redraw with the next spinner frame / updated elapsed time.
    pub fn tick(&mut self) {
        self.frame = self.frame.wrapping_add(1);
        if self.tty {
            self.draw();
        }
    }

    /// Print a message above the bar without corrupting it.
    pub fn say(&mut self, msg: &str, to_stderr: bool) {
        self.clear();
        if to_stderr {
            eprintln!("{msg}");
        } else {
            println!("{msg}");
        }
        if self.tty && !self.label.is_empty() {
            self.draw();
        }
    }

    /// Remove the bar from the screen (e.g. before handing the terminal to a child process).
    pub fn clear(&mut self) {
        if self.tty && self.drawn {
            print!("\r\x1b[2K");
            let _ = io::stdout().flush();
            self.drawn = false;
        }
    }

    pub fn finish(&mut self, label: &str) {
        self.set(Some(100), label);
        self.end_line();
    }

    /// Stop drawing and leave the last state on screen (used on failure).
    pub fn abort(&mut self) {
        self.clear();
        self.label.clear();
    }

    fn end_line(&mut self) {
        if self.tty && self.drawn {
            println!();
            self.drawn = false;
        }
    }

    fn draw(&mut self) {
        let secs = self.start.elapsed().as_secs();
        let time = format!("{}:{:02}", secs / 60, secs % 60);
        let line = match self.pct {
            Some(p) => {
                let filled = WIDTH * p as usize / 100;
                format!(
                    "\x1b[36m[{}{}]\x1b[0m {p:>3}% {} \x1b[2m{time}\x1b[0m",
                    "█".repeat(filled),
                    "░".repeat(WIDTH - filled),
                    self.label
                )
            }
            None => format!(
                "\x1b[36m{}\x1b[0m {} \x1b[2m{time}\x1b[0m",
                SPINNER[self.frame % SPINNER.len()],
                self.label
            ),
        };
        print!("\r\x1b[2K{line}");
        let _ = io::stdout().flush();
        self.drawn = true;
    }
}
