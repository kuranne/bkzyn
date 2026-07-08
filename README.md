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

To completely bootstrap the system on a new machine, you should clone your **personal dotfiles repository** (which contains `setup.sh`) first, and run it:

```bash
git clone git@github.com:kuranne/backup.git ~/.local/share/bkzyn/data
cd ~/.local/share/bkzyn/data
./setup.sh
```

**What your dotfiles' `setup.sh` does:**

1. Installs Homebrew (if not present).
2. Installs `mise` (for managing Rust, Python, etc.).
3. Clones this `bkzyn` CLI repository and installs it by calling its `./install.sh`.
4. Automatically runs `bkzyn setup` to restore your configurations.
5. Installs Oh-My-Zsh and sets Zsh as your default shell.

*Note: If you only want to install the `bkzyn` CLI tool without bootstrapping the rest of the system packages, you can just run `./install.sh` directly (requires `cargo`).*

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
  Bootstraps the environment by copying `config/*` and rendering `.tmpl` files to `$XDG_CONFIG_HOME`, running `brew bundle` with the provided `Brewfile`, and configuring `/etc/zshenv`.

- **`bkzyn backup`**  
  Reads `backup.toml` and copies modified items from `$XDG_CONFIG_HOME` into the repository. Excludes/includes are heavily respected via glob sets.

- **`bkzyn restore`**  

  Specifically restores the configuration by copy files/folders from the repository `config/` directory into your `$XDG_CONFIG_HOME`.

### Tracking Configurations

Instead of editing `backup.toml` by hand, use the provided subcommands:

- **Track a new application**

  ```bash
  bkzyn add ~/.config/nvim
  ```

  Copies the directory into the repository and registers it in `backup.toml` to be tracked.

- **Stop Tracking an application**

  ```bash
  bkzyn remove nvim
  ```
  (Alias: `bkzyn rm`) Reverses `add` by removing the application from `backup.toml` tracking and deleting it from the repository. It does **not** touch your system host files.

- **Add an Include Pattern**

  ```bash
  bkzyn include config nvim "*.lua"
  ```

  Updates `backup.toml` to always include `.lua` files for `nvim`.

- **Add an Exclude Pattern**
  ```bash
  bkzyn ignore config nvim ".git" "*.json"
  ```
  Updates `backup.toml` to explicitly ignore multiple patterns like `.git` when running `bkzyn backup`.

### Templating

`bkzyn restore` automatically supports dynamic configurations via the `minijinja` templating engine.

1. **Create Host Variables**: Define your variables in `$XDG_CONFIG_HOME/bkzyn/host.toml`:
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

The configuration lives at `$XDG_CONFIG_HOME/bkzyn/backup.toml` (or at the repository root). Example structure:

```toml
[config]
whitelists = ["git", "nvim", "zsh", "tmux"]

[config.nvim]
ignores = [".git", "plugged"]

[config.zsh]
whitelists = [".z*", "*.zsh"]
ignores = [".zcompdump*"]
```

---

## Future Plans

We are continuously evolving `bkzyn`. Upcoming features include:

- **Secrets Management**: Integration with lightweight encryption tools like `age`, `sops`, or `git-crypt` so sensitive API keys or SSH configurations can be securely committed.
- **Background Automation**: Adding `--daemon` mode or a systemd/launchd service generator to automatically snapshot your configurations daily.
