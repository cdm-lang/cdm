# CDM Plugin: Docs

Generate documentation from CDM schemas in multiple formats (Markdown, HTML, JSON).

## Installation

```cdm
@docs {
    format: "markdown",
    include_examples: true,
    include_inheritance: true,
    title: "My API Documentation"
}
```

## Configuration

### Global Settings

- `format`: Output format - `"markdown"` (default), `"html"`, or `"json"`
- `include_examples`: Include example code blocks (boolean, optional)
- `include_inheritance`: Show model inheritance relationships (boolean, optional)
- `title`: Documentation title (string, optional)

### Model Settings

- `description`: Model description text (string, optional)
- `example`: Example JSON for the model (string, optional)
- `hidden`: Hide this model from documentation (boolean, optional)

### Field Settings

- `description`: Field description text (string, optional)
- `example`: Example value for the field (string, optional)
- `deprecated`: Mark field as deprecated (boolean, optional)

## Example Usage

```cdm
@docs {
    format: "markdown",
    include_examples: true,
    title: "User API Schema"
}

User {
    @docs { description: "Represents a user in the system" }

    id: string {
        @docs { description: "Unique user identifier" }
    }

    email: string {
        @docs {
            description: "User's email address",
            example: "user@example.com"
        }
    }

    name: string {
        @docs { description: "User's full name" }
    }

    createdAt: string {
        @docs { description: "Account creation timestamp" }
    }
}
```

## Development

### Quick Start

```bash
# Setup (install dependencies and verify environment)
./setup.sh

# Or manually
make setup
```

### Building

```bash
# Build for production (optimized WASM)
make build

# Build for development (faster compilation)
make build-debug

# View all available commands
make help
```

The compiled WASM file will be at:
`target/wasm32-wasip1/release/cdm_plugin_docs.wasm`

### Testing

```bash
# Run all tests
make test

# Run unit tests only
make test-unit

# Build and verify WASM
make test-wasm
```

### Using as a Template

This plugin serves as a complete template for creating your own CDM plugins. See [TEMPLATE_README.md](TEMPLATE_README.md) for detailed instructions on:

- Plugin structure and architecture
- Implementing validation and generation
- Testing strategies
- Publishing your plugin

## Files and Structure

```
cdm-plugin-docs/
├── cdm-plugin.json          # Plugin manifest
├── schema.cdm               # Configuration schema
├── Makefile                 # Development tasks
├── setup.sh                 # Dependency setup script
├── README.md                # This file (user documentation)
├── TEMPLATE_README.md       # Plugin development guide
├── src/
│   ├── lib.rs              # Plugin entry point
│   ├── validate.rs         # Configuration validation
│   └── generate.rs         # Documentation generation
├── tests/
│   └── integration_test.rs # Integration tests
└── example/
    └── schema.cdm          # Example usage
```

## License

MIT
