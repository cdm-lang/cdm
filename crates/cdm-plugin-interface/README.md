# CDM Plugin System Documentation

## Overview

CDM plugins extend the language with code generators and migration tools. They run as WebAssembly modules in a sandboxed environment, receiving your schema as input and producing output files like SQL migrations, TypeScript types, or validation code.

## Importing Plugins

Plugins are imported at the top of CDM files using `@name` syntax. All imports must appear before any type definitions.

```cdm
// Registry plugin (built-in)
@sql {
    dialect: "postgres",
    schema: "public",
    build_output: "./db/schema",
    migrations_output: "./db/migrations"
}

// External plugin from git
@analytics from git:https://github.com/myorg/cdm-analytics.git {
    version: "1.0.0"
}

// Local plugin (for development)
@custom from "./plugins/my-plugin" {
    debug: true
}
```

### Reserved Configuration Keys

CDM extracts these keys before passing config to plugins:

| Key                 | Purpose                                  |
| ------------------- | ---------------------------------------- |
| `version`           | Version constraint for plugin resolution |
| `build_output`   | Output directory for generated files     |
| `migrations_output` | Output directory for migration files     |

## Plugin Sources

### Registry Plugins

Official and curated plugins resolved by name:

```cdm
@sql
@typescript
@validation
```

### Git Plugins

Any accessible git repository:

```cdm
@plugin from git:https://github.com/user/repo.git { version: "1.0.0" }
@plugin from git:git@github.com:org/private-repo.git { version: "main" }
```

Version can be a tag (`1.0.0`, `v2.0.0`), branch (`main`), or commit SHA.

### Local Plugins

Filesystem paths for development:

```cdm
@custom from "./plugins/my-plugin"
@shared from "../shared-plugins/common"
```

## Configuration Levels

Plugins receive configuration at three levels:

### Global Configuration

Applied at the plugin import:

```cdm
@sql {
    dialect: "postgres",
    naming_convention: "snake_case"
}
```

### Model Configuration

Applied to specific models or type aliases:

```cdm
User {
    id: UUID
    email: string

    @sql {
        table: "users",
        indexes: [{ fields: ["email"], unique: true }]
    }
}
```

### Field Configuration

Applied to specific fields:

```cdm
User {
    bio: string {
        @sql { type: "TEXT" }
        @validation { max_length: 5000 }
    }
}
```

## Creating a Plugin

### Quick Start

```bash
cdm plugin new my-plugin
cd cdm-plugin-my-plugin
```

### Plugin Structure

```
cdm-plugin-my-plugin/
├── cdm-plugin.json       # Manifest (required)
├── schema.cdm            # Settings schema (required)
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── validate.rs
│   ├── generate.rs
│   └── migrate.rs
└── README.md
```

### Manifest (`cdm-plugin.json`)

```json
{
  "name": "my-plugin",
  "version": "1.0.0",
  "description": "My custom CDM plugin",
  "schema": "schema.cdm",
  "wasm": {
    "file": "target/wasm32-wasip1/release/cdm_plugin_my_plugin.wasm"
  },
  "capabilities": ["build", "migrate"]
}
```

### Settings Schema (`schema.cdm`)

Define what configuration your plugin accepts:

```cdm
GlobalSettings {
    output_format: "json" | "yaml" = "json"
    include_comments: boolean = true
}

ModelSettings {
    custom_name?: string
    skip: boolean = false
}

FieldSettings {
    override_type?: string
    exclude: boolean = false
}
```

## Plugin Functions

### validate_config (Required)

Validates user configuration. Called for every config block.

```rust
use cdm_plugin_interface::*;

pub fn validate_config(
    level: ConfigLevel,
    config: JSON,
    utils: &Utils,
) -> Vec<ValidationError> {
    let mut errors = vec![];

    match level {
        ConfigLevel::Global => {
            if let Some(format) = config.get("output_format") {
                if !["json", "yaml"].contains(&format.as_str().unwrap_or("")) {
                    errors.push(ValidationError {
                        path: vec![PathSegment {
                            kind: "global".into(),
                            name: "output_format".into()
                        }],
                        message: "must be 'json' or 'yaml'".into(),
                        severity: Severity::Error,
                    });
                }
            }
        }
        ConfigLevel::Model { name } => { /* validate model config */ }
        ConfigLevel::Field { model, field } => { /* validate field config */ }
    }

    errors
}
```

