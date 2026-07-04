#!/usr/bin/env bash
set -e

# Get the directory of the script and go to parent (project root)
cd "$(dirname "$0")/.."

echo "--> Building bkzyn..."
cargo build --release
echo "--> Build finished. Binary is at target/release/bkzyn"
