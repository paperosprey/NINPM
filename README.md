# NINPM 🦀🔥

> **NINPM Is Not a Package Manager**

A blazingly fast, colorful CLI utility written in Rust that simplifies package management for NixOS users. NINPM automates the tedious task of manually editing `configuration.nix` and provides a safe, declarative way to add or remove system packages with automatic rollback protection.

---

## ✨ Features

- **🎯 Batch Package Management**: Add or remove multiple packages in a single command
- **🎨 Colored Terminal UI**: Beautiful, eye-pleasing ANSI color output for better readability
- **⚡ Fast Search**: Integrated nixpkgs search functionality (flake-enabled)
- **📋 Package Listing**: Instantly view all packages in your `systemPackages` block
- **🔄 Smart Rollback**: Automatic configuration rollback if `nixos-rebuild` fails
- **💾 Automatic Backups**: Timestamped backups of every change, with smart pruning to prevent disk bloat
- **✅ Syntax Validation**: Pre-validates Nix syntax before committing changes (when `nix-instantiate` is available)
- **🏃 Dry Run Mode**: Preview changes without modifying files or rebuilding
- **🛡️ Safe by Default**: Interactive confirmation for all destructive operations
- **⚙️ Flexible Config Path**: Support for custom `configuration.nix` locations

---

## 📦 Installation

### Prerequisites

NINPM requires:
- **NixOS** system (or flakes enabled on your Nix setup)
- **Rust toolchain** (for building from source)
- Sudo/root access (to rebuild your system)

### Building from Source

```bash
git clone https://github.com/paperosprey/NINPM
cd NINPM
cargo build --release
sudo cp target/release/ninpm /usr/local/bin/
```

### Via Nix Flakes (Coming Soon)

Once flakes are fully set up, you'll be able to use:
```bash
nix run github:paperosprey/NINPM -- <command>
```

---

## 🚀 Quick Start

### Add Packages

```bash
# Add a single package
sudo ninpm create neovim

# Add multiple packages at once
sudo ninpm create htop btop lazygit ripgrep
```

NINPM will:
1. Check if packages are already declared
2. Show you what will be added
3. Ask for confirmation
4. Back up your current configuration
5. Inject packages into the `systemPackages` block
6. Run `nixos-rebuild switch`
7. Automatically rollback if the rebuild fails

### Remove Packages

```bash
# Remove a single package
sudo ninpm explode htop

# Remove multiple packages
sudo ninpm explode htop btop firefox
```

### Search for Packages

```bash
sudo ninpm search firefox
sudo ninpm search python
```

This delegates to `nix search nixpkgs <keyword>`, so all nixpkgs search syntax is supported.

### List Current Packages

```bash
sudo ninpm list
```

Displays all packages currently in your `systemPackages` block with a clean, colored output.

### Manage Backups

```bash
# View available backups
sudo ninpm rollback

# Restore a specific backup (1 = most recent)
sudo ninpm rollback 1
sudo ninpm rollback 3

# Clean old backups, keeping only the 5 most recent
sudo ninpm clean-backups --keep 5

# Keep only the last 3 backups
sudo ninpm clean-backups --keep 3
```

---

## 🎮 Advanced Usage

### Global Flags

```bash
--config <PATH>        # Use a custom configuration.nix path (default: /etc/nixos/configuration.nix)
--dry-run              # Preview changes without modifying files or rebuilding
--no-rebuild           # Update configuration.nix but skip `nixos-rebuild switch`
--give-me-details      # Show full nixos-rebuild output instead of hiding it
-y, --yes              # Skip confirmation prompts (use with caution!)
```

### Examples

```bash
# Preview what would happen without making changes
sudo ninpm create --dry-run neovim

# Add packages but don't rebuild yet
sudo ninpm create --no-rebuild neovim lazygit

# Update a custom config file
sudo ninpm --config /home/user/.config/nixos/config.nix create neovim

# Add packages and skip confirmations
sudo ninpm create -y htop btop

# See detailed rebuild output if something goes wrong
sudo ninpm create --give-me-details neovim
```

---

## 🔧 How It Works

### Architecture

NINPM consists of several key modules:

#### `cli.rs` – Command Interface
Parses command-line arguments using `clap` (with derive macros). Defines all available subcommands and global flags.

#### `actions.rs` – Core Logic
Implements the main workflows:
- **Create**: Parses the Nix file, identifies the `systemPackages` block, checks for duplicates, validates syntax, backs up, writes changes, and rebuilds
- **Explode**: Finds and removes packages from `systemPackages`
- **Search**: Delegates to `nix search nixpkgs`
- **List**: Parses and displays current packages
- **Rollback**: Restores from timestamped backups
- **CleanBackups**: Prunes old backups

#### `nixlist.rs` – Nix Parsing
Custom parser that:
- Locates the `environment.systemPackages = [ ... ]` block
- Tokenizes items inside the block
- Handles both simple (`pkgs.neovim`) and complex (`pkgs.python3.withPackages(...)`) expressions
- Matches package names intelligently

