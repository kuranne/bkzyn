#!/usr/bin/env bash
ALREADYINSTALLED="is already installed, skip"
set -e

echo "--> Bootstrapping bkzyn backup system..."

# 1. install homebrew first
if ! command -v brew >/dev/null 2>&1; then
    echo "--> Installing Homebrew..."
    NONINTERACTIVE=1 /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
else
    echo "--> Homebrew $ALREADYINSTALLED"
fi

# Always ensure brew is in PATH for this script session
if [ -f "/opt/homebrew/bin/brew" ]; then
    eval "$(/opt/homebrew/bin/brew shellenv)"
elif [ -f "/usr/local/bin/brew" ]; then
    eval "$(/usr/local/bin/brew shellenv)"
fi

# 2. install mise via homebrew
if ! command -v mise >/dev/null 2>&1; then
    echo "--> Installing mise..."
    brew install mise
else
    echo "--> mise $ALREADYINSTALLED"
fi

# 3. use mise to install rust and python
echo "--> Installing rust and python via mise..."
mise use -g rust@latest 1> /dev/null
mise use -g python@latest 1> /dev/null

# 4. install oh-my-zsh
if [ ! -d "$HOME/.oh-my-zsh" ]; then
    echo "--> Installing oh-my-zsh..."
    sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)" "" --unattended
else
    echo "--> Oh-my-Zsh! $ALREADYINSTALLED"
fi

# 5. link the current repository to $XDG_DATA_HOME/backup
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/backup"
if [ "$PWD" != "$DATA_DIR" ]; then
    echo "--> Linking current repository to $DATA_DIR..."
    if [ -e "$DATA_DIR" ] && [ ! -L "$DATA_DIR" ]; then
        echo "--> WARNING: $DATA_DIR already exists and is not a symlink."
        echo "--> Proceeding, but bkzyn might read from the old directory."
    else
        mkdir -p "$(dirname "$DATA_DIR")"
        ln -snf "$PWD" "$DATA_DIR"
    fi
fi

# 6. compile command line
echo "--> Compiling bkzyn..."
cargo build --release

# 7. move binary to ~/.local/bin
echo "--> Installing bkzyn binary to ~/.local/bin..."
if [ ! -d "$HOME/.local/bin" ]; then
    mkdir -p "$HOME/.local/bin"
fi
cp target/release/bkzyn "$HOME/.local/bin/bkzyn"

# 8. Use the command line to set up
echo "--> Running bkzyn setup..."
export PATH="$HOME/.local/bin:$PATH"

bkzyn setup

echo "--> Setup complete!"
