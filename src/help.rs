//! Custom, colourful help screens for ninpm.
//!
//! clap still does the argument parsing; we just replace its built-in help
//! output. Colours are disabled automatically when stdout is not a terminal,
//! when NO_COLOR is set, or when TERM=dumb.

use anyhow::{bail, Result};
use std::io::{self, IsTerminal};

struct Theme {
    reset: &'static str,
    bold: &'static str,
    dim: &'static str,
    accent: &'static str,
    green: &'static str,
    yellow: &'static str,
    pink: &'static str,
}

const COLOR: Theme = Theme {
    reset: "\x1b[0m",
    bold: "\x1b[1m",
    dim: "\x1b[2m",
    accent: "\x1b[38;5;81m",
    green: "\x1b[38;5;114m",
    yellow: "\x1b[38;5;221m",
    pink: "\x1b[38;5;212m",
};

const PLAIN: Theme = Theme {
    reset: "",
    bold: "",
    dim: "",
    accent: "",
    green: "",
    yellow: "",
    pink: "",
};

fn theme() -> &'static Theme {
    let color = io::stdout().is_terminal()
        && std::env::var_os("NO_COLOR").is_none()
        && std::env::var("TERM").map(|t| t != "dumb").unwrap_or(true);
    if color {
        &COLOR
    } else {
        &PLAIN
    }
}

struct Opt {
    flag: &'static str,
    desc: &'static str,
}

struct Cmd {
    name: &'static str,
    icon: &'static str,
    args: &'static str,
    short: &'static str,
    long: &'static [&'static str],
    options: &'static [Opt],
    examples: &'static [(&'static str, &'static str)],
}

const CMDS: [Cmd; 6] = [
    Cmd {
        name: "create",
        icon: "📦",
        args: "<PACKAGE>...",
        short: "Add package(s) to configuration.nix and rebuild",
        long: &[
            "Adds the given names to environment.systemPackages, saves a backup,",
            "then runs nixos-rebuild switch. If the rebuild fails, your previous",
            "configuration is restored automatically.",
        ],
        options: &[],
        examples: &[
            ("ninpm create firefox", "install one package"),
            ("ninpm create git htop", "install several at once"),
            ("ninpm create btop --dry-run", "preview only, change nothing"),
            ("ninpm create btop --no-rebuild", "edit the file, skip the rebuild"),
        ],
    },
    Cmd {
        name: "explode",
        icon: "💥",
        args: "<PACKAGE>...",
        short: "Remove package(s) from configuration.nix and rebuild",
        long: &[
            "Removes the given names from environment.systemPackages, saves a",
            "backup, then rebuilds. Shows what will change and asks first.",
        ],
        options: &[],
        examples: &[
            ("ninpm explode htop", "remove one package"),
            ("ninpm explode htop git -y", "remove several, no prompt"),
            ("ninpm explode htop --dry-run", "preview only"),
        ],
    },
    Cmd {
        name: "search",
        icon: "🔍",
        args: "<KEYWORD>",
        short: "Search nixpkgs and print the best match",
        long: &[
            "Searches nixpkgs and prints the single best match together with an",
            "install hint. Requires `nix` on your PATH.",
        ],
        options: &[Opt {
            flag: "--all",
            desc: "Show the top 10 ranked matches instead of just one",
        }],
        examples: &[
            ("ninpm search firefox", "best match only"),
            ("ninpm search python --all", "top 10 matches"),
        ],
    },
    Cmd {
        name: "list",
        icon: "📋",
        args: "",
        short: "List packages in the systemPackages block",
        long: &["Prints every package currently declared in environment.systemPackages."],
        options: &[],
        examples: &[
            ("ninpm list", "your current config"),
            ("ninpm list --config ./test.nix", "a different file"),
        ],
    },
    Cmd {
        name: "rollback",
        icon: "⏪",
        args: "[INDEX]",
        short: "Restore a previous backup of configuration.nix",
        long: &[
            "Without INDEX, lists the available backups (1 = most recent).",
            "With INDEX, restores that backup and rebuilds. Asks before overwriting.",
        ],
        options: &[],
        examples: &[
            ("ninpm rollback", "list backups"),
            ("ninpm rollback 1", "restore the most recent one"),
            ("ninpm rollback 2 -y", "restore #2 without asking"),
        ],
    },
    Cmd {
        name: "clean-backups",
        icon: "🧹",
        args: "",
        short: "Delete old backups, keeping the N most recent",
        long: &["Removes older backups of configuration.nix. Keeps 5 unless told otherwise."],
        options: &[Opt {
            flag: "--keep <N>",
            desc: "How many recent backups to keep (default: 5)",
        }],
        examples: &[
            ("ninpm clean-backups", "keep the newest 5"),
            ("ninpm clean-backups --keep 2", "keep the newest 2"),
        ],
    },
];

const GLOBAL_OPTS: [Opt; 7] = [
    Opt { flag: "-y, --yes", desc: "Skip confirmation prompts" },
    Opt { flag: "    --dry-run", desc: "Show what would change, touch nothing" },
    Opt { flag: "    --no-rebuild", desc: "Edit configuration.nix but skip nixos-rebuild" },
    Opt { flag: "    --give-me-details", desc: "Show nixos-rebuild's real output" },
    Opt { flag: "    --config <PATH>", desc: "Config file to edit (default: /etc/nixos/configuration.nix)" },
    Opt { flag: "-h, --help", desc: "Show this help" },
    Opt { flag: "-V, --version", desc: "Show version" },
];

