#!/usr/bin/env bash
set -e

echo "--> Installing bkzyn CLI tool..."



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

