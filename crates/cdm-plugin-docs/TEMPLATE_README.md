# CDM Plugin Template

This repository serves as a complete template for creating CDM plugins. Use this as a starting point for developing your own custom plugins.

## What is a CDM Plugin?

CDM plugins extend the CDM language with custom code generation, validation, and migration capabilities. Plugins run as WebAssembly modules in a sandboxed environment, receiving your schema as input and producing output files.

## Quick Start

### 1. Prerequisites

- **Rust** (latest stable) - [Install from rustup.rs](https://rustup.rs/)
- **WASM target** - Run `make install-deps` or `rustup target add wasm32-wasip1`

### 2. Clone and Customize

```bash
# Clone this template (or use it as a reference)
git clone <your-repo-url> cdm-plugin-myname
cd cdm-plugin-myname

# Run setup to verify dependencies
make setup
```

### 3. Customize Your Plugin

Update these files with your plugin's information:

1. **`cdm-plugin.json`** - Plugin metadata (name, version, description)
2. **`schema.cdm`** - Define what configuration your plugin accepts
3. **`Cargo.toml`** - Update package name and description
4. **`src/validate.rs`** - Implement your configuration validation logic
5. **`src/generate.rs`** - Implement your code generation logic

### 4. Build and Test

```bash
# Build the plugin
make build

# Run tests
make test

# Run all verification
make verify
```

## Project Structure

```
cdm-plugin-docs/
├── cdm-plugin.json          # Plugin manifest (required)
├── schema.cdm               # Settings schema definition (required)
├── Cargo.toml              # Rust package configuration
├── Makefile                # Development tasks
├── README.md               # User-facing documentation
├── TEMPLATE_README.md      # This file (for developers)
│
├── src/
│   ├── lib.rs              # Plugin entry point & WASM exports
│   ├── validate.rs         # Configuration validation logic
│   └── generate.rs         # Code generation logic
│
├── tests/
│   └── integration_test.rs # Integration tests
│
└── example/
    └── schema.cdm          # Example CDM file using your plugin
```

## Development Workflow

### Common Tasks (Makefile)

```bash
make help           # Show all available commands
make setup          # Check and install dependencies
make build          # Build plugin (release mode)
make build-debug    # Build plugin (debug mode, faster)
make test           # Run all tests
make test-unit      # Run unit tests only
make test-wasm      # Build and verify WASM loads
make run-example    # Run example schema with plugin
make clean          # Clean build artifacts
make verify         # Build + test (full verification)
make size           # Show WASM file size
```

### Watch Mode (Auto-rebuild)

```bash
# Install cargo-watch
cargo install cargo-watch

# Watch for changes and rebuild
make watch
```

## Plugin Implementation Guide

### 1. Define Your Configuration Schema

Edit `schema.cdm` to define what configuration your plugin accepts at different levels:

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

### 2. Implement Configuration Validation

Edit `src/validate.rs` to validate user configuration:

```rust
pub fn validate_config(
    level: ConfigLevel,
    config: JSON,
    utils: &Utils,
) -> Vec<ValidationError> {
    let mut errors = vec![];

    match level {
        ConfigLevel::Global => {
            // Validate global config
            if let Some(format) = config.get("output_format") {
                // Add validation logic
            }
        }
        ConfigLevel::Model { name } => {
            // Validate model-level config
        }
        ConfigLevel::Field { model, field } => {
            // Validate field-level config
        }
    }

    errors
}
```

### 3. Implement Code Generation

Edit `src/generate.rs` to generate your output files:

```rust
pub fn generate(
    schema: Schema,
    config: JSON,
    utils: &Utils,
) -> Vec<OutputFile> {
    let mut output = String::new();

    for (name, model) in &schema.models {
        // Generate code for each model
        output.push_str(&format!("Model: {}\n", name));

        for field in &model.fields {
            // Generate code for each field
        }
    }

    vec![OutputFile {
        path: "output.txt".into(),
        content: output,
    }]
}
```

### 4. Optional: Implement Migrations

If your plugin supports schema migrations, create `src/migrate.rs`:

```rust
pub fn migrate(
    schema: Schema,
    deltas: Vec<Delta>,
    config: JSON,
    utils: &Utils,
) -> Vec<OutputFile> {
    // Generate migration files based on schema changes
    vec![]
}
```

Then update `src/lib.rs` to export the migrate function:

```rust
mod migrate;
pub use migrate::migrate;

cdm_plugin_api::export_migrate!(migrate);
```

And update `cdm-plugin.json` to include the "migrate" capability:

```json
{
  "capabilities": ["generate", "migrate"]
}
```

## Testing Your Plugin

### Unit Tests

Add tests directly in your source files:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation() {
        // Test your validation logic
    }
}
```

### Integration Tests

Create tests in `tests/integration_test.rs` that verify the full plugin behavior:

```rust
#[test]
fn test_generate_output() {
    let schema = create_test_schema();
    let config = json!({ "output_format": "json" });
    let utils = Utils {};

    let outputs = generate(schema, config, &utils);

    assert_eq!(outputs.len(), 1);
    assert!(outputs[0].content.contains("expected content"));
}
```

Run tests with:

```bash
make test           # All tests
make test-unit      # Unit tests only
cargo test -- --nocapture  # Show println! output
```

## Using Your Plugin

### Local Development

Create a CDM file that imports your plugin from a local path:

```cdm
@myplugin from ./path/to/cdm-plugin-myplugin {
    output_format: "json"
}

