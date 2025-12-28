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

test *args:
  cargo llvm-cov -- {{args}}
  cd editors/vscode-cdm && npm test

testcoverage:
  cargo llvm-cov --html --open
  cd editors/vscode-cdm && npm run test:coverage

# Release a plugin (creates and optionally pushes a version tag)
# Usage: just release-plugin <plugin-name> <version>
# Example: just release-plugin cdm-plugin-docs 0.1.0
release-plugin plugin_name version:
  #!/usr/bin/env bash
  set -e

  # Validate version format
  if ! [[ {{version}} =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Error: Version must be in format X.Y.Z (e.g., 0.1.0)"
    exit 1
  fi

  # Check if plugin directory exists and has cdm-plugin.json
  PLUGIN_DIR="crates/{{plugin_name}}"
  if [ ! -d "$PLUGIN_DIR" ]; then
    echo "Error: Plugin directory $PLUGIN_DIR does not exist"
    echo ""
    echo "Available plugins:"
    for dir in crates/cdm-plugin-*; do
      if [ -f "$dir/cdm-plugin.json" ]; then
        basename "$dir"
      fi
    done
    exit 1
  fi

  if [ ! -f "$PLUGIN_DIR/cdm-plugin.json" ]; then
    echo "Error: $PLUGIN_DIR is not a valid plugin (missing cdm-plugin.json)"
    echo ""
    echo "Available plugins:"
    for dir in crates/cdm-plugin-*; do
      if [ -f "$dir/cdm-plugin.json" ]; then
        basename "$dir"
      fi
    done
    exit 1
  fi

  # Create tag name
  TAG="{{plugin_name}}-v{{version}}"

  echo "Creating release for {{plugin_name}} version {{version}}"
  echo "Tag: $TAG"
  echo ""

  # Check if tag already exists
  if git rev-parse "$TAG" >/dev/null 2>&1; then
    echo "Error: Tag $TAG already exists"
    exit 1
  fi

  # Build the plugin
  echo "Building plugin..."
  cd "$PLUGIN_DIR"
  make build
  cd ../..

  # Check for uncommitted changes
  if ! git diff-index --quiet HEAD --; then
    echo ""
    echo "Warning: You have uncommitted changes"
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
      echo "Cancelled"
      exit 0
    fi
  fi

  # Create tag
  echo "Creating tag $TAG..."
  git tag -a "$TAG" -m "Release {{plugin_name}} v{{version}}"

  echo ""
  echo "✓ Tag created successfully!"
  echo ""
  echo "To push the tag and trigger the release workflow, run:"
  echo "  git push origin $TAG"
  echo ""
  echo "To delete the tag if you made a mistake, run:"
  echo "  git tag -d $TAG"

# List available plugins
list-plugins:
  #!/usr/bin/env bash
  echo "Available plugins:"
  for dir in crates/cdm-plugin-*; do
    if [ -f "$dir/cdm-plugin.json" ]; then
      basename "$dir"
    fi
  done
