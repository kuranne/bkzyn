# ==============================================================================
# ENVIRONMENT VARIABLES & PATHS (.zshenv)
# ==============================================================================

# --- 1. System & Tool Environment Variables ---
export LANG="en_US.UTF-8"

# ---------- XDG base directories ----------
# Centralizes config/cache/data locations
export XDG_CONFIG_HOME="$HOME/.config"
export XDG_CACHE_HOME="$HOME/.cache"
export XDG_DATA_HOME="$HOME/.local/share"
export XDG_STATE_HOME="$HOME/.local/state"

# ---------- XDG compliance overrides ----------
export CARGO_HOME="$XDG_DATA_HOME/cargo"
export RUSTUP_HOME="$XDG_DATA_HOME/rustup"
export GRADLE_USER_HOME="$XDG_DATA_HOME/gradle"
export GNUPGHOME="$XDG_DATA_HOME/gnupg"
export DOCKER_CONFIG="$XDG_CONFIG_HOME/docker"
export NPM_CONFIG_USERCONFIG="$XDG_CONFIG_HOME/npm/npmrc"
export NPM_CONFIG_CACHE="$XDG_CACHE_HOME/npm"
export BUN_INSTALL="$XDG_DATA_HOME/bun"
export PYTHON_HISTORY="$XDG_STATE_HOME/python/history"
export IPYTHONDIR="$XDG_CONFIG_HOME/ipython"
export JUPYTER_CONFIG_DIR="$XDG_CONFIG_HOME/jupyter"
export MYPY_CACHE_DIR="$XDG_CACHE/.mypy_cache"
export MPLCONFIGDIR="$XDG_CONFIG_HOME/matplotlib"
export NODE_REPL_HISTORY="$XDG_STATE_HOME/node_repl_history"
export COLIMA_HOME="$XDG_DATA_HOME/colima"
export ANDROID_USER_HOME="$XDG_CONFIG_HOME/android"
export ANDROID_EMULATOR_HOME="$XDG_CONFIG_HOME/android"
export ANDROID_AVD_HOME="$XDG_DATA_HOME/android/avd"
export GOPATH="$XDG_DATA_HOME/go"
export MONO_REGISTRY_PATH="$XDG_CONFIG_HOME/mono/registry"
export OLLAMA_MODELS="$XDG_DATA_HOME/ollama/models"
export VSCODE_EXTENSIONS="$XDG_DATA_HOME/vscode/extensions"
export LLDBINIT="$XDG_CONFIG_HOME/lldb/lldbinit"
export SWIFTPM_CACHE_DIR="$XDG_CACHE_HOME/swiftpm"
export PASSWORD_STORE_DIR="$XDG_DATA_HOME/password-store"

# ---------- Editor ----------
# Default editor used by git, crontab, etc.
export EDITOR="nvim"
export VISUAL="vim"

# ---------- Other Customize -----------
export ZSH_TMUX_CONF="$XDG_CONFIG_HOME/tmux/tmux.conf"
export FZF_DEFAULT_OPTS="--height 75% --layout=reverse --border --inline-info"

# --- PATH Management ---
typeset -U path fpath

# Identify Homebrew location dynamically but safely
if [[ -d "/opt/homebrew" ]]; then
    BREW_PREFIX="/opt/homebrew"
elif [[ -d "/usr/local" ]]; then
    BREW_PREFIX="/usr/local"
fi

path=(
  $HOME/.local/bin
  $CARGO_HOME/bin
  $HOME/.nix-profile/bin
  /run/current-system/sw/bin
  $BREW_PREFIX/sbin
  $BREW_PREFIX/opt/openjdk/bin
  $BREW_PREFIX/opt/llvm/bin
  $BREW_PREFIX/opt/bison/bin
  $BREW_PREFIX/bin
  $path
)
export PATH

# GPG Settings (Requires an interactive TTY)
export GPG_TTY=$(tty)
