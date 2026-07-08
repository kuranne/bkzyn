#!/usr/bin/env bash
set -e

echo "==== E2E Test Suite for bkzyn ===="

# Build first in the mounted directory so we can use target/debug/bkzyn
echo "--> Compiling bkzyn..."
touch src/*.rs src/**/*.rs 2>/dev/null || true
cargo build
BKZYN_BIN="$PWD/target/debug/bkzyn"

# Set up an isolated test environment to avoid modifying host files
E2E_DIR="/tmp/bkzyn_e2e"
rm -rf "$E2E_DIR"
cp -r "$PWD" "$E2E_DIR"
cd "$E2E_DIR"
echo "" > backup.toml

export XDG_CONFIG_HOME="$E2E_DIR/mock_xdg_config"
export XDG_DATA_HOME="$E2E_DIR/mock_xdg_data"
mkdir -p "$XDG_CONFIG_HOME" "$XDG_DATA_HOME"

# Re-init git to avoid messing with the host repo and allow clean history testing
rm -rf .git data
mkdir data
git init -b main
git config --global user.email "test@example.com"
git config --global user.name "E2E Test"
git config --global commit.gpgsign false
git add .
git commit -m "Initial commit"

# Function to run bkzyn
run_bkzyn() {
    "$BKZYN_BIN" "$@"
}

echo "--> 1. Initialize Dummy Data"
mkdir -p "$XDG_CONFIG_HOME/myapp"
echo "setting = true" > "$XDG_CONFIG_HOME/myapp/config.toml"

echo "--> 2. bkzyn add"
run_bkzyn add "$XDG_CONFIG_HOME/myapp"
# Verification: the file should now be in the repo
if [ ! -d "$XDG_CONFIG_HOME/myapp" ]; then
    echo "Error: myapp original directory missing"
    exit 1
fi
if [ ! -f "data/config/myapp/config.toml" ]; then
    echo "Error: myapp not found in repo"
    exit 1
fi

echo "--> 3. bkzyn ignore & deep add"
run_bkzyn ignore "$XDG_CONFIG_HOME/myapp/.git"
# Check backup.toml was updated
grep -q "\".git\"" backup.toml || (echo "Ignore failed" && exit 1)

mkdir -p "$XDG_CONFIG_HOME/myapp/deep"
echo "deep" > "$XDG_CONFIG_HOME/myapp/deep/file.txt"
run_bkzyn add "$XDG_CONFIG_HOME/myapp/deep/file.txt"
grep -q "\"deep/file.txt\"" backup.toml || (echo "Deep add failed" && exit 1)

echo "--> 3.5. bkzyn remove"
run_bkzyn rm "$XDG_CONFIG_HOME/myapp/deep/file.txt"
grep -q "\"deep/file.txt\"" backup.toml && (echo "Remove failed to clear toml" && exit 1)
if [ -f "data/config/myapp/deep/file.txt" ]; then
    echo "Error: Remove failed to clear from repo"
    exit 1
fi
# Host file should still exist
if [ ! -f "$XDG_CONFIG_HOME/myapp/deep/file.txt" ]; then
    echo "Error: Remove accidentally deleted host file"
    exit 1
fi

echo "--> 4. bkzyn backup"
# Modify source config (by removing it first so we can simulate real disconnected changes)
rm -rf "$XDG_CONFIG_HOME/myapp"
mkdir -p "$XDG_CONFIG_HOME/myapp"
echo "setting = false" > "$XDG_CONFIG_HOME/myapp/config.toml"
run_bkzyn backup
# Verify repo is updated
grep -q "setting = false" data/config/myapp/config.toml || (echo "Backup failed to copy changes" && exit 1)

echo "--> 5. bkzyn status"
run_bkzyn status

echo "--> 6. bkzyn save"
run_bkzyn save -m "test save"
git -C data log -1 | grep -q "test save" || (echo "Save failed to create commit" && exit 1)

echo "--> 7. bkzyn log"
run_bkzyn log

echo "--> 8. bkzyn sync"
# Create a dummy remote and set upstream so sync works
rm -rf /tmp/dummy_remote.git
git init --bare /tmp/dummy_remote.git
git -C data remote add origin /tmp/dummy_remote.git
git -C data push -u origin HEAD
run_bkzyn sync

echo "--> 9. bkzyn restore"
# Delete local config and restore
rm -rf "$XDG_CONFIG_HOME/myapp"
run_bkzyn restore
if [ ! -f "$XDG_CONFIG_HOME/myapp/config.toml" ]; then
    echo "Error: Restore failed"
    exit 1
fi

echo "--> 10. bkzyn rollback"
# Modify again, save, then rollback
echo "setting = modified" > "$XDG_CONFIG_HOME/myapp/config.toml"
run_bkzyn backup
run_bkzyn save -m "bad commit"
run_bkzyn rollback HEAD~1
# Verify we rolled back
grep -q "setting = false" data/config/myapp/config.toml || (echo "Rollback failed" && exit 1)

echo ""
echo "==== All E2E tests passed successfully! ===="
