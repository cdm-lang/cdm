#!/bin/sh
set -e

# CDM CLI Installer for Unix/Linux/macOS
# Usage: curl -fsSL https://raw.githubusercontent.com/cdm-lang/cdm/main/install.sh | sh

REPO="cdm-lang/cdm"
MANIFEST_URL="https://raw.githubusercontent.com/cdm-lang/cdm/main/cli-releases.json"
INSTALL_DIR="${CDM_INSTALL_DIR:-$HOME/.cdm/bin}"
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
    exit 1
}

# Detect platform and architecture
detect_platform() {
    local os
    local arch

    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Darwin)
            case "$arch" in
                x86_64)
                    echo "x86_64-apple-darwin"
                    ;;
                arm64|aarch64)
                    echo "aarch64-apple-darwin"
                    ;;
                *)
                    error "Unsupported architecture: $arch"
                    ;;
            esac
            ;;
        Linux)
            case "$arch" in
                x86_64)
                    echo "x86_64-unknown-linux-gnu"
                    ;;
                aarch64|arm64)
                    echo "aarch64-unknown-linux-gnu"
                    ;;
                *)
                    error "Unsupported architecture: $arch"
                    ;;
            esac
            ;;
        *)
            error "Unsupported operating system: $os"
            ;;
    esac
}

# Check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Fetch the latest version from manifest
get_latest_version() {
    if command_exists curl; then
        curl -fsSL "$MANIFEST_URL" | grep -o '"latest"[[:space:]]*:[[:space:]]*"[^"]*"' | sed 's/.*"\([^"]*\)".*/\1/'
    elif command_exists wget; then
        wget -qO- "$MANIFEST_URL" | grep -o '"latest"[[:space:]]*:[[:space:]]*"[^"]*"' | sed 's/.*"\([^"]*\)".*/\1/'
    else
        error "Neither curl nor wget found. Please install one of them."
    fi
}

# Download file with progress
download() {
    local url="$1"
    local output="$2"

    if command_exists curl; then
        curl -fSL --progress-bar "$url" -o "$output"
    elif command_exists wget; then
        wget --show-progress -qO "$output" "$url"
    else
        error "Neither curl nor wget found. Please install one of them."
    fi
}

# Verify checksum
verify_checksum() {
    local file="$1"
    local expected_checksum="$2"
    local actual_checksum

    if command_exists sha256sum; then
        actual_checksum="$(sha256sum "$file" | awk '{print $1}')"
    elif command_exists shasum; then
        actual_checksum="$(shasum -a 256 "$file" | awk '{print $1}')"
    else
        warn "Neither sha256sum nor shasum found. Skipping checksum verification."
        return 0
    fi

    if [ "$actual_checksum" != "$expected_checksum" ]; then
        error "Checksum verification failed!\nExpected: $expected_checksum\nActual:   $actual_checksum"
    fi
}

# Main installation function
main() {
    info "Installing CDM CLI..."

    # Detect platform
    local platform
    platform="$(detect_platform)"
    info "Detected platform: $platform"

    # Get latest version
    local version
    version="$(get_latest_version)"
    if [ -z "$version" ]; then
        error "Failed to fetch latest version"
    fi
    info "Latest version: $version"

    # Construct download URLs
    local tag="cdm-cli-v${version}"
    local binary_url="https://github.com/${REPO}/releases/download/${tag}/cdm-${platform}"
    local checksum_url="https://github.com/${REPO}/releases/download/${tag}/cdm-${platform}.sha256"

    # Create temporary directory
    local tmp_dir
    tmp_dir="$(mktemp -d)"
    trap 'rm -rf "$tmp_dir"' EXIT

    local tmp_binary="${tmp_dir}/${BINARY_NAME}"
    local tmp_checksum="${tmp_dir}/${BINARY_NAME}.sha256"

    # Download binary
    info "Downloading CDM CLI v${version}..."
    download "$binary_url" "$tmp_binary"

    # Download checksum
    info "Downloading checksum..."
    download "$checksum_url" "$tmp_checksum"

    # Verify checksum
    info "Verifying checksum..."
    local expected_checksum
    expected_checksum="$(cat "$tmp_checksum")"
    verify_checksum "$tmp_binary" "$expected_checksum"

    # Make binary executable
    chmod +x "$tmp_binary"

    # Create installation directory
    mkdir -p "$INSTALL_DIR"

    # Move binary to installation directory
    local install_path="${INSTALL_DIR}/${BINARY_NAME}"
    mv "$tmp_binary" "$install_path"

    # On macOS, remove quarantine extended attributes to prevent Gatekeeper issues.
    # Downloaded binaries get a com.apple.quarantine attribute that can cause
    # "killed" errors when executed without proper Developer ID signing.
    if [ "$(uname -s)" = "Darwin" ]; then
        xattr -c "$install_path" 2>/dev/null || true
    fi

    info "CDM CLI v${version} installed successfully!"
    printf "\n"
    info "Binary location: ${install_path}"
    printf "\n"

    # Setup shell configuration (PATH and completions)
    setup_shell "$install_path"
}

