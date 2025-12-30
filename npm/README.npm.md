# CDM CLI

**CDM (Common Data Model)** is a language for defining data models and generating code across multiple platforms.

## Installation

Install via npm:

```bash
npm install -g @cdm-lang/cli
```

Or use it in a project:

```bash
npm install --save-dev @cdm-lang/cli
```

## Usage

After installation, the `cdm` command will be available:

```bash
cdm --help
```

### Command Line

```bash
# Run CDM CLI
cdm <command> [options]
```

### Programmatic Usage

You can also use CDM programmatically in Node.js:

```javascript
const cdm = require('@cdm-lang/cli');

// Get the path to the binary
const binaryPath = cdm.getBinaryPath();
console.log('CDM binary location:', binaryPath);

// Run CDM with arguments
const exitCode = cdm.run(['--help']);
```

## How It Works

This npm package automatically downloads the appropriate pre-built CDM binary for your platform during installation. The binary is built from Rust and distributed via GitHub releases.

### Supported Platforms

- macOS (Intel x64)
- macOS (Apple Silicon arm64)
- Linux (x64)
- Linux (ARM64)
- Windows (x64)

## Updating

To update CDM to a newer version, use npm:

```bash
# For global installations
npm update -g @cdm-lang/cli

# For local project installations
npm update @cdm-lang/cli
```

**Note**: The CDM CLI has a built-in `cdm update` command, but when installed via npm, you should use `npm update` instead to keep your npm package registry in sync with the actual binary version.

## Troubleshooting

If you encounter installation issues:

1. Make sure you have a stable internet connection
2. Try clearing npm cache: `npm cache clean --force`
3. Reinstall: `npm uninstall -g @cdm-lang/cli && npm install -g @cdm-lang/cli`

If the binary fails to download, you can manually install CDM from the [GitHub releases page](https://github.com/cdm-lang/cdm/releases).

## License

MPL-2.0

## Links

- [GitHub Repository](https://github.com/cdm-lang/cdm)
- [Issue Tracker](https://github.com/cdm-lang/cdm/issues)
- [Releases](https://github.com/cdm-lang/cdm/releases)

## Contributing

Contributions are welcome! Please visit the [GitHub repository](https://github.com/cdm-lang/cdm) for more information.
