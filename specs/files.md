# CDM Codebase File Reference

**Last Updated:** 2025-12-20
**Purpose:** Comprehensive overview of key files in the CDM codebase based on implementation analysis

---

## Table of Contents

1. [Grammar and Parsing](#grammar-and-parsing)
2. [Core Validation](#core-validation)
3. [Symbol Table](#symbol-table)
4. [File Resolution](#file-resolution)
5. [Plugin System](#plugin-system)
6. [CLI](#cli)
7. [Examples](#examples)
8. [Tests](#tests)
9. [Documentation](#documentation)

---

## Grammar and Parsing

### `/crates/grammar/grammar.js`

**Purpose:** Tree-sitter grammar definition for the CDM language
**Size:** ~800 lines
**Status:** ✅ Complete and comprehensive

**Key Features:**

- Defines complete syntax for CDM language
- Plugin imports with `@name` syntax
- Git and local plugin sources (`git:url`, `./path`)
- Extends directive (`@extends path`)
- Type aliases with plugin configuration
- Model definitions with inheritance
- Field definitions (optional, typed, with defaults)
- Plugin configuration blocks at all levels
- Union types and array types
- Field removal syntax (`-field_name`)
- Comment support

**Structure:**

```javascript
module.exports = grammar({
  name: "cdm",
  rules: {
    source_file: ($) =>
      seq(
        optional($.extends_directive),
        repeat($.plugin_import),
        repeat($._definition)
      ),
    // ... extensive rule definitions
  },
});
```

**Notable Rules:**

- `plugin_import` - Handles @plugin syntax with optional source and config
- `extends_directive` - Parses @extends file paths
- `model_definition` - Models with inheritance and fields
- `type_alias` - Type aliases with optional config blocks
- `field_definition` - Fields with optionality, types, defaults, and config
- `type_expression` - Union types, arrays, identifiers
- `plugin_config_block` - Plugin-specific configuration

**Quality:** Well-structured, handles complex nested structures, good support for recovery

---

## Core Validation

### `/crates/cdm/src/validate.rs`

**Purpose:** Core semantic validation engine for CDM files
**Size:** ~1,200 lines of implementation + 4,270 lines of tests
**Status:** ✅ Highly complete and well-tested
**Last Updated:** 2025-12-21 (Added validate_tree for LoadedFileTree integration)

**Key Responsibilities:**

1. **Symbol Table Building** - Collects all type aliases and models
2. **Type Resolution** - Resolves all type references
3. **Inheritance Processing** - Applies extends clauses and field inheritance
4. **Semantic Validation** - Enforces all language rules
5. **Error Collection** - Gathers and reports diagnostic errors
6. **File Tree Validation** - Validates loaded file trees from FileResolver

**Main Structures:**

```rust
pub struct ValidationResult {
    pub diagnostics: Vec<Diagnostic>,
    pub tree: Option<tree_sitter::Tree>,
    pub symbol_table: SymbolTable,
    pub model_fields: HashMap<String, Vec<FieldInfo>>,
}
```

**Public API:**

1. **`validate(source: &str, ancestors: &[Ancestor]) -> ValidationResult`**

   - Low-level validation of a single source string
   - Takes pre-built ancestors for cross-file type resolution
   - Returns ValidationResult with diagnostics, tree, symbols
   - Use when you have raw source and ancestors already built

2. **`validate_tree(tree: LoadedFileTree) -> Result<ValidationResult, Vec<Diagnostic>>`** ⭐ NEW

   - High-level validation of a complete file tree from FileResolver
   - Streaming validation: validates ancestors one-by-one, converts to Ancestor, frees memory
   - Minimizes peak memory usage (~60% savings vs keeping all ValidationResults)
   - Returns validated main file or errors
   - **Recommended API** for validating CDM schemas with @extends

   **Example:**

   ```rust
   use cdm::{FileResolver, validate_tree};

   let tree = FileResolver::load("schema.cdm")?;
   let result = validate_tree(tree)?;

   // result.symbol_table has all definitions
   // result.model_fields has all resolved fields
   ```

**Internal Validation Methods:**

3. **`build_symbol_table()`**

   - First pass: collect all type aliases and models
   - Handles basic syntax validation
   - Detects duplicate definitions

4. **`resolve_type_aliases()`**

   - Resolves all type expressions in aliases
   - Detects circular dependencies
   - Builds type alias dependency graph

5. **`resolve_model_fields()`**

   - Resolves field types for all models
   - Applies inheritance (extends clauses)
   - Processes field removals and overrides
   - Validates default value types

6. **`apply_field_inheritance(model_name, parent_chain)`**

   - Recursive inheritance processing
   - Multiple parent support with conflict resolution
   - Field removal handling
   - Deep inheritance chain support

7. **`check_default_value_type(field_type, default_value)`**
   - Type-checks default values against field types
   - Supports primitives, arrays, unions, objects
   - Detailed error messages for mismatches

**Validation Rules Implemented:**

**Type System:**

- ✅ E101: Duplicate type alias detection
- ✅ E102: Circular type alias detection
- ✅ E103: Unknown type reference detection
- ✅ Type shadowing warnings (ancestors and built-ins)

**Model System:**

- ✅ E201: Duplicate model detection
- ✅ E202: Duplicate field detection
- ✅ E203: Unknown parent in extends
- ✅ E204: Invalid field removal (not in parent)
- ✅ E205: Invalid field override (not inherited)
- ✅ Circular inheritance detection

**Default Values:**

- ✅ String literal type checking
- ✅ Number literal type checking
- ✅ Boolean literal type checking
- ✅ Array element type checking
- ✅ Union member type checking
- ✅ Nested type resolution

**Test Coverage:** Exceptional - 30+ test functions covering:

- Basic type resolution
- Union types (string literals, type references, mixed)
- Array types
- Optional fields
- Default values with type checking
- Inheritance (single, multiple, deep chains)
- Field removals
- Field overrides
- Circular dependencies
- Cross-file type resolution (with ancestors)
- Error recovery

**Code Quality:**

- Well-organized with clear separation of concerns
- Comprehensive error messages with source locations
- Good use of Rust idioms
- Extensive comments explaining complex logic

---

### `/crates/cdm/src/resolved_schema.rs`

**Purpose:** Merged schema view after inheritance and removals
**Size:** ~250 lines
**Status:** ✅ Complete
**Added:** 2025-12-21

**Design Philosophy:**
ResolvedSchema provides a "final view" of a CDM schema after applying inheritance and removals. It keeps per-file symbol tables file-scoped (with file-relative spans) while providing an on-demand merged view for validation and code generation.

**Key Features:**

- ✅ **Merged view** - Combines current file + all inherited definitions
- ✅ **Source tracking** - Each resolved item tracks which file it came from
- ✅ **Removal application** - Respects `-TypeName` and `-ModelName` removals
- ✅ **Reference finding** - Locates all references to a specific definition
- ✅ **Reusable** - Used for removal validation, will be used for schema builder

**Exported Types:**

```rust
/// The fully resolved schema
pub struct ResolvedSchema {
    pub type_aliases: HashMap<String, ResolvedTypeAlias>,
    pub models: HashMap<String, ResolvedModel>,
}

pub struct ResolvedTypeAlias {
    pub name: String,
    pub type_expr: String,
    pub references: Vec<String>,
    pub source_file: String,  // For error reporting
    pub source_span: Span,
}

pub struct ResolvedModel {
    pub name: String,
    pub fields: Vec<ResolvedField>,
    pub source_file: String,
    pub source_span: Span,
}

pub struct ResolvedField {
    pub name: String,
    pub type_expr: Option<String>,
    pub optional: bool,
    pub source_file: String,
    pub source_span: Span,
}
```

**Public API:**

1. **`build_resolved_schema(...) -> ResolvedSchema`**

   - Merges symbol tables from ancestors (oldest first)
   - Applies current file's definitions (override ancestors)
   - Applies removals
   - Returns final merged view

2. **`find_references_in_resolved(...) -> Vec<String>`**
   - Finds all references to a specific definition name
   - Returns formatted list with source file info
   - Used for removal validation (E302, E303)

**Usage:**

```rust
use cdm::{build_resolved_schema, find_references_in_resolved};

// Build merged view
let resolved = build_resolved_schema(
    &symbol_table,
    &model_fields,
    &ancestors,
    &removals
);

// Check if Email is referenced anywhere
let refs = find_references_in_resolved(&resolved, "Email");
// refs = ["User.email", "Admin.contact (inherited from base.cdm)"]
```

**Integration:**

- Used by `validate_removals()` for E302 and E303 validation
- Will be used by schema builder for generating plugin input
- Provides foundation for type checking and code generation

**Architectural Benefits:**

- ✅ **Separation of concerns** - File-scoped storage vs. runtime view
- ✅ **Error reporting** - Can point to original source file and span
- ✅ **Cache friendly** - Per-file symbol tables can be cached
- ✅ **Standard pattern** - Mirrors how TypeScript, Rust, Java compilers work

---

## File Resolution

### `/crates/cdm/src/file_resolver.rs`

**Purpose:** Loads CDM files and resolves @extends dependencies with lazy loading (DOES NOT validate)
**Size:** ~200 lines (including tests)
**Status:** ✅ Complete and tested
**Added:** 2025-12-20
**Last Updated:** 2025-12-21 (Lazy loading + complete decoupling from validation)

**Design Philosophy:**
FileResolver is responsible ONLY for file I/O and dependency resolution. It does NOT perform validation or parsing - that's the responsibility of the `validate` module. Files are loaded lazily and cached on first access, minimizing memory usage.

**Key Features:**

- ✅ **Lazy loading** - Files not read until `.source()` is called
- ✅ **Cached reading** - First `.source()` call caches result for subsequent calls
- ✅ **Zero validation coupling** - No imports from validate module
- ✅ **Minimal memory** - ~100 bytes per LoadedFile before reading (~5-20KB after)

**Exported Types:**

```rust
/// A loaded CDM file with lazy source reading and caching
pub struct LoadedFile {
    pub path: PathBuf,
    cached_source: RefCell<Option<String>>,  // Lazy + cached
}

impl LoadedFile {
    /// Get source content, reading and caching on first access
    pub fn source(&self) -> Result<String, std::io::Error>
}

/// The complete file tree with a main file and its ancestors
pub struct LoadedFileTree {
    pub main: LoadedFile,
    pub ancestors: Vec<LoadedFile>,  // In dependency order
}

/// File resolver with circular dependency detection
pub struct FileResolver {
    loaded_files: HashSet<PathBuf>,
}
```

**Public API:**

1. **`FileResolver::load(file_path) -> Result<LoadedFileTree, Vec<Diagnostic>>`**
   - Loads a CDM file and all its @extends dependencies (paths only)
   - Returns LoadedFileTree with lazy-loaded files
   - NO validation performed - pure file I/O
   - Files not read until you call `.source()`
   - Memory: ~100 bytes per file (before reading)

**Internal Methods:**

2. **`load_file_tree(file_path) -> Result<LoadedFileTree, Vec<Diagnostic>>`**

   - Internal: Recursively loads all files in dependency tree
   - Returns raw LoadedFileTree without validation
   - Detects circular dependencies
   - Reads files to extract @extends directives (then caches)

3. **`load_single_file(file_path) -> Result<LoadedFile, Vec<Diagnostic>>`**

   - Internal: Creates a LoadedFile without reading the file
   - Checks for circular dependencies
   - File is read lazily on first `.source()` call

4. **`resolve_path(current_file, extends_path) -> PathBuf`**

   - Resolves relative paths from @extends directives
   - Handles `./types.cdm`, `../shared/base.cdm`, etc.

5. **`to_absolute_path(path) -> Result<PathBuf, Vec<Diagnostic>>`**
   - Converts relative paths to absolute
   - Uses `canonicalize()` for path resolution

**Error Handling:**

- **Circular Dependencies**: Detects when a file appears twice in @extends chain
- **File Not Found**: Reports when @extends references non-existent file
- **Invalid Paths**: Reports path resolution failures
- **Read Errors**: I/O errors propagated from `.source()` calls

**Test Coverage:** 6 comprehensive tests

1. **`test_load_single_file`** - Raw file loading, no ancestors
2. **`test_load_with_single_extends`** - Raw load with 1 ancestor
3. **`test_load_with_multiple_extends`** - Raw load with 2 ancestors
4. **`test_load_nested_chain`** - Raw load of 3-level chain
5. **`test_load_circular_detected`** - Circular detection
6. **`test_load_file_not_found`** - File not found error

**Test Fixtures:** Comprehensive test fixtures in `test_fixtures/file_resolver/`:

- `single_file/simple.cdm` - Standalone file
- `single_extends/base.cdm` and `child.cdm` - Simple inheritance
- `multiple_extends/types.cdm`, `mixins.cdm`, `child.cdm` - Multiple @extends
- `nested_chain/base.cdm`, `client.cdm`, `mobile.cdm` - 3-level chain
- `circular/a.cdm`, `b.cdm` - Circular dependency test
- `invalid/missing_extends.cdm` - Error handling test

**Integration:**

- Exported types: `FileResolver`, `LoadedFile`, `LoadedFileTree`
- Used with `validate_tree()` from validate module for validation
- Foundation for `cdm build` and `cdm migrate` commands
- Lazy loading reduces memory for large schemas

**Code Quality:**

- ✅ **Perfect separation of concerns** - Zero coupling to validation
- ✅ **Lazy loading** - Files read only when needed
- ✅ **Cached reading** - `RefCell<Option<String>>` pattern for caching
- ✅ Proper error handling with Diagnostic structs
- ✅ Recursive algorithm with circular dependency detection
- ✅ Well-documented with rustdoc comments
- ✅ Idiomatic Rust with interior mutability for caching

**Architecture Benefits:**

- Zero coupling: FileResolver has NO dependency on validate module
- Testability: File loading tested completely independently
- Reusability: Can be used by formatters, linters, LSP servers, etc.
- Memory efficiency: ~100 bytes per file before reading (vs 5-20KB eager loading)
- Performance: Only reads files that are actually needed

**Memory Profile:**

- **Before reading**: ~100 bytes per file (PathBuf + RefCell overhead)
- **After reading**: ~5-20KB per file (path + source string cached)
- **Lazy benefit**: If you only need some files, others stay at ~100 bytes

---

### `/crates/cdm/src/grammar_parser.rs`

**Purpose:** Tree-sitter parsing with caching for CDM files
**Size:** ~196 lines (including tests)
**Status:** ✅ Complete and tested
**Added:** 2025-12-21
**Last Updated:** 2025-12-21

**Design Philosophy:**
GrammarParser provides a caching layer on top of tree-sitter parsing. It wraps a LoadedFile and lazily parses the source code, caching the result. This eliminates duplicate parsing when multiple operations need the syntax tree.

**Key Features:**

- ✅ **Lazy parsing** - Tree parsed only on first use
- ✅ **Cached tree** - Uses `RefCell<Option<tree_sitter::Tree>>` for caching
- ✅ **@extends extraction** - Extracts @extends paths from parsed tree
- ✅ **Lifetime-correct** - Returns `Ref<'_, tree_sitter::Tree>` tied to GrammarParser lifetime
- ✅ **Zero copy** - Reuses cached tree without cloning

**Exported Type:**

```rust
pub struct GrammarParser<'a> {
    loaded_file: &'a LoadedFile,
    cached_tree: RefCell<Option<tree_sitter::Tree>>,
}
```

**Public API:**

1. **`GrammarParser::new(&LoadedFile) -> GrammarParser`**

   - Creates parser for a loaded file
   - Does not parse immediately (lazy)

2. **`parse(&self) -> Result<Ref<'_, tree_sitter::Tree>, String>`**

   - Parses the file using tree-sitter (once)
   - Returns reference to cached tree
   - Subsequent calls return cached tree
   - Example:

   ```rust
   let parser = GrammarParser::new(&loaded_file);
   let tree = parser.parse()?;
   assert_eq!(tree.root_node().kind(), "source_file");
   ```

3. **`extract_extends_paths(&self) -> Vec<String>`**
   - Parses file (if not already) and extracts @extends paths
   - Returns paths in order they appear
   - Uses cached tree if available
   - Example:
   ```rust
   let parser = GrammarParser::new(&loaded_file);
   let paths = parser.extract_extends_paths();
   // paths = ["./base.cdm", "./mixins.cdm"]
   ```

**Integration:**

- Used by FileResolver to extract @extends directives
- Can be used by LSP server for syntax highlighting
- Can be used by formatter/linter tools
- Replaces duplicate parsing logic previously in validate module

**Test Coverage:** 5 comprehensive tests

1. **`test_parse_single_file`** - Parse and verify root node
2. **`test_extract_extends_no_extends`** - File with no @extends
3. **`test_extract_extends_single`** - Single @extends directive
4. **`test_extract_extends_multiple`** - Multiple @extends directives
5. **`test_parse_caching`** - Verify caching works (multiple calls)

**Code Quality:**

- ✅ **Caching with RefCell** - Interior mutability for lazy init
- ✅ **Lifetime annotations** - Explicit lifetime `'_` for Ref return
- ✅ **Error handling** - Returns Result with clear error messages
- ✅ **Separation of concerns** - Only handles parsing, not validation
- ✅ **Well-documented** - Rustdoc with examples
- ✅ **Idiomatic Rust** - Uses `Ref::map` for returning cached reference

**Architecture Benefits:**

- Eliminates duplicate parsing (FileResolver and Validate used to parse separately)
- Single source of truth for tree-sitter parsing logic
- Can be reused by other tools (LSP, linters, formatters)
- Clean layering: FileResolver → GrammarParser → Validate

**Memory Profile:**

- Unparsed: ~16 bytes (reference + RefCell overhead)
- Parsed: ~5-15KB (tree structure, depends on file size)
- Tree cached indefinitely while GrammarParser exists

---

### `/crates/cdm/src/symbol_table.rs`

**Purpose:** Symbol table data structures for type and model information
**Size:** ~300 lines
**Status:** ✅ Complete

**Key Structures:**

```rust
pub struct TypeAliasInfo {
    pub name: String,
    pub node_id: usize,
    pub alias_type: Option<TypeExpression>,  // Resolved type
    pub plugin_configs: Vec<PluginConfig>,
    pub source_file: Option<String>,
}

pub struct ModelInfo {
    pub name: String,
    pub node_id: usize,
    pub parents: Vec<String>,
    pub fields: HashMap<String, FieldInfo>,
    pub plugin_configs: Vec<PluginConfig>,
    pub source_file: Option<String>,
}

pub struct FieldInfo {
    pub name: String,
    pub node_id: usize,
    pub field_type: Option<TypeExpression>,
    pub optional: bool,
    pub default: Option<Value>,
    pub plugin_configs: Vec<PluginConfig>,
    pub is_inherited: bool,
    pub inherited_from: Option<String>,
}

pub struct Ancestor {
    pub path: PathBuf,
    pub type_aliases: HashMap<String, TypeAliasInfo>,
    pub models: HashMap<String, ModelInfo>,
}
```

**Key Enums:**

```rust
pub enum TypeExpression {
    Identifier(String),
    Array(Box<TypeExpression>),
    Union(Vec<TypeExpression>),
    StringLiteral(String),
}

pub enum Value {
    String(String),
    Number(f64),
    Boolean(bool),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}
```

**Purpose of Each:**

- **TypeAliasInfo**: Stores type alias definitions with resolved types
- **ModelInfo**: Stores model definitions with inheritance and fields
- **FieldInfo**: Stores field details including inheritance tracking
- **Ancestor**: Stores symbol tables from parent contexts (for @extends)
- **TypeExpression**: Represents all possible type forms
- **Value**: Represents all possible default value forms

**Notable Features:**

- `is_inherited` flag tracks whether field came from parent
- `inherited_from` tracks which parent provided the field
- `source_file` enables cross-file error reporting
- `node_id` links back to tree-sitter AST nodes
- Support for multiple plugin configs per definition

---

## Plugin System

### `/crates/cdm-plugin-api/src/lib.rs`

**Purpose:** Public API for CDM plugin development
**Size:** ~400 lines
**Status:** ✅ Complete API definition

**Exported Types:**

```rust
// Configuration validation
pub enum ConfigLevel {
    Global,
    Model { name: String },
    Field { model: String, field: String },
}

pub struct PathSegment {
    pub kind: String,  // "global", "model", "field", "config", etc.
    pub name: String,
}

pub enum Severity {
    Error,
    Warning,
}

pub struct ValidationError {
    pub path: Vec<PathSegment>,
    pub message: String,
    pub severity: Severity,
}

// Schema representation
pub struct Schema {
    pub type_aliases: Vec<TypeAliasDefinition>,
    pub models: Vec<ModelDefinition>,
}

pub struct TypeAliasDefinition {
    pub name: String,
    pub alias_type: TypeExpression,
    pub config: HashMap<String, serde_json::Value>,
}

pub struct ModelDefinition {
    pub name: String,
    pub parents: Vec<String>,
    pub fields: Vec<FieldDefinition>,
    pub config: HashMap<String, serde_json::Value>,
}

pub struct FieldDefinition {
    pub name: String,
    pub field_type: TypeExpression,
    pub optional: bool,
    pub default: Option<Value>,
    pub config: HashMap<String, serde_json::Value>,
}

// Type system
pub enum TypeExpression {
    Identifier(String),
    Array(Box<TypeExpression>),
    Union(Vec<TypeExpression>),
    StringLiteral(String),
}

pub enum Value {
    String(String),
    Number(f64),
    Boolean(bool),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

// Deltas for migrations
pub enum Delta {
    ModelAdded { name: String, after: ModelDefinition },
    ModelRemoved { name: String, before: ModelDefinition },
    ModelRenamed { old_name: String, new_name: String, before: ModelDefinition, after: ModelDefinition },
    FieldAdded { model: String, field: String, after: FieldDefinition },
    FieldRemoved { model: String, field: String, before: FieldDefinition },
    FieldRenamed { model: String, old_name: String, new_name: String, before: FieldDefinition, after: FieldDefinition },
    FieldTypeChanged { model: String, field: String, before: TypeExpression, after: TypeExpression },
    FieldOptionalityChanged { model: String, field: String, before: bool, after: bool },
    FieldDefaultChanged { model: String, field: String, before: Option<Value>, after: Option<Value> },
    TypeAliasAdded { name: String, after: TypeAliasDefinition },
    TypeAliasRemoved { name: String, before: TypeAliasDefinition },
    TypeAliasTypeChanged { name: String, before: TypeExpression, after: TypeExpression },
    InheritanceAdded { model: String, parent: String },
    InheritanceRemoved { model: String, parent: String },
    GlobalConfigChanged { before: serde_json::Value, after: serde_json::Value },
    ModelConfigChanged { model: String, before: serde_json::Value, after: serde_json::Value },
    FieldConfigChanged { model: String, field: String, before: serde_json::Value, after: serde_json::Value },
}

// Output files
pub struct OutputFile {
    pub path: String,
    pub content: String,
}

// Utilities
pub struct Utils;

impl Utils {
    pub fn change_case(&self, input: &str, format: CaseFormat) -> String;
}

pub enum CaseFormat {
    Snake,      // user_profile
    Camel,      // userProfile
    Pascal,     // UserProfile
    Kebab,      // user-profile
    Constant,   // USER_PROFILE
    Title,      // User Profile
}
```

**Plugin Function Signatures:**

```rust
// Required: Returns plugin's settings schema
pub fn schema() -> String;

// Required: Validates configuration at each level
pub fn validate_config(
    level: ConfigLevel,
    config: serde_json::Value,
    utils: &Utils,
) -> Vec<ValidationError>;

// Optional: Generates output files
pub fn generate(
    schema: Schema,
    config: serde_json::Value,
    utils: &Utils,
) -> Vec<OutputFile>;

// Optional: Generates migration files from deltas
pub fn migrate(
    schema: Schema,
    deltas: Vec<Delta>,
    config: serde_json::Value,
    utils: &Utils,
) -> Vec<OutputFile>;
```

**Key Features:**

- All types are serializable (derive Serialize, Deserialize)
- Comprehensive delta types for all possible schema changes
- Structured error paths for precise error reporting
- Utility functions for common transformations
- Clean separation between required and optional functions

---

### `/crates/cdm/src/plugin_runner.rs`

**Purpose:** WASM plugin execution engine using wasmtime
**Size:** ~400 lines
**Status:** 🚧 Core infrastructure complete, integration pending

**Key Structure:**

```rust
pub struct PluginRunner {
    engine: Engine,
    linker: Linker<()>,
}

impl PluginRunner {
    pub fn new() -> Result<Self>;
    pub fn load_plugin(&self, wasm_path: &Path) -> Result<LoadedPlugin>;
    pub fn call_schema(&self, plugin: &LoadedPlugin) -> Result<String>;
    pub fn call_validate_config(
        &self,
        plugin: &LoadedPlugin,
        level: ConfigLevel,
        config: serde_json::Value,
    ) -> Result<Vec<ValidationError>>;
    pub fn call_generate(
        &self,
        plugin: &LoadedPlugin,
        schema: Schema,
        config: serde_json::Value,
    ) -> Result<Vec<OutputFile>>;
    pub fn call_migrate(
        &self,
        plugin: &LoadedPlugin,
        schema: Schema,
        deltas: Vec<Delta>,
        config: serde_json::Value,
    ) -> Result<Vec<OutputFile>>;
}

pub struct LoadedPlugin {
    instance: Instance,
    store: Store<()>,
}
```

**Implementation Details:**

1. **Memory Management:**

   - Calls `_alloc(size: u32) -> u32` in WASM to allocate memory
   - Calls `_dealloc(ptr: u32, size: u32)` to free memory
   - Handles memory growth automatically

2. **Data Passing:**

   - Serializes Rust structs to JSON
   - Writes JSON bytes to WASM memory
   - Passes pointer and length to WASM function
   - Reads return value from WASM memory
   - Deserializes JSON response

3. **Function Calling:**

   - `schema()` - No args, returns string
   - `validate_config(level_ptr, level_len, config_ptr, config_len)` - Returns ValidationError array
   - `generate(schema_ptr, schema_len, config_ptr, config_len)` - Returns OutputFile array
   - `migrate(schema_ptr, schema_len, deltas_ptr, deltas_len, config_ptr, config_len)` - Returns OutputFile array

4. **Error Handling:**
   - WASM trap detection
   - Memory allocation failures
   - Serialization/deserialization errors
   - Missing function exports

**Features:**

- ✅ WASM module loading
- ✅ Memory allocation/deallocation
- ✅ JSON serialization for data exchange
- ✅ All four plugin functions supported
- ⏳ Resource limits (memory, time) not yet enforced
- ⏳ Error context improvement needed

---

### `/crates/cdm-plugin-docs/src/lib.rs`

**Purpose:** Example CDM plugin that generates documentation
**Size:** ~600 lines
**Status:** ✅ Fully functional example plugin

**Capabilities:**

- Generates markdown documentation
- Generates HTML documentation
- Generates JSON schema export

**Implementation:**

```rust
use cdm_plugin_api::*;

#[no_mangle]
pub extern "C" fn schema() -> *const u8 {
    let schema_content = include_str!("../schema.cdm");
    write_string_to_wasm_memory(schema_content)
}

#[no_mangle]
pub extern "C" fn validate_config(
    level_ptr: *const u8,
    level_len: usize,
    config_ptr: *const u8,
    config_len: usize,
) -> *const u8 {
    // Read inputs from WASM memory
    let level: ConfigLevel = read_from_wasm_memory(level_ptr, level_len);
    let config: serde_json::Value = read_from_wasm_memory(config_ptr, config_len);

    // Validate configuration
    let errors = validate_config_impl(level, config);

    // Write result to WASM memory
    write_json_to_wasm_memory(&errors)
}

#[no_mangle]
pub extern "C" fn generate(
    schema_ptr: *const u8,
    schema_len: usize,
    config_ptr: *const u8,
    config_len: usize,
) -> *const u8 {
    let schema: Schema = read_from_wasm_memory(schema_ptr, schema_len);
    let config: serde_json::Value = read_from_wasm_memory(config_ptr, config_len);

    let output_files = generate_impl(schema, config);
    write_json_to_wasm_memory(&output_files)
}

// Memory management exports
#[no_mangle]
pub extern "C" fn _alloc(size: u32) -> *mut u8 {
    let mut buf = Vec::with_capacity(size as usize);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

#[no_mangle]
pub extern "C" fn _dealloc(ptr: *mut u8, size: u32) {
    unsafe {
        let _ = Vec::from_raw_parts(ptr, size as usize, size as usize);
    }
}
```

**Generated Output:**

- Markdown with model and field tables
- HTML with styled documentation
- JSON with full schema export

**Key Learnings:**

- Shows complete plugin implementation pattern
- Demonstrates memory management
- Good example of configuration validation
- Shows output file generation

---

### `/crates/cdm-plugin-docs/src/schema.rs`

**Purpose:** Settings schema for the docs plugin
**Size:** ~40 lines
**Status:** ✅ Example of plugin schema

Plugins expose their schema via the `schema()` WASM function:

```rust
#[no_mangle]
pub extern "C" fn schema() -> *const u8 {
    let schema = r#"
GlobalSettings {
    format: "markdown" | "html" | "json" = "markdown"
    output_file?: string
    include_inherited: boolean = true
    include_plugin_config: boolean = false
}

ModelSettings {
    skip: boolean = false
    heading_level: number = 2
}

FieldSettings {
    skip: boolean = false
    description?: string
}
"#;
    write_string_to_wasm_memory(schema)
}
```

**Purpose:**

- Defines valid configuration for the plugin
- Shows three configuration levels
- Demonstrates default values
- Example of union types in config
- Returned as a string from WASM function

---

## CLI

### `/crates/cdm/src/main.rs`

**Purpose:** Command-line interface entry point
**Size:** ~150 lines
**Status:** 🚧 Basic validate command only

**Current Implementation:**

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cdm")]
#[command(about = "CDM language tooling")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate CDM files
    Validate {
        /// File to validate
        file: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Validate { file } => {
            let source = fs::read_to_string(&file)?;
            match validate(&source) {
                Ok(_) => {
                    println!("✓ Validation successful");
                    Ok(())
                }
                Err(diagnostics) => {
                    for diag in diagnostics {
                        println!("{}", diag);
                    }
                    std::process::exit(1);
                }
            }
        }
    }
}
```

**Implemented:**

- ✅ Basic CLI structure with clap
- ✅ `validate` subcommand for single file
- ✅ Error reporting
- ✅ Exit codes

**Missing:**

- ⏳ `build` command
- ⏳ `migrate` command
- ⏳ `plugin` commands
- ⏳ Multi-file validation
- ⏳ Output formatting options
- ⏳ Context chain resolution

---

## Examples

### `/examples/base.cdm`

**Purpose:** Base schema example showing core features
**Size:** ~80 lines
**Status:** ✅ Comprehensive example

**Contents:**

```cdm
@sql {
    dialect: "postgres",
    schema: "public",
    generate_output: "./db/schema",
    migrations_output: "./db/migrations"
}

Email: string {
    @sql { type: "VARCHAR(320)" }
}

Status: "active" | "pending" | "suspended"

Timestamped {
    created_at: string
    updated_at: string
}

User extends Timestamped {
    id: string
    email: Email
    name: string
    status: Status = "pending"

    @sql {
        table: "users",
        indexes: [{ fields: ["email"], unique: true }]
    }
}

Post extends Timestamped {
    id: string
    author: User
    title: string
    content: string
    published: boolean = false

    @sql { table: "posts" }
}
```

**Demonstrates:**

- Plugin imports with configuration
- Type aliases (simple and with config)
- Union types
- Model inheritance
- Field types and defaults
- Model-level plugin configuration
- Relationships between models

---

### `/examples/client.cdm`

**Purpose:** Context file example showing @extends
**Size:** ~30 lines
**Status:** ✅ Shows context capabilities

**Contents:**

```cdm
@extends ./base.cdm

User {
    -created_at
    -updated_at

    avatar_url?: string
    is_online: boolean = false
}

Post {
    -content

    summary?: string
}
```

**Demonstrates:**

- @extends directive
- Model modification in context
- Field removal
- Field addition
- Inheriting from base schema

---

## Tests

### `/crates/cdm/src/validate.rs` (Test Module)

**Purpose:** Comprehensive validation tests
**Size:** 4,189 lines (larger than implementation!)
**Status:** ✅ Exceptional test coverage

**Test Categories:**

1. **Type Resolution (10+ tests)**

   - `test_basic_type_resolution()`
   - `test_union_type_resolution()`
   - `test_array_type_resolution()`
   - `test_unknown_type_error()`
   - `test_circular_type_alias()`

2. **Default Values (10+ tests)**

   - `test_default_value_type_checking_string()`
   - `test_default_value_type_checking_number()`
   - `test_default_value_type_checking_boolean()`
   - `test_default_value_type_checking_array()`
   - `test_default_value_union_type()`
   - `test_default_value_type_mismatch()`

3. **Inheritance (15+ tests)**

   - `test_single_inheritance()`
   - `test_multiple_inheritance()`
   - `test_field_removal()`
   - `test_field_override()`
   - `test_deep_inheritance_chain()`
   - `test_circular_inheritance_detection()`
   - `test_field_conflict_resolution()`

4. **Cross-file Resolution (5+ tests)**

   - `test_type_resolution_with_ancestors()`
   - `test_model_inheritance_with_ancestors()`
   - `test_field_type_from_ancestor()`

5. **Error Detection (10+ tests)**
   - `test_duplicate_model_error()`
   - `test_duplicate_field_error()`
   - `test_unknown_parent_error()`
   - `test_invalid_field_removal()`
   - `test_invalid_field_override()`

**Test Quality:**

- Comprehensive coverage of all features
- Clear test names describing scenario
- Both positive (should work) and negative (should error) tests
- Edge case coverage
- Good error message validation

---

## Documentation

### `/specs/spec.md`

**Purpose:** Complete CDM language specification
**Size:** 1,808 lines
**Status:** ✅ Comprehensive and well-structured

**Sections:**

1. Introduction (purpose, design goals, core concepts)
2. Lexical Structure (whitespace, comments, identifiers, literals)
3. Type System (built-in types, type expressions, compatibility)
4. Type Aliases (basic, with config, union types)
5. Models (fields, relationships, plugin config)
6. Inheritance (single, multiple, field removal/override)
7. Context System (@extends, capabilities, merging)
8. Plugin System (imports, sources, execution)
9. Semantic Validation (phases, rules, error catalog)
10. File Structure and Resolution
11. CLI Interface (all commands documented)
12. Plugin Development (structure, API, types)
13. Appendix A: Grammar (EBNF)
14. Appendix B: Error Catalog (all error codes)
15. Appendix C: Registry Format
16. Appendix D: Data Exchange Format (JSON schemas)

**Quality:**

- Extremely detailed and well-organized
- Clear examples for all features
- Complete grammar specification
- Full error code catalog
- JSON format specifications

---

### `/specs/plugins.md`

**Purpose:** Plugin system specification and developer guide
**Size:** 1,031 lines
**Status:** ✅ Comprehensive hybrid documentation

**Structure:**

1. Quick Start (using and creating plugins)
2. Importing Plugins (syntax, sources)
3. Plugin Sources (registry, git, local)
4. Configuration Levels (global, model, field)
5. Creating a Plugin (structure, manifest)
6. Plugin Functions (schema, validate_config, generate, migrate)
7. Settings Schema (GlobalSettings, ModelSettings, FieldSettings)
8. Deltas (all delta types with examples)
9. Utilities (change_case)
10. Architecture (resolver, cache, loader, runtime)
11. CLI Commands (plugin management)
12. Plugin Development (building, testing, publishing)
13. Appendix: Grammar Changes

**Quality:**

- Hybrid approach: user-friendly + technical reference
- Extensive code examples
- Architecture diagrams
- Complete API reference
- Publishing workflow documented

---

### `/specs/tasks.md`

**Purpose:** Implementation task breakdown and progress tracking
**Size:** ~900 lines
**Status:** ✅ Comprehensive roadmap

**Contents:**

- Task breakdown by spec section
- Status indicators (✅ ✓ 🚧 ⏳ 🔍)
- ~250+ individual tasks tracked
- Progress statistics (60% overall)
- Critical path to MVP (4 phases)
- Implementation notes and findings

---

### `/crates/cdm-plugin-api/README.md`

**Purpose:** Plugin development guide
**Size:** 364 lines
**Status:** ✅ User-friendly getting started guide

**Contents:**

- Quick start example
- Plugin structure
- Configuration levels
- Function signatures with examples
- Delta types table
- Utilities reference
- CLI commands
- Testing locally

**Audience:** Plugin developers new to CDM

---

## Project Configuration Files

### `/Cargo.toml` (Workspace)

**Purpose:** Rust workspace configuration
**Status:** ✅ Well-organized workspace

**Members:**

- `crates/cdm` - Main CLI and validator
- `crates/grammar` - Tree-sitter grammar
- `crates/cdm-plugin-api` - Plugin API types
- `crates/cdm-plugin-docs` - Example plugin

**Workspace Dependencies:**

```toml
tree-sitter = "0.20"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
wasmtime = "15.0"
clap = { version = "4.0", features = ["derive"] }
```

---

### `/crates/cdm/Cargo.toml`

**Purpose:** Main crate configuration

**Dependencies:**

- `tree-sitter` - Parser
- `serde`, `serde_json` - Serialization
- `wasmtime` - WASM runtime
- `clap` - CLI framework
- Local: `grammar`, `cdm-plugin-api`

---

### `/crates/grammar/Cargo.toml`

**Purpose:** Grammar crate configuration

**Dependencies:**

- `tree-sitter` - Parser generation

**Build:**

- Custom build script compiles grammar.js

---

### `/crates/cdm-plugin-api/Cargo.toml`

**Purpose:** Plugin API crate configuration

**Dependencies:**

- `serde`, `serde_json` - Serialization
- `convert_case` - String case conversion

**Features:**

- `#![no_std]` compatible for WASM

---

### `/crates/cdm-plugin-docs/Cargo.toml`

**Purpose:** Docs plugin configuration

**Dependencies:**

- `cdm-plugin-api` - Plugin types
- `serde`, `serde_json` - Serialization

**Build:**

- Target: `wasm32-wasip1`

---

## Summary of Key Findings

### Strongest Areas:

1. **Validation** (`validate.rs`) - 4,189 lines of tests, comprehensive coverage
2. **Grammar** (`grammar.js`) - Complete language support
3. **Plugin API** (`cdm-plugin-api`) - Well-designed, complete
4. **Documentation** (`specs/spec.md`) - Extremely detailed
5. **Type System** - Fully implemented with all features

### Areas Needing Work:

1. **CLI** (`main.rs`) - Only validate command exists
2. **Plugin Integration** - Runner exists but not integrated
3. **File Resolution** - @extends path resolution not implemented
4. **Build System** - No build/migrate commands
5. **Schema Builder** - AST → Schema conversion missing

### Code Quality:

- **Excellent:** Validation, grammar, plugin API
- **Good:** Symbol table, plugin runner
- **Needs work:** CLI integration, file I/O

### Test Coverage:

- **Validation:** Exceptional (4,189 lines)
- **CLI:** Minimal (5 basic tests)
- **Plugin System:** Example plugin only
- **Integration:** None

### Documentation Quality:

- **Specification:** World-class
- **Plugin Docs:** Excellent
- **API Docs:** Good (could use more rustdoc)
- **Examples:** Good coverage

---

## File Organization Assessment

**Strengths:**

- Clear separation of concerns (grammar, validation, plugins)
- Good use of workspace for modularity
- Examples directory with real use cases
- Comprehensive specs directory

**Suggestions:**

- Add integration tests directory
- Add benchmarks for validation performance
- Consider splitting validate.rs (it's quite large)
- Add CLI tests directory

**Overall Assessment:** Well-organized, production-quality foundation with excellent testing for core features. Main gap is CLI integration layer.
