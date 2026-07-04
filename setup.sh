#!/usr/bin/env bash

# ==============================================================================
# macOS Bootstrap & Setup Script
# Highly optimized for restoring environments from Backup.
# ==============================================================================

# Exit immediately if a command exits with a non-zero status
set -e

# Get the absolute path of this backup repository directory
REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=========================================="
echo " Starting macOS Environment Setup Script  "
echo "=========================================="

# 1. Install Homebrew if not already installed
if ! command -v brew >/dev/null 2>&1; then
    echo "--> Homebrew not found. Installing Homebrew..."
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
    
    # Configure Homebrew for the current script session (Apple Silicon and Intel paths)
    if [ -f "/opt/homebrew/bin/brew" ]; then
        eval "$(/opt/homebrew/bin/brew shellenv)"
    elif [ -f "/usr/local/bin/brew" ]; then
        eval "$(/usr/local/bin/brew shellenv)"
    fi
else
    echo "--> Homebrew is already installed."
    eval "$(brew shellenv)"
fi

# Ensure brew command is available in active shell environment
if ! command -v brew >/dev/null 2>&1; then
    echo "Error: Homebrew installation failed or not found in PATH." >&2
    exit 1
fi

# 2. Install mise first
echo "--> Installing mise..."
brew install mise

# 3. Create ~/.config directory and symlink configuration folders
echo "--> Linking configuration folders to ~/.config..."
mkdir -p "$HOME/.config"

for dir in "$REPO_DIR/config/"*; do
    if [ -d "$dir" ]; then
        name=$(basename "$dir")
        target="$HOME/.config/$name"
        
        # Safe check: if target exists and is a physical directory (not a symlink), back it up
        if [ -e "$target" ] && [ ! -L "$target" ]; then
            echo "    [Backup] Moving existing physical folder/file $target to ${target}.bak"
            mv "$target" "${target}.bak"
        fi
        
        echo "    [Link] $target -> $dir"
        ln -sfn "$dir" "$target"
    fi
done

# 4. Run mise install to pre-install all runtimes (java, node, python, rust, zig)
echo "--> Pre-installing all mise runtimes/packages..."
if command -v mise >/dev/null 2>&1; then
    # Run mise install (using config from the linked directory ~/.config/mise/config.toml)
    # --yes/--non-interactive flags to ensure no prompts block the script
    mise install --yes
else
    echo "Error: mise command not found after installation." >&2
    exit 1
fi

# 5. Bootstrap Zsh ZDOTDIR configuration in /etc/zshenv
echo "--> Checking Zsh bootstrap config in /etc/zshenv..."
ZSHENV_SNIPPET=$(cat << 'EOF'

# --- XDG & ZDOTDIR bootstrap ---
if [[ -z "$XDG_CONFIG_HOME" ]]; then
    export XDG_CONFIG_HOME="$HOME/.config"
fi

if [[ -d "$XDG_CONFIG_HOME/zsh" ]]; then
    export ZDOTDIR="$XDG_CONFIG_HOME/zsh"
fi
EOF
)

if [ ! -f /etc/zshenv ] || ! grep -q "ZDOTDIR" /etc/zshenv; then
    echo "    Adding ZDOTDIR bootstrap to /etc/zshenv (requires sudo)..."
    echo "$ZSHENV_SNIPPET" | sudo tee -a /etc/zshenv > /dev/null
else
    echo "    ZDOTDIR configuration already present in /etc/zshenv."
fi

# 6. Install remaining Homebrew formulae via Brewfile
if [ -f "$REPO_DIR/Brewfile" ]; then
    echo "--> Installing remaining Homebrew packages from Brewfile..."
    # brew bundle will check/verify 'mise' since it is already installed, and install the rest
    brew bundle --file="$REPO_DIR/Brewfile"
else
    echo "Warning: Brewfile not found at $REPO_DIR/Brewfile"
fi

echo "=========================================="
echo " Setup completed successfully!            "
echo " Please restart your terminal/shell.      "
echo "=========================================="
