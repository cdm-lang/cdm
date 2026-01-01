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

# Build all plugins
build-plugins:
  #!/usr/bin/env bash
  set -e
  for dir in crates/cdm-plugin-*; do
    if [ -f "$dir/cdm-plugin.json" ]; then
      echo "Building $(basename "$dir")..."
      cd "$dir"
      make build
      cd ../..
    fi
  done
  echo "✓ All plugins built successfully"

# Run
run *args: generate
  cargo run -- {{args}}

# Clean everything
clean:
  rm -rf crates/grammar/src
  cargo clean

test *args: build-plugins
  cargo test -- {{args}}
  cd editors/vscode-cdm && npm test

testcoverage: build-plugins
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
    echo ""
    echo "To remove the existing tag and try again:"
    echo "  # Delete local tag"
    echo "  git tag -d $TAG"
    echo ""
    echo "  # If already pushed, delete remote tag"
    echo "  git push --delete origin $TAG"
    echo ""
    echo "  # Then run this command again"
    echo "  just release-plugin {{plugin_name}} {{version}}"
    exit 1
  fi

  # Build the plugin
  echo "Building plugin..."
  cd "$PLUGIN_DIR"
  make build
  cd ../..

  # Add the built WASM files to git
  WASM_FILE="$PLUGIN_DIR/target/wasm32-wasip1/release/"*.wasm
  CHECKSUM_FILE="$PLUGIN_DIR/target/wasm32-wasip1/release/"*.wasm.sha256

  # Check if there are other uncommitted changes besides the WASM files
  if ! git diff-index --quiet HEAD --; then
    echo ""
    echo "Warning: You have other uncommitted changes besides the plugin build"
    git status --short
    echo ""
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
      echo "Cancelled"
      exit 0
    fi
  fi

  # Commit the WASM files
  if ! git diff --cached --quiet; then
    echo "Committing built artifacts..."
    git commit -m "Build {{plugin_name}} v{{version}}"
  fi

  # Create tag
  echo "Creating tag $TAG..."
  git tag -a "$TAG" -m "Release {{plugin_name}} v{{version}}"

  echo ""
  echo "✓ Tag created successfully!"
  echo ""
  echo "To push the commit and tag to trigger the release workflow, run:"
  echo "  git push origin main $TAG"
  echo ""
  echo "To undo if you made a mistake, run:"
  echo "  git tag -d $TAG"
  echo "  git reset --soft HEAD~1"

# List available plugins
list-plugins:
  #!/usr/bin/env bash
  echo "Available plugins:"
  for dir in crates/cdm-plugin-*; do
    if [ -f "$dir/cdm-plugin.json" ]; then
      basename "$dir"
    fi
  done

# Release the CLI (creates and optionally pushes a version tag)
# Usage: just release-cli <version>
# Example: just release-cli 0.2.0
release-cli version:
  #!/usr/bin/env bash
  set -e

  # Validate version format
  if ! [[ {{version}} =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Error: Version must be in format X.Y.Z (e.g., 0.2.0)"
    exit 1
  fi

  # Create tag name
  TAG="cdm-cli-v{{version}}"

  echo "Creating release for CDM CLI version {{version}}"
  echo "Tag: $TAG"
  echo ""

  # Check if tag already exists
  if git rev-parse "$TAG" >/dev/null 2>&1; then
    echo "Error: Tag $TAG already exists"
    echo ""
    echo "To remove the existing tag and try again:"
    echo "  # Delete local tag"
    echo "  git tag -d $TAG"
    echo ""
    echo "  # If already pushed, delete remote tag"
    echo "  git push --delete origin $TAG"
    echo ""
    echo "  # Then run this command again"
    echo "  just release-cli {{version}}"
    exit 1
  fi

  # Check for uncommitted changes BEFORE making any modifications
  if ! git diff-index --quiet HEAD --; then
    echo ""
    echo "Warning: You have uncommitted changes"
    git status --short
    echo ""
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
      echo "Cancelled"
      exit 0
    fi
  fi

  # Update version in Cargo.toml
  echo "Updating version in Cargo.toml..."
  sed -i.bak 's/^version = ".*"/version = "{{version}}"/' crates/cdm/Cargo.toml
  rm crates/cdm/Cargo.toml.bak

  # Update Cargo.lock
  echo "Updating Cargo.lock..."
  cargo check --manifest-path crates/cdm/Cargo.toml

  # Commit the version update
  echo "Committing version update..."
  git add crates/cdm/Cargo.toml Cargo.lock
  git commit -m "Release CDM CLI {{version}}"

  # Create tag
  echo "Creating tag $TAG..."
  git tag -a "$TAG" -m "Release CDM CLI v{{version}}"

  echo ""
  echo "✓ Tag created successfully!"
  echo ""
  echo "To push the commit and tag to trigger the release workflow, run:"
  echo "  git push origin main $TAG"
  echo ""
  echo "To undo if you made a mistake, run:"
  echo "  git tag -d $TAG"
  echo "  git reset --soft HEAD~1"
