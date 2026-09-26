# NINPM 🦀🔥
> **NINPM Is Not a Package Manager**

A lightning-fast, colorful, esnaf-style helper utility written in Rust for NixOS users who are tired of manual `configuration.nix` chores! 

---

## 🚀 Features

* **Batch Package Support:** Add or explode multiple packages at once with a single command (`create` / `explode`).
* **Colored UI 🎨:** Eye-pleasing ANSI colored terminal interface.
* **Smart Search:** Fast flake-enabled search directly in nixpkgs (`search`).
* **List View:** Instantly list everything inside your `systemPackages` block (`list`).
* **Safety Net:** Automatically rolls back if the rebuild fails, keeping your system safe. Plus, it automatically handles git/libgit2 ownership quirks in the background.

---

## 📌 Usage

```bash
# Add packages and rebuild system
sudo ninpm create htop btop fastfetch

# Remove packages (explode)
sudo ninpm explode htop

# Search for packages
sudo ninpm search firefox

# List installed packages
sudo ninpm list

# Detailed logs (for debugging)
sudo ninpm create neovim --give-me-details
