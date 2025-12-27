#!/bin/bash

# CDM Plugin Setup Script
# Checks for required dependencies and helps install them

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Emoji support (works on macOS/Linux)
CHECK="✓"
CROSS="✗"
INFO="ℹ"

echo ""
echo "================================================"
echo "  CDM Plugin Development Setup"
echo "================================================"
echo ""

# Track if any errors occurred
ERRORS=0

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to print success
print_success() {
    echo -e "${GREEN}${CHECK}${NC} $1"
}

# Function to print error
print_error() {
    echo -e "${RED}${CROSS}${NC} $1"
    ERRORS=$((ERRORS + 1))
}

# Function to print info
print_info() {
    echo -e "${BLUE}${INFO}${NC} $1"
}

# Function to print warning
print_warning() {
    echo -e "${YELLOW}!${NC} $1"
}

echo "Checking dependencies..."
echo ""

# Check for Rust
if command_exists rustc; then
    RUST_VERSION=$(rustc --version | cut -d' ' -f2)
    print_success "Rust is installed (version $RUST_VERSION)"
else
    print_error "Rust is not installed"
    print_info "Install from: https://rustup.rs/"
    print_info "Run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
fi

# Check for Cargo
if command_exists cargo; then
    CARGO_VERSION=$(cargo --version | cut -d' ' -f2)
    print_success "Cargo is installed (version $CARGO_VERSION)"
else
    print_error "Cargo is not installed"
    print_info "Cargo is installed with Rust. Visit: https://rustup.rs/"
fi

# Check for WASM target
if command_exists rustup; then
    if rustup target list | grep -q "wasm32-wasip1 (installed)"; then
        print_success "WASM target (wasm32-wasip1) is installed"
    else
        print_warning "WASM target (wasm32-wasip1) is not installed"
        echo ""
        read -p "Would you like to install it now? (y/n) " -n 1 -r
        echo ""
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            print_info "Installing wasm32-wasip1 target..."
            if rustup target add wasm32-wasip1; then
                print_success "WASM target installed successfully"
            else
                print_error "Failed to install WASM target"
            fi
        else
            print_error "WASM target is required to build the plugin"
            print_info "Install manually with: rustup target add wasm32-wasip1"
        fi
    fi
else
    print_error "rustup is not installed"
    print_info "rustup is needed to manage Rust toolchains"
fi

# Check for Make
if command_exists make; then
    print_success "Make is installed"
else
    print_warning "Make is not installed (optional, but recommended)"
    print_info "On macOS: Install Xcode Command Line Tools"
    print_info "On Linux: Install with your package manager (apt install make, yum install make, etc.)"
fi

# Optional: Check for cargo-watch
if command_exists cargo-watch; then
    print_success "cargo-watch is installed (optional, for auto-rebuild)"
else
    print_info "cargo-watch is not installed (optional, enables 'make watch')"
    print_info "Install with: cargo install cargo-watch"
fi

# Optional: Check for wasm-opt (from binaryen)
if command_exists wasm-opt; then
    print_success "wasm-opt is installed (optional, for WASM optimization)"
else
    print_info "wasm-opt is not installed (optional, for smaller WASM files)"
    print_info "Install binaryen from: https://github.com/WebAssembly/binaryen"
fi

echo ""
echo "================================================"

# Summary
if [ $ERRORS -eq 0 ]; then
    print_success "All required dependencies are installed!"
    echo ""
    print_info "You're ready to start developing. Try these commands:"
    echo ""
    echo "  make build         - Build the plugin"
    echo "  make test          - Run tests"
    echo "  make run-example   - Run the example schema"
    echo "  make help          - See all available commands"
    echo ""
else
    print_error "Found $ERRORS error(s). Please install missing dependencies."
    echo ""
    print_info "After installing dependencies, run this script again:"
    echo "  ./setup.sh"
    echo ""
    exit 1
fi

echo "================================================"
echo ""

# Optionally run a test build
read -p "Would you like to run a test build now? (y/n) " -n 1 -r
echo ""
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo ""
    print_info "Running test build..."
    if command_exists make; then
        make build
    else
        cargo build --release --target wasm32-wasip1
    fi

    if [ $? -eq 0 ]; then
        echo ""
        print_success "Test build completed successfully!"
        echo ""

        # Show WASM file size
        WASM_FILE="target/wasm32-wasip1/release/cdm_plugin_sql.wasm"
        if [ -f "$WASM_FILE" ]; then
            SIZE=$(ls -lh "$WASM_FILE" | awk '{print $5}')
            print_info "WASM file size: $SIZE"
            print_info "Location: $WASM_FILE"
        fi
    else
        echo ""
        print_error "Test build failed. Check the error messages above."
        exit 1
    fi
fi

echo ""
print_success "Setup complete! Happy plugin development!"
echo ""
