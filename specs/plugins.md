# CDM Plugin System Specification

## Overview

The CDM plugin system allows extending the CDM language with code generators and migration tools. Plugins are distributed as WebAssembly modules that run in a sandboxed environment without access to the host filesystem or network.

---

## Table of Contents

1. [Plugin Import Syntax](#1-plugin-import-syntax)
2. [Plugin Sources](#2-plugin-sources)
3. [Plugin Interface](#3-plugin-interface)
4. [Settings Schema](#4-settings-schema)
5. [Plugin Functions](#5-plugin-functions)
6. [Deltas](#6-deltas)
7. [Utilities](#7-utilities)
8. [Architecture](#8-architecture)
9. [CLI Commands](#9-cli-commands)
10. [Plugin Development](#10-plugin-development)

---

## 1. Plugin Import Syntax

Plugins are imported at the top of CDM files using `@name` syntax. All imports must appear before any type definitions (grammar-enforced).

### Basic Syntax

```cdm
// Registry plugin (no config)
@validation

// Registry plugin with config
@sql {
    dialect: "postgres",
    schema: "public",
    generate_output: "./db/schema",
    migrations_output: "./db/migrations"
}

// Git plugin
@analytics from git:https://github.com/myorg/cdm-analytics.git {
    version: "1.0.0",
    endpoint: "https://analytics.example.com"
}

// Local plugin
@custom from ./plugins/my-plugin {
    debug: true
}
```

### Config Format

Configuration uses JSON/JavaScript object notation with optional quotes on keys. Commas separate entries.

```cdm
@sql {
    dialect: "postgres",
    naming_convention: "snake_case",
    indexes: [
        { fields: ["email"], unique: true }
    ]
}
```

### Reserved Config Keys

CDM extracts these keys before passing config to plugins:

| Key | Purpose |
|-----|---------|
| `version` | Version constraint for plugin resolution |
| `generate_output` | Output directory for `generate` function |
| `migrations_output` | Output directory for `migrate` function |

---

## 2. Plugin Sources

### Registry Plugins

Resolved via a curated index file hosted in the CDM repository.

```cdm
@sql
@typescript
@validation
```

**Registry Format:**

```json
{
    "version": 1,
    "updated_at": "2024-01-15T10:30:00Z",
    "plugins": {
        "sql": {
            "description": "Generate SQL schemas and migrations",
            "repository": "git:https://github.com/cdm-lang/cdm-plugin-sql.git",
            "official": true,
            "versions": {
                "1.0.0": {
                    "wasm_url": "https://github.com/.../releases/download/v1.0.0/plugin.wasm",
                    "checksum": "sha256:a1b2c3d4..."
                }
            },
            "latest": "1.0.0"
        }
    }
}
```

### Git Plugins

Any git repository accessible via HTTPS or SSH.

```cdm
// HTTPS
@plugin from git:https://github.com/user/repo.git { version: "1.0.0" }
@plugin from git:https://gitlab.com/user/repo.git { version: "main" }
@plugin from git:https://bitbucket.org/user/repo.git { version: "v2.0.0" }

// SSH (private repos)
@plugin from git:git@github.com:org/private-repo.git { version: "1.0.0" }
```

**Version Resolution:**

- Git tag: `"1.0.0"` or `"v1.0.0"`
- Branch: `"main"`, `"develop"`
- Commit SHA: `"a1b2c3d4..."`

### Local Plugins

File system paths for development.

```cdm
@custom from ./plugins/my-plugin
@shared from ../shared-plugins/common
```

---

## 3. Plugin Interface

Plugins are WebAssembly modules that export three functions and a settings schema.

### Plugin Repository Structure

```
cdm-plugin-example/
├── cdm-plugin.json       # Manifest (required)
├── schema.cdm            # Settings schema (required)
├── plugin.wasm           # Pre-built WASM (optional if release_url provided)
├── src/                  # Source code
└── README.md
```

### Manifest Format

```json
{
    "name": "example",
    "version": "1.0.0",
    "description": "An example CDM plugin",
    "schema": "schema.cdm",
    "wasm": {
        "file": "target/wasm32-wasip1/release/cdm_plugin_example.wasm",
        "release_url": "https://github.com/.../releases/download/v{version}/plugin.wasm"
    },
    "capabilities": ["generate", "migrate"]
}
```

**Capabilities:**

| Capability | Description |
|------------|-------------|
| `generate` | Plugin implements the `generate` function |
| `migrate` | Plugin implements the `migrate` function |

**Note:** The `validate_config` function is **required** for all plugins and is not listed in capabilities. It is always called to validate user-provided configuration regardless of other capabilities.

---

## 4. Settings Schema

Each plugin defines its allowed configuration using CDM syntax.

```cdm
// schema.cdm for sql plugin

GlobalSettings {
    dialect: "postgres" | "mysql" | "sqlite" = "postgres"
    schema?: string
    naming_convention: "snake_case" | "camelCase" = "snake_case"
}

ModelSettings {
    table?: string
    indexes?: Index[]
    primary_key?: string | string[]
}

FieldSettings {
    type?: string
    column?: string
    default?: string
    index?: boolean
    unique?: boolean
}

// Supporting types
Index {
    fields: string[]
    unique?: boolean
    type?: "btree" | "hash" | "gin" | "gist"
    where?: string
}
```

**Schema Sections:**

| Section | Applied To | Example |
|---------|------------|---------|
| `GlobalSettings` | Plugin import block | `@sql { dialect: "postgres" }` |
| `ModelSettings` | Model/type alias blocks | `User { @sql { table: "users" } }` |
| `FieldSettings` | Field blocks | `email: string { @sql { type: "VARCHAR(320)" } }` |

---

## 5. Plugin Functions

### validate_config

Validates user-provided configuration against the plugin's schema. This function is **required** for all plugins.

```rust
fn validate_config(
    level: ConfigLevel,
    config: JSON,
    utils: Utils,
) -> Vec<ValidationError>
```

**Input:**

```rust
enum ConfigLevel {
    Global,
    Model { name: String },
    Field { model: String, field: String },
}
```

**Output:**

```rust
struct PathSegment {
    kind: String,   // "global", "model", "field", "config", "table", "column", etc.
    name: String,   // "UserProfile", "email", "dialect", etc.
}

enum Severity {
    Error,
    Warning,
}

struct ValidationError {
    path: Vec<PathSegment>,
    message: String,
    severity: Severity,
}
```

**Path Segment Kinds:**

Plugins can define their own domain-specific segment kinds. Common examples:

| Kind | Description | Example |
|------|-------------|---------|
| `global` | Global config key | `{ kind: "global", name: "dialect" }` |
| `model` | Model name | `{ kind: "model", name: "UserProfile" }` |
| `field` | Field name | `{ kind: "field", name: "email" }` |
| `config` | Config property | `{ kind: "config", name: "table" }` |
| `table` | SQL table (plugin-specific) | `{ kind: "table", name: "user_profiles" }` |
| `column` | SQL column (plugin-specific) | `{ kind: "column", name: "created_at" }` |

**Example ValidationErrors:**

```rust
// Warning on a model's table name
ValidationError {
    path: vec![
        PathSegment { kind: "model", name: "UserProfile" },
        PathSegment { kind: "config", name: "table" },
    ],
    message: "table name 'UserProfile' should be in snake_case",
    severity: Severity::Warning,
}

// Error on a field's column config
ValidationError {
    path: vec![
        PathSegment { kind: "model", name: "User" },
        PathSegment { kind: "field", name: "createdAt" },
        PathSegment { kind: "config", name: "column" },
    ],
    message: "column name 'createdAt' should be in snake_case",
    severity: Severity::Warning,
}

// Error on global config
ValidationError {
    path: vec![
        PathSegment { kind: "global", name: "dialect" },
    ],
    message: "unknown dialect 'postgresql', did you mean 'postgres'?",
    severity: Severity::Error,
}
```

### generate

Transforms the CDM schema into output files.

```rust
fn generate(
    schema: Schema,
    config: JSON,
    utils: Utils,
) -> Vec<OutputFile>
```

**Output:**

```rust
struct OutputFile {
    path: String,      // Relative path, e.g., "schema.sql"
    content: String,
}
```

CDM writes files to the configured `generate_output` directory.

### migrate

Generates migration files from schema changes.

```rust
fn migrate(
    schema: Schema,
    deltas: Vec<Delta>,
    config: JSON,
    utils: Utils,
) -> Vec<OutputFile>
```

CDM:
- Computes deltas by diffing previous and current schemas
- Creates a migration directory with a generated name (e.g., `002_add_user_avatar`)
- Writes returned files into that directory

---

## 6. Deltas

Schema changes passed to the `migrate` function. All deltas use `before` and `after` naming to clearly indicate the previous and new state.

```rust
enum Delta {
    // Models
    ModelAdded { name: String, after: ModelDefinition },
    ModelRemoved { name: String, before: ModelDefinition },
    ModelRenamed { old_name: String, new_name: String, before: ModelDefinition, after: ModelDefinition },

    // Fields
    FieldAdded { model: String, field: String, after: FieldDefinition },
    FieldRemoved { model: String, field: String, before: FieldDefinition },
    FieldRenamed { model: String, old_name: String, new_name: String, before: FieldDefinition, after: FieldDefinition },
    FieldTypeChanged { model: String, field: String, before: TypeExpression, after: TypeExpression },
    FieldOptionalityChanged { model: String, field: String, before: bool, after: bool },
    FieldDefaultChanged { model: String, field: String, before: Option<Value>, after: Option<Value> },

    // Type Aliases
    TypeAliasAdded { name: String, after: TypeAliasDefinition },
    TypeAliasRemoved { name: String, before: TypeAliasDefinition },
    TypeAliasTypeChanged { name: String, before: TypeExpression, after: TypeExpression },

    // Inheritance
    InheritanceAdded { model: String, parent: String },
    InheritanceRemoved { model: String, parent: String },

    // Config Changes
    GlobalConfigChanged { before: JSON, after: JSON },
    ModelConfigChanged { model: String, before: JSON, after: JSON },
    FieldConfigChanged { model: String, field: String, before: JSON, after: JSON },
}
```

**Supporting Types:**

```rust
struct ModelDefinition {
    name: String,
    parents: Vec<String>,
    fields: Vec<FieldDefinition>,
    config: JSON,
}

struct FieldDefinition {
    name: String,
    field_type: TypeExpression,
    optional: bool,
    default: Option<Value>,
    config: JSON,
}

struct TypeAliasDefinition {
    name: String,
    alias_type: TypeExpression,
    config: JSON,
}

enum TypeExpression {
    Identifier(String),
    Array(Box<TypeExpression>),
    Union(Vec<TypeExpression>),
    StringLiteral(String),
}
```

**Notes:**

- `*Added` deltas: Include full `after` definition (self-contained for up migration)
- `*Removed` deltas: Include full `before` definition (needed for down migration)
- `*Changed` deltas: Include both `before` and `after` values
- `*Renamed` deltas: Include both `before` and `after` definitions
- Rename detection: Deferred to future version

---

## 7. Utilities

CDM provides utility functions to plugins via a `Utils` object.

### change_case

Converts strings between naming conventions.

```rust
fn change_case(input: &str, format: CaseFormat) -> String

enum CaseFormat {
    Snake,     // user_profile
    Camel,     // userProfile
    Pascal,    // UserProfile
    Kebab,     // user-profile
    Constant,  // USER_PROFILE
    Title,     // User Profile
}
```

**Example:**

```rust
utils.change_case("UserProfile", CaseFormat::Snake)  // "user_profile"
utils.change_case("user_profile", CaseFormat::Pascal) // "UserProfile"
```

---

## 8. Architecture

### Components

```
┌─────────────────────────────────────────────────────────────────┐
│                       Plugin Manager                            │
│  Orchestrates plugin lifecycle                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌────────┐ │
│  │  Resolver   │  │   Cache     │  │   Loader    │  │Runtime │ │
│  │             │  │             │  │             │  │        │ │
│  │ - Registry  │  │ - Download  │  │ - Load WASM │  │ - Call │ │
│  │ - Git       │  │ - Store     │  │ - Validate  │  │   fns  │ │
│  │ - Local     │  │ - Lookup    │  │ - Setup     │  │ - JSON │ │
│  └─────────────┘  └─────────────┘  └─────────────┘  └────────┘ │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Plugin Resolver

Maps plugin references to downloadable artifacts.

```rust
enum PluginSource {
    Registry { name: String, version: Option<String> },
    Git { url: String, version: Option<String> },
    Local { path: PathBuf },
}

trait PluginResolver {
    fn resolve(&self, source: PluginSource) -> Result<ResolvedPlugin>;
}
```

### Plugin Cache

Manages downloaded WASM files.

```
.cdm/
├── cache/
│   └── plugins/
│       ├── sql/
│       │   └── 1.0.0_a1b2c3d4.wasm
│       └── typescript/
│           └── 2.0.0_e5f6g7h8.wasm
├── previous_schema.json
└── registry.json
```

```rust
trait PluginCache {
    fn get(&self, key: &CacheKey) -> Option<PathBuf>;
    fn put(&self, key: &CacheKey, wasm_bytes: &[u8]) -> Result<PathBuf>;
    fn remove(&self, key: &CacheKey) -> Result<()>;
    fn list(&self) -> Vec<CacheKey>;
    fn prune(&self, keep_versions: usize) -> Result<()>;
}
```

### Plugin Loader

Loads and validates WASM modules.

```rust
struct LoadedPlugin {
    manifest: PluginManifest,
    settings_schema: SettingsSchema,
    instance: WasmInstance,
}

trait PluginLoader {
    fn load(&self, wasm_path: &Path) -> Result<LoadedPlugin>;
    fn validate_interface(&self, instance: &WasmInstance) -> Result<()>;
}
```

### Plugin Runtime

Executes plugin functions in sandbox.

```rust
struct PluginRuntime {
    plugins: HashMap<String, LoadedPlugin>,
}

impl PluginRuntime {
    fn validate_config(&self, plugin: &str, level: ConfigLevel, config: JSON) -> Result<Vec<ValidationError>>;
    fn generate(&self, plugin: &str, schema: &Schema, config: JSON) -> Result<Vec<OutputFile>>;
    fn migrate(&self, plugin: &str, schema: &Schema, deltas: &[Delta], config: JSON) -> Result<Vec<OutputFile>>;
}
```

### Sandbox Limits

```rust
struct SandboxLimits {
    max_memory_bytes: u64,      // e.g., 256MB
    max_execution_ms: u64,      // e.g., 30 seconds
    max_output_bytes: u64,      // e.g., 10MB
}
```

### Technology Choices

| Component | Technology | Rationale |
|-----------|------------|-----------|
| WASM Runtime | Wasmtime | Security, WASI support, Rust-first |
| Git Operations | git2 | Pure library, no git CLI dependency |
| Data Format | JSON | Simplicity, can optimize later |

---

## 9. CLI Commands

### Plugin Management

```bash
# Create new plugin
cdm plugin new <name>                    # Rust plugin (default)
cdm plugin new <name> --lang rust        # Explicit language
cdm plugin new <name> --output ./path    # Custom output directory

# Plugin discovery
cdm plugin list                          # List registry plugins
cdm plugin list --cached                 # List cached plugins
cdm plugin info <name>                   # Show plugin details

# Cache management
cdm plugin cache <name>                  # Pre-download plugin
cdm plugin cache --all                   # Cache all plugins in current CDM
cdm plugin clear-cache                   # Clear plugin cache
```

### Build Commands

```bash
# Validate CDM files and plugin configs
cdm validate

# Run generate for all plugins
cdm build

# Run migrate for all plugins (computes deltas, generates migrations)
cdm migrate
```

---

## 10. Plugin Development

### Creating a New Plugin

```bash
cdm plugin new my-plugin
cd cdm-plugin-my-plugin
```

### Generated Structure

```
cdm-plugin-my-plugin/
├── cdm-plugin.json
├── schema.cdm
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── validate.rs
│   ├── generate.rs
│   └── migrate.rs
├── .gitignore
└── README.md
```

### Building

```bash
# Install WASM target
rustup target add wasm32-wasip1

# Build the plugin
cargo build --release --target wasm32-wasip1
```

### Testing Locally

```cdm
// In a CDM project
@my-plugin from ./path/to/cdm-plugin-my-plugin {
    // config options
}
```

### Publishing

1. Create a GitHub repository
2. Build WASM and create a release with the `.wasm` file
3. Submit PR to add plugin to CDM registry (optional)

### cdm-plugin-api Crate

Plugins depend on the `cdm-plugin-api` crate for types and the export macro.

```toml
[dependencies]
cdm-plugin-api = "0.1"
```

```rust
use cdm_plugin_api::{
    export_plugin,
    ConfigLevel, ValidationError, PathSegment, Severity,
    Schema, Delta, OutputFile,
    Utils, CaseFormat,
};

#[export_plugin]
pub fn validate_config(level: ConfigLevel, config: JSON, utils: &Utils) -> Vec<ValidationError> {
    // Required for all plugins
    // Return validation errors with structured paths and severity
}

#[export_plugin]
pub fn generate(schema: Schema, config: JSON, utils: &Utils) -> Vec<OutputFile> {
    // Optional - include "generate" in capabilities
}

#[export_plugin]
pub fn migrate(schema: Schema, deltas: Vec<Delta>, config: JSON, utils: &Utils) -> Vec<OutputFile> {
    // Optional - include "migrate" in capabilities
}
```

---

## Appendix: Grammar Changes

```javascript
// Enforce plugin imports before definitions
source_file: ($) =>
    seq(repeat($.plugin_import), repeat($._definition)),

// Plugin import: @name [from source] [{ config }]
plugin_import: ($) =>
    seq(
        "@",
        field("name", $.identifier),
        optional(seq("from", field("source", $.plugin_source))),
        optional(field("config", $.object_literal))
    ),

// Plugin source: git URL or local path
plugin_source: ($) => choice($.git_reference, $.plugin_path),

// Git reference: git:<url>
git_reference: ($) => /git:[^\s\n{}]+/,

// Local plugin path: ./path or ../path
plugin_path: ($) => /\.\.?\/[^\s\n{}]+/,
```

---

## Future Considerations (Out of Scope for v1)

- Rename detection (heuristic, annotation, or interactive)
- Plugin signing for official plugins
- Hot-reload for local plugin development
- Additional languages for plugin scaffolding (TypeScript, Go)
- Async plugin operations
- Plugin dependencies
