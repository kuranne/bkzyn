# ==============================================================================
# BINDING
# ==============================================================================

copy-command() {
    echo -n $BUFFER | pbcopy
    zle -M "Copied to clipboard"
}
zle -N copy-command
bindkey '^Xc' copy-command