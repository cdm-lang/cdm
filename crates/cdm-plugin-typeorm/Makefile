# CDM Plugin Makefile
# Common tasks for developing CDM plugins

.PHONY: help setup build build-debug test clean run-example install-deps check-deps

help:
	@echo "CDM Plugin Development Tasks"
	@echo ""
	@echo "Setup & Dependencies:"
	@echo "  make check-deps    - Check if required dependencies are installed"
	@echo "  make install-deps  - Install required dependencies (Rust WASM target)"
	@echo "  make setup         - Full setup (check + install dependencies)"
	@echo ""
	@echo "Building:"
	@echo "  make build         - Build plugin for WASM (release mode)"
	@echo "  make build-debug   - Build plugin for WASM (debug mode)"
	@echo ""
	@echo "Testing:"
	@echo "  make test          - Run all tests"
	@echo "  make test-unit     - Run unit tests only"
	@echo "  make test-wasm     - Build WASM and verify it loads"
	@echo ""
	@echo "Development:"
	@echo "  make run-example   - Run example CDM file with this plugin"
	@echo "  make watch         - Watch for changes and rebuild"
	@echo "  make clean         - Clean build artifacts"
	@echo ""
	@echo "Verification:"
	@echo "  make verify        - Run all checks (build + test)"
	@echo "  make size          - Show WASM file size"

# Variables
WASM_TARGET = wasm32-wasip1
RELEASE_WASM = target/$(WASM_TARGET)/release/cdm_plugin_typeorm.wasm
DEBUG_WASM = target/$(WASM_TARGET)/debug/cdm_plugin_typeorm.wasm
PLUGIN_NAME = cdm-plugin-typeorm

# Check if dependencies are installed
check-deps:
	@echo "Checking dependencies..."
	@command -v rustc >/dev/null 2>&1 || { echo "Error: Rust is not installed. Visit https://rustup.rs/"; exit 1; }
	@command -v cargo >/dev/null 2>&1 || { echo "Error: Cargo is not installed."; exit 1; }
	@rustup target list | grep -q "$(WASM_TARGET) (installed)" || { echo "Error: WASM target not installed. Run 'make install-deps'"; exit 1; }
	@echo "✓ All dependencies are installed"

# Install required dependencies
install-deps:
	@echo "Installing WASM target..."
	rustup target add $(WASM_TARGET)
	@echo "✓ Dependencies installed"

# Full setup
setup: check-deps
	@echo "✓ Setup complete"

# Build plugin in release mode (optimized for size)
build: check-deps
	@echo "Building plugin (release mode)..."
	cargo build --release --target $(WASM_TARGET) --target-dir target
	@echo "Generating checksum..."
	@shasum -a 256 $(RELEASE_WASM) | awk '{print $$1}' > $(RELEASE_WASM).sha256
	@echo "✓ Build complete: $(RELEASE_WASM)"
	@echo "✓ Checksum: $$(cat $(RELEASE_WASM).sha256)"
	@make size

# Build plugin in debug mode (faster compilation)
build-debug: check-deps
	@echo "Building plugin (debug mode)..."
	cargo build --target $(WASM_TARGET) --target-dir target
	@echo "✓ Build complete: $(DEBUG_WASM)"

# Run all tests
test:
	@echo "Running tests..."
	cargo test
	@echo "✓ All tests passed"

# Run unit tests only (no WASM compilation needed)
test-unit:
	@echo "Running unit tests..."
	cargo test --lib
	@echo "✓ Unit tests passed"

# Build WASM and verify it can be loaded
test-wasm: build
	@echo "Verifying WASM module..."
	@test -f $(RELEASE_WASM) || { echo "Error: WASM file not found"; exit 1; }
	@echo "✓ WASM module built successfully"
	@ls -lh $(RELEASE_WASM)

# Run example CDM file (requires CDM CLI to be built)
run-example: build
	@echo "Running example..."
	@if [ -f "example/schema.cdm" ]; then \
		cdm validate example/schema.cdm; \
	else \
		echo "No example file found. Create example/schema.cdm to test your plugin."; \
	fi

# Watch for changes and rebuild (requires cargo-watch)
watch:
	@command -v cargo-watch >/dev/null 2>&1 || { echo "Install cargo-watch: cargo install cargo-watch"; exit 1; }
	cargo watch -x 'build --target $(WASM_TARGET) --target-dir target'

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	cargo clean
	@echo "✓ Clean complete"

# Run all verification steps
verify: build test
	@echo "✓ All verification passed"

# Show WASM file size
size:
	@if [ -f $(RELEASE_WASM) ]; then \
		echo "WASM file size:"; \
		ls -lh $(RELEASE_WASM) | awk '{print "  " $$9 ": " $$5}'; \
		wc -c < $(RELEASE_WASM) | awk '{printf "  Raw bytes: %d (%.2f KB)\n", $$1, $$1/1024}'; \
	fi

# Development workflow: setup + build + test
dev: setup build test
	@echo "✓ Development environment ready"
