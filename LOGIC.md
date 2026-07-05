# Logic & Workflows

## Concept

A centralized backup repository for managing, restoring, and updating macOS configuration files (dotfiles). It targets XDG compliance and automates environment bootstrapping.

---

## Workflows

### 1. Bootstrap Setup (`install.sh` & `bkzyn setup`)

Automates the configuration and package installation on macOS:

- **Homebrew**: Installs Homebrew and sets up shell environments (via `install.sh`).
- **Mise**: Installs `mise` tool manager and uses it to install global versions of rust and python.
- **Repository Setup**: Links the current repository to `$XDG_DATA_HOME/backup` (or `~/.local/share/backup`), compiles the CLI tool (`bkzyn`), and places it into `~/.local/bin`.
- **Oh-My-Zsh**: Installs Oh-My-Zsh unattended, and sets zsh as the default shell.
- **Configuration Symlinks**: Symlinks config directories from the repository to `$XDG_CONFIG_HOME` (`$HOME/.config/`).
- **Brew Bundle**: Installs packages listed in `Brewfile`.
- **Zsh Setup**: Bootstraps `$XDG_CONFIG_HOME` and `$ZDOTDIR` in `/etc/zshenv` to keep `$HOME` clean.

### 2. Backup Sync (`bkzyn backup`)

Synchronizes changes from local system configurations back into the repository:

- **Archiving**: Backs up existing repository configs into `.old/config_$date.tar.zst` using `zstd` compression.
- **Config Resolution**: Looks for `backup.toml` in `$XDG_CONFIG_HOME/backup/` first, falling back to the repository.
- **Selective Sync**: For each app tracked in `backup.toml`, copies modified items from the local config back into the repository `config/` directory, respecting explicit `include` and `exclude` glob patterns.

### 3. Managing tracked files

Easily track new configurations or modify patterns without manually editing `backup.toml`:

- **`bkzyn add <path>`**: Moves a directory or file from `$XDG_CONFIG_HOME` into the backup repository, creates a symlink back to the original location, and registers it in `backup.toml`.
- **`bkzyn include <app> <pattern>` / `bkzyn exclude <app> <pattern>`**: Quickly updates the glob patterns in `backup.toml` for fine-grained control.
- **`bkzyn save [-m message]`**: Automates committing all modifications to the Git repository.

---

## Purpose

An all-in-one CLI tool written in Rust for backing up dotfiles, data, and configurations, and seamlessly setting them up on another device.

- Splits massive logic into distinct commands (`cmd/backup.rs`, `cmd/setup.rs`, etc.) for maintainability.
- Path management and core logic abstracted away in `lib.rs`.

---

## CLI Commands

Name: `bkzyn`
Global flags:

- `-v`, `--verbose`: Enable verbose logging output
- `--dry-run`: Run without making any modifications to the filesystem

### Subcommands

#### `bkzyn backup`

Reads `backup.toml` for include/exclude patterns and synchronizes apps from `$XDG_CONFIG_HOME` to the repository `config/` folder. It safely archives the previous state in `.old/` before syncing.

#### `bkzyn setup`

Bootstraps the environment by symlinking `config/*` to `$XDG_CONFIG_HOME`, running `brew bundle` with the provided `Brewfile`, and setting up `/etc/zshenv`.

#### `bkzyn restore`

Specifically restores the configuration by symlinking `config/*` from the repository to `$XDG_CONFIG_HOME` (bypassing `Brewfile` and environment bootstrapping).

#### `bkzyn add <path>`

Moves a configuration file or folder from your local `~/.config` into the backup repository and replaces it with a symlink.

#### `bkzyn include <app> <pattern>`

Adds an include pattern for an app in `backup.toml`.

#### `bkzyn exclude <app> <pattern>`

Adds an exclude pattern for an app in `backup.toml`.

#### `bkzyn save [-m message]`

Stages all changes in the backup repository and creates a Git commit automatically.

---

## Toml config file

The primary configuration file is `backup.toml` (located at `$XDG_CONFIG_HOME/backup/backup.toml` or the repository root). It maintains a list of tracked apps and their include/exclude patterns using glob syntax.
