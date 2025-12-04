# justfile
# Install dependencies
setup:
    cargo install tree-sitter-cli
    cargo fetch

# Generate parser
generate:
    tree-sitter generate src/grammar/grammar.js --output src/grammar/build

# Build everything
build: generate
    cargo build

# Run
run: generate
    cargo run

# Clean everything
clean:
    rm -rf src/grammar/build
    cargo clean