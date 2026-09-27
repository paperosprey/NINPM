<p align="center">
  <img src="ninpm-logo.png" alt="NINPM logo" width="500">
</p>

<h1 align="center">NINPM 🦀</h1>

<p align="center">
  <strong>NINPM Is Not a Package Manager</strong>
</p>

<p align="center">
  A friendly little CLI that helps new NixOS users install and remove system packages without wrestling with <code>configuration.nix</code> on day one.
</p>

<p align="center">
  <em>Because learning Nix is fun. Finding the right bracket while your coffee gets cold is less fun.</em>
</p>

---

## What is NINPM?

NINPM is a small Rust command-line tool for managing entries in the `environment.systemPackages` block of your NixOS configuration.

It is designed primarily for **people who are new to NixOS** and want a simple answer to a simple question:

> “I just want to install a package. What do I type?”

With NINPM:

```bash
sudo ninpm create neovim
```

NINPM shows what will change, asks for confirmation, creates a backup, updates your configuration, and can run `nixos-rebuild switch` for you.

No ceremony. No configuration scavenger hunt. No opening your editor and wondering whether that bracket belongs there. 🦀

### What NINPM is — and is not

NINPM is:

- a beginner-friendly helper for editing `configuration.nix`;
- a convenient add/remove interface for simple `systemPackages` entries;
- a safety net with backups, dry runs, and rollback support;
- a small tool that helps you get comfortable using NixOS.

NINPM is **not** a replacement for Nix, `nixos-rebuild`, flakes, Home Manager, or any other Nix tooling. Those tools remain available whenever you are ready to learn them. NINPM simply makes the first steps less intimidating.

---

## Why was it made?

Installing one application should not feel like an entrance exam.

On other systems, a new user might type:

```bash
sudo apt install neovim
```

or:

```bash
brew install neovim
```

On a typical NixOS setup, the same task usually means:

1. Find the package name.
2. Open `/etc/nixos/configuration.nix`.
3. Locate `environment.systemPackages`.
4. Add the package in the right place.
5. Save the file.
6. Run `sudo nixos-rebuild switch`.
7. Figure out what went wrong if the rebuild fails.

That workflow is perfectly valid. It is also a lot to throw at someone who has just installed NixOS and would like to use their applications before midnight.

NINPM focuses on that onboarding gap. It gives beginners a practical, reversible way to install software while they gradually learn how NixOS works underneath.

> You do not have to understand every part of the engine before turning on the car.

---

## Features

- **Beginner-first commands** for adding and removing packages
- **Batch operations** to add or remove several packages at once
- **Preview before changes** so you can see what NINPM plans to do
- **Interactive confirmation** before modifying your configuration
- **Timestamped backups** before edits are written
- **Automatic rollback** when `nixos-rebuild switch` fails
- **Dry-run mode** for completely risk-free previews
- **Best-effort Nix syntax validation** before writing
- **Package search** through `nix search nixpkgs`
- **Package listing** for the detected `systemPackages` block
- **Custom configuration paths** for non-default setups
- **Readable, colorful output** because terminals do not have to look like tax forms

---

## Requirements

NINPM currently expects:

- NixOS, or a Nix installation with the required commands available;
- Rust and Cargo when building from source;
- permission to read and write your configuration file;
- permission to run `nixos-rebuild switch` when you want NINPM to rebuild the system.

By default, NINPM works with:

```text
/etc/nixos/configuration.nix
```

You can provide another file with `--config`.

---

## Installation

### Build from source

```bash
git clone https://github.com/paperosprey/NINPM.git
cd NINPM
cargo build --release
```

The compiled binary will be here:

```text
target/release/ninpm
```

You can try it directly:

```bash
sudo ./target/release/ninpm --help
```

### Make it available as `ninpm`

The simplest temporary option is an alias:

```bash
alias ninpm='sudo /absolute/path/to/NINPM/target/release/ninpm'
```

For a persistent setup, add the binary to your NixOS configuration through a wrapper:

```nix
environment.systemPackages = with pkgs; [
  (writeShellScriptBin "ninpm" ''
    exec /absolute/path/to/NINPM/target/release/ninpm "$@"
  '')
];
```

Then rebuild once in the usual way:

```bash
sudo nixos-rebuild switch
```

> A flake-based installation and binary releases are planned improvements. For now, building from source keeps the installation transparent and easy to inspect.

---

## Quick start

### Add a package

```bash
sudo ninpm create neovim
```

Add multiple packages in one command:

```bash
sudo ninpm create htop btop ripgrep lazygit
```

Before changing anything, NINPM will:

1. check whether the packages are already present;
2. show the entries it intends to add;
3. ask for confirmation;
4. create a backup of the current file;
5. validate the resulting Nix syntax when possible;
6. update the `systemPackages` block;
7. run `nixos-rebuild switch`, unless disabled;
8. restore the previous content if the rebuild fails.

A normal successful run looks roughly like this:

```text
The following would be added:
  + neovim

Apply this change? [y/N]: y
📦 Backup saved: /etc/nixos/configuration.nix.ninpm-backup-...
Configuration updated.
⚙️  Triggering nixos-rebuild switch...
🚀 System rebuild completed successfully!
```

### Remove a package

```bash
sudo ninpm explode htop
```

Multiple packages work too:

```bash
sudo ninpm explode htop btop firefox
```

`explode` is intentionally a slightly dramatic name for “remove”. The packages will survive. Probably.

### Search for packages

```bash
sudo ninpm search firefox
sudo ninpm search python
```

This delegates to `nix search nixpkgs`, so NINPM does not maintain a second package database.

### List packages

```bash
sudo ninpm list
```

This displays the entries found in the detected `environment.systemPackages` block.

---

