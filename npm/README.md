# CDM CLI - npm Package

This directory contains the npm distribution package for CDM CLI. The npm package automatically downloads and installs the appropriate pre-built binary for the user's platform.

## Structure

```
npm/
├── package.json          # npm package configuration
├── index.js              # Module entry point for programmatic usage
├── bin/
│   └── cdm.js           # CLI wrapper script
├── scripts/
│   └── install.js       # Post-install script to download binary
├── .npmignore           # Files to exclude from npm package
└── README.npm.md        # README shown on npm registry
```

## How It Works

1. User runs `npm install @cdm-lang/cli`
2. The `postinstall` script (`scripts/install.js`) runs automatically
3. It detects the user's platform and architecture
4. Downloads the appropriate pre-built binary from GitHub releases
5. Verifies the checksum for security
6. Makes the binary executable (Unix-like systems)
7. The `bin/cdm.js` wrapper allows users to run `cdm` commands

## Publishing to npm

### Prerequisites

1. Make sure you're logged into npm:
   ```bash
   npm login
   ```

2. Ensure the version in `package.json` matches the Cargo version

### Publish Steps

1. Navigate to the npm directory:
   ```bash
   cd npm
   ```

2. Test the package locally:
   ```bash
   npm install
   npm pack
   ```

3. Publish to npm:
   ```bash
   npm publish --access public
   ```

   Note: The `--access public` flag is required for scoped packages (@cdm-lang/cli)

### Automation

You can automate npm publishing in the GitHub release workflow by adding an npm publish step after the CLI binaries are released.

## Testing Locally

Before publishing, test the package locally:

```bash
cd npm
npm install
./bin/cdm.js --help
```

Or test as if installed globally:

```bash
cd npm
npm pack
npm install -g cdm-lang-cli-<version>.tgz
cdm --help
npm uninstall -g @cdm-lang/cli
```

## Version Management

Keep the version in `npm/package.json` synchronized with `crates/cdm/Cargo.toml`. When cutting a new release:

1. Update version in `Cargo.toml`
2. Update version in `npm/package.json`
3. Create git tag and push (triggers GitHub release)
4. After binaries are built, publish to npm

## Important Notes on Self-Update

The CDM CLI has a built-in `cdm update` command that can update the binary directly. However, when CDM is installed via npm, users should be aware:

- `cdm update` will work and update the binary in `node_modules/@cdm-lang/cli/bin/cdm`
- However, this creates a version mismatch - npm will still think the old version is installed
- Running `npm install` again will re-download the version specified in `package.json`, overwriting manual updates

**Recommendation**: Users should update via npm (`npm update -g @cdm-lang/cli`) rather than using `cdm update` to keep package versions in sync. This is documented in `README.npm.md`.
