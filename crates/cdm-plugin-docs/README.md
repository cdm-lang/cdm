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

## Building

```bash
# Install WASM target
rustup target add wasm32-wasip1

# Build the plugin
cargo build --release --target wasm32-wasip1
```

The compiled WASM file will be at:
`target/wasm32-wasip1/release/cdm_plugin_docs.wasm`

## License

MIT
