#!/usr/bin/env bash
ALREADYINSTALLED="is already installed, skip"
set -e

echo "--> Bootstrapping system environment..."

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
elif [ -f "/home/linuxbrew/.linuxbrew/bin/brew" ]; then
    eval "$(/home/linuxbrew/.linuxbrew/bin/brew shellenv)"
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

# 4. Install bkzyn CLI tool
echo "--> Installing bkzyn CLI tool..."
./install.sh

export PATH="$HOME/.local/bin:$PATH"

# 5. Setup data repository
DATA_DIR="$HOME/.local/share/bkzyn/data"
if [ ! -d "$DATA_DIR" ]; then
    echo "The data/ directory does not exist."
    echo "Do you want to setup data from a backup repository?"
    echo "Enter the Git URL, or press Enter/type 'skip' to skip."
    read -r -p "URL (or 'skip'): " DATA_URL
    if [ -z "$DATA_URL" ] || [ "$(echo "$DATA_URL" | tr '[:upper:]' '[:lower:]')" = "skip" ]; then
        mkdir -p "$DATA_DIR"
        (cd "$DATA_DIR" && git init)
        echo "Created empty data/ directory."
        echo "You can set the github link for data/ later using: bkzyn backup --set-url <url>"
        echo "Skipping further bkzyn setup."
    else
        echo "--> Cloning $DATA_URL into data/..."
        git clone "$DATA_URL" "$DATA_DIR"
        echo "--> Running bkzyn setup..."
        bkzyn setup
    fi
else
    echo "--> Running bkzyn setup..."
    bkzyn setup
fi

# 6. install oh-my-zsh (after zsh is guaranteed installed by brew/bkzyn)
if [ ! -d "$HOME/.oh-my-zsh" ]; then
    echo "--> Installing oh-my-zsh..."
    sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)" "" --unattended
else
    echo "--> Oh-my-Zsh! $ALREADYINSTALLED"
fi

# 7. Change default shell to zsh
if command -v zsh >/dev/null 2>&1; then
    ZSH_PATH="$(command -v zsh)"
    if [ "$SHELL" != "$ZSH_PATH" ]; then
        echo "--> Changing default shell to zsh ($ZSH_PATH)..."
        # Ensure it is in /etc/shells
        if ! grep -Fxq "$ZSH_PATH" /etc/shells; then
            echo "--> Adding $ZSH_PATH to /etc/shells (requires sudo)..."
            echo "$ZSH_PATH" | sudo tee -a /etc/shells > /dev/null
        fi
        
        # Change shell
        chsh -s "$ZSH_PATH" "$USER" || sudo chsh -s "$ZSH_PATH" "$USER"
    fi
fi

echo "--> System setup complete! Please restart your terminal or log out/in."
