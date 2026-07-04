# ==============================================================================
# INTERACTIVE SHELL CONFIGURATION (.zshrc)
# ==============================================================================

typeset -U path fpath

# ==============================================================================
# SHELL SETTING & SERVICES OPTIONS
# ==============================================================================

# --- Shell Options ---
setopt AUTOCD
setopt NOBEEP
setopt NUMERIC_GLOB_SORT

# --- Shell History Settings ---
HISTFILE="$XDG_STATE_HOME/zsh/history"
HISTSIZE=100000
SAVEHIST=100000

[[ ! -d "${HISTFILE:h}" ]] && mkdir -p "${HISTFILE:h}"

setopt APPEND_HISTORY
setopt SHARE_HISTORY
setopt HIST_IGNORE_DUPS
setopt HIST_IGNORE_SPACE
setopt HIST_EXPIRE_DUPS_FIRST
setopt HIST_FIND_NO_DUPS

# ==============================================================================
# COMPLETION & FPATHs
# ==============================================================================

if command -v brew > /dev/null; then
    fpath=(
      $BREW_PREFIX/share/zsh/site-functions
      $BREW_PREFIX/share/zsh-completions
      $DOCKER_CONFIG/completions
      $fpath
    )

    # Define custom plugin sources for later activation
    typeset -U source_files=(
      $XDG_DATA_HOME/go/bin

      $BREW_PREFIX/share/zsh-autosuggestions/zsh-autosuggestions.zsh
      $BREW_PREFIX/share/zsh-syntax-highlighting/zsh-syntax-highlighting.zsh
      $BREW_PREFIX/share/zsh-autopair/autopair.zsh
      
      $BREW_PREFIX/opt/fzf/shell/key-bindings.zsh
      $BREW_PREFIX/opt/fzf/shell/completion.zsh

      # Manual Scripts Extension
      $ZDOTDIR/fzf.zsh
      $ZDOTDIR/aliases.zsh
      $ZDOTDIR/binding.zsh
    )
fi

# --- Oh-My-Zsh Framework ---
export ZSH="$XDG_DATA_HOME/oh-my-zsh"
ZSH_THEME=""

plugins=(
  git
  docker
  docker-compose
  brew 
  extract
  copyfile                  
  sudo
  tmux
  eza 
  node
  command-not-found
)

# OMZ internal script will automatically call compinit using our modified fpath
source $ZSH/oh-my-zsh.sh

# Re-route zcompdump location to follow XDG Base Directory specification
[[ ! -d "$XDG_CACHE_HOME/zsh" ]] && mkdir -p "$XDG_CACHE_HOME/zsh"
compinit -d "$XDG_CACHE_HOME/zsh/zcompdump"

# --- Custom Sourced Plugins (Must source after OMZ/compinit) ---
for s in $source_files; do
  [[ -f "$s" ]] && source "$s"
done

# ==============================================================================
# Cli INTEREACTION SETUP
# ==============================================================================

# mise-en-place (Environment manager)
if command -v mise > /dev/null; then
  eval "$(mise activate zsh)"
fi

# zoxide (Smarter cd command)
if command -v zoxide > /dev/null; then
  eval "$(zoxide init --cmd cd zsh)"
fi

# Starship Prompt Configuration
if [[ "$TERM_PROGRAM" == "vscode" ]]; then
    export STARSHIP_CONFIG="$XDG_CONFIG_HOME/starship/vscode.toml"
elif [[ "$TERM_PROGRAM" == "WarpTerminal" ]]; then
    export STARSHIP_CONFIG="$XDG_CONFIG_HOME/starship/warp.toml"
else
    export STARSHIP_CONFIG="$XDG_CONFIG_HOME/starship/default.toml"
fi

if command -v starship > /dev/null; then
  eval "$(starship init zsh)"
fi

if command -v atuin > /dev/null; then
  eval "$(atuin init zsh)"
fi

# Add this in latest, to ensure that all paths order in correct sequence.
path=(
  $HOME/bin
  $CARGO_HOME/bin
  $HOME/.local/share/notebooklm-py/.venv/bin
  $BREW_PREFIX/sbin
  $BREW_PREFIX/opt/openjdk/bin
  $BREW_PREFIX/opt/llvm/bin
  $BREW_PREFIX/opt/bison/bin
  $BREW_PREFIX/bin
  $path
)
export PATH