User {
    id: string
    name: string
}
```

### Example File

The `example/schema.cdm` file demonstrates your plugin with realistic usage. Update it to show:

- Global configuration options
- Model-level configuration
- Field-level configuration
- Different features of your plugin

Run the example with:

```bash
make run-example
```

## Plugin API Reference

### Available Types

Import from `cdm_plugin_interface`:

```rust
use cdm_plugin_interface::{
    ConfigLevel,      // Global | Model | Field
    ValidationError,  // Error with path, message, severity
    OutputFile,       // Generated file (path + content)
    Schema,          // Models and type aliases
    Model,           // Model definition with fields
    Field,           // Field definition with type
    TypeExpression,  // Type representation
    Delta,           // Schema change (for migrations)
    Utils,           // Utility functions
    CaseFormat,      // snake_case, PascalCase, etc.
    JSON,            // serde_json::Value
    Severity,        // Error | Warning | Info
};
```

### Utility Functions

The `utils` parameter provides helper functions:

```rust
// Convert between naming conventions
utils.change_case("UserProfile", CaseFormat::Snake)    // "user_profile"
utils.change_case("user_profile", CaseFormat::Pascal)  // "UserProfile"
utils.change_case("userProfile", CaseFormat::Kebab)    // "user-profile"
utils.change_case("user_profile", CaseFormat::Constant) // "USER_PROFILE"
```

Available case formats: `Snake`, `Pascal`, `Camel`, `Kebab`, `Constant`, `Title`

### Configuration Levels

Your validation function receives config at three levels:

```rust
match level {
    ConfigLevel::Global => {
        // Config from plugin import:
        // @myplugin { option: "value" }
    }
    ConfigLevel::Model { name } => {
        // Config on a model:
        // User { @myplugin { option: "value" } }
    }
    ConfigLevel::Field { model, field } => {
        // Config on a field:
        // email: string { @myplugin { option: "value" } }
    }
}
```

### Schema Structure

The schema passed to generate() contains:

```rust
schema.models        // HashMap<String, Model>
schema.type_aliases  // HashMap<String, TypeAlias>

// Each Model has:
model.name          // Model name
model.fields        // Vec<Field>
model.parents       // Vec<String> (inheritance)
model.config        // JSON (your plugin's config)

