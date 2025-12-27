# CDM Language Server Implementation Plan

**Version**: 1.0
**Date**: 2025-12-26
**Status**: Planning Phase

---

## Table of Contents

1. [Overview](#overview)
2. [Current State Analysis](#current-state-analysis)
3. [Architecture Design](#architecture-design)
4. [Feature Roadmap](#feature-roadmap)
5. [Implementation Phases](#implementation-phases)
6. [Technical Specifications](#technical-specifications)
7. [VS Code Extension](#vs-code-extension)
8. [Testing Strategy](#testing-strategy)
9. [Appendix: LSP Protocol Coverage](#appendix-lsp-protocol-coverage)

---

## 1. Overview

### 1.1 Goal

Build a production-ready Language Server Protocol (LSP) implementation for CDM that provides:
- Real-time syntax validation and error reporting
- Context-aware code completion
- Hover information and documentation
- Go-to-definition and find references
- Document formatting on save
- Multi-file workspace support with @extends resolution

### 1.2 Target Editors

Primary:
- **VS Code** (via official extension)

Secondary (via generic LSP support):
- Neovim (nvim-lspconfig)
- Emacs (lsp-mode)
- Sublime Text (LSP package)
- IntelliJ IDEA (LSP4IJ plugin)

### 1.3 Key Advantages

The CDM codebase is **exceptionally well-positioned** for LSP development:

✅ **Tree-sitter grammar** (11,351 lines) - production-ready incremental parser
✅ **Semantic validation** (1,685 lines) - complete error detection
✅ **Symbol table** - type resolution and definition lookup
✅ **File resolver** - @extends chain resolution
✅ **Formatter** (1,420 lines) - AST-aware formatting
✅ **Diagnostics** - structured error reporting with spans

**Estimated reusable code:** ~6,000 lines of production-ready Rust

---

## 2. Current State Analysis

### 2.1 What Already Exists ✅

#### **Parsing Infrastructure**
- **Location:** `crates/grammar/`
- **Status:** Production-ready
- **Components:**
  - Complete tree-sitter grammar (11,351 lines)
  - WASM compilation target (13KB binary)
  - NPM package ready for publishing
  - Incremental parsing support
  - Error recovery built-in

#### **Validation Infrastructure**
- **Location:** `crates/cdm/src/validate.rs`
- **Status:** Production-ready (1,685 lines, 250+ tests)
- **Capabilities:**
  - Full semantic validation
  - Type checking with circular reference detection
  - Cross-file validation with @extends
  - Error codes E101-E503
  - Warning codes W005-W006
  - Returns structured `Diagnostic` objects

#### **Symbol Table & Type Resolution**
- **Location:** `crates/cdm/src/symbol_table.rs`
- **Status:** Production-ready (305 lines)
- **Capabilities:**
  - Model and type alias definitions
  - Field information with types
  - Plugin configuration tracking
  - Ancestor chain resolution
  - Helper functions: `is_type_defined()`, `resolve_definition()`, `get_inherited_fields()`

#### **File Resolution**
- **Location:** `crates/cdm/src/file_resolver.rs`
- **Status:** Production-ready (289 lines)
- **Capabilities:**
  - Recursive @extends loading
  - Circular dependency detection
  - Lazy file loading with caching
  - Relative path resolution
  - Dependency ordering

#### **Formatting**
- **Location:** `crates/cdm/src/format.rs`
- **Status:** Production-ready (1,420 lines, 20 tests)
- **Capabilities:**
  - AST-aware whitespace formatting
  - Entity ID assignment
  - Configurable indentation
  - Source reconstruction preserving structure

#### **Diagnostic Infrastructure**
- **Location:** `crates/cdm/src/diagnostics.rs`
- **Status:** Production-ready (51 lines)
- **Capabilities:**
  - `Diagnostic` struct with message, severity, span
  - `Severity` enum (Error, Warning)
  - Span tracking with line/column positions
  - Display implementation for formatted output

### 2.2 What Needs to Be Built ⏳

| Component | Priority | Estimated Lines | Effort |
|-----------|----------|-----------------|--------|
| LSP Server Core | HIGH | 600-800 | 1-2 weeks |
| Position Mapping | HIGH | 200-300 | 2-3 days |
| Diagnostic Publishing | HIGH | 100-150 | 1-2 days |
| Hover Provider | MEDIUM | 200-300 | 3-4 days |
| Go-to-Definition | MEDIUM | 200-250 | 3-4 days |
| Find References | MEDIUM | 150-200 | 2-3 days |
| Code Completion | MEDIUM | 400-500 | 5-7 days |
| Document Formatting | MEDIUM | 100-150 | 1-2 days |
| Workspace Management | MEDIUM | 300-400 | 3-5 days |
| VS Code Extension | MEDIUM | 300-400 | 3-5 days |
| Syntax Highlighting | LOW | 200-300 | 2-3 days |
| **TOTAL** | | **2,850-3,750** | **4-5 weeks** |

---

## 3. Architecture Design

### 3.1 Project Structure

```
cdm/
├── crates/
│   ├── cdm-lsp/                    # NEW: Language server binary
│   │   ├── src/
│   │   │   ├── main.rs            # LSP server entry point (100 lines)
│   │   │   ├── server.rs          # Core LSP implementation (600 lines)
│   │   │   ├── position.rs        # Position mapping utils (200 lines)
│   │   │   ├── handlers/
│   │   │   │   ├── hover.rs       # Hover provider (250 lines)
│   │   │   │   ├── completion.rs  # Code completion (400 lines)
│   │   │   │   ├── definition.rs  # Go-to-def + references (300 lines)
│   │   │   │   ├── formatting.rs  # Document formatting (150 lines)
│   │   │   │   └── diagnostics.rs # Diagnostic publishing (100 lines)
│   │   │   ├── document.rs        # Document management (200 lines)
│   │   │   └── workspace.rs       # Workspace management (300 lines)
│   │   ├── Cargo.toml
│   │   └── README.md
│   │
│   ├── grammar/                    # EXISTING: Tree-sitter grammar
│   │   ├── queries/               # NEW: Tree-sitter queries
│   │   │   ├── highlights.scm    # Syntax highlighting (150 lines)
│   │   │   ├── injections.scm    # Embedded languages (50 lines)
│   │   │   └── locals.scm        # Scope tracking (50 lines)
│   │   └── ...
│   │
│   └── cdm/                        # EXISTING: Core CDM library
│       └── ...                     # Reuse: validate, format, symbol_table, etc.
│
├── editors/
│   └── vscode-cdm/                 # NEW: VS Code extension
│       ├── src/
│       │   ├── extension.ts       # Client initialization (150 lines)
│       │   ├── commands.ts        # Command implementations (200 lines)
│       │   ├── settings.ts        # Configuration (100 lines)
│       │   └── types.ts           # TypeScript types (50 lines)
│       ├── syntaxes/
│       │   └── cdm.tmLanguage.json # TextMate grammar (fallback)
│       ├── package.json            # VS Code metadata
│       ├── tsconfig.json
│       └── README.md
│
└── docs/
    └── lsp-implementation-plan.md  # This document
```

### 3.2 Technology Stack

**LSP Server (Rust):**
- `tower-lsp` - LSP protocol framework
- `tokio` - Async runtime
- `serde_json` - JSON serialization
- `tree-sitter` - Parser (already integrated)
- `cdm` crate - Core functionality (reuse)

**VS Code Extension (TypeScript):**
- `vscode-languageclient` - LSP client
- `@types/vscode` - VS Code API types
- `esbuild` - Bundler for distribution

**Build Tools:**
- `cargo` - Rust build system
- `npm` / `pnpm` - JS package manager
- `vsce` - VS Code extension packager

### 3.3 Communication Flow

```
┌─────────────────┐
│   VS Code       │
│   (Client)      │
└────────┬────────┘
         │ JSON-RPC over stdio
         │
┌────────▼────────┐
│   cdm-lsp       │
│   (Server)      │
│                 │
│  ┌───────────┐  │
│  │ tower-lsp │  │  ← LSP protocol handling
│  └─────┬─────┘  │
│        │        │
│  ┌─────▼─────┐  │
│  │ Handlers  │  │  ← Feature implementations
│  └─────┬─────┘  │
│        │        │
│  ┌─────▼──────────────────┐
│  │ CDM Core (reuse)       │
│  │ - validate()           │
│  │ - format_file()        │
│  │ - SymbolTable          │
│  │ - FileResolver         │
│  │ - GrammarParser        │
│  └────────────────────────┘
└──────────────────┘
```

### 3.4 Data Flow for Validation

```
1. User types in VS Code
   ↓
2. Client sends didChange notification
   ↓
3. LSP server receives updated document
   ↓
4. Parse with tree-sitter (incremental)
   ↓
5. Run validate() from cdm crate
   ↓
6. Convert Diagnostic to LSP format
   ↓
7. Publish diagnostics to client
   ↓
8. VS Code displays errors/warnings inline
```

---

## 4. Feature Roadmap

### 4.1 Phase 1: Foundation (Week 1) - HIGH PRIORITY

**Goal:** Basic LSP server with error reporting

**Features:**
- ✅ LSP server binary with stdio communication
- ✅ Initialize/shutdown handlers
- ✅ Document synchronization (open/change/close)
- ✅ Position mapping (LSP ↔ tree-sitter)
- ✅ Real-time validation with diagnostics
- ✅ Error squiggles in editor

**Deliverables:**
- Working LSP server that validates on file open/change
- Error messages appear in editor
- Basic VS Code extension to launch server

**Success Criteria:**
- Open a `.cdm` file → see errors inline
- Fix error → error disappears
- Multiple files work independently

### 4.2 Phase 2: Navigation (Week 2) - MEDIUM PRIORITY

**Goal:** Code navigation and context

**Features:**
- ✅ Hover information
  - Type definitions show resolved type
  - Fields show type + optional status
  - Models show extends chain
- ✅ Go-to-definition
  - Click on type → jump to definition
  - Works across @extends files
- ✅ Find all references
  - See all uses of a type/model
  - Works in workspace

**Deliverables:**
- Hover shows useful information
- F12 (go-to-definition) works
- Shift+F12 (find references) works

**Success Criteria:**
- Hover over `Email` type → see underlying `string` type
- Go-to-definition on field type → jump to type alias
- Find references on model → see all uses

### 4.3 Phase 3: Productivity (Week 3) - MEDIUM PRIORITY

**Goal:** Developer experience enhancements

**Features:**
- ✅ Code completion
  - Type names (built-in + user-defined)
  - Keywords (@extends, @sql, etc.)
  - Field names in model body
  - Snippets for common patterns
- ✅ Document formatting
  - Format on save
  - Format on paste
  - Configurable indent width
- ✅ Workspace management
  - Multi-file validation
  - Dependency tracking for @extends

**Deliverables:**
- Autocomplete works in all contexts
- Format on save produces consistent code
- Changes in base file validate in child contexts

**Success Criteria:**
- Type `str` → autocomplete suggests `string`
- Type `@` → autocomplete suggests plugins
- Format file → consistent indentation and spacing
- Edit `base.cdm` → `api.cdm` updates errors

### 4.4 Phase 4: Polish (Week 4) - LOW PRIORITY

**Goal:** Production-ready experience

**Features:**
- ✅ Syntax highlighting (tree-sitter queries)
- ✅ Document symbols (outline view)
- ✅ Folding ranges (collapse models/configs)
- ✅ Rename symbol (refactor across files)
- ✅ Code actions (quick fixes)
  - Add missing entity ID
  - Import missing type
  - Convert to optional field
- ✅ Semantic tokens (better coloring)

**Deliverables:**
- Rich syntax highlighting
- Outline view shows models/types
- Code folding works
- Rename refactors correctly

**Success Criteria:**
- Colors distinguish keywords, types, strings
- Outline view navigable
- Rename `User` → updates all references

---

## 5. Implementation Phases

### 5.1 Phase 1: Foundation (Week 1)

#### Step 1.1: Create `cdm-lsp` Crate (Day 1)

**Tasks:**
1. Create new crate: `cargo new --bin crates/cdm-lsp`
2. Add dependencies to `Cargo.toml`:
   ```toml
   [dependencies]
   tower-lsp = "0.20"
   tokio = { version = "1", features = ["full"] }
   serde_json = "1"
   cdm = { path = "../cdm" }
   tree-sitter = "0.20"
   ```
3. Create basic LSP server in `src/server.rs`
4. Implement `initialize()` and `shutdown()` handlers
5. Set up stdio communication in `src/main.rs`

**Output:**
- LSP server starts and responds to initialize
- Can be launched from command line
- Logs to stderr for debugging

#### Step 1.2: Position Mapping (Day 1-2)

**Tasks:**
1. Create `src/position.rs` module
2. Implement functions:
   - `lsp_position_to_byte_offset(text: &str, position: Position) -> usize`
   - `byte_offset_to_lsp_position(text: &str, offset: usize) -> Position`
   - `span_to_lsp_range(text: &str, span: Span) -> Range`
3. Handle UTF-8 vs UTF-16 encoding (LSP uses UTF-16)
4. Write unit tests for edge cases

**Output:**
- Accurate position conversion between LSP and tree-sitter
- Tests for multi-byte characters, line endings

#### Step 1.3: Document Synchronization (Day 2)

**Tasks:**
1. Create `src/document.rs` module
2. Implement `DocumentStore` struct:
   - `HashMap<Url, String>` for open documents
   - `insert()`, `get()`, `remove()` methods
3. Handle LSP notifications:
   - `textDocument/didOpen`
   - `textDocument/didChange`
   - `textDocument/didClose`
4. Store document versions for incremental updates

**Output:**
- Server tracks open documents in memory
- Changes update document content
- Close removes from memory

#### Step 1.4: Diagnostic Publishing (Day 3)

**Tasks:**
1. Create `src/handlers/diagnostics.rs`
2. Implement `publish_diagnostics()`:
   - Call `cdm::validate()` on document
   - Convert `cdm::Diagnostic` to `lsp_types::Diagnostic`
   - Map span to LSP range
   - Set severity (Error/Warning)
   - Include error code (E101, etc.)
3. Publish on document open/change
4. Clear diagnostics on document close

**Output:**
- Errors appear inline in editor
- Warnings show with different icon
- Error codes visible on hover

#### Step 1.5: VS Code Extension Skeleton (Day 3-4)

**Tasks:**
1. Create `editors/vscode-cdm/` directory
2. Initialize NPM project: `npm init`
3. Install dependencies:
   ```bash
   npm install --save vscode-languageclient
   npm install --save-dev @types/vscode @types/node
   npm install --save-dev esbuild
   ```
4. Create `src/extension.ts`:
   - Activate on `.cdm` files
   - Launch LSP server (find binary in path or bundle)
   - Configure client options
5. Create `package.json` with metadata:
   - Name: `cdm`
   - Display name: `CDM Language Support`
   - Language configuration (file extension, comment style)
6. Test with `F5` (launch extension development host)

**Output:**
- Extension activates on `.cdm` files
- Launches LSP server
- Shows errors inline

#### Step 1.6: Integration Testing (Day 4-5)

**Tasks:**
1. Create test workspace in `test_fixtures/lsp/`
2. Test files with various errors:
   - Unknown type (E103)
   - Duplicate model (E201)
   - Circular extends (E301)
3. Verify diagnostics appear correctly
4. Test document sync edge cases:
   - Rapid typing
   - Large files (>10KB)
   - Multiple files open

**Output:**
- All test cases pass
- No crashes or hangs
- Diagnostics accurate

### 5.2 Phase 2: Navigation (Week 2)

#### Step 2.1: Hover Provider (Day 6-7)

**Tasks:**
1. Create `src/handlers/hover.rs`
2. Implement `hover()`:
   - Find node at cursor using tree-sitter
   - Determine node type (identifier, type reference, etc.)
   - Look up in symbol table
   - Format hover text:
     - Type aliases: `Email: string` → show resolved type
     - Fields: `email: Email` → show type + optional status
     - Models: `User` → show extends chain + field count
3. Return as Markdown for rich formatting
4. Handle edge cases:
   - Cursor on whitespace → no hover
   - Unknown symbols → no hover
   - Built-in types → show documentation

**Example Hover Text:**
```markdown
### Type Alias: `Email`
```cdm
Email: string {
  @validation { format: "email", max_length: 320 }
  @sql { type: "VARCHAR(320)" }
} #1
```

**Resolved Type:** `string`

**Defined in:** `base.cdm:12`
```

**Output:**
- Hover shows useful information
- Works for types, fields, models
- Markdown formatting renders correctly

#### Step 2.2: Go-to-Definition (Day 7-8)

**Tasks:**
1. Create `src/handlers/definition.rs`
2. Implement `goto_definition()`:
   - Find symbol at cursor
   - Use `symbol_table.resolve_definition()` to find definition
   - Handle different symbol types:
     - Type reference → type alias or model definition
     - Field type → type definition
     - @extends path → file location
   - Return file URI + range
3. Support cross-file navigation:
   - Load ancestor symbol tables
   - Track definition source file
4. Handle edge cases:
   - Built-in types → no definition
   - Plugin names → no definition (future: jump to plugin docs)

**Output:**
- F12 jumps to definition
- Works across @extends files
- Shows "no definition found" for built-ins

#### Step 2.3: Find References (Day 8-9)

**Tasks:**
1. Extend `src/handlers/definition.rs`
2. Implement `find_references()`:
   - Use `find_references_in_resolved()` from resolved_schema
   - Search all open documents + workspace
   - Find all uses of symbol:
     - Field type references
     - Model extends references
     - Type alias usages
   - Return list of locations (file + range)
3. Support workspace-wide search:
   - Scan all `.cdm` files in workspace
   - Use tree-sitter to find identifier nodes
   - Match against symbol name
4. Optimize for performance:
   - Cache parse trees
   - Only search relevant files (via dependency graph)

**Output:**
- Shift+F12 shows all references
- Works across workspace
- Results grouped by file

### 5.3 Phase 3: Productivity (Week 3)

#### Step 3.1: Code Completion (Day 10-12)

**Tasks:**
1. Create `src/handlers/completion.rs`
2. Implement `completion()`:
   - Determine context from tree-sitter:
     - In field type position → suggest types
     - After `@` → suggest plugin names
     - In model body → suggest keywords (field syntax)
     - In union → suggest string literals or types
   - Gather completion items:
     - Built-in types: `string`, `number`, `boolean`, `JSON`
     - User-defined types (from symbol table)
     - User-defined models (from symbol table)
     - Keywords: `extends`, `true`, `false`
   - Rank by relevance:
     - Exact prefix match > fuzzy match
     - Recently used > rarely used
     - Types from current file > ancestor files
3. Add snippets for common patterns:
   - Type alias: `$1: $2 { $0 }`
   - Model: `$1 {\n  $0\n}`
   - Field: `$1: $2 $0`
4. Include documentation in completion items:
   - Show resolved type for type aliases
   - Show extends chain for models

**Example Completions:**

**Context: Field type position**
```
Email         (type alias)  Email: string
User          (model)       User { id, email, name }
string        (built-in)    Built-in string type
number        (built-in)    Built-in number type
```

**Context: After `@` at top level**
```
@extends      (directive)   Extend another CDM file
@sql          (plugin)      SQL schema generation plugin
@typescript   (plugin)      TypeScript type generation
```

**Output:**
- Intelligent completions in all contexts
- Snippets for rapid coding
- Documentation shown inline

#### Step 3.2: Document Formatting (Day 12-13)

**Tasks:**
1. Create `src/handlers/formatting.rs`
2. Implement `format_document()`:
   - Read document content
   - Call `cdm::format_file()` or `cdm::format_source()`
   - Return formatting edits (LSP TextEdit array)
   - Support indent width from client settings
3. Implement `format_on_save`:
   - Hook into `willSaveWaitUntil` notification
   - Return formatting edits before save
4. Handle errors gracefully:
   - Parse errors → no formatting
   - Return original text on error

**Output:**
- Format on save works (Ctrl+S)
- Configurable indent width
- Preserves structure, fixes whitespace

#### Step 3.3: Workspace Management (Day 13-15)

**Tasks:**
1. Create `src/workspace.rs`
2. Implement `WorkspaceManager`:
   - Track all `.cdm` files in workspace
   - Build dependency graph from @extends
   - Detect when file changes affect dependents
   - Re-validate affected files
3. Implement workspace-wide validation:
   - On workspace open: validate all files
   - On file save: validate + dependents
   - On dependency change: cascade validation
4. Optimize performance:
   - Lazy loading (don't load all files upfront)
   - Incremental validation (only changed files)
   - Parallel validation (tokio tasks)
5. Handle workspace events:
   - `workspace/didChangeWatchedFiles` → re-validate
   - `workspace/didChangeConfiguration` → update settings

**Output:**
- Multi-file projects work correctly
- Changes cascade to dependents
- No unnecessary re-validation

### 5.4 Phase 4: Polish (Week 4)

#### Step 4.1: Syntax Highlighting (Day 16-17)

**Tasks:**
1. Create `crates/grammar/queries/highlights.scm`:
   ```scheme
   ; Keywords
   (extends_directive) @keyword

   ; Types
   (type_identifier) @type
   (built_in_type) @type.builtin

   ; Strings
   (string_literal) @string

   ; Numbers
   (number_literal) @number

   ; Comments
   (comment) @comment

   ; Fields
   (field_definition name: (identifier) @variable.field)

   ; Models
   (model_definition name: (identifier) @type.definition)

   ; Plugin directives
   (plugin_import "@" @keyword (identifier) @function.macro)
   ```

2. Create `highlights.scm` and other queries:
   - `injections.scm` for embedded languages (JSON in configs)
   - `locals.scm` for scope tracking
3. Configure VS Code extension:
   - Add `semanticTokens` provider in extension
   - Map tree-sitter captures to VS Code token types
4. Fallback TextMate grammar:
   - Create `syntaxes/cdm.tmLanguage.json`
   - Basic regex-based highlighting

**Output:**
- Rich syntax highlighting
- Distinguishes keywords, types, strings, comments
- Works in editors without semantic tokens

#### Step 4.2: Document Symbols & Outline (Day 17)

**Tasks:**
1. Create `src/handlers/symbols.rs`
2. Implement `document_symbols()`:
   - Traverse tree-sitter tree
   - Extract symbols:
     - Models (SymbolKind::Class)
     - Type aliases (SymbolKind::TypeParameter)
     - Fields (SymbolKind::Field)
   - Build hierarchy (models contain fields)
   - Return with ranges for navigation
3. Configure VS Code extension to show outline view

**Output:**
- Outline view shows all models/types
- Click to navigate
- Hierarchical structure (models > fields)

#### Step 4.3: Folding Ranges (Day 18)

**Tasks:**
1. Create `src/handlers/folding.rs`
2. Implement `folding_ranges()`:
   - Find foldable regions:
     - Model bodies `{ ... }`
     - Plugin config blocks
     - Union types (long lists)
   - Return ranges with kind (region/comment)
3. Test with nested structures

**Output:**
- Collapse/expand model bodies
- Collapse plugin configs
- Visual indicators in gutter

#### Step 4.4: Rename Symbol (Day 18-19)

**Tasks:**
1. Create `src/handlers/rename.rs`
2. Implement `rename()`:
   - Find all references to symbol (reuse find_references)
   - Validate new name (valid identifier)
   - Return workspace edit with all renames
   - Support cross-file renames
3. Handle edge cases:
   - Built-in types → error
   - Conflicts with existing names → error
   - Field names (scoped to model) → only rename in that model

**Output:**
- F2 (rename) updates all references
- Works across files
- Validates identifier syntax

#### Step 4.5: Code Actions (Day 19-20)

**Tasks:**
1. Create `src/handlers/code_actions.rs`
2. Implement quick fixes:
   - **Add missing entity ID**:
     - Detect W005/W006 warnings
     - Suggest: "Add entity ID #N"
     - Generate ID based on next available
   - **Make field optional**:
     - Detect required field
     - Suggest: "Make 'fieldName' optional"
     - Add `?` suffix
   - **Remove unused type**:
     - Detect W001 warning
     - Suggest: "Remove unused type alias"
   - **Import from ancestor**:
     - Detect E103 (unknown type)
     - Check if type exists in ancestor
     - Suggest: "Type defined in base.cdm"
3. Return code actions with edits

**Output:**
- Lightbulb appears on warnings
- Quick fixes available
- Reduces manual editing

---

## 6. Technical Specifications

### 6.1 LSP Server Configuration

**Capabilities:**
```json
{
  "textDocumentSync": {
    "openClose": true,
    "change": "Incremental",
    "save": { "includeText": false }
  },
  "hoverProvider": true,
  "definitionProvider": true,
  "referencesProvider": true,
  "documentFormattingProvider": true,
  "documentRangeFormattingProvider": false,
  "completionProvider": {
    "triggerCharacters": ["@", ":", " "],
    "resolveProvider": false
  },
  "documentSymbolProvider": true,
  "foldingRangeProvider": true,
  "renameProvider": { "prepareProvider": true },
  "codeActionProvider": {
    "codeActionKinds": ["quickfix", "refactor"]
  },
  "semanticTokensProvider": {
    "legend": {
      "tokenTypes": ["keyword", "type", "variable", "string", "number", "comment"],
      "tokenModifiers": ["declaration", "definition", "readonly"]
    },
    "range": false,
    "full": { "delta": false }
  }
}
```

### 6.2 Position Mapping Details

**Challenge:** LSP uses UTF-16 code units, tree-sitter uses byte offsets

**Solution:**
```rust
pub fn lsp_position_to_byte_offset(text: &str, position: Position) -> usize {
    let mut current_line = 0;
    let mut byte_offset = 0;

    for (i, line) in text.lines().enumerate() {
        if i == position.line as usize {
            // Convert UTF-16 offset to byte offset within line
            let utf16_offset = position.character as usize;
            let mut utf16_count = 0;

            for (byte_idx, ch) in line.char_indices() {
                if utf16_count >= utf16_offset {
                    return byte_offset + byte_idx;
                }
                utf16_count += ch.len_utf16();
            }

            return byte_offset + line.len();
        }

        byte_offset += line.len() + 1; // +1 for newline
    }

    byte_offset
}
```

**Test Cases:**
- ASCII text (1 byte = 1 UTF-16 unit)
- Emoji (4 bytes, 2 UTF-16 units)
- Combining characters
- Line endings (LF vs CRLF)

### 6.3 Diagnostic Conversion

**CDM Diagnostic → LSP Diagnostic:**

```rust
pub fn to_lsp_diagnostic(
    diagnostic: &cdm::Diagnostic,
    text: &str,
) -> lsp_types::Diagnostic {
    let range = span_to_lsp_range(text, &diagnostic.span);

    lsp_types::Diagnostic {
        range,
        severity: Some(match diagnostic.severity {
            cdm::Severity::Error => DiagnosticSeverity::ERROR,
            cdm::Severity::Warning => DiagnosticSeverity::WARNING,
        }),
        code: diagnostic.code.as_ref().map(|c| {
            NumberOrString::String(c.clone())
        }),
        source: Some("cdm".to_string()),
        message: diagnostic.message.clone(),
        related_information: None,
        tags: None,
        code_description: None,
        data: None,
    }
}
```

### 6.4 Symbol Table Lookup for Hover

**Example:**
```rust
pub fn get_hover_info(
    symbol_name: &str,
    symbol_table: &SymbolTable,
) -> Option<String> {
    if let Some(definition) = symbol_table.resolve_definition(symbol_name) {
        match definition {
            Definition::TypeAlias { alias_type, config, .. } => {
                let resolved = format_type_expression(alias_type);
                Some(format!(
                    "### Type Alias: `{}`\n\n**Resolved Type:** `{}`\n\n{}",
                    symbol_name,
                    resolved,
                    format_plugin_config(config)
                ))
            }
            Definition::Model { fields, parents, .. } => {
                Some(format!(
                    "### Model: `{}`\n\n**Extends:** {}\n\n**Fields:** {}",
                    symbol_name,
                    parents.join(", "),
                    fields.len()
                ))
            }
        }
    } else {
        None
    }
}
```

### 6.5 Completion Context Detection

**Using tree-sitter to determine context:**

```rust
pub fn get_completion_context(
    tree: &Tree,
    byte_offset: usize,
) -> CompletionContext {
    let node = tree.root_node()
        .descendant_for_byte_range(byte_offset, byte_offset)?;

    match node.kind() {
        "field_definition" => {
            // In field, check if after ":"
            if is_after_colon(node, byte_offset) {
                CompletionContext::FieldType
            } else {
                CompletionContext::FieldName
            }
        }
        "plugin_import" => CompletionContext::PluginName,
        "union_type" => CompletionContext::UnionMember,
        "extends_directive" => CompletionContext::FilePath,
        _ => CompletionContext::Unknown,
    }
}
```

---

## 7. VS Code Extension

### 7.1 Extension Structure

**`package.json`:**
```json
{
  "name": "cdm",
  "displayName": "CDM Language Support",
  "description": "Language support for CDM (Contextual Data Model)",
  "version": "0.1.0",
  "publisher": "cdm-lang",
  "repository": "https://github.com/cdm-lang/cdm",
  "engines": {
    "vscode": "^1.75.0"
  },
  "categories": ["Programming Languages"],
  "keywords": ["cdm", "schema", "database", "code generation"],
  "activationEvents": ["onLanguage:cdm"],
  "main": "./dist/extension.js",
  "contributes": {
    "languages": [{
      "id": "cdm",
      "aliases": ["CDM", "cdm"],
      "extensions": [".cdm"],
      "configuration": "./language-configuration.json",
      "icon": {
        "light": "./icons/cdm-light.svg",
        "dark": "./icons/cdm-dark.svg"
      }
    }],
    "grammars": [{
      "language": "cdm",
      "scopeName": "source.cdm",
      "path": "./syntaxes/cdm.tmLanguage.json"
    }],
    "configuration": {
      "title": "CDM",
      "properties": {
        "cdm.server.path": {
          "type": "string",
          "default": "cdm-lsp",
          "description": "Path to cdm-lsp server binary"
        },
        "cdm.format.indentSize": {
          "type": "number",
          "default": 2,
          "description": "Number of spaces for indentation"
        },
        "cdm.validation.checkIds": {
          "type": "boolean",
          "default": true,
          "description": "Check for missing entity IDs (W005/W006)"
        },
        "cdm.trace.server": {
          "type": "string",
          "enum": ["off", "messages", "verbose"],
          "default": "off",
          "description": "Trace LSP communication"
        }
      }
    },
    "commands": [
      {
        "command": "cdm.validate",
        "title": "CDM: Validate File"
      },
      {
        "command": "cdm.build",
        "title": "CDM: Build Schema"
      },
      {
        "command": "cdm.migrate",
        "title": "CDM: Generate Migration"
      },
      {
        "command": "cdm.format",
        "title": "CDM: Format File"
      },
      {
        "command": "cdm.restartServer",
        "title": "CDM: Restart Language Server"
      }
    ]
  },
  "scripts": {
    "vscode:prepublish": "npm run esbuild-base -- --minify",
    "esbuild-base": "esbuild ./src/extension.ts --bundle --outfile=dist/extension.js --external:vscode --format=cjs --platform=node",
    "build": "npm run esbuild-base -- --sourcemap",
    "watch": "npm run esbuild-base -- --sourcemap --watch",
    "package": "vsce package",
    "publish": "vsce publish"
  },
  "devDependencies": {
    "@types/vscode": "^1.75.0",
    "@types/node": "^18.0.0",
    "esbuild": "^0.19.0",
    "vsce": "^2.15.0",
    "typescript": "^5.0.0"
  },
  "dependencies": {
    "vscode-languageclient": "^9.0.0"
  }
}
```

### 7.2 Extension Implementation

**`src/extension.ts`:**
```typescript
import * as path from 'path';
import * as vscode from 'vscode';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: vscode.ExtensionContext) {
  // Server binary path from settings or bundled
  const config = vscode.workspace.getConfiguration('cdm');
  const serverPath = config.get<string>('server.path') || 'cdm-lsp';

  // Server options
  const serverOptions: ServerOptions = {
    command: serverPath,
    args: [],
    transport: TransportKind.stdio
  };

  // Client options
  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: 'file', language: 'cdm' }],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher('**/*.cdm')
    },
    initializationOptions: {
      checkIds: config.get('validation.checkIds'),
      indentSize: config.get('format.indentSize')
    }
  };

  // Create and start client
  client = new LanguageClient(
    'cdm',
    'CDM Language Server',
    serverOptions,
    clientOptions
  );

  client.start();

  // Register commands
  context.subscriptions.push(
    vscode.commands.registerCommand('cdm.validate', validateCommand),
    vscode.commands.registerCommand('cdm.build', buildCommand),
    vscode.commands.registerCommand('cdm.migrate', migrateCommand),
    vscode.commands.registerCommand('cdm.restartServer', restartServer)
  );
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}

async function validateCommand() {
  const editor = vscode.window.activeTextEditor;
  if (!editor || editor.document.languageId !== 'cdm') {
    return;
  }

  // Trigger validation via LSP
  await client.sendRequest('workspace/executeCommand', {
    command: 'cdm.validate',
    arguments: [editor.document.uri.toString()]
  });
}

async function buildCommand() {
  // Execute cdm build command in terminal
  const terminal = vscode.window.createTerminal('CDM Build');
  terminal.sendText('cdm build');
  terminal.show();
}

async function migrateCommand() {
  // Prompt for migration name
  const name = await vscode.window.showInputBox({
    prompt: 'Migration name (optional)',
    placeHolder: 'add_user_fields'
  });

  const terminal = vscode.window.createTerminal('CDM Migrate');
  const cmd = name ? `cdm migrate --name ${name}` : 'cdm migrate';
  terminal.sendText(cmd);
  terminal.show();
}

async function restartServer() {
  await client.stop();
  await client.start();
  vscode.window.showInformationMessage('CDM language server restarted');
}
```

### 7.3 Language Configuration

**`language-configuration.json`:**
```json
{
  "comments": {
    "lineComment": "//"
  },
  "brackets": [
    ["{", "}"],
    ["[", "]"]
  ],
  "autoClosingPairs": [
    { "open": "{", "close": "}" },
    { "open": "[", "close": "]" },
    { "open": "\"", "close": "\"" }
  ],
  "surroundingPairs": [
    ["{", "}"],
    ["[", "]"],
    ["\"", "\""]
  ],
  "folding": {
    "markers": {
      "start": "^\\s*\\{\\s*$",
      "end": "^\\s*\\}\\s*$"
    }
  },
  "indentationRules": {
    "increaseIndentPattern": "\\{[^}\"']*$",
    "decreaseIndentPattern": "^\\s*\\}"
  }
}
```

### 7.4 Publishing to Marketplace

**Steps:**
1. Create publisher account at https://marketplace.visualstudio.com/manage
2. Get Personal Access Token from Azure DevOps
3. Login: `vsce login <publisher>`
4. Package: `vsce package` → creates `.vsix` file
5. Publish: `vsce publish` or upload `.vsix` manually
6. Update README with installation instructions

**README.md for extension:**
```markdown
# CDM Language Support

Official VS Code extension for CDM (Contextual Data Model).

## Features

- ✅ Syntax highlighting
- ✅ Real-time validation
- ✅ Code completion
- ✅ Go-to-definition
- ✅ Find references
- ✅ Document formatting
- ✅ Hover information

## Installation

Install from VS Code marketplace or run:
```bash
code --install-extension cdm-lang.cdm
```

## Requirements

- CDM CLI installed (`cargo install cdm`)
- Or bundled LSP server

## Settings

- `cdm.server.path`: Path to cdm-lsp binary
- `cdm.format.indentSize`: Indentation (default: 2)
- `cdm.validation.checkIds`: Check entity IDs (default: true)

## Commands

- **CDM: Validate File** - Validate current file
- **CDM: Build Schema** - Run cdm build
- **CDM: Generate Migration** - Run cdm migrate
- **CDM: Restart Language Server** - Restart LSP server

## License

MIT
```

---

## 8. Testing Strategy

### 8.1 Unit Tests

**Test Coverage:**
- Position mapping (20+ test cases)
- Diagnostic conversion (10+ test cases)
- Symbol lookup (30+ test cases)
- Completion context detection (25+ test cases)
- Hover formatting (15+ test cases)

**Example Test:**
```rust
#[test]
fn test_lsp_position_to_byte_offset_emoji() {
    let text = "User { 😀 }";
    let position = Position { line: 0, character: 9 }; // After emoji
    let offset = lsp_position_to_byte_offset(text, position);
    assert_eq!(offset, 11); // 7 bytes "User { " + 4 bytes emoji
}
```

### 8.2 Integration Tests

**Test Scenarios:**
1. **Full validation flow**:
   - Open file with errors
   - Verify diagnostics received
   - Fix error
   - Verify diagnostics cleared

2. **Go-to-definition across files**:
   - Open `api.cdm` extending `base.cdm`
   - Go-to-definition on type from base
   - Verify jumps to `base.cdm`

3. **Workspace validation**:
   - Open workspace with multiple files
   - Edit base file
   - Verify child files re-validated

4. **Completion in various contexts**:
   - Type in field type position
   - Verify type completions appear
   - Type after `@`
   - Verify plugin completions appear

### 8.3 End-to-End Tests

**Using VS Code Extension Test Framework:**

```typescript
import * as vscode from 'vscode';
import * as assert from 'assert';

suite('CDM Extension E2E', () => {
  test('Diagnostics appear on file open', async () => {
    const doc = await vscode.workspace.openTextDocument({
      language: 'cdm',
      content: 'User { email: UnknownType }'
    });

    await vscode.window.showTextDocument(doc);

    // Wait for diagnostics
    await sleep(2000);

    const diagnostics = vscode.languages.getDiagnostics(doc.uri);
    assert.strictEqual(diagnostics.length, 1);
    assert.strictEqual(diagnostics[0].code, 'E103');
    assert.ok(diagnostics[0].message.includes('Unknown type'));
  });

  test('Go-to-definition works', async () => {
    const baseDoc = await vscode.workspace.openTextDocument({
      language: 'cdm',
      content: 'Email: string #1'
    });

    const apiDoc = await vscode.workspace.openTextDocument({
      language: 'cdm',
      content: '@extends ./base.cdm\n\nUser { email: Email #1 } #10'
    });

    await vscode.window.showTextDocument(apiDoc);

    // Position on "Email" in field type
    const position = new vscode.Position(2, 15);

    const locations = await vscode.commands.executeCommand<vscode.Location[]>(
      'vscode.executeDefinitionProvider',
      apiDoc.uri,
      position
    );

    assert.strictEqual(locations.length, 1);
    assert.strictEqual(locations[0].uri.fsPath, baseDoc.uri.fsPath);
  });
});
```

### 8.4 Performance Tests

**Benchmarks:**
- Parse time for large files (10KB, 100KB, 1MB)
- Validation time for complex schemas (100+ models)
- Completion response time (< 100ms)
- Hover response time (< 50ms)
- Memory usage (< 100MB for typical workspace)

**Test Files:**
- `test_fixtures/lsp/performance/large_schema.cdm` (1000 models)
- `test_fixtures/lsp/performance/deep_extends.cdm` (10-level chain)
- `test_fixtures/lsp/performance/many_files.cdm` (100+ files)

---

## 9. Appendix: LSP Protocol Coverage

### 9.1 Phase 1 (Foundation)

| Method | Status | Priority |
|--------|--------|----------|
| `initialize` | ✅ Planned | HIGH |
| `shutdown` | ✅ Planned | HIGH |
| `textDocument/didOpen` | ✅ Planned | HIGH |
| `textDocument/didChange` | ✅ Planned | HIGH |
| `textDocument/didClose` | ✅ Planned | HIGH |
| `textDocument/publishDiagnostics` | ✅ Planned | HIGH |

### 9.2 Phase 2 (Navigation)

| Method | Status | Priority |
|--------|--------|----------|
| `textDocument/hover` | ✅ Planned | MEDIUM |
| `textDocument/definition` | ✅ Planned | MEDIUM |
| `textDocument/references` | ✅ Planned | MEDIUM |
| `textDocument/documentHighlight` | ⏳ Future | LOW |

### 9.3 Phase 3 (Productivity)

| Method | Status | Priority |
|--------|--------|----------|
| `textDocument/completion` | ✅ Planned | MEDIUM |
| `textDocument/formatting` | ✅ Planned | MEDIUM |
| `textDocument/rangeFormatting` | ⏳ Future | LOW |
| `textDocument/onTypeFormatting` | ⏳ Future | LOW |

### 9.4 Phase 4 (Polish)

| Method | Status | Priority |
|--------|--------|----------|
| `textDocument/documentSymbol` | ✅ Planned | MEDIUM |
| `textDocument/foldingRange` | ✅ Planned | LOW |
| `textDocument/rename` | ✅ Planned | MEDIUM |
| `textDocument/prepareRename` | ✅ Planned | MEDIUM |
| `textDocument/codeAction` | ✅ Planned | MEDIUM |
| `textDocument/semanticTokens/full` | ✅ Planned | LOW |
| `workspace/didChangeWatchedFiles` | ✅ Planned | MEDIUM |
| `workspace/didChangeConfiguration` | ✅ Planned | MEDIUM |

### 9.5 Future Enhancements

| Feature | Description | Effort |
|---------|-------------|--------|
| Workspace symbols | Search all symbols in workspace | 2 days |
| Call hierarchy | Show type usage hierarchy | 3 days |
| Inlay hints | Show inferred types inline | 2 days |
| Signature help | Show parameter info for plugins | 2 days |
| Linked editing | Edit all occurrences simultaneously | 2 days |
| Selection range | Smart selection expansion | 1 day |

---

## Next Steps

**Immediate (This Week):**
1. Review this plan with team
2. Set up `crates/cdm-lsp` crate structure
3. Add `tower-lsp` dependencies
4. Create initial server skeleton

**Short Term (2-4 Weeks):**
1. Implement Phase 1 (Foundation)
2. Test with basic VS Code extension
3. Gather user feedback
4. Iterate on diagnostics UX

**Medium Term (1-2 Months):**
1. Complete Phase 2 (Navigation)
2. Complete Phase 3 (Productivity)
3. Publish VS Code extension to marketplace
4. Document editor setup for Neovim/Emacs

**Long Term (3+ Months):**
1. Complete Phase 4 (Polish)
2. Add advanced features (workspace symbols, call hierarchy)
3. Support additional editors (IntelliJ, Sublime)
4. Build syntax highlighting packages for GitHub, Linguist

---

## Questions & Considerations

1. **Bundle LSP binary with VS Code extension?**
   - Pros: Easy installation, no setup required
   - Cons: Large extension size (~5-10MB), platform-specific builds
   - Recommendation: Support both bundled and external binary

2. **Support multiple LSP server versions?**
   - Allow extension to work with different CDM versions
   - Version negotiation in initialize
   - Graceful degradation for missing features

3. **Offline mode?**
   - All features work without network
   - Plugin metadata cached locally
   - Future: Online plugin search/install

4. **Multi-root workspace support?**
   - Each workspace folder has own CDM root
   - Separate validation contexts
   - Cross-workspace references?

5. **Real-time collaboration?**
   - Support Live Share extension
   - Sync diagnostics across users
   - Future enhancement

---

**End of Plan**
