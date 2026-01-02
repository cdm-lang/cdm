# CDM Language Server

Language Server Protocol (LSP) implementation for the CDM (Contextual Data Model) language.

## Features

### Phase 1 (Current) - Foundation ✅

- [x] Real-time validation with diagnostics
- [x] Error and warning reporting
- [x] Document synchronization (open/change/close)
- [x] Position mapping (UTF-16 ↔ byte offsets)

### Phase 2 - Navigation (Planned)

- [ ] Hover information
- [ ] Go-to-definition
- [ ] Find references

### Phase 3 - Productivity (Planned)

- [ ] Code completion
- [ ] Document formatting
- [ ] Workspace management

### Phase 4 - Polish (Planned)

- [ ] Syntax highlighting (tree-sitter queries)
- [ ] Document symbols (outline view)
- [ ] Rename refactoring
- [ ] Code actions (quick fixes)

## Building

```bash
cd crates/cdm-lsp
cargo build --release
```

## Running

The LSP server communicates via JSON-RPC over stdin/stdout:

```bash
cdm-lsp
```

## Usage with Editors

### VS Code

See the `editors/cdm-extension` directory for the editor extension.

### Neovim

Add to your `init.lua`:

```lua
vim.api.nvim_create_autocmd('FileType', {
  pattern = 'cdm',
  callback = function()
    vim.lsp.start({
      name = 'cdm-lsp',
      cmd = { 'cdm-lsp' },
      root_dir = vim.fs.dirname(vim.fs.find({ '.git' }, { upward = true })[1]),
    })
  end,
})
```

### Emacs

Add to your config:

```elisp
(add-to-list 'lsp-language-id-configuration '(cdm-mode . "cdm"))

(lsp-register-client
 (make-lsp-client
  :new-connection (lsp-stdio-connection '("cdm-lsp"))
  :activation-fn (lsp-activate-on "cdm")
  :server-id 'cdm-lsp))
```

## Architecture

```
cdm-lsp/
├── src/
│   ├── main.rs           # Entry point, stdio setup
│   └── server/
│       ├── mod.rs        # LSP server implementation
│       ├── document.rs   # Document store (in-memory cache)
│       ├── position.rs   # Position mapping utilities
│       └── diagnostics.rs # Diagnostic computation
```

## Testing

```bash
# Run unit tests
cargo test

# Run with debug logging
RUST_LOG=debug cdm-lsp
```

## Implementation Status

| Feature | Status | Lines | Tests |
|---------|--------|-------|-------|
| LSP Server Core | ✅ Complete | 170 | - |
| Document Store | ✅ Complete | 70 | 1 |
| Position Mapping | ✅ Complete | 120 | 6 |
| Diagnostics | ✅ Complete | 100 | 3 |
| Hover Provider | ⏳ Planned | - | - |
| Completion | ⏳ Planned | - | - |
| Go-to-Definition | ⏳ Planned | - | - |
| Formatting | ⏳ Planned | - | - |

**Total:** ~460 lines of Rust

## Next Steps

1. ✅ Create VS Code extension client
2. ✅ Test end-to-end with `.cdm` files
3. Implement hover provider
4. Implement code completion
5. Implement go-to-definition

## License

MIT
