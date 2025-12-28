# Plugin Release Guide

This document explains how to release CDM plugins from this monorepo.

## Overview

Each plugin is released independently using semantic versioning and Git tags. The release process is automated via GitHub Actions.

## Release Process

### 1. List Available Plugins

```bash
just list-plugins
```

### 2. Build and Tag a Plugin

```bash
just release-plugin <plugin-name> <version>
```

**Example:**
```bash
just release-plugin cdm-plugin-docs 0.1.0
```

This command will:
1. Validate the version format (must be `X.Y.Z`)
2. Check if the plugin directory exists
3. Build the plugin WASM file
4. Generate a SHA256 checksum
5. Check for uncommitted changes
6. Create a Git tag in the format `<plugin-name>-v<version>`
7. Provide instructions for pushing the tag

### 3. Push the Tag

After the tag is created locally, push it to trigger the GitHub release:

```bash
git push origin cdm-plugin-docs-v0.1.0
```

### 4. GitHub Actions Automation

When you push a tag matching the pattern `cdm-plugin-*-v*.*.*`, the GitHub Actions workflow will automatically:

1. Extract the plugin name and version from the tag
2. Build the plugin WASM file
3. Generate a SHA256 checksum
4. Create a GitHub Release with:
   - Release notes with installation instructions
   - The compiled `.wasm` file
   - The `.wasm.sha256` checksum file
5. Update `registry.json` with the new version information
6. Commit the registry update directly to the main branch

## Tag Naming Convention

Tags follow the pattern: `<plugin-name>-v<version>`

**Examples:**
- `cdm-plugin-docs-v0.1.0`
- `cdm-plugin-sql-v1.2.3`
- `cdm-plugin-typescript-v0.5.0`

## Installing a Plugin

Users can install a plugin by downloading it from the GitHub Releases page:

```bash
# Download the plugin
curl -LO https://github.com/OWNER/REPO/releases/download/cdm-plugin-docs-v0.1.0/cdm_plugin_docs.wasm

# Download the checksum
curl -LO https://github.com/OWNER/REPO/releases/download/cdm-plugin-docs-v0.1.0/cdm_plugin_docs.wasm.sha256

# Verify the checksum
echo "$(cat cdm_plugin_docs.wasm.sha256)  cdm_plugin_docs.wasm" | shasum -a 256 -c
```

## Versioning Strategy

Plugins use [Semantic Versioning](https://semver.org/):

- **MAJOR** version: Incompatible API changes
- **MINOR** version: Backward-compatible functionality additions
- **PATCH** version: Backward-compatible bug fixes

## Troubleshooting

### Delete a Tag

If you created a tag by mistake:

```bash
# Delete locally
git tag -d cdm-plugin-docs-v0.1.0

# Delete remotely (if already pushed)
git push --delete origin cdm-plugin-docs-v0.1.0
```

### Tag Already Exists

If a tag already exists, you'll need to either:
1. Delete the existing tag (see above)
2. Use a different version number

### Build Fails

If the build fails during `just release-plugin`:
1. Fix the build issues in the plugin
2. Commit your changes
3. Try the release command again

## Files Included in Release

Each release includes:

1. **`<crate_name>.wasm`** - The compiled WebAssembly plugin
2. **`<crate_name>.wasm.sha256`** - SHA256 checksum for verification

Example for `cdm-plugin-docs`:
- `cdm_plugin_docs.wasm`
- `cdm_plugin_docs.wasm.sha256`
