# justfile
# Install dependencies
setup:
  cargo install tree-sitter-cli
  cargo fetch

# Generate parser
generate:
  cd crates/grammar && tree-sitter generate grammar.js

# Build everything
build: generate
  cargo build

# Run
run *args: generate
  cargo run -- {{args}}

# Clean everything
clean:
  rm -rf crates/grammar/src
  cargo clean