## Safety tools

### Preview without changing anything

Use `--dry-run` when you want to see the plan first:

```bash
sudo ninpm create --dry-run neovim
```

This does not write files and does not trigger a rebuild.

### Update the file without rebuilding

If you want to inspect or rebuild later:

```bash
sudo ninpm create --no-rebuild neovim
```

### Restore a backup

List available backups:

```bash
sudo ninpm rollback
```

Restore the most recent backup:

```bash
sudo ninpm rollback 1
```

Restore another backup by its displayed index:

```bash
sudo ninpm rollback 3
```

NINPM asks for confirmation before restoring unless you use `--yes`.

### Remove old backups

Keep the five most recent backups:

```bash
sudo ninpm clean-backups --keep 5
```

Keep only the three most recent:

```bash
sudo ninpm clean-backups --keep 3
```

---

## Command reference

```text
ninpm create <PACKAGE>...
    Add one or more package entries.

ninpm explode <PACKAGE>...
    Remove one or more package entries.

ninpm search <KEYWORD>
    Search nixpkgs through the Nix CLI.

ninpm list
    List entries in the detected systemPackages block.

ninpm rollback
    List available configuration backups.

ninpm rollback <INDEX>
    Restore a backup by index, where 1 is the newest.

ninpm clean-backups --keep <N>
    Remove old backups and keep the newest N files.
```

### Global options

```text
--config <PATH>       Use a custom configuration file.
--dry-run             Preview changes without writing or rebuilding.
--no-rebuild          Write the configuration but skip nixos-rebuild.
--give-me-details     Show the full nixos-rebuild output.
-y, --yes             Skip confirmation prompts.
```

Examples:

```bash
# Use a custom configuration file
sudo ninpm --config /home/me/nixos/configuration.nix create neovim

# Add packages without rebuilding yet
sudo ninpm create --no-rebuild neovim lazygit

# Show the complete rebuild output
sudo ninpm create --give-me-details neovim

# Skip confirmation when you know exactly what you are doing
sudo ninpm create --yes htop btop
```

> `--yes` is convenient, but it is not a magic “please prevent all mistakes” button. Read the preview first when experimenting.

---

## How NINPM edits the configuration

NINPM looks for an `environment.systemPackages = [ ... ]` block and works with the package entries inside it.

For example, given:

```nix
environment.systemPackages = with pkgs; [
  git
  curl
];
```

running:

```bash
sudo ninpm create neovim
```

adds the requested entry to that block.

NINPM performs a text-based edit rather than attempting to become a full Nix language formatter or evaluator. This keeps the tool small and predictable, but it also means there are some boundaries.

### Current limitations

- The target must contain an `environment.systemPackages` list.
- The parser is intended for normal package entries such as `neovim`, `pkgs.neovim`, or similar simple expressions.
- Complex expressions, custom functions, unusual formatting, and heavily abstracted configurations may need to be edited manually.
- NINPM currently focuses on system packages, not Home Manager packages or arbitrary Nix modules.
- Syntax validation is best-effort. If `nix-instantiate` is unavailable, NINPM warns and continues.

When in doubt, use `--dry-run`, keep your configuration in version control, and make sure you understand the proposed change before confirming. Nix is powerful; it also occasionally looks at a semicolon like it has personal feelings about it.

---

## The beginner-friendly workflow

NINPM is meant to be a stepping stone, not a locked door.

A comfortable learning path might look like this:

1. Use NINPM to install your first few packages.
2. Notice what changes in `configuration.nix`.
3. Try `--dry-run` to understand the proposed edit.
4. Open the file and explore it when you feel ready.
5. Gradually learn `nixos-rebuild`, flakes, Home Manager, and the rest of the Nix ecosystem.

You can start with a friendly command and learn the deeper machinery at your own pace. There is no requirement to understand every Nix concept before installing a text editor.

---

## Development

Build a debug version:

```bash
cargo build
```

Build an optimized version:

```bash
cargo build --release
```

Run tests:

```bash
cargo test
```

The project is organized into focused modules:

```text
src/
├── main.rs       # Application entry point
├── cli.rs        # Command-line argument definitions
├── actions.rs    # Create, remove, search, list, and rollback workflows
├── nixlist.rs    # systemPackages parsing and matching
├── backup.rs     # Backup, restore, and pruning logic
├── rebuild.rs    # Syntax validation and nixos-rebuild execution
└── util.rs       # Confirmation prompts, colors, and helpers
```

### Contributing

Bug reports, documentation improvements, test cases, and ideas are welcome.

Especially useful contributions include:

- tests for unusual `configuration.nix` layouts;
- support for more complex Nix expressions;
- better error messages for beginners;
- shell completions;
- a flake package definition;
- Home Manager support;
- packaged binaries and release automation.

If you report a rebuild problem, include the command you ran and the output from:

```bash
sudo ninpm create --give-me-details <package>
```

Please do not include secrets, tokens, or private configuration content in bug reports.

---

## Roadmap

- [ ] Flake package definition
- [ ] Shell completions for bash, zsh, and fish
- [ ] Man page
- [ ] Binary releases
- [ ] Interactive package selection
- [ ] Support for Home Manager configurations
- [ ] More parser coverage and edge-case tests
- [ ] Improved diff presentation

---

## License

NINPM is released under the MIT License. See [LICENSE](LICENSE) for details.

---

## Final word

NixOS has a lot to teach, but a new user should not need a three-hour lecture before installing their first application.

NINPM exists to make that first experience friendlier:

```bash
sudo ninpm create firefox
```

Small command. Fewer sharp edges. More time actually using your system. 🦀

Made for beginners, useful for everyone, and perfectly happy to get out of your way when you are ready to use the lower-level tools directly.
