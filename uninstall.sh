#!/bin/sh
set -e

# CDM CLI Uninstaller for Unix/Linux/macOS
# Usage: curl -fsSL https://raw.githubusercontent.com/cdm-lang/cdm/main/uninstall.sh | sh

INSTALL_DIR="${CDM_INSTALL_DIR:-$HOME/.cdm}"
BINARY_NAME="cdm"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() {
    printf "${GREEN}==>${NC} %s\n" "$1"
}

warn() {
    printf "${YELLOW}Warning:${NC} %s\n" "$1"
}

error() {
    printf "${RED}Error:${NC} %s\n" "$1" >&2
}

# Detect current shell
detect_shell() {
    if [ -n "$BASH_VERSION" ]; then
        echo "bash"
    elif [ -n "$ZSH_VERSION" ]; then
        echo "zsh"
    else
        case "$SHELL" in
            */bash) echo "bash" ;;
            */zsh) echo "zsh" ;;
            */fish) echo "fish" ;;
            *) echo "unknown" ;;
        esac
    fi
}

# Remove shell completions
remove_completions() {
    local shell_name
    shell_name="$(detect_shell)"
    local removed=0

    case "$shell_name" in
        bash)
            if [ -f "$HOME/.local/share/bash-completion/completions/cdm" ]; then
                rm -f "$HOME/.local/share/bash-completion/completions/cdm"
                info "Removed bash completions from ~/.local/share/bash-completion/completions/"
                removed=1
            fi
            if [ -f "$HOME/.bash_completion.d/cdm" ]; then
                rm -f "$HOME/.bash_completion.d/cdm"
                info "Removed bash completions from ~/.bash_completion.d/"
                removed=1
            fi
            ;;
        zsh)
            if [ -f "$HOME/.zsh/completions/_cdm" ]; then
                rm -f "$HOME/.zsh/completions/_cdm"
                info "Removed zsh completions from ~/.zsh/completions/"
                removed=1
                printf "\n"
                warn "You may want to remove these lines from ~/.zshrc:"
                printf "    fpath=(~/.zsh/completions \$fpath)\n"
                printf "    autoload -Uz compinit && compinit\n"
                printf "\n"
            fi
            ;;
        fish)
            if [ -f "$HOME/.config/fish/completions/cdm.fish" ]; then
                rm -f "$HOME/.config/fish/completions/cdm.fish"
                info "Removed fish completions from ~/.config/fish/completions/"
                removed=1
            fi
            ;;
    esac

    # Try to remove from all possible locations regardless of detected shell
    for completion_file in \
        "$HOME/.local/share/bash-completion/completions/cdm" \
        "$HOME/.bash_completion.d/cdm" \
        "$HOME/.zsh/completions/_cdm" \
        "$HOME/.config/fish/completions/cdm.fish"; do
        if [ -f "$completion_file" ] && [ "$removed" -eq 0 ]; then
            rm -f "$completion_file"
            info "Removed completions from $completion_file"
            removed=1
        fi
    done

    if [ "$removed" -eq 0 ]; then
        info "No shell completions found"
    fi
}

# Remove plugin cache
remove_cache() {
    local cache_dir

    # Determine cache directory based on platform
    case "$(uname -s)" in
        Darwin)
            cache_dir="$HOME/Library/Caches/cdm"
            ;;
        Linux)
            cache_dir="${XDG_CACHE_HOME:-$HOME/.cache}/cdm"
            ;;
        *)
            cache_dir="${XDG_CACHE_HOME:-$HOME/.cache}/cdm"
            ;;
    esac

    if [ -d "$cache_dir" ]; then
        rm -rf "$cache_dir"
        info "Removed plugin cache from $cache_dir"
    else
        info "No plugin cache found"
    fi
}

# Main uninstall function
main() {
    info "Uninstalling CDM CLI..."
    printf "\n"

    # Check if CDM is installed
    if [ ! -d "$INSTALL_DIR" ] && [ ! -f "$INSTALL_DIR/bin/$BINARY_NAME" ]; then
        warn "CDM CLI does not appear to be installed at $INSTALL_DIR"
        printf "\n"
        info "Checking for completions and cache anyway..."
        remove_completions
        printf "\n"
        remove_cache
        exit 0
    fi

    # Remove the binary and installation directory
    if [ -d "$INSTALL_DIR" ]; then
        rm -rf "$INSTALL_DIR"
        info "Removed CDM CLI from $INSTALL_DIR"
    fi

    # Remove shell completions
    printf "\n"
    remove_completions

    # Remove plugin cache
    printf "\n"
    remove_cache

    printf "\n"
    info "CDM CLI has been uninstalled successfully!"
    printf "\n"
    warn "You may want to remove the following from your shell profile:"
    printf "    export PATH=\"\$PATH:$INSTALL_DIR/bin\"\n"
    printf "\n"
    info "Restart your shell to complete the uninstallation:"
    printf "    exec \$SHELL\n"
}

main "$@"
