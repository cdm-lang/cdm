# Publishing CDM CLI to npm

The npm package `@cdm-lang/cli` is **automatically published** when you release the CLI using `just release-cli`.

## How It Works

1. Run `just release-cli <version>` - this updates both `crates/cdm/Cargo.toml` and `npm/package.json` to the same version
2. Push the commit and tag to trigger the GitHub workflow
3. The workflow builds binaries, creates a GitHub release, and automatically publishes to npm

## Prerequisites

**Configure npm Trusted Publishing (OIDC)**

This uses GitHub's OIDC integration with npm - no access tokens needed.

1. Go to https://www.npmjs.com/package/@cdm-lang/cli/access
2. Under "Publishing access", click "Add new trusted publisher"
3. Configure:
   - **Owner**: `cdm-lang`
   - **Repository**: `cdm`
   - **Workflow filename**: `cli-release.yml`
   - **Environment**: (leave empty)

## Release Workflow

```bash
# Release CLI version 0.2.0 (also updates npm package version)
just release-cli 0.2.0

# Push to trigger the workflow
git push origin main cdm-cli-v0.2.0
```

The GitHub workflow will:
1. Build binaries for all platforms
2. Create a GitHub release
3. Update `cli-releases.json`
4. Publish `@cdm-lang/cli` to npm with the same version

## Manual Publishing (if needed)

If automatic publishing fails or you need to republish:

```bash
cd npm
npm login  # if not already logged in
npm publish --access public
```

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
