# ==============================================================================
# FUNCTIONS & ALIASES
# ==============================================================================

# --- Global Command Replacements & Previews ---

# Native Pager overrides
# if command -v bat >/dev/null 2>&1; then
  # export MANPAGER="bat -l man -p"
# elif command -v batcat >/dev/null 2>&1; then
  # export MANPAGER="batcat -l man -p"
# fi

# eza (Modern ls replacement)
if command -v eza > /dev/null; then
  alias ls='eza --icons --group-directories-first'
  alias ll='eza -l --icons --group-directories-first'
  alias la='eza -la --icons --group-directories-first'
  alias tree='eza -T --icons --group-directories-first'
fi

# fzf (Fuzzy finder integrated with previewer)

if command -v bat > /dev/null; then
  alias f="fzf --preview 'bat --style=numbers --color=always --line-range :500 {}'"
else
  alias f="fzf --preview 'cat {}'"
fi

passc() {
    if [ -z "$1" ]; then
        echo "require 1 agument: ex email/email"
        return 1
    fi

    local password=$(pass "$1" | head -n 1)

    if [ -z "$password" ]; then
        return 1
    fi

    echo "script will paste the password in 5 second."
    sleep 3

    osascript -e "tell application \"System Events\" to keystroke \"$password\""
    unset password
}
