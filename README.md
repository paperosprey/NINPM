# NINPM 🦀

> **NINPM is Not a Package Manager**

Welcome to **NINPM**, the ultimate state-of-mind utility for NixOS users who secretly want the convenience of traditional package managers without losing their sanity (or messing up their declarative configuration files manually).

## 🚀 What is this?

NixOS is amazing, but sometimes you just want to quickly add or remove a package without digging through your `configuration.nix` files, dealing with syntax errors, or writing complex scripts. 

NINPM is a lightweight, compiled CLI tool written in **Rust** that automatically parses your NixOS configuration, injects or removes packages under `environment.systemPackages` safely, and shows you a clean diff—all before you even think about running `nixos-rebuild`.

## ⚡ Features

- **Zero Traditional Bloat:** Written in pure Rust for high performance and a single static binary.
- **Smart Parsing:** Automatically finds your system package blocks without breaking your formatting.
- **Dry-Run Diff View:** Shows you exact changes before anything touches your system files.
- **Explosive Cleanup:** Easily remove packages with clean deletion logic.

## 🛠️ Usage

```bash
# Add a package to your configuration
sudo ninpm create <package_name>

# Remove a package from your configuration
sudo ninpm explode <package_name>
