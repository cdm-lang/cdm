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

    info "CDM CLI v${version} installed successfully!"
    printf "\n"
    info "Binary location: ${install_path}"
    printf "\n"

    # Check if install directory is in PATH
    case ":$PATH:" in
        *":$INSTALL_DIR:"*)
            info "You can now run: cdm --help"
            ;;
        *)
            warn "Installation directory is not in your PATH"
            printf "\n"
            printf "Add the following line to your shell profile (~/.bashrc, ~/.zshrc, etc.):\n"
            printf "\n"
            printf "    export PATH=\"\$PATH:${INSTALL_DIR}\"\n"
            printf "\n"
            printf "Then restart your shell or run:\n"
            printf "\n"
            printf "    source ~/.bashrc  # or ~/.zshrc\n"
            printf "\n"
            printf "Alternatively, run CDM with the full path:\n"
            printf "\n"
            printf "    ${install_path} --help\n"
            ;;
    esac
}

main "$@"