fn pad(plain_len: usize, width: usize) -> String {
    " ".repeat(width.saturating_sub(plain_len))
}

fn section(t: &Theme, title: &str) {
    println!("\n  {}▸{} {}{}{title}{}", t.accent, t.reset, t.bold, t.yellow, t.reset);
}

fn banner(t: &Theme) {
    const INNER: usize = 44;
    let lines = [
        format!("  ninpm v{}", env!("CARGO_PKG_VERSION")),
        "  NINPM Is Not a Package Manager".to_string(),
    ];
    println!();
    println!("  {}╭{}╮{}", t.accent, "─".repeat(INNER), t.reset);
    for (i, line) in lines.iter().enumerate() {
        let style = if i == 0 { t.bold } else { t.dim };
        println!(
            "  {a}│{r}{style}{line}{r}{sp}{a}│{r}",
            a = t.accent,
            r = t.reset,
            sp = pad(line.chars().count(), INNER),
        );
    }
    println!("  {}╰{}╯{}", t.accent, "─".repeat(INNER), t.reset);
}

fn print_options(t: &Theme, opts: &[Opt]) {
    let width = opts.iter().map(|o| o.flag.chars().count()).max().unwrap_or(0) + 3;
    for o in opts {
        println!(
            "  {}{}{}{}{}",
            t.accent,
            o.flag,
            t.reset,
            pad(o.flag.chars().count(), width),
            o.desc
        );
    }
}

fn print_examples(t: &Theme, examples: &[(&str, &str)]) {
    let width = examples.iter().map(|(c, _)| c.chars().count()).max().unwrap_or(0) + 3;
    for (cmd, note) in examples {
        println!(
            "  {}${} {}{}{}{}{}# {note}{}",
            t.dim,
            t.reset,
            t.bold,
            cmd,
            t.reset,
            pad(cmd.chars().count(), width),
            t.dim,
            t.reset
        );
    }
}

fn main_help() {
    let t = theme();
    banner(t);
    println!(
        "\n  {}A thin, safe helper around environment.systemPackages in configuration.nix.{}",
        t.dim, t.reset
    );

    section(t, "USAGE");
    println!(
        "  {}ninpm{} {}[OPTIONS]{} {}<COMMAND>{}",
        t.bold, t.reset, t.dim, t.reset, t.green, t.reset
    );

    section(t, "COMMANDS");
    let width = CMDS
        .iter()
        .map(|c| c.name.chars().count() + 1 + c.args.chars().count())
        .max()
        .unwrap_or(0)
        + 3;
    for c in CMDS.iter() {
        let plain = c.name.chars().count() + 1 + c.args.chars().count();
        println!(
            "  {} {}{}{}{} {}{}{}{}{}",
            c.icon,
            t.bold,
            t.green,
            c.name,
            t.reset,
            t.dim,
            c.args,
            t.reset,
            pad(plain, width),
            c.short
        );
    }

    section(t, "OPTIONS");
    print_options(t, &GLOBAL_OPTS);

    section(t, "EXAMPLES");
    print_examples(
        t,
        &[
            ("ninpm search firefox", "find the package name"),
            ("ninpm create firefox git", "install two packages"),
            ("ninpm create htop --dry-run", "preview, change nothing"),
            ("ninpm rollback", "pick a backup to restore"),
        ],
    );

    println!(
        "\n  {}🛡  Every change makes a backup first; a failed rebuild rolls back on its own.{}",
        t.pink, t.reset
    );
    println!(
        "  {}💡 Run{} {}ninpm help <command>{} {}for details on a command.{}\n",
        t.dim, t.reset, t.bold, t.reset, t.dim, t.reset
    );
}

fn command_help(c: &Cmd) {
    let t = theme();
    println!(
        "\n  {} {}{}ninpm {}{}  {}— {}{}",
        c.icon, t.bold, t.accent, c.name, t.reset, t.dim, c.short, t.reset
    );

    println!();
    for line in c.long {
        println!("  {line}");
    }

    section(t, "USAGE");
    let args = if c.args.is_empty() {
        String::new()
    } else {
        format!(" {}{}{}", t.green, c.args, t.reset)
    };
    println!(
        "  {}ninpm{} {}[OPTIONS]{} {}{}{}{args}",
        t.bold, t.reset, t.dim, t.reset, t.bold, c.name, t.reset
    );

    if !c.options.is_empty() {
        section(t, "COMMAND OPTIONS");
        print_options(t, c.options);
    }

    section(t, "EXAMPLES");
    print_examples(t, c.examples);

    println!(
        "\n  {}Global options (--yes, --dry-run, --config, ...): see{} {}ninpm --help{}\n",
        t.dim, t.reset, t.bold, t.reset
    );
}

/// Show help for a topic (a command name), or the main screen if `None`.
pub fn show(topic: Option<&str>) -> Result<()> {
    match topic {
        None => {
            main_help();
            Ok(())
        }
        Some(name) => match CMDS.iter().find(|c| c.name == name) {
            Some(c) => {
                command_help(c);
                Ok(())
            }
            None => {
                let names: Vec<&str> = CMDS.iter().map(|c| c.name).collect();
                bail!("no help for '{name}'. Available commands: {}", names.join(", "))
            }
        },
    }
}
