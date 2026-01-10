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

# Check which components need releases (have changes since last release)
# Usage: just check-releases
check-releases:
  #!/usr/bin/env bash
  set -e

  # Colors for output
  RED='\033[0;31m'
  GREEN='\033[0;32m'
  YELLOW='\033[1;33m'
  BLUE='\033[0;34m'
  NC='\033[0m' # No Color

  echo ""
  echo "Checking for components with changes since last release..."
  echo ""

  NEEDS_RELEASE=()

  # Helper function to get latest tag matching a pattern
  get_latest_tag() {
    local pattern="$1"
    git tag -l "$pattern" --sort=-v:refname 2>/dev/null | head -n 1
  }

  # Helper function to extract version from tag
  extract_version() {
    local tag="$1"
    echo "$tag" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+$'
  }

  # Helper function to bump patch version
  bump_patch() {
    local version="$1"
    local major=$(echo "$version" | cut -d. -f1)
    local minor=$(echo "$version" | cut -d. -f2)
    local patch=$(echo "$version" | cut -d. -f3)
    echo "$major.$minor.$((patch + 1))"
  }

  # Helper function to check if there are changes since a tag
  has_changes_since_tag() {
    local tag="$1"
    local path="$2"
    if [ -z "$tag" ]; then
      # No tag exists, so any files in the path mean it needs a release
      if [ -d "$path" ] && [ -n "$(ls -A "$path" 2>/dev/null)" ]; then
        return 0
      fi
      return 1
    fi
    # Check if there are commits affecting this path since the tag
    local changes=$(git log "$tag"..HEAD --oneline -- "$path" 2>/dev/null | wc -l)
    [ "$changes" -gt 0 ]
  }

  # Check CLI
  echo -e "${BLUE}Checking CLI...${NC}"
  CLI_TAG=$(get_latest_tag "cdm-cli-v*")
  CLI_VERSION=$(extract_version "$CLI_TAG")
  if has_changes_since_tag "$CLI_TAG" "crates/cdm/"; then
    if [ -n "$CLI_VERSION" ]; then
      CLI_NEW_VERSION=$(bump_patch "$CLI_VERSION")
      echo -e "  ${YELLOW}CLI${NC}: Changes detected since $CLI_TAG -> needs release v$CLI_NEW_VERSION"
      NEEDS_RELEASE+=("cli:$CLI_NEW_VERSION")
    else
      echo -e "  ${YELLOW}CLI${NC}: No previous release found, needs initial release"
      NEEDS_RELEASE+=("cli:0.1.0")
    fi
  else
    echo -e "  ${GREEN}CLI${NC}: No changes since $CLI_TAG"
  fi

  # Check LSP
  echo -e "${BLUE}Checking LSP...${NC}"
  LSP_TAG=$(get_latest_tag "cdm-lsp-v*")
  LSP_VERSION=$(extract_version "$LSP_TAG")
  if has_changes_since_tag "$LSP_TAG" "crates/cdm-lsp/"; then
    if [ -n "$LSP_VERSION" ]; then
      LSP_NEW_VERSION=$(bump_patch "$LSP_VERSION")
      echo -e "  ${YELLOW}LSP${NC}: Changes detected since $LSP_TAG -> needs release v$LSP_NEW_VERSION"
      NEEDS_RELEASE+=("lsp:$LSP_NEW_VERSION")
    else
      echo -e "  ${YELLOW}LSP${NC}: No previous release found, needs initial release"
      NEEDS_RELEASE+=("lsp:0.1.0")
    fi
  else
    echo -e "  ${GREEN}LSP${NC}: No changes since $LSP_TAG"
  fi

  # Check Extension
  echo -e "${BLUE}Checking Extension...${NC}"
  EXT_TAG=$(get_latest_tag "cdm-extension-v*")
  EXT_VERSION=$(extract_version "$EXT_TAG")
  if has_changes_since_tag "$EXT_TAG" "editors/cdm-extension/"; then
    if [ -n "$EXT_VERSION" ]; then
      EXT_NEW_VERSION=$(bump_patch "$EXT_VERSION")
      echo -e "  ${YELLOW}Extension${NC}: Changes detected since $EXT_TAG -> needs release v$EXT_NEW_VERSION"
      NEEDS_RELEASE+=("extension:$EXT_NEW_VERSION")
    else
      echo -e "  ${YELLOW}Extension${NC}: No previous release found, needs initial release"
      NEEDS_RELEASE+=("extension:0.1.0")
    fi
  else
    echo -e "  ${GREEN}Extension${NC}: No changes since $EXT_TAG"
  fi

  # Check Plugins
  echo -e "${BLUE}Checking Plugins...${NC}"
  for dir in crates/cdm-plugin-*; do
    if [ -f "$dir/cdm-plugin.json" ]; then
      PLUGIN_NAME=$(basename "$dir")
      PLUGIN_TAG=$(get_latest_tag "${PLUGIN_NAME}-v*")
      PLUGIN_VERSION=$(extract_version "$PLUGIN_TAG")
      if has_changes_since_tag "$PLUGIN_TAG" "$dir/"; then
        if [ -n "$PLUGIN_VERSION" ]; then
          PLUGIN_NEW_VERSION=$(bump_patch "$PLUGIN_VERSION")
          echo -e "  ${YELLOW}$PLUGIN_NAME${NC}: Changes detected since $PLUGIN_TAG -> needs release v$PLUGIN_NEW_VERSION"
          NEEDS_RELEASE+=("plugin:$PLUGIN_NAME:$PLUGIN_NEW_VERSION")
        else
          echo -e "  ${YELLOW}$PLUGIN_NAME${NC}: No previous release found, needs initial release"
          NEEDS_RELEASE+=("plugin:$PLUGIN_NAME:0.1.0")
        fi
      else
        echo -e "  ${GREEN}$PLUGIN_NAME${NC}: No changes since $PLUGIN_TAG"
      fi
    fi
  done

  # Check Templates
  echo -e "${BLUE}Checking Templates...${NC}"
  if [ -d "templates" ]; then
    for dir in templates/*/; do
      if [ -f "$dir/cdm-template.json" ]; then
        TEMPLATE_NAME=$(basename "$dir")
        TEMPLATE_TAG=$(get_latest_tag "${TEMPLATE_NAME}-v*")
        TEMPLATE_VERSION=$(extract_version "$TEMPLATE_TAG")
        if has_changes_since_tag "$TEMPLATE_TAG" "$dir"; then
          if [ -n "$TEMPLATE_VERSION" ]; then
            TEMPLATE_NEW_VERSION=$(bump_patch "$TEMPLATE_VERSION")
            echo -e "  ${YELLOW}$TEMPLATE_NAME${NC}: Changes detected since $TEMPLATE_TAG -> needs release v$TEMPLATE_NEW_VERSION"
            NEEDS_RELEASE+=("template:$TEMPLATE_NAME:$TEMPLATE_NEW_VERSION")
          else
            echo -e "  ${YELLOW}$TEMPLATE_NAME${NC}: No previous release found, needs initial release"
            NEEDS_RELEASE+=("template:$TEMPLATE_NAME:1.0.0")
          fi
        else
          echo -e "  ${GREEN}$TEMPLATE_NAME${NC}: No changes since $TEMPLATE_TAG"
        fi
      fi
    done
  fi

  echo ""

  # Summary
  if [ ${#NEEDS_RELEASE[@]} -eq 0 ]; then
    echo -e "${GREEN}All components are up to date! No releases needed.${NC}"
    exit 0
  fi

  echo "========================================"
  echo -e "${YELLOW}Components needing release:${NC}"
  echo "========================================"
  for item in "${NEEDS_RELEASE[@]}"; do
    IFS=':' read -ra PARTS <<< "$item"
    TYPE="${PARTS[0]}"
    case "$TYPE" in
      cli)
        echo "  - CLI -> v${PARTS[1]}"
        ;;
      lsp)
        echo "  - LSP -> v${PARTS[1]}"
        ;;
      extension)
        echo "  - Extension -> v${PARTS[1]}"
        ;;
      plugin)
        echo "  - Plugin: ${PARTS[1]} -> v${PARTS[2]}"
        ;;
      template)
        echo "  - Template: ${PARTS[1]} -> v${PARTS[2]}"
        ;;
    esac
  done
  echo ""
  echo "Run 'just release-all' to release all components with changes"
  echo "Or release individually with:"
  for item in "${NEEDS_RELEASE[@]}"; do
    IFS=':' read -ra PARTS <<< "$item"
    TYPE="${PARTS[0]}"
    case "$TYPE" in
      cli)
        echo "  just release-cli ${PARTS[1]}"
        ;;
      lsp)
        echo "  just release-lsp ${PARTS[1]}"
        ;;
      extension)
        echo "  just release-extension ${PARTS[1]}"
        ;;
      plugin)
        echo "  just release-plugin ${PARTS[1]} ${PARTS[2]}"
        ;;
      template)
        echo "  just release-template ${PARTS[1]} ${PARTS[2]}"
        ;;
    esac
  done

# Show diff of changes since last release for each component
# Usage: just show-changes [component]
# Examples:
#   just show-changes          # Show changes for all components needing release
#   just show-changes cli      # Show changes for CLI only
#   just show-changes cdm-plugin-sql  # Show changes for a specific plugin
show-changes component="":
  #!/usr/bin/env bash
  set -e

  # Colors for output
  RED='\033[0;31m'
  GREEN='\033[0;32m'
  YELLOW='\033[1;33m'
  BLUE='\033[0;34m'
  CYAN='\033[0;36m'
  NC='\033[0m' # No Color
  BOLD='\033[1m'

  # Helper function to get latest tag matching a pattern
  get_latest_tag() {
    local pattern="$1"
    git tag -l "$pattern" --sort=-v:refname 2>/dev/null | head -n 1
  }

  # Helper function to check if there are changes since a tag
  has_changes_since_tag() {
    local tag="$1"
    local path="$2"
    if [ -z "$tag" ]; then
      if [ -d "$path" ] && [ -n "$(ls -A "$path" 2>/dev/null)" ]; then
        return 0
      fi
      return 1
    fi
    local changes=$(git log "$tag"..HEAD --oneline -- "$path" 2>/dev/null | wc -l)
    [ "$changes" -gt 0 ]
  }

  # Helper function to show changes for a component
  show_component_changes() {
    local name="$1"
    local tag="$2"
    local path="$3"

    echo ""
    echo -e "${BOLD}════════════════════════════════════════════════════════════════${NC}"
    echo -e "${CYAN}${BOLD}$name${NC}"
    echo -e "${BOLD}════════════════════════════════════════════════════════════════${NC}"

    if [ -z "$tag" ]; then
      echo -e "${YELLOW}No previous release tag found${NC}"
      echo "Showing all files in $path:"
      echo ""
      git ls-files "$path" 2>/dev/null | head -20
      local total=$(git ls-files "$path" 2>/dev/null | wc -l)
      if [ "$total" -gt 20 ]; then
        echo "... and $((total - 20)) more files"
      fi
      return
    fi

    echo -e "Changes since ${GREEN}$tag${NC}:"
    echo ""

    # Show commit log
    echo -e "${BLUE}Commits:${NC}"
    git log "$tag"..HEAD --oneline --no-merges -- "$path" 2>/dev/null | head -20
    local commit_count=$(git log "$tag"..HEAD --oneline --no-merges -- "$path" 2>/dev/null | wc -l)
    if [ "$commit_count" -gt 20 ]; then
      echo "... and $((commit_count - 20)) more commits"
    fi
    if [ "$commit_count" -eq 0 ]; then
      echo "  (no commits)"
    fi
    echo ""

    # Show file changes summary
    echo -e "${BLUE}Files changed:${NC}"
    git diff --stat "$tag"..HEAD -- "$path" 2>/dev/null | tail -20
    echo ""

    # Show actual diff (limited)
    echo -e "${BLUE}Diff preview (first 100 lines):${NC}"
    git diff "$tag"..HEAD -- "$path" 2>/dev/null | head -100
    local diff_lines=$(git diff "$tag"..HEAD -- "$path" 2>/dev/null | wc -l)
    if [ "$diff_lines" -gt 100 ]; then
      echo ""
      echo -e "${YELLOW}... diff truncated ($diff_lines total lines)${NC}"
      echo -e "Run ${CYAN}git diff $tag..HEAD -- $path${NC} to see full diff"
    fi
  }

  FILTER="{{component}}"

  # If a specific component is requested
  if [ -n "$FILTER" ]; then
    case "$FILTER" in
      cli)
        TAG=$(get_latest_tag "cdm-cli-v*")
        show_component_changes "CLI" "$TAG" "crates/cdm/"
        ;;
      lsp)
        TAG=$(get_latest_tag "cdm-lsp-v*")
        show_component_changes "LSP" "$TAG" "crates/cdm-lsp/"
        ;;
      extension)
        TAG=$(get_latest_tag "cdm-extension-v*")
        show_component_changes "Extension" "$TAG" "editors/cdm-extension/"
        ;;
      cdm-plugin-*)
        if [ -d "crates/$FILTER" ]; then
          TAG=$(get_latest_tag "${FILTER}-v*")
          show_component_changes "$FILTER" "$TAG" "crates/$FILTER/"
        else
          echo -e "${RED}Error: Plugin $FILTER not found${NC}"
          exit 1
        fi
        ;;
      *)
        # Check if it's a template
        if [ -d "templates/$FILTER" ]; then
          TAG=$(get_latest_tag "${FILTER}-v*")
          show_component_changes "Template: $FILTER" "$TAG" "templates/$FILTER/"
        else
          echo -e "${RED}Error: Unknown component '$FILTER'${NC}"
          echo ""
          echo "Available components:"
          echo "  cli, lsp, extension"
          echo "  Plugins:"
          for dir in crates/cdm-plugin-*; do
            if [ -f "$dir/cdm-plugin.json" ]; then
              echo "    $(basename "$dir")"
            fi
          done
          echo "  Templates:"
          for dir in templates/*/; do
            if [ -f "$dir/cdm-template.json" ]; then
              echo "    $(basename "$dir")"
            fi
          done
          exit 1
        fi
        ;;
    esac
    exit 0
  fi

  # Show changes for all components that need release
  echo ""
  echo -e "${BOLD}Showing changes for all components with unreleased changes...${NC}"

  FOUND_CHANGES=false

  # Check CLI
  CLI_TAG=$(get_latest_tag "cdm-cli-v*")
  if has_changes_since_tag "$CLI_TAG" "crates/cdm/"; then
    show_component_changes "CLI" "$CLI_TAG" "crates/cdm/"
    FOUND_CHANGES=true
  fi

  # Check LSP
  LSP_TAG=$(get_latest_tag "cdm-lsp-v*")
  if has_changes_since_tag "$LSP_TAG" "crates/cdm-lsp/"; then
    show_component_changes "LSP" "$LSP_TAG" "crates/cdm-lsp/"
    FOUND_CHANGES=true
  fi

  # Check Extension
  EXT_TAG=$(get_latest_tag "cdm-extension-v*")
  if has_changes_since_tag "$EXT_TAG" "editors/cdm-extension/"; then
    show_component_changes "Extension" "$EXT_TAG" "editors/cdm-extension/"
    FOUND_CHANGES=true
  fi

  # Check Plugins
  for dir in crates/cdm-plugin-*; do
    if [ -f "$dir/cdm-plugin.json" ]; then
      PLUGIN_NAME=$(basename "$dir")
      PLUGIN_TAG=$(get_latest_tag "${PLUGIN_NAME}-v*")
      if has_changes_since_tag "$PLUGIN_TAG" "$dir/"; then
        show_component_changes "$PLUGIN_NAME" "$PLUGIN_TAG" "$dir/"
        FOUND_CHANGES=true
      fi
    fi
  done

  # Check Templates
  if [ -d "templates" ]; then
    for dir in templates/*/; do
      if [ -f "$dir/cdm-template.json" ]; then
        TEMPLATE_NAME=$(basename "$dir")
        TEMPLATE_TAG=$(get_latest_tag "${TEMPLATE_NAME}-v*")
        if has_changes_since_tag "$TEMPLATE_TAG" "$dir"; then
          show_component_changes "Template: $TEMPLATE_NAME" "$TEMPLATE_TAG" "$dir"
          FOUND_CHANGES=true
        fi
      fi
    done
  fi

  if [ "$FOUND_CHANGES" = false ]; then
    echo ""
    echo -e "${GREEN}All components are up to date! No unreleased changes found.${NC}"
  fi

# Release all components that have changes since their last release
# Usage: just release-all
release-all:
  #!/usr/bin/env bash
  set -e

  # Colors for output
  RED='\033[0;31m'
  GREEN='\033[0;32m'
  YELLOW='\033[1;33m'
  BLUE='\033[0;34m'
  NC='\033[0m' # No Color

  echo ""
  echo "Identifying components that need releases..."
  echo ""

  NEEDS_RELEASE=()
  TAGS_TO_PUSH=()

  # Helper function to get latest tag matching a pattern
  get_latest_tag() {
    local pattern="$1"
    git tag -l "$pattern" --sort=-v:refname 2>/dev/null | head -n 1
  }

  # Helper function to extract version from tag
  extract_version() {
    local tag="$1"
    echo "$tag" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+$'
  }

  # Helper function to bump patch version
  bump_patch() {
    local version="$1"
    local major=$(echo "$version" | cut -d. -f1)
    local minor=$(echo "$version" | cut -d. -f2)
    local patch=$(echo "$version" | cut -d. -f3)
    echo "$major.$minor.$((patch + 1))"
  }

  # Helper function to check if there are changes since a tag
  has_changes_since_tag() {
    local tag="$1"
    local path="$2"
    if [ -z "$tag" ]; then
      if [ -d "$path" ] && [ -n "$(ls -A "$path" 2>/dev/null)" ]; then
        return 0
      fi
      return 1
    fi
    local changes=$(git log "$tag"..HEAD --oneline -- "$path" 2>/dev/null | wc -l)
    [ "$changes" -gt 0 ]
  }

  # Check CLI
  CLI_TAG=$(get_latest_tag "cdm-cli-v*")
  CLI_VERSION=$(extract_version "$CLI_TAG")
  if has_changes_since_tag "$CLI_TAG" "crates/cdm/"; then
    if [ -n "$CLI_VERSION" ]; then
      CLI_NEW_VERSION=$(bump_patch "$CLI_VERSION")
    else
      CLI_NEW_VERSION="0.1.0"
    fi
    NEEDS_RELEASE+=("cli:$CLI_NEW_VERSION")
  fi

  # Check LSP
  LSP_TAG=$(get_latest_tag "cdm-lsp-v*")
  LSP_VERSION=$(extract_version "$LSP_TAG")
  if has_changes_since_tag "$LSP_TAG" "crates/cdm-lsp/"; then
    if [ -n "$LSP_VERSION" ]; then
      LSP_NEW_VERSION=$(bump_patch "$LSP_VERSION")
    else
      LSP_NEW_VERSION="0.1.0"
    fi
    NEEDS_RELEASE+=("lsp:$LSP_NEW_VERSION")
  fi

  # Check Extension
  EXT_TAG=$(get_latest_tag "cdm-extension-v*")
  EXT_VERSION=$(extract_version "$EXT_TAG")
  if has_changes_since_tag "$EXT_TAG" "editors/cdm-extension/"; then
    if [ -n "$EXT_VERSION" ]; then
      EXT_NEW_VERSION=$(bump_patch "$EXT_VERSION")
    else
      EXT_NEW_VERSION="0.1.0"
    fi
    NEEDS_RELEASE+=("extension:$EXT_NEW_VERSION")
  fi

  # Check Plugins
  for dir in crates/cdm-plugin-*; do
    if [ -f "$dir/cdm-plugin.json" ]; then
      PLUGIN_NAME=$(basename "$dir")
      PLUGIN_TAG=$(get_latest_tag "${PLUGIN_NAME}-v*")
      PLUGIN_VERSION=$(extract_version "$PLUGIN_TAG")
      if has_changes_since_tag "$PLUGIN_TAG" "$dir/"; then
        if [ -n "$PLUGIN_VERSION" ]; then
          PLUGIN_NEW_VERSION=$(bump_patch "$PLUGIN_VERSION")
        else
          PLUGIN_NEW_VERSION="0.1.0"
        fi
        NEEDS_RELEASE+=("plugin:$PLUGIN_NAME:$PLUGIN_NEW_VERSION")
      fi
    fi
  done

  # Check Templates
  if [ -d "templates" ]; then
    for dir in templates/*/; do
      if [ -f "$dir/cdm-template.json" ]; then
        TEMPLATE_NAME=$(basename "$dir")
        TEMPLATE_TAG=$(get_latest_tag "${TEMPLATE_NAME}-v*")
        TEMPLATE_VERSION=$(extract_version "$TEMPLATE_TAG")
        if has_changes_since_tag "$TEMPLATE_TAG" "$dir"; then
          if [ -n "$TEMPLATE_VERSION" ]; then
            TEMPLATE_NEW_VERSION=$(bump_patch "$TEMPLATE_VERSION")
          else
            TEMPLATE_NEW_VERSION="1.0.0"
          fi
          NEEDS_RELEASE+=("template:$TEMPLATE_NAME:$TEMPLATE_NEW_VERSION")
        fi
      fi
    done
  fi

  # Check if anything needs release
  if [ ${#NEEDS_RELEASE[@]} -eq 0 ]; then
    echo -e "${GREEN}All components are up to date! No releases needed.${NC}"
    exit 0
  fi

  # Show what will be released
  echo "========================================"
  echo -e "${YELLOW}The following releases will be created:${NC}"
  echo "========================================"
  for item in "${NEEDS_RELEASE[@]}"; do
    IFS=':' read -ra PARTS <<< "$item"
    TYPE="${PARTS[0]}"
    case "$TYPE" in
      cli)
        echo "  - CLI v${PARTS[1]} (tag: cdm-cli-v${PARTS[1]})"
        ;;
      lsp)
        echo "  - LSP v${PARTS[1]} (tag: cdm-lsp-v${PARTS[1]})"
        ;;
      extension)
        echo "  - Extension v${PARTS[1]} (tag: cdm-extension-v${PARTS[1]})"
        ;;
      plugin)
        echo "  - Plugin ${PARTS[1]} v${PARTS[2]} (tag: ${PARTS[1]}-v${PARTS[2]})"
        ;;
      template)
        echo "  - Template ${PARTS[1]} v${PARTS[2]} (tag: ${PARTS[1]}-v${PARTS[2]})"
        ;;
    esac
  done
  echo ""

  # Ask for confirmation
  read -p "Proceed with creating these releases? (y/N) " -n 1 -r
  echo
  if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Cancelled"
    exit 0
  fi

  echo ""

  # Process each release
  for item in "${NEEDS_RELEASE[@]}"; do
    IFS=':' read -ra PARTS <<< "$item"
    TYPE="${PARTS[0]}"

    case "$TYPE" in
      cli)
        VERSION="${PARTS[1]}"
        TAG="cdm-cli-v$VERSION"
        echo -e "${BLUE}Releasing CLI v$VERSION...${NC}"

        # Update version in Cargo.toml
        sed -i.bak 's/^version = ".*"/version = "'"$VERSION"'"/' crates/cdm/Cargo.toml
        rm -f crates/cdm/Cargo.toml.bak

        # Update Cargo.lock
        cargo check --manifest-path crates/cdm/Cargo.toml 2>/dev/null || true

        # Commit and tag
        git add crates/cdm/Cargo.toml Cargo.lock 2>/dev/null || true
        git commit -m "Release CDM CLI $VERSION" 2>/dev/null || true
        git tag -a "$TAG" -m "Release CDM CLI v$VERSION"

        TAGS_TO_PUSH+=("$TAG")
        echo -e "${GREEN}  Created tag $TAG${NC}"
        ;;

      lsp)
        VERSION="${PARTS[1]}"
        TAG="cdm-lsp-v$VERSION"
        echo -e "${BLUE}Releasing LSP v$VERSION...${NC}"

        # Update version in Cargo.toml
        sed -i.bak 's/^version = ".*"/version = "'"$VERSION"'"/' crates/cdm-lsp/Cargo.toml
        rm -f crates/cdm-lsp/Cargo.toml.bak

        # Update Cargo.lock
        cargo check --manifest-path crates/cdm-lsp/Cargo.toml 2>/dev/null || true

        # Commit and tag
        git add crates/cdm-lsp/Cargo.toml Cargo.lock 2>/dev/null || true
        git commit -m "Release CDM LSP $VERSION" 2>/dev/null || true
        git tag -a "$TAG" -m "Release CDM LSP v$VERSION"

        TAGS_TO_PUSH+=("$TAG")
        echo -e "${GREEN}  Created tag $TAG${NC}"
        ;;

      extension)
        VERSION="${PARTS[1]}"
        TAG="cdm-extension-v$VERSION"
        EXTENSION_DIR="editors/cdm-extension"
        echo -e "${BLUE}Releasing Extension v$VERSION...${NC}"

        # Update version in package.json
        cd "$EXTENSION_DIR"
        npm version "$VERSION" --no-git-tag-version 2>/dev/null || true
        cd ../..

        # Commit and tag
        git add "$EXTENSION_DIR/package.json" "$EXTENSION_DIR/package-lock.json" 2>/dev/null || true
        git commit -m "Release CDM Extension $VERSION" 2>/dev/null || true
        git tag -a "$TAG" -m "Release CDM Extension v$VERSION"

        TAGS_TO_PUSH+=("$TAG")
        echo -e "${GREEN}  Created tag $TAG${NC}"
        ;;

      plugin)
        PLUGIN_NAME="${PARTS[1]}"
        VERSION="${PARTS[2]}"
        TAG="$PLUGIN_NAME-v$VERSION"
        PLUGIN_DIR="crates/$PLUGIN_NAME"
        echo -e "${BLUE}Releasing $PLUGIN_NAME v$VERSION...${NC}"

        # Update version in Cargo.toml
        sed -i.bak 's/^version = ".*"/version = "'"$VERSION"'"/' "$PLUGIN_DIR/Cargo.toml"
        rm -f "$PLUGIN_DIR/Cargo.toml.bak"

        # Update cdm-plugin.json
        if [ -f "$PLUGIN_DIR/cdm-plugin.json" ]; then
          node -e "
            const fs = require('fs');
            const path = '$PLUGIN_DIR/cdm-plugin.json';
            const data = JSON.parse(fs.readFileSync(path, 'utf8'));
            data.version = '$VERSION';
            fs.writeFileSync(path, JSON.stringify(data, null, 2) + '\n');
          " 2>/dev/null || true
        fi

        # Build the plugin
        echo "  Building plugin..."
        cd "$PLUGIN_DIR"
        make build 2>/dev/null || cargo build --release --target wasm32-wasip1 2>/dev/null || true
        cd ../..

        # Commit and tag
        git add "$PLUGIN_DIR/Cargo.toml" "$PLUGIN_DIR/cdm-plugin.json" 2>/dev/null || true
        git add Cargo.lock 2>/dev/null || true
        git commit -m "Release $PLUGIN_NAME $VERSION" 2>/dev/null || true
        git tag -a "$TAG" -m "Release $PLUGIN_NAME v$VERSION"

        TAGS_TO_PUSH+=("$TAG")
        echo -e "${GREEN}  Created tag $TAG${NC}"
        ;;

      template)
        TEMPLATE_NAME="${PARTS[1]}"
        VERSION="${PARTS[2]}"
        TAG="$TEMPLATE_NAME-v$VERSION"
        TEMPLATE_DIR="templates/$TEMPLATE_NAME"
        echo -e "${BLUE}Releasing template $TEMPLATE_NAME v$VERSION...${NC}"

        # Update version in cdm-template.json
        node -e "
          const fs = require('fs');
          const path = '$TEMPLATE_DIR/cdm-template.json';
          const data = JSON.parse(fs.readFileSync(path, 'utf8'));
          data.version = '$VERSION';
          fs.writeFileSync(path, JSON.stringify(data, null, 2) + '\n');
        " 2>/dev/null || true

        # Update templates.json registry if it exists
        if [ -f "templates.json" ]; then
          node -e "
            const fs = require('fs');
            const data = JSON.parse(fs.readFileSync('templates.json', 'utf8'));
            if (data.templates && data.templates['$TEMPLATE_NAME']) {
              const template = data.templates['$TEMPLATE_NAME'];
              template.versions['$VERSION'] = {
                git_url: 'https://github.com/cdm-lang/cdm.git',
                git_ref: '$TEMPLATE_NAME-v$VERSION',
                git_path: 'templates/$TEMPLATE_NAME'
              };
              template.latest = '$VERSION';
              data.updated_at = new Date().toISOString().split('T')[0] + 'T00:00:00Z';
            }
            fs.writeFileSync('templates.json', JSON.stringify(data, null, 2) + '\n');
          " 2>/dev/null || true
        fi

        # Commit and tag
        git add "$TEMPLATE_DIR/cdm-template.json" 2>/dev/null || true
        git add templates.json 2>/dev/null || true
        git commit -m "Release template $TEMPLATE_NAME $VERSION" 2>/dev/null || true
        git tag -a "$TAG" -m "Release template $TEMPLATE_NAME v$VERSION"

        TAGS_TO_PUSH+=("$TAG")
        echo -e "${GREEN}  Created tag $TAG${NC}"
        ;;
    esac
  done

  echo ""
  echo "========================================"
  echo -e "${GREEN}All releases created successfully!${NC}"
  echo "========================================"
  echo ""
  echo "Tags created:"
  for tag in "${TAGS_TO_PUSH[@]}"; do
    echo "  - $tag"
  done
  echo ""

  # Ask to push tags
  read -p "Push all commits and tags to origin? (y/N) " -n 1 -r
  echo
  if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo ""
    echo "Pushing commits..."
    git push origin HEAD
    echo ""
    echo "Pushing tags..."
    for tag in "${TAGS_TO_PUSH[@]}"; do
      echo "  Pushing $tag..."
      git push origin "$tag"
    done
    echo ""
    echo -e "${GREEN}All releases have been pushed and will trigger GitHub Actions workflows.${NC}"
  else
    echo ""
    echo "To push manually, run:"
    echo "  git push origin HEAD ${TAGS_TO_PUSH[*]}"
  fi