// Each Field has:
field.name          // Field name
field.field_type    // TypeExpression
field.optional      // bool
field.default       // Option<DefaultValue>
field.config        // JSON (your plugin's config)
```

### Type Expressions

Fields have types represented as `TypeExpression`:

```rust
match &field.field_type {
    TypeExpression::Identifier { name } => {
        // Simple type like "string", "UUID", "User"
    }
    TypeExpression::Array { element_type } => {
        // Array type like "string[]"
    }
    TypeExpression::Union { types } => {
        // Union type like "active" | "inactive"
    }
    TypeExpression::StringLiteral { value } => {
        // String literal like "active"
    }
}
```

### Validation Errors

Return validation errors with full context:

```rust
ValidationError {
    path: vec![
        PathSegment {
            kind: "model".to_string(),
            name: "User".to_string(),
        },
        PathSegment {
            kind: "field".to_string(),
            name: "email".to_string(),
        },
    ],
    message: "Invalid email format".to_string(),
    severity: Severity::Error,  // Error | Warning | Info
}
```

## Publishing Your Plugin

### 1. Update Metadata

Ensure these files are ready:

- `cdm-plugin.json` - Correct name, version, description
- `Cargo.toml` - Correct package metadata
- `README.md` - User-facing documentation
- `LICENSE` - Choose a license (MIT, Apache-2.0, etc.)

### 2. Build Release

```bash
make build
make verify
```

### 3. Version Your Release

```bash
git tag v1.0.0
git push origin v1.0.0
```

### 4. Publish WASM Binary

Upload the WASM file to your releases:

```bash
# The file to publish:
target/wasm32-wasip1/release/cdm_plugin_[name].wasm
```

Update `cdm-plugin.json` with the release URL:

```json
{
  "wasm": {
    "file": "target/wasm32-wasip1/release/cdm_plugin_myname.wasm",
    "release_url": "https://github.com/user/repo/releases/download/v{version}/plugin.wasm"
  }
}
```

## Troubleshooting

### WASM Target Not Found

```bash
make install-deps
# or manually:
rustup target add wasm32-wasip1
```

### Build Errors

```bash
# Clean and rebuild
make clean
make build

# Check Rust version
rustc --version  # Should be 1.70+
```

### Tests Failing

```bash
# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name -- --nocapture
```

### Large WASM File

The release profile is already optimized for size:

```toml
[profile.release]
opt-level = "z"      # Optimize for size
lto = true           # Link-time optimization
codegen-units = 1    # Better optimization
```

Check size with:

```bash
make size
```

Further optimization:
- Use `wasm-opt` from binaryen
- Avoid heavy dependencies
- Use feature flags to exclude unused code

## Best Practices

### Configuration Design

- Use sensible defaults for optional fields
- Provide clear validation messages
- Use union types for enums in schema.cdm
- Document all configuration options

### Code Generation

- Generate idiomatic code for target language
- Respect existing code style conventions
- Include helpful comments in generated code
- Handle edge cases (empty models, optional fields)

### Error Messages

- Be specific about what's wrong
- Suggest how to fix the issue
- Include context (model/field names)
- Use appropriate severity levels

### Testing

- Test all configuration levels
- Test invalid configurations
- Test edge cases (empty schema, all optional fields)
- Test different output formats
- Include integration tests

### Documentation

- Provide clear examples in README.md
- Document all configuration options
- Include a working example/schema.cdm
- Explain when to use your plugin

## Examples of Plugin Ideas

- **SQL Generator** - Generate CREATE TABLE statements
- **TypeScript Types** - Generate TypeScript interfaces
- **API Client** - Generate REST API client code
- **GraphQL Schema** - Generate GraphQL SDL
- **Validation** - Generate validation schemas (Zod, Yup, etc.)
- **ORM Models** - Generate Prisma, TypeORM, Sequelize models
- **Documentation** - Generate API documentation (like this plugin!)
- **Testing** - Generate test fixtures and factories
- **Protobuf** - Generate .proto files
- **OpenAPI** - Generate OpenAPI/Swagger specs

## Additional Resources

- [CDM Plugin API Documentation](../cdm-plugin-api/README.md)
- [CDM Language Documentation](../../README.md)
- [WebAssembly Documentation](https://webassembly.org/)
- [Rust WASM Book](https://rustwasm.github.io/docs/book/)

## Getting Help

- Check existing plugin examples in the CDM repository
- Read the plugin API documentation
- File issues on the CDM repository
- Join the CDM community discussions

## License

This template is provided as-is for creating CDM plugins. Choose your own license for your plugin.