#### `backup.rs` – Backup System
- Creates timestamped backups alongside your config file (e.g., `configuration.nix.ninpm-backup-1695123456`)
- Lists and restores backups by index
- Automatically prunes old backups (default: keeps last 10)
- Best-effort backup creation (won't block main operations if it fails)

#### `rebuild.rs` – System Rebuild
- Validates Nix syntax before writing to disk using `nix-instantiate --parse`
- Executes `nixos-rebuild switch` with optional output suppression
- Returns exit status for success/failure detection

#### `util.rs` – Utilities
- ANSI color codes for terminal UI
- Interactive confirmation prompts
- Package name validation (allows alphanumerics, `-`, `_`, `.`, `/`)

### Workflow Example: `ninpm create neovim`

1. **Read** `/etc/nixos/configuration.nix`
2. **Parse** to find `environment.systemPackages = [ ... ]` boundaries
3. **Extract** existing packages and deduplicate input
4. **Validate** package names (must be alphanumeric + `-`, `_`, `.`, `/`)
5. **Check** if `neovim` is already declared (skip if found)
6. **Display** preview: `+ neovim`
7. **Confirm** with user (interactive or `--yes`)
8. **Backup** old content to `configuration.nix.ninpm-backup-<timestamp>`
9. **Validate** new Nix syntax via `nix-instantiate --parse -` (best-effort)
10. **Write** updated configuration with `neovim` added
11. **Rebuild** via `nixos-rebuild switch`
12. **Monitor** rebuild output
13. **Rollback** if rebuild fails (restore from backup, show error)
14. **Prune** old backups if count exceeds 10

### Safety Guarantees

- **Pre-write validation**: Nix syntax is checked before touching the file
- **Automatic backups**: Every change is timestamped and recoverable
- **Automatic rollback**: If rebuild fails, config is immediately restored
- **Confirmation prompts**: Destructive operations (create/explode) require user approval
- **Dry-run mode**: Preview all changes risk-free with `--dry-run`

---

## 🎨 UI & Colors

NINPM uses ANSI colors to make output clear and scannable:

| Color  | Usage                           |
|--------|----------------------------------|
| 🔵 Cyan    | Information & status updates     |
| 🟢 Green   | Success messages & additions     |
| 🟡 Yellow  | Warnings & confirmations         |
| 🔴 Red     | Errors & removals                |
| ⚪ Bold    | Important headers                |

Example output:
```
📦 Backup saved: /etc/nixos/configuration.nix.ninpm-backup-1695123456
⚙️  Triggering nixos-rebuild switch...
🚀 System rebuild completed successfully!
```

---

## 🐛 Troubleshooting

### "Are you root? Does the path exist?"

NINPM needs sudo to read/write to `/etc/nixos/configuration.nix`. Run with `sudo`:
```bash
sudo ninpm create neovim
```

Or specify a custom readable config file:
```bash
sudo ninpm --config /path/to/config.nix create neovim
```

### "The resulting file would not parse as valid Nix"

Your package entry probably isn't a simple attribute path. NINPM expects entries like:
- ✅ `pkgs.neovim`
- ✅ `pkgs.python3`
- ❌ `(pkgs.python3.withPackages(ps: [...]))` — Complex expressions should be added manually

### Rebuild failed but no rollback?

If `nixos-rebuild switch` returns a non-zero exit code, NINPM automatically restores the backup. If rollback still fails, manually restore from the backup file:
```bash
sudo cp /etc/nixos/configuration.nix.ninpm-backup-<timestamp> /etc/nixos/configuration.nix
```

### "Could not run nix-instantiate" warning

NINPM tried to validate your Nix syntax but couldn't find `nix-instantiate` in PATH. This is a warning, not an error—the tool continues anyway. To fix:
```bash
nix-shell -p nix
```

### No packages found in systemPackages block?

Make sure:
1. Your config has an `environment.systemPackages = [ ... ]` block
2. The block isn't commented out
3. You're pointing to the right file with `--config`

---

## 🛠️ Development

### Building

```bash
cargo build
```

### Running Tests (if any)

```bash
cargo test
```

### Release Build

```bash
cargo build --release
# Binary: target/release/ninpm
```

### Dependencies

- **clap 4.4.18**: Command-line argument parsing (derive macros)
- **anyhow 1.0.86**: Error handling with context
- Uses Rust 2021 edition

### Code Structure

```
src/
├── main.rs       # Entry point, command dispatch
├── cli.rs        # Argument parser
├── actions.rs    # Core workflows (create, explode, search, etc.)
├── nixlist.rs    # Nix file parsing
├── backup.rs     # Backup & restore logic
├── rebuild.rs    # nixos-rebuild invocation
└── util.rs       # Colors & utilities
```

---

## 📜 License

MIT License – See [LICENSE](LICENSE) file for details.

---

## 🤝 Contributing

Found a bug or have an idea? Open an issue or submit a pull request!

### What Would Help

- Flake package definition for easier installation
- Additional tests and edge case handling
- Support for more complex Nix expressions
- Better error messages for common issues
- Man pages or shell completions

---

## ⚡ Performance

NINPM is optimized for speed:
- **Release build** uses `-O3 -lto` (maximum optimizations)
- **Parsing** is linear, even for large configs
- **Search** delegates to `nix search` (fast flakes-based implementation)
- **No unnecessary rebuilds** — config validation happens before write

---

## 🎯 Roadmap

- [ ] Flake package definition
- [ ] Shell completions (bash, zsh, fish)
- [ ] Man page documentation
- [ ] Interactive package selection from search results
- [ ] Support for homeManager and nixos modules
- [ ] Configuration profiles
- [ ] Diff preview before rebuild
- [ ] Binary releases for easier installation

---

## 💬 Support

For questions or issues:
1. Check the **Troubleshooting** section above
2. Search existing [GitHub Issues](https://github.com/paperosprey/NINPM/issues)
3. Open a new issue with details about your setup and the command you ran

---

**Happy hacking! 🚀** 

NINPM makes NixOS package management painless and safe. Enjoy the speed and simplicity! 🦀🔥
