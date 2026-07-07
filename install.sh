#!/usr/bin/env bash
set -e

echo "--> Installing bkzyn CLI tool..."

DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/bkzyn"
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

if ! command -v cargo >/dev/null 2>&1; then
    echo "--> Error: 'cargo' is not installed. Please install Rust or run setup.sh first."
    exit 1
fi

echo "--> Compiling bkzyn..."
cargo build --release

echo "--> Installing bkzyn binary to ~/.local/bin..."
if [ ! -d "$HOME/.local/bin" ]; then
    mkdir -p "$HOME/.local/bin"
fi
cp target/release/bkzyn "$HOME/.local/bin/bkzyn"

echo "--> bkzyn CLI installed successfully! Make sure ~/.local/bin is in your PATH."

