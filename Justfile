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
  cd editors/cdm-extension && npm test

testcoverage: build-plugins
  cargo llvm-cov --html --open
  cd editors/cdm-extension && npm run test:coverage

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

# Release the LSP server (creates and optionally pushes a version tag)
# Usage: just release-lsp <version>
# Example: just release-lsp 0.1.0
release-lsp version:
  #!/usr/bin/env bash
  set -e

  # Validate version format
  if ! [[ {{version}} =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Error: Version must be in format X.Y.Z (e.g., 0.1.0)"
    exit 1
  fi

  # Create tag name
  TAG="cdm-lsp-v{{version}}"

  echo "Creating release for CDM LSP version {{version}}"
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
    echo "  just release-lsp {{version}}"
    exit 1
  fi

  # Check for uncommitted changes
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
  echo "Updating version in crates/cdm-lsp/Cargo.toml..."
  sed -i.bak 's/^version = ".*"/version = "{{version}}"/' crates/cdm-lsp/Cargo.toml
  rm crates/cdm-lsp/Cargo.toml.bak

  # Update Cargo.lock
  echo "Updating Cargo.lock..."
  cargo check --manifest-path crates/cdm-lsp/Cargo.toml

  # Commit the version update
  echo "Committing version update..."
  git add crates/cdm-lsp/Cargo.toml Cargo.lock
  git commit -m "Release CDM LSP {{version}}"

  # Create tag
  echo "Creating tag $TAG..."
  git tag -a "$TAG" -m "Release CDM LSP v{{version}}"

  echo ""
  echo "✓ Tag created successfully!"
  echo ""
  echo "To push the commit and tag to trigger the release workflow, run:"
  echo "  git push origin main $TAG"
  echo ""
  echo "To undo if you made a mistake, run:"
  echo "  git tag -d $TAG"
  echo "  git reset --soft HEAD~1"

# Release the VS Code extension (creates and optionally pushes a version tag)
# Usage: just release-extension <version>
# Example: just release-extension 0.2.0
release-extension version:
  #!/usr/bin/env bash
  set -e

  # Validate version format
  if ! [[ {{version}} =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Error: Version must be in format X.Y.Z (e.g., 0.2.0)"
    exit 1
  fi

  # Create tag name
  TAG="cdm-extension-v{{version}}"
  EXTENSION_DIR="editors/cdm-extension"

  echo "Creating release for CDM Extension version {{version}}"
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
    echo "  just release-extension {{version}}"
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

  # Check current version in package.json
  CURRENT_VERSION=$(node -p "require('./$EXTENSION_DIR/package.json').version")
  if [ "$CURRENT_VERSION" != "{{version}}" ]; then
    echo "Updating version in package.json from $CURRENT_VERSION to {{version}}..."
    cd "$EXTENSION_DIR"
    npm version {{version}} --no-git-tag-version
    cd ../..
  fi

  # Commit the version update if there are changes
  if ! git diff --quiet "$EXTENSION_DIR/package.json"; then
    echo "Committing version update..."
    git add "$EXTENSION_DIR/package.json" "$EXTENSION_DIR/package-lock.json"
    git commit -m "Release CDM Extension {{version}}"
  fi

  # Create tag
  echo "Creating tag $TAG..."
  git tag -a "$TAG" -m "Release CDM Extension v{{version}}"

  echo ""
  echo "✓ Tag created successfully!"
  echo ""
  echo "To push the commit and tag to trigger the release workflow, run:"
  echo "  git push origin main $TAG"
  echo ""
  echo "Note: The release workflow requires these secrets to be configured:"
  echo "  - VSCE_PAT: Personal Access Token for VS Code Marketplace"
  echo "  - OVSX_PAT: Personal Access Token for Open VSX Registry"
  echo ""
  echo "To undo if you made a mistake, run:"
  echo "  git tag -d $TAG"
  echo "  git reset --soft HEAD~1"

# Release a template (creates and optionally pushes a version tag)
# Usage: just release-template <template-name> <version>
# Example: just release-template sql-types 1.0.0
release-template template_name version:
  #!/usr/bin/env bash
  set -e

  # Validate version format
  if ! [[ {{version}} =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Error: Version must be in format X.Y.Z (e.g., 1.0.0)"
    exit 1
  fi

  # Check if template directory exists and has cdm-template.json
  TEMPLATE_DIR="templates/{{template_name}}"
  if [ ! -d "$TEMPLATE_DIR" ]; then
    echo "Error: Template directory $TEMPLATE_DIR does not exist"
    echo ""
    echo "Available templates:"
    for dir in templates/*/; do
      if [ -f "$dir/cdm-template.json" ]; then
        basename "$dir"
      fi
    done
    exit 1
  fi

  if [ ! -f "$TEMPLATE_DIR/cdm-template.json" ]; then
    echo "Error: $TEMPLATE_DIR is not a valid template (missing cdm-template.json)"
    echo ""
    echo "Available templates:"
    for dir in templates/*/; do
      if [ -f "$dir/cdm-template.json" ]; then
        basename "$dir"
      fi
    done
    exit 1
  fi

  # Create tag name
  TAG="{{template_name}}-v{{version}}"

  echo "Creating release for template {{template_name}} version {{version}}"
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
    echo "  just release-template {{template_name}} {{version}}"
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

  # Update version in cdm-template.json
  echo "Updating version in $TEMPLATE_DIR/cdm-template.json..."
  # Use node for reliable JSON manipulation
  node -e "
    const fs = require('fs');
    const path = '$TEMPLATE_DIR/cdm-template.json';
    const data = JSON.parse(fs.readFileSync(path, 'utf8'));
    data.version = '{{version}}';
    fs.writeFileSync(path, JSON.stringify(data, null, 2) + '\n');
  "

  # Update templates.json registry if it exists
  if [ -f "templates.json" ]; then
    echo "Updating templates.json registry..."
    node -e "
      const fs = require('fs');
      const data = JSON.parse(fs.readFileSync('templates.json', 'utf8'));
      if (data.templates && data.templates['{{template_name}}']) {
        const template = data.templates['{{template_name}}'];
        template.versions['{{version}}'] = template.versions[template.latest] || {
          git_url: 'https://github.com/cdm-lang/cdm.git',
          git_ref: '{{template_name}}-v{{version}}',
          git_path: 'templates/{{template_name}}'
        };
        template.versions['{{version}}'].git_ref = '{{template_name}}-v{{version}}';
        template.latest = '{{version}}';
        data.updated_at = new Date().toISOString().split('T')[0] + 'T00:00:00Z';
      }
      fs.writeFileSync('templates.json', JSON.stringify(data, null, 2) + '\n');
    "
  fi

  # Commit the version updates
  echo "Committing version update..."
  git add "$TEMPLATE_DIR/cdm-template.json"
  if [ -f "templates.json" ]; then
    git add templates.json
  fi
  git commit -m "Release template {{template_name}} {{version}}"

  # Create tag
  echo "Creating tag $TAG..."
  git tag -a "$TAG" -m "Release template {{template_name}} v{{version}}"

  echo ""
  echo "✓ Tag created successfully!"
  echo ""
  echo "To push the commit and tag to trigger the release, run:"
  echo "  git push origin main $TAG"
  echo ""
  echo "To undo if you made a mistake, run:"
  echo "  git tag -d $TAG"
  echo "  git reset --soft HEAD~1"

# List available templates
list-templates:
  #!/usr/bin/env bash
  echo "Available templates:"
  for dir in templates/*/; do
    if [ -f "$dir/cdm-template.json" ]; then
      name=$(basename "$dir")
      version=$(node -p "require('./$dir/cdm-template.json').version" 2>/dev/null || echo "unknown")
      echo "  $name (v$version)"
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
