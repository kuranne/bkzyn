# Logic & Workflows

## Concept

A centralized backup repository for managing, restoring, and updating macOS configuration files (dotfiles). It targets XDG compliance and automates environment bootstrapping.

---

## Workflows

### 1. Bootstrap Setup

Automates the configuration and package installation on macOS:

- **Homebrew**: Installs Homebrew and sets up shell environments.
- **Mise**: Installs `mise` tool manager.
- **Configuration Symlinks**: Safe-links config directories from `./config` to `$HOME/.config/`. Backs up pre-existing configurations as `.bak`.
- **Mise Install**: Pre-installs configured language runtimes non-interactively.
- **Zsh Setup**: Bootstraps `$XDG_CONFIG_HOME` and `$ZDOTDIR` in `/etc/zshenv`.
- **Brew Bundle**: Installs packages listed in `Brewfile`.

### 2. Backup Sync

Synchronizes changes from local system configurations back into the repository:

- **Config Resolution**: Determines the current config root path (`$XDG_CONFIG_HOME` or `$HOME/.config`).
- **Selective Sync**: For each folder tracked in `backup.toml`, copies modified items from local config back into the repository config directory, ensuring only tracked assets are updated.

## Purpose

All in one cli tool which write in Rust, for backup dotfiles, data and some other important thing and setup on another device.

There must

1. A shell script for directly curl content from [github](https://github.com/kuranne/backup.git)
   - In shell script (write in bash): install homebrew first then install mise via homebrew.
   - use mise to install rust and python to compile command line
   - install oh-my-zsh
   - clone the github repository to `$XDG_DATA_HOME/backup`, cd then compile.
   - After compiled, move binary to `~/.local/bin`
   - Use the command line to cp `./config/*` to `$XDG_CONFIG_HOME` and rest operator.

2. A command line write from Rust
   - There must split massive function to each file.rs, good for maintain.
   - Core of command such backup sync, setup must write as lib.rs

## Backup command

Name: bkzyn
Subcommand: backup, setup, restore  
flags:
-v verbose
--dry dry run

Use backup.toml as main config file where found on the top of this repository, and make it support for XDG base.

### bkzyn backup

list files in config which one must to back up then copy ./config/ to .old/config\_$date ($date is ISO8601 date format) then use zstd to compress .old/config\_$date.

read config file for include and exclude files/folder then copy them to new ./config/ (In exclude files has same as list files in old backup or same as include files, recarding to exclude first and ignore include and list files, but don't delete old config just keep it)

### bkzyn setup

- brew bundle a Brewfile
- copy config/\* to $XDG_CONFIG_HOME
- add a line in /etc/zshrc or /etc/zshenv to use $ZDOTDIR for zsh

### bkzyn restore

- copy config/\* to $XDG_CONFIG_HOME

## Toml config file

see ./backup.toml for learn config.
