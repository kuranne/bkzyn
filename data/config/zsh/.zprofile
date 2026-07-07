# ==============================================================================
# LOGIN SHELL CONFIGURATION (.zprofile)
# ==============================================================================

# --- Homebrew / Compilers Environment ---
if command -v brew > /dev/null; then
    eval "$($(brew --prefix)/bin/brew shellenv)"

    brew_prefix_tcl=$(brew --prefix tcl-tk 2>/dev/null)
    if [[ -n "$brew_prefix_tcl" ]]; then
        export LDFLAGS="-L$brew_prefix_tcl/lib $LDFLAGS"
        export CPPFLAGS="-I$brew_prefix_tcl/include $CPPFLAGS"
        export PKG_CONFIG_PATH="$brew_prefix_tcl/lib/pkgconfig:$PKG_CONFIG_PATH"
    fi
fi

# --- GNUPG SSH Control ---

export SSH_AUTH_SOCK=$(gpgconf --list-dirs agent-ssh-socket)
gpgconf --launch gpg-agent