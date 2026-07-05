#!/usr/bin/env bash
ALREADYINSTALLED="is already installed, skip"
set -e

echo "--> Bootstrapping bkzyn backup system..."

# 1. install homebrew first
if ! command -v brew >/dev/null 2>&1; then
    echo "--> Installing Homebrew..."
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
    if [ -f "/opt/homebrew/bin/brew" ]; then
        eval "$(/opt/homebrew/bin/brew shellenv)"
    elif [ -f "/usr/local/bin/brew" ]; then
        eval "$(/usr/local/bin/brew shellenv)"
    fi
else
    echo "--> Homebrew $ALREADYINSTALLED"
fi

# 2. install mise via homebrew
if ! command -v mise >/dev/null 2>&1; then
    echo "--> Installing mise..."
    brew install -y mise
else
    echo "--> mise $ALREADYINSTALLED"
fi

# 3. use mise to install rust and python
echo "--> Installing rust and python via mise..."
mise use -g rust@latest 1> /dev/null
mise use -g python@latest 1> /dev/null

# 4. install oh-my-zsh
if [ ! command -v omz &> /dev/null ]; then
    echo "--> Installing oh-my-zsh..."
    sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)" "" --unattended
else
    echo "--> Oh-my-Zsh! $ALREADYINSTALLED"
fi

# 5. clone the github repository to $XDG_DATA_HOME/backup
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/backup"
if [ ! -d "$DATA_DIR" ]; then
    echo "--> Cloning repository to $DATA_DIR..."
    git clone https://github.com/kuranne/backup.git "$DATA_DIR"
else
    echo "--> Repository already exists at $DATA_DIR, pulling latest..."
    cd "$DATA_DIR" && git pull
fi

cd "$DATA_DIR"

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
typedef -U path

path=(
    $HOME/.local/bin
    $path
)
export PATH

bkzyn setup

echo "--> Setup complete!"