# Detect the user's shell and config file
detect_shell_config() {
    local shell_name=""
    local config_file=""

    # Detect user's preferred shell
    # When run via curl | sh, we're executing in sh/bash, not the user's interactive shell
    # So prioritize SHELL environment variable over VERSION variables
    case "$SHELL" in
        */bash)
            shell_name="bash"
            ;;
        */zsh)
            shell_name="zsh"
            ;;
        */fish)
            shell_name="fish"
            ;;
        *)
            # Fallback to checking version variables if SHELL is not set or unrecognized
            if [ -n "$ZSH_VERSION" ]; then
                shell_name="zsh"
            elif [ -n "$BASH_VERSION" ]; then
                shell_name="bash"
            else
                # Shell not detected
                return 1
            fi
            ;;
    esac

    # Determine config file
    case "$shell_name" in
        bash)
            # Check common bash config files in order of preference
            if [ -f "$HOME/.bashrc" ]; then
                config_file="$HOME/.bashrc"
            elif [ -f "$HOME/.bash_profile" ]; then
                config_file="$HOME/.bash_profile"
            elif [ -f "$HOME/.profile" ]; then
                config_file="$HOME/.profile"
            else
                config_file="$HOME/.bashrc"
            fi
            ;;
        zsh)
            if [ -f "$HOME/.zshrc" ]; then
                config_file="$HOME/.zshrc"
            else
                config_file="$HOME/.zshrc"
            fi
            ;;
        fish)
            config_file="$HOME/.config/fish/config.fish"
            ;;
        *)
            return 1
            ;;
    esac

    # Return shell name and config file (using a convention: shell:config)
    echo "${shell_name}:${config_file}"
    return 0
}

# Add a line to a config file if it doesn't already exist
add_to_config() {
    local config_file="$1"
    local line="$2"
    local comment="$3"

    # Create config file if it doesn't exist
    if [ ! -f "$config_file" ]; then
        mkdir -p "$(dirname "$config_file")"
        touch "$config_file"
    fi

    # Check if line already exists (with some flexibility for whitespace)
    if grep -qF "$line" "$config_file" 2>/dev/null; then
        return 0
    fi

    # Add the line with a comment
    {
        echo ""
        echo "# $comment"
        echo "$line"
    } >> "$config_file"

    return 0
}

# Setup shell configuration (PATH and completions)
setup_shell() {
    local binary="$1"
    local shell_config
    local shell_name
    local config_file
    local modified=0

    # Check if user wants to skip automatic modifications
    if [ -n "$CDM_NO_MODIFY_PATH" ]; then
        info "Skipping automatic shell configuration (CDM_NO_MODIFY_PATH is set)"
        print_manual_instructions
        return 0
    fi

    # Detect shell and config file
    shell_config="$(detect_shell_config)"
    if [ $? -ne 0 ] || [ -z "$shell_config" ]; then
        warn "Could not detect shell configuration file"
        print_manual_instructions
        return 0
    fi

    shell_name="${shell_config%%:*}"
    config_file="${shell_config#*:}"

    info "Detected shell: $shell_name (config: $config_file)"

    # Add PATH if not already in PATH
    case ":$PATH:" in
        *":$INSTALL_DIR:"*)
            info "Installation directory already in PATH"
            ;;
        *)
            if add_to_config "$config_file" "export PATH=\"\$PATH:${INSTALL_DIR}\"" "CDM CLI - added by install script"; then
                info "Added $INSTALL_DIR to PATH in $config_file"
                modified=1
            fi
            ;;
    esac

    # Install completions and setup
    install_completions "$binary" "$shell_name" "$config_file"
    if [ $? -eq 0 ]; then
        modified=1
    fi

    # Print next steps
    printf "\n"
    if [ $modified -eq 1 ]; then
        info "Shell configuration updated!"

        # Clear zsh completion cache for fresh installs
        if [ "$shell_name" = "zsh" ]; then
            rm -f "$HOME/.zcompdump"* 2>/dev/null
        fi

        printf "\n"
        info "To reload your shell and activate cdm, copy and paste this command:"
        printf "\n"
        printf "  exec %s\n" "$shell_name"
        printf "\n"
    fi
    info "Installation complete! Run: cdm --help"
}

