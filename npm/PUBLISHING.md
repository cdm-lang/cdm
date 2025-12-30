# Publishing CDM CLI to npm

This guide covers how to publish the CDM CLI package to npm.

## Prerequisites

1. **npm Account**: You must have an npm account with publish access to the `@cdm-lang` scope
   ```bash
   npm login
   ```

2. **Version Synchronization**: Ensure the version in `npm/package.json` matches `crates/cdm/Cargo.toml`

3. **GitHub Release**: The corresponding version must be released on GitHub with binaries built for all platforms

## Pre-Publish Checklist

Before publishing, ensure:

- [ ] Version in `npm/package.json` matches `crates/cdm/Cargo.toml`
- [ ] GitHub release exists for this version (tag: `cdm-cli-v<version>`)
- [ ] All platform binaries are built and uploaded to GitHub release
- [ ] `cli-releases.json` in the main branch has been updated with the new version
- [ ] You've tested the package locally (see Testing section below)

## Publishing Steps

### 1. Navigate to npm directory

```bash
cd npm
```

### 2. Test the package locally

```bash
# Install dependencies and test postinstall
npm install

# Verify the binary works
npx cdm --version

# Create a tarball and verify contents
npm pack
tar -tzf cdm-lang-cdm-*.tgz

# Test installation in a clean directory
cd /tmp
mkdir cdm-npm-test
cd cdm-npm-test
npm init -y
npm install /path/to/cdm/npm/cdm-lang-cdm-*.tgz
npx cdm --version
```

### 3. Publish to npm

```bash
cd npm
npm publish --access public
```

The `--access public` flag is required for scoped packages (@cdm-lang/cli).

### 4. Verify the published package

```bash
# Check on npm website
open https://www.npmjs.com/package/@cdm-lang/cli

# Test installing globally
npm install -g @cdm-lang/cli
cdm --version
npm uninstall -g @cdm-lang/cli

# Test installing locally
mkdir test-install
cd test-install
npm init -y
npm install @cdm-lang/cli
npx cdm --version
```

## Version Bumping Workflow

When releasing a new version:

1. **Update Cargo.toml**
   ```bash
   # In crates/cdm/Cargo.toml
   version = "x.y.z"
   ```

2. **Update package.json**
   ```bash
   # In npm/package.json
   "version": "x.y.z"
   ```

3. **Commit and tag**
   ```bash
   git add crates/cdm/Cargo.toml npm/package.json
   git commit -m "Bump version to x.y.z"
   git tag cdm-cli-vx.y.z
   git push origin main
   git push origin cdm-cli-vx.y.z
   ```

4. **Wait for GitHub Actions** to build binaries and update `cli-releases.json`

5. **Publish to npm** (see Publishing Steps above)

## Automation Options

You can automate npm publishing by adding a step to the GitHub release workflow:

### Option 1: Automatic Publishing

Add to `.github/workflows/cli-release.yml` after the binaries are built:

```yaml
- name: Publish to npm
  working-directory: npm
  run: |
    echo "//registry.npmjs.org/:_authToken=${{ secrets.NPM_TOKEN }}" > ~/.npmrc
    npm publish --access public
  env:
    NPM_TOKEN: ${{ secrets.NPM_TOKEN }}
```

Then add `NPM_TOKEN` to your GitHub repository secrets.

### Option 2: Manual Approval

Keep npm publishing manual for more control. Just follow the steps above after each GitHub release.

## Troubleshooting

### Package size too large

The package should be ~4KB. If it's larger:
- Check that binaries are excluded from the tarball
- Verify `npm/package.json` files array only includes necessary files
- Run `npm pack --dry-run` to see what will be included

### Binary download fails during postinstall

- Verify the GitHub release exists
- Check that `cli-releases.json` has been updated on the main branch
- Ensure all platform binaries are uploaded to the GitHub release
- Verify checksums in `cli-releases.json` match the binaries

### Version mismatch

The downloaded binary version should match the npm package version. If they don't:
- Ensure `cli-releases.json` has the correct version as "latest"
- Verify the GitHub release tag matches the npm version
- Check that you pushed the updated `cli-releases.json` to main

## Package Structure

The published package includes:

- `bin/cdm.js` - CLI wrapper script that spawns the binary
- `scripts/install.js` - Postinstall script that downloads the platform-specific binary
- `index.js` - Module entry point for programmatic usage
- `README.npm.md` - Package documentation
- `package.json` - Package metadata

The actual CDM binary is NOT included in the package. It's downloaded during postinstall from the GitHub release based on the user's platform.

## Notes

- The package name is scoped: `@cdm-lang/cli`
- The binary command is `cdm`
- Supports Node.js 14+
- Supports macOS (x64, arm64), Linux (x64, arm64), Windows (x64)
- Each platform downloads approximately 14MB binary during installation
