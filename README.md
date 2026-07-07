# Bkzyn (Backup)

A centralized repository and robust CLI tool to manage, restore, and update macOS configuration files (dotfiles) and system setups for kuranne. It targets XDG compliance and heavily automates environment bootstrapping.

## Tech Stack

- **Rust**: Core CLI (`bkzyn`) offering blazing fast execution and safe path resolution.
- **Bash**: Bootstrapping and installation scripts (`install.sh`).
- **TOML**: Configuration (`backup.toml`) for managing includes/excludes with glob patterns.
- **Homebrew**: Package management (via `Brewfile`).
- **Mise**: Global tool manager for language runtimes (Rust, Python).
- **Zstd & Tar**: Efficient archival and compression for configuration backups.

---

## Installation

To bootstrap the system on a new machine, clone the repository and run the installation script:

```bash
git clone https://github.com/kuranne/backup.git ~/.local/share/backup
cd ~/.local/share/backup
./install.sh
```

**What the install script does:**

1. Installs Homebrew (if not present).
2. Installs `mise` (for managing Rust, Python, etc.).
3. Compiles the `bkzyn` Rust CLI tool in release mode.
4. Places the binary in `~/.local/bin/bkzyn`.
5. Symlinks the repository to your `$XDG_DATA_HOME/backup` if not already present.
6. Automatically runs `bkzyn setup` to restore your configurations.
7. Installs Oh-My-Zsh and sets Zsh as your default shell.

---

## Quickstart

Once installed, your `bkzyn` CLI is ready. To pull your configurations from your system `~/.config` back into this repository:

```bash
bkzyn backup
```

_This safely archives your repository's `.old` configuration state before pulling in your local changes based on `backup.toml`._

To commit those modifications into Git:

```bash
bkzyn save -m "chore: update dotfiles"
```

---

## How to Use

The `bkzyn` tool serves as an all-in-one system state manager.

### Core Commands

- **`bkzyn setup`**  
  Bootstraps the environment by symlinking `config/*` to `$XDG_CONFIG_HOME`, running `brew bundle` with the provided `Brewfile`, and configuring `/etc/zshenv`.

- **`bkzyn backup`**  
  Reads `backup.toml` and copies modified items from `$XDG_CONFIG_HOME` into the repository. Excludes/includes are heavily respected via glob sets.

- **`bkzyn restore`**  
  Specifically restores the configuration by applying symlinks from the repository `config/` directory into your `$XDG_CONFIG_HOME`.

### Tracking Configurations

Instead of editing `backup.toml` by hand, use the provided subcommands:

- **Track a new application**

  ```bash
  bkzyn add ~/.config/nvim
  ```

  Moves the directory into the repository, symlinks it back to `~/.config/nvim`, and registers it in `backup.toml`.

- **Add an Include Pattern**

  ```bash
  bkzyn include nvim "*.lua"
  ```

  Updates `backup.toml` to always include `.lua` files for `nvim`.

- **Add an Exclude Pattern**
  ```bash
  bkzyn exclude nvim ".git"
  ```
  Updates `backup.toml` to explicitly ignore the `.git` folder in `nvim` when running `bkzyn backup`.

### Templating

`bkzyn restore` automatically supports dynamic configurations via the `minijinja` templating engine.

1. **Create Host Variables**: Define your variables in `$XDG_CONFIG_HOME/backup/host.toml`:
   ```toml
   font_size = 14
   theme = "dark"
   ```
2. **Template files**: In your repository (`config/` folder), rename a file with `.tmpl` (e.g., `alacritty.toml.tmpl`).
3. **Use variables**: Inside the file, use `minijinja` syntax:
   ```toml
   size = {{ host.font_size }}
   ```
When `bkzyn restore` runs, it will parse the template, inject the values, and render it to `~/.config/.../alacritty.toml` without `.tmpl` in the filename.

### Version Control

- **`bkzyn save [-m "message"]`**  
  Automates the `git add` and `git commit` process directly from the CLI.

---

## Configuration (`backup.toml`)

The configuration lives at `$XDG_CONFIG_HOME/backup/backup.toml` (or at the repository root). Example structure:

```toml
configs = ["git", "nvim", "zsh", "tmux"]

[nvim]
exclude = [".git", "plugged"]

[zsh]
include = [".z*", "*.zsh"]
exclude = [".zcompdump*"]
```