# Install shell completions
install_completions() {
    local binary="$1"
    local shell_name="$2"
    local config_file="$3"
    local completion_dir
    local completion_file

    # Determine completion directory based on shell
    case "$shell_name" in
        bash)
            # Try common bash completion directories
            if [ -d "$HOME/.local/share/bash-completion/completions" ]; then
                completion_dir="$HOME/.local/share/bash-completion/completions"
            else
                completion_dir="$HOME/.bash_completion.d"
                mkdir -p "$completion_dir"
            fi
            completion_file="${completion_dir}/cdm"
            ;;
        zsh)
            completion_dir="$HOME/.zsh/completions"
            mkdir -p "$completion_dir"
            completion_file="${completion_dir}/_cdm"
            ;;
        fish)
            completion_dir="$HOME/.config/fish/completions"
            mkdir -p "$completion_dir"
            completion_file="${completion_dir}/cdm.fish"
            ;;
        *)
            return 1
            ;;
    esac

    # Clean up any old completion files from other shells to prevent conflicts
    case "$shell_name" in
        zsh)
            # Remove bash completion if it exists (can conflict via bash_completion sourcing)
            rm -f "$HOME/.bash_completion.d/cdm" 2>/dev/null
            rm -f "$HOME/.local/share/bash-completion/completions/cdm" 2>/dev/null
            ;;
        bash)
            # Remove zsh completion if it exists
            rm -f "$HOME/.zsh/completions/_cdm" 2>/dev/null
            ;;
    esac

    # Generate and install completion
    if ! "$binary" completions "$shell_name" > "$completion_file" 2>/dev/null; then
        warn "Failed to generate completions for $shell_name"
        return 1
    fi

    info "Installed ${shell_name} completions to ${completion_file}"

    # Setup completion loading for zsh
    if [ "$shell_name" = "zsh" ]; then
        local needs_fpath=0
        local needs_compinit=0
        local has_compinit_line=0

        # Check if fpath setup exists
        if ! grep -q "fpath=.*\.zsh/completions" "$config_file" 2>/dev/null; then
            needs_fpath=1
        fi

        # Check if compinit exists
        if grep -q "compinit" "$config_file" 2>/dev/null; then
            has_compinit_line=1
        else
            needs_compinit=1
        fi

        # Add fpath if needed
        # Important: fpath must be set BEFORE compinit is called
        if [ $needs_fpath -eq 1 ]; then
            if [ $has_compinit_line -eq 1 ]; then
                # Insert fpath before the first compinit line
                local compinit_line
                compinit_line=$(grep -n "compinit" "$config_file" | head -1 | cut -d: -f1)
                # Use a temporary file to insert before compinit
                local tmp_file
                tmp_file=$(mktemp)
                {
                    head -n $((compinit_line - 1)) "$config_file"
                    echo ""
                    echo "# CDM CLI completions - added by install script"
                    echo "fpath=(~/.zsh/completions \$fpath)"
                    tail -n +$compinit_line "$config_file"
                } > "$tmp_file"
                mv "$tmp_file" "$config_file"
            else
                # No compinit yet, just add it normally
                add_to_config "$config_file" "fpath=(~/.zsh/completions \$fpath)" "CDM CLI completions - added by install script"
            fi
        fi

        # Add compinit if needed (only if it doesn't exist at all)
        if [ $needs_compinit -eq 1 ]; then
            add_to_config "$config_file" "autoload -Uz compinit && compinit" "Initialize zsh completions - added by install script"
        fi
    fi

    # Setup completion loading for bash
    if [ "$shell_name" = "bash" ] && [ "$completion_dir" = "$HOME/.bash_completion.d" ]; then
        # Check if bash completion loading exists
        if ! grep -q "\.bash_completion\.d" "$config_file" 2>/dev/null; then
            add_to_config "$config_file" "for f in ~/.bash_completion.d/*; do source \$f; done" "Load bash completions - added by install script"
        fi
    fi

    return 0
}

# Print manual installation instructions
print_manual_instructions() {
    printf "\n"
    printf "Manual setup required:\n"
    printf "\n"
    printf "1. Add CDM to your PATH by adding this to your shell profile:\n"
    printf "   export PATH=\"\$PATH:${INSTALL_DIR}\"\n"
    printf "\n"
    printf "2. For completions, run the appropriate command:\n"
    printf "   cdm completions bash > ~/.bash_completion.d/cdm  # bash\n"
    printf "   cdm completions zsh > ~/.zsh/completions/_cdm    # zsh\n"
    printf "   cdm completions fish > ~/.config/fish/completions/cdm.fish  # fish\n"
    printf "\n"
    printf "3. Restart your shell or source your config file\n"
    printf "\n"
}

main "$@"
