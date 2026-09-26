use std::env;
use std::fs;
use std::process::Command;

// ANSI Renk Kodları (Esnaf UI 🎨)
const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const BOLD: &str = "\x1b[1m";

fn print_help() {
    println!(r#"
{CYAN}╔═══════════════════════════════════════════════════════╗
║                        NINPM                          ║
║            NINPM Is Not a Package Manager             ║
╚═══════════════════════════════════════════════════════{}

{BOLD}Usage:{RESET}
  sudo ninpm <command> <package(s)> [options]

{BOLD}Commands:{RESET}
  create <pkg1> [pkg2...] -> Adds packages to configuration.nix and rebuilds.
  explode <pkg1> [pkg2...] -> Removes packages from system and rebuilds.
  search <keyword>        -> Searches for packages in nixpkgs.
  list                    -> Lists packages in systemPackages block.
  help                    -> Shows this help menu.

{BOLD}Options:{RESET}
  --give-me-details       -> Shows hidden logs during rebuild (for debugging).

{BOLD}Examples:{RESET}
  sudo ninpm create htop fastfetch
  sudo ninpm explode htop --give-me-details
  sudo ninpm search firefox
  sudo ninpm list
"#, RESET);
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 || args[1] == "help" || args[1] == "-h" {
        print_help();
        std::process::exit(0);
    }

    let action = &args[1];
    let show_details = args.iter().any(|arg| arg == "--give-me-details");
    let config_path = "/etc/nixos/configuration.nix";

    let mut content = fs::read_to_string(config_path)
        .expect("Failed to read configuration.nix");

    // 1. LİSTELEME KOMUTU
    if action == "list" {
        println!("{CYAN}📦 Installed packages in systemPackages block:{RESET}");
        if let Some(start) = content.find("environment.systemPackages = with pkgs; [") {
            let slice = &content[start..];
            if let Some(end) = slice.find(']') {
                let pkg_block = &slice[..end];
                for line in pkg_block.lines().skip(1) {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        println!("  {}•{} {}", CYAN, RESET, trimmed);
                    }
                }
            }
        }
        return;
    }

    // 2. ARAMA KOMUTU (ninpm search <keyword>)
    if action == "search" {
        if args.len() < 3 {
            eprintln!("{RED}Error: Keyword required for search! Use: sudo ninpm search <keyword>{RESET}");
            std::process::exit(1);
        }
        let keyword = &args[2];
        println!("{YELLOW}🔍 Searching nixpkgs for '{}'...{RESET}", keyword);
        
        let status = Command::new("nix")
            .args([
                "--extra-experimental-features",
                "nix-command flakes",
                "search",
                "nixpkgs",
                keyword,
            ])
            .status();

        if !status.map_or(false, |s| s.success()) {
            eprintln!("{RED}❌ Search failed or no results found.{RESET}");
        }
        return;
    }

    // Paket adları kontrolü (create ve explode için çoklu paket desteği)
    let mut packages: Vec<String> = Vec::new();
    for arg in args.iter().skip(2) {
        if !arg.starts_with("--") {
            packages.push(arg.clone());
        }
    }

    if packages.is_empty() {
        eprintln!("{RED}Error: Package name(s) required for '{}' action! Type 'ninpm help' for usage.{RESET}", action);
        std::process::exit(1);
    }

    let original_content = content.clone(); // Rollback yedeği 🔄

    if action == "create" {
        let mut added_count = 0;
        for package in &packages {
            let pkg_identifier = format!("pkgs.{}", package);
            if content.contains(&pkg_identifier) || content.contains(package) {
                println!("{YELLOW}⚠️ Warning: Package '{}' is already in configuration.nix, skipping.{RESET}", package);
                continue;
            }

            println!("{GREEN}Creating package: pkgs.{}{RESET}", package);
            let target = "environment.systemPackages = with pkgs; [";
            let replacement = &format!("environment.systemPackages = with pkgs; [\n    pkgs.{}", package);
            
            if content.contains(target) {
                content = content.replacen(target, replacement, 1);
                added_count += 1;
            } else {
                eprintln!("{RED}Error: systemPackages block not found!{RESET}");
                std::process::exit(1);
            }
        }

        if added_count == 0 {
            println!("{YELLOW}No new packages were added. Rebuild skipped.{RESET}");
            return;
        }

        fs::write(config_path, &content).expect("Failed to write configuration.nix");
        println!("{CYAN}Configuration updated, rebuilding nix...{RESET}");

    } else if action == "explode" {
        let mut removed_count = 0;
        for package in &packages {
            println!("{RED}Exploding package: pkgs.{}{RESET}", package);
            let target = format!("    pkgs.{}", package);
            let target_alt = format!("    {}", package);
            
            if content.contains(&target) {
                content = content.replace(&target, "");
                removed_count += 1;
            } else if content.contains(&target_alt) {
                content = content.replace(&target_alt, "");
                removed_count += 1;
            } else {
                println!("{YELLOW}⚠️ Warning: Package '{}' not found in config, skipping.{RESET}", package);
            }
        }

        if removed_count == 0 {
            println!("{YELLOW}No packages were removed. Rebuild skipped.{RESET}");
            return;
        }

        fs::write(config_path, &content).expect("Failed to write configuration.nix");
        println!("{CYAN}Configuration updated, rebuilding nix...{RESET}");

    } else {
        eprintln!("{RED}Invalid action! Use 'create', 'explode', 'search', 'list', or 'help'.{RESET}");
        std::process::exit(1);
    }

    // Git ownership krizini kodun içinde otomatik ekarte ediyoruz amk! 👑
    let _ = Command::new("git")
        .args(["config", "--global", "--add", "safe.directory", "/etc/nixos"])
        .status();

    // Rebuild işlemi
    let mut cmd = Command::new("nixos-rebuild");
    cmd.arg("switch");

    if !show_details {
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());
    }

    let status = cmd.status();

    match status {
        Ok(s) if s.success() => {
            println!("{GREEN}System rebuild completed successfully! 🚀{RESET}");
        }
        _ => {
            fs::write(config_path, original_content).expect("Failed to restore configuration.nix");
            eprintln!("{RED}❌ Rebuild failed! Changes rolled back automatically, config is safe.{RESET}");
            
            if !show_details {
                eprintln!("{YELLOW}Tip: Run with '--give-me-details' to see what went wrong.{RESET}");
            }
            std::process::exit(1);
        }
    }
}
