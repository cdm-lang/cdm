# Editor Extension Setup Guide

This guide will help you set up and test the CDM editor extension. This extension works with VS Code, Cursor, and other editors supporting the VS Code extension API.

## Prerequisites

- Node.js 18+ and npm
- VS Code, Cursor, or another compatible editor
- Rust and Cargo (for building the LSP server)

## Step 1: Build the LSP Server

First, build the CDM language server:

```bash
# From the CDM repository root
cargo build -p cdm-lsp --release

# The binary will be at: target/release/cdm-lsp
```

**Optional**: Install the LSP server globally:

```bash
cargo install --path crates/cdm-lsp
```

If you install globally, `cdm-lsp` will be in your PATH and the extension will find it automatically.

## Step 2: Install Extension Dependencies

```bash
cd editors/vscode-cdm
npm install
```

This will install:
- `vscode-languageclient` - LSP client library
- TypeScript and build tools
- ESLint for code quality

## Step 3: Compile the Extension

```bash
npm run compile
```

This compiles TypeScript to JavaScript in the `out/` directory.

## Step 4: Test in Development Mode

### Option A: Using F5 (Recommended for Development)

1. Open the extension directory in your editor:
   ```bash
   code editors/vscode-cdm  # or: cursor editors/vscode-cdm
   ```

2. Press `F5` (or Run → Start Debugging)
   - This opens a new "Extension Development Host" window
   - The extension is automatically loaded in this window

3. In the Extension Development Host window:
   - Create a new file with `.cdm` extension
   - Start typing CDM code
   - Errors should appear inline!

### Option B: Install as VSIX

1. Package the extension:
   ```bash
   npm run package
   ```
   This creates `cdm-0.1.0.vsix`

2. Install the VSIX:
   ```bash
   # VS Code
   code --install-extension cdm-0.1.0.vsix
   # Cursor
   cursor --install-extension cdm-0.1.0.vsix
   ```

3. Reload your editor and open a `.cdm` file

## Step 5: Configure LSP Server Path (If Needed)

If you didn't install `cdm-lsp` globally, you need to tell the extension where to find it:

1. Open your editor's settings (Cmd+, or Ctrl+,)
2. Search for "cdm server path"
3. Set `cdm.server.path` to the absolute path of your LSP server:
   ```
   /Users/yourname/projects/cdm/target/release/cdm-lsp
   ```

## Testing

### Test 1: Valid CDM File

Create `test.cdm`:

```cdm
User {
  id: string #1
  name: string #2
  email: string #3
} #10
```

**Expected**: No errors, no warnings (if all entities have IDs)

### Test 2: Unknown Type Error

Create `test-error.cdm`:

```cdm
User {
  email: UnknownType #1
} #10
```

**Expected**: Red squiggly under `UnknownType` with error message:
```
Unknown type 'UnknownType'
```

### Test 3: Missing Entity IDs Warning

Create `test-warning.cdm`:

```cdm
User {
  name: string
}
```

**Expected**: Yellow squiggly warnings:
- Warning on `User` line: "Entity 'User' has no ID for migration tracking"
- Warning on `name` line: "Field 'User.name' has no ID for migration tracking"

### Test 4: Multiple Errors

Create `test-multiple.cdm`:

```cdm
User {
  email: UnknownType #1
  status: AnotherUnknown #2
} #10
```

**Expected**: Two error squigglies, one for each unknown type

### Test 5: Real-time Updates

1. Open any `.cdm` file with an error
2. Fix the error (e.g., change `UnknownType` to `string`)
3. **Expected**: Error disappears immediately without saving

## Debugging

### Check if LSP Server is Running

1. View → Output
2. Select "CDM Language Server" from the dropdown
3. You should see messages like:
   ```
   [Info] Starting CDM Language Server...
   [Info] CDM Language Server initialized
   ```

### Enable Verbose Logging

1. Settings → Search for "cdm trace"
2. Set `cdm.trace.server` to `"verbose"`
3. View → Output → "CDM Language Server Trace"
4. You'll see all LSP protocol messages

### Common Issues

#### "Language server not found"

**Solution**: Make sure `cdm-lsp` is in PATH or set the full path in settings:
```json
{
  "cdm.server.path": "/absolute/path/to/cdm-lsp"
}
```

#### "No diagnostics appearing"

**Checklist**:
- [ ] File has `.cdm` extension
- [ ] Language mode shows "CDM" in bottom right
- [ ] LSP server is running (check Output channel)
- [ ] Try: Cmd+Shift+P (or Ctrl+Shift+P) → "CDM: Restart Language Server"

#### "Extension not activating"

**Solution**: Check your editor's developer console:
- Help → Toggle Developer Tools
- Look for errors in the Console tab

## Development Workflow

### Making Changes to the Extension

1. Edit `src/extension.ts`
2. Run `npm run compile` (or `npm run watch` for auto-compile)
3. Reload the Extension Development Host window:
   - In the Extension Development Host: Cmd+R (or Reload Window command)

### Making Changes to the LSP Server

1. Edit Rust code in `crates/cdm-lsp/`
2. Rebuild: `cargo build -p cdm-lsp --release`
3. Restart the language server:
   - Press Cmd+Shift+P (or Ctrl+Shift+P) → "CDM: Restart Language Server"

## Publishing

When ready to publish to the marketplaces:

```bash
# Build and package
npm run vscode:prepublish
npm run package

# Publish to VS Code Marketplace only
npm run publish:vscode

# Publish to Open VSX Registry only (for Cursor, VSCodium, etc.)
npm run publish:openvsx

# Publish to both marketplaces
npm run publish
```

Note: Publishing requires accounts on the respective marketplaces:
- [VS Code Marketplace](https://marketplace.visualstudio.com/manage)
- [Open VSX Registry](https://open-vsx.org/)

## Next Steps

After confirming the extension works:

1. **Add syntax highlighting** - Create TextMate grammar or tree-sitter queries
2. **Implement hover** - Show type information on hover
3. **Implement completion** - Autocomplete types and keywords
4. **Implement go-to-definition** - Jump to type definitions
5. **Add formatting** - Hook up to `cdm format` command

## Resources

- [VS Code Extension API](https://code.visualstudio.com/api)
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)
- [vscode-languageclient docs](https://github.com/microsoft/vscode-languageserver-node)

---

**Need Help?** Check the [CDM repository issues](https://github.com/cdm-lang/cdm/issues) or create a new issue.
