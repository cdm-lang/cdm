# CDM Language Support

Editor extension for the CDM (Contextual Data Model) language. Works with VS Code, Cursor, and other editors supporting the VS Code extension API.

## Features

- ✅ **Syntax highlighting** - Basic syntax highlighting for CDM files
- ✅ **Real-time validation** - Errors and warnings appear as you type
- ✅ **Error diagnostics** - Inline error messages with severity indicators
- 🚧 **Code completion** - Autocomplete for types, keywords, and more (coming soon)
- 🚧 **Go to definition** - Jump to type and model definitions (coming soon)
- 🚧 **Hover information** - See type information on hover (coming soon)
- 🚧 **Document formatting** - Format files on save (coming soon)

## Requirements

The extension requires the `cdm` CLI, which includes the language server. **It will be downloaded automatically** on first activation if not already installed.

### Automatic Installation

When you first open a `.cdm` file, the extension will:
1. Check if `cdm` is already in your PATH
2. If not found, automatically download the appropriate binary for your platform
3. Store it in the extension's global storage directory

### Manual Installation (Optional)

If you prefer to install the CLI manually:

```bash
# From the CDM repository root
cargo install --path crates/cdm
```

Or download a pre-built binary from the [releases page](https://github.com/cdm-lang/cdm/releases).

## Installation

### From VSIX (Local Development)

1. Build the extension:
   ```bash
   cd editors/cdm-extension
   npm install
   npm run compile
   npm run package
   ```

2. Install the generated `.vsix` file:
   ```bash
   # VS Code
   code --install-extension cdm-0.1.0.vsix
   # Cursor
   cursor --install-extension cdm-0.1.0.vsix
   ```

### From Marketplace

- **VS Code**: Search for "CDM Language Support" in the VS Code extensions marketplace
- **Open VSX** (Cursor, VSCodium, etc.): Search for "CDM Language Support" in the Open VSX Registry

## Extension Settings

This extension contributes the following settings:

* `cdm.cli.path`: Path to the cdm CLI binary (default: `"cdm"`)
* `cdm.format.indentSize`: Number of spaces for indentation when formatting (default: `2`)
* `cdm.validation.checkIds`: Check for missing entity IDs and show warnings (default: `true`)
* `cdm.trace.server`: Trace LSP communication for debugging (default: `"off"`)

## Commands

* `CDM: Restart Language Server` - Restart the CDM language server
* `CDM: Update CLI` - Check for and install updates to the CDM CLI

## Usage

1. Open or create a `.cdm` file
2. Start editing - the language server will provide real-time validation
3. Errors and warnings will appear inline with red/yellow squiggles
4. Hover over errors to see detailed messages

## Example

Create a file `schema.cdm`:

```cdm
// Type alias with validation
Email: string {
  @validation { format: "email", max_length: 320 }
} #1

// Model with fields
User {
  id: string #1
  email: Email #2
  name: string #3
  status: "active" | "pending" = "pending" #4

  @sql { table: "users" }
} #10
```

The extension will:
- Highlight syntax
- Validate the schema in real-time
- Show errors if you reference undefined types
- Warn about missing entity IDs (if `checkIds` is enabled)

## Troubleshooting

### Language server not starting

1. Verify `cdm` is in your PATH:
   ```bash
   which cdm
   ```

2. Check the output channel:
   - View → Output
   - Select "CDM Language Server" from the dropdown

3. Try updating the CLI:
   - Open Command Palette (Ctrl+Shift+P / Cmd+Shift+P)
   - Run "CDM: Update CLI"

4. Enable verbose logging:
   - Set `cdm.trace.server` to `"verbose"` in settings
   - Check the "CDM Language Server Trace" output channel

### Errors not showing up

1. Ensure the file has a `.cdm` extension
2. Check if the language mode is set to "CDM" (bottom right of the editor)
3. Try restarting the language server with the command palette (Ctrl+Shift+P / Cmd+Shift+P): "CDM: Restart Language Server"

## Development

### Running the Extension in Development Mode

1. Open the extension directory in your editor:
   ```bash
   cd editors/cdm-extension
   code .  # or: cursor .
   ```

2. Install dependencies:
   ```bash
   npm install
   ```

3. Press `F5` to open an Extension Development Host window

4. Open a `.cdm` file to test the extension

### Building for Production

```bash
npm run compile
npm run package
```

This creates a `.vsix` file that can be installed in any compatible editor.

### Publishing

Publishing is automated via GitHub Actions. To release a new version:

```bash
# From the repository root, use the just command:
just release-extension 0.2.0
```

This will:
1. Update the version in `package.json`
2. Commit the version change
3. Create a git tag (`cdm-extension-v0.2.0`)

Then push to trigger the release workflow:
```bash
git push origin main cdm-extension-v0.2.0
```

The workflow automatically:
- Builds and packages the extension
- Publishes to VS Code Marketplace
- Publishes to Open VSX Registry
- Creates a GitHub Release with the `.vsix` file

**Required secrets**: `VSCE_PAT` and `OVSX_PAT` must be configured in GitHub repository settings.

#### Manual Publishing

For local development or manual releases:
```bash
npm run publish:vscode   # VS Code Marketplace only
npm run publish:openvsx  # Open VSX Registry only
npm run publish          # Both marketplaces
```

## Known Issues

- Code completion not yet implemented
- Go-to-definition not yet implemented
- Hover information not yet implemented
- Multi-file validation (via `@extends`) shows errors for inherited types

See the [GitHub issues](https://github.com/cdm-lang/cdm/issues) for a full list.

## Release Notes

### 0.1.0 (Initial Release)

- Basic syntax highlighting
- Real-time validation with diagnostics
- Error and warning reporting
- Language server integration

## Contributing

Contributions are welcome! Please see the [CDM repository](https://github.com/cdm-lang/cdm) for contribution guidelines.

## License

MIT
