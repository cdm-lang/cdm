# CDM Plugin Templates

This directory contains templates for generating new CDM plugins using the `cdm plugin new` command.

## Structure

Each subdirectory represents a supported programming language:

- `rust/` - Rust/WASM plugin templates

## Template Variables

Templates use a simple variable substitution system with `{{VARIABLE_NAME}}` placeholders:

### Available Variables

- `{{PLUGIN_NAME}}` - The plugin name as provided by the user (e.g., `my-awesome-plugin`)
- `{{CRATE_NAME}}` - The Rust crate name (plugin name with hyphens converted to underscores, e.g., `my_awesome_plugin`)

## Adding New Templates

To add a new language template:

1. Create a new directory for the language (e.g., `typescript/`, `python/`)
2. Add template files with `.template` extension
3. Use `{{VARIABLE_NAME}}` placeholders where substitution is needed
4. Update `plugin_new.rs` to support the new language

## Template Files (Rust)

The Rust template includes:

- `Cargo.toml.template` - Cargo package manifest
- `cdm-plugin.json.template` - CDM plugin manifest
- `schema.cdm.template` - Plugin settings schema
- `.gitignore.template` - Git ignore file
- `README.md.template` - Plugin documentation
- `src/lib.rs.template` - Main entry point with WASM exports
- `src/build.rs.template` - Build function implementation
- `src/migrate.rs.template` - Migration function with all Delta types
- `src/validate.rs.template` - Configuration validation

## Usage

Users create new plugins with:

```bash
cdm plugin new my-plugin --lang rust
```

The command will:
1. Validate the plugin name
2. Find the appropriate template directory
3. Copy and process each template file
4. Replace all `{{VARIABLE_NAME}}` placeholders
5. Write the processed files to the output directory

## Template Resolution

The `cdm plugin new` command searches for templates in this order:

1. Relative to the CDM executable (for installed versions)
2. Relative to the current working directory (for development)
3. `CDM_TEMPLATE_DIR` environment variable (for custom locations)
4. `CARGO_MANIFEST_DIR` at compile time (for workspace-relative paths)

## Editing Templates

When editing templates:

1. Make changes directly to the `.template` files in this directory
2. Test by running `cdm plugin new test-plugin --lang rust`
3. Verify the generated plugin compiles and tests pass
4. No rebuild of CDM is needed - templates are read at runtime

## Example

Input template (`Cargo.toml.template`):
```toml
[package]
name = "cdm-plugin-{{PLUGIN_NAME}}"
version = "1.0.0"
```

Command:
```bash
cdm plugin new my-plugin --lang rust
```

Output (`Cargo.toml`):
```toml
[package]
name = "cdm-plugin-my-plugin"
version = "1.0.0"
```