### build (Optional)

Transforms the schema into output files.

```rust
pub fn build(
    schema: Schema,
    config: JSON,
    utils: &Utils,
) -> Vec<OutputFile> {
    let mut output = String::new();

    for model in &schema.models {
        let name = utils.change_case(&model.name, CaseFormat::Snake);
        output.push_str(&format!("// Model: {}\n", name));
        // Build code...
    }

    vec![OutputFile {
        path: "output.ts".into(),
        content: output,
    }]
}
```

### migrate (Optional)

Generates migration files from schema changes.

```rust
pub fn migrate(
    schema: Schema,
    deltas: Vec<Delta>,
    config: JSON,
    utils: &Utils,
) -> Vec<OutputFile> {
    let mut up = String::new();
    let mut down = String::new();

    for delta in &deltas {
        match delta {
            Delta::ModelAdded { name, after } => {
                up.push_str(&format!("CREATE TABLE {}...\n", name));
                down.push_str(&format!("DROP TABLE {};\n", name));
            }
            Delta::FieldAdded { model, field, after } => {
                up.push_str(&format!("ALTER TABLE {} ADD COLUMN {}...\n", model, field));
                down.push_str(&format!("ALTER TABLE {} DROP COLUMN {};\n", model, field));
            }
            // Handle other deltas...
            _ => {}
        }
    }

    vec![
        OutputFile { path: "up.sql".into(), content: up },
        OutputFile { path: "down.sql".into(), content: down },
    ]
}
```

## Delta Types

When schemas change, plugins receive deltas describing what changed:

| Delta                     | Description                                        |
| ------------------------- | -------------------------------------------------- |
| `ModelAdded`              | New model added (includes full `after` definition) |
| `ModelRemoved`            | Model deleted (includes full `before` definition)  |
| `ModelRenamed`            | Model renamed (includes both definitions)          |
| `FieldAdded`              | New field on a model                               |
| `FieldRemoved`            | Field deleted from a model                         |
| `FieldTypeChanged`        | Field type modified                                |
| `FieldOptionalityChanged` | Field changed to/from optional                     |
| `FieldDefaultChanged`     | Default value modified                             |
| `TypeAliasAdded`          | New type alias                                     |
| `TypeAliasRemoved`        | Type alias deleted                                 |
| `InheritanceAdded`        | Model now extends a parent                         |
| `InheritanceRemoved`      | Model no longer extends a parent                   |
| `GlobalConfigChanged`     | Plugin config at import level changed              |
| `ModelConfigChanged`      | Plugin config on a model changed                   |
| `FieldConfigChanged`      | Plugin config on a field changed                   |

## Utilities

### change_case

Convert strings between naming conventions:

```rust
utils.change_case("UserProfile", CaseFormat::Snake)    // "user_profile"
utils.change_case("user_profile", CaseFormat::Pascal)  // "UserProfile"
utils.change_case("userProfile", CaseFormat::Kebab)    // "user-profile"
utils.change_case("user_profile", CaseFormat::Constant) // "USER_PROFILE"
```

## Building Your Plugin

```bash
# Install WASM target
rustup target add wasm32-wasip1

# Build
cargo build --release --target wasm32-wasip1
```

## Testing Locally

Reference your plugin from a CDM file:

```cdm
@my-plugin from ./path/to/cdm-plugin-my-plugin {
    output_format: "yaml"
}

User {
    name: string
    email: string
}
```

Then run:

```bash
cdm validate    # Check configs
cdm build       # Run build for all plugins
cdm migrate     # Run migrations
```

## CLI Commands

```bash
cdm plugin new <name>           # Create new plugin
cdm plugin list                 # List available plugins
cdm plugin info <name>          # Show plugin details
cdm plugin cache <name>         # Pre-download a plugin
cdm plugin clear-cache          # Clear plugin cache

cdm validate                    # Validate CDM files and configs
cdm build                       # Run build for all plugins
cdm migrate                     # Generate migrations from changes
```

## Sandbox Environment

Plugins run in a WebAssembly sandbox with no access to the host filesystem or network. They receive schema data as JSON input and return output files that CDM writes to configured directories.

Limits apply to memory usage, execution time, and output size to prevent runaway plugins.
