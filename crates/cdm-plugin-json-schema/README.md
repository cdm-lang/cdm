# cdm-plugin-json-schema

A CDM plugin for [describe what this plugin does].

## Features

- TODO: List features

## Configuration

### Global Settings

```cdm
@json-schema {
  // Add configuration options here
}
```

### Model Settings

```cdm
MyModel {
  // fields...

  @json-schema {
    // Model-specific configuration
  }
}
```

### Field Settings

```cdm
MyModel {
  my_field: string {
    @json-schema {
      // Field-specific configuration
    }
  }
}
```

## Quick Start

### Initial Setup

Run the setup script to check dependencies and get started:

```bash
chmod +x setup.sh
./setup.sh
```

This will:
- Check if Rust and required tools are installed
- Install the WASM target if needed
- Optionally run a test build

### Building

#### Using Make (Recommended)

```bash
# Build the plugin (release mode, optimized)
make build

# Build in debug mode (faster compilation)
make build-debug

# See all available commands
make help
```

#### Manual Build

```bash
rustup target add wasm32-wasip1
cargo build --release --target wasm32-wasip1
```

The compiled WASM file will be at `target/wasm32-wasip1/release/cdm_plugin_json_schema.wasm`

## Usage

Use this plugin in your CDM schema:

```cdm
@json-schema from ./path/to/cdm-plugin-json-schema {
  build_output: "./generated"
  // Add configuration here
}

// Your schema definitions...
```

## Development

### Available Make Commands

Run `make help` to see all available commands:

**Setup & Dependencies:**
- `make check-deps` - Check if required dependencies are installed
- `make install-deps` - Install required dependencies (Rust WASM target)
- `make setup` - Full setup (check + install dependencies)

**Building:**
- `make build` - Build plugin for WASM (release mode, optimized)
- `make build-debug` - Build plugin for WASM (debug mode, faster)
- `make size` - Show WASM file size

**Testing:**
- `make test` - Run all tests
- `make test-unit` - Run unit tests only (faster, no WASM build)
- `make test-wasm` - Build WASM and verify it loads

**Development:**
- `make run-example` - Run example CDM file with this plugin
- `make watch` - Watch for changes and rebuild (requires cargo-watch)
- `make clean` - Clean build artifacts
- `make verify` - Run all checks (build + test)
- `make dev` - Full development setup (setup + build + test)

### Running Tests

```bash
# Run all tests
make test

# Or using cargo directly
cargo test
```

### Watch Mode

Auto-rebuild on file changes (requires `cargo-watch`):

```bash
# Install cargo-watch (first time only)
cargo install cargo-watch

# Start watching
make watch
```

### Local Testing

Create an example CDM file to test your plugin:

1. Create `example/schema.cdm`:

```cdm
@json-schema from ./ {
  build_output: "./generated"
  // Add configuration here
}

// Your test schema...
User {
  id: string
  name: string
}
```

2. Run the example:

```bash
make run-example
```

Or use the plugin from any CDM file with a relative path:

```cdm
@json-schema from ./path/to/cdm-plugin-json-schema {
  build_output: "./generated"
}
```

Then run:

```bash
cdm build schema.cdm
```

## License

TODO: Add license information
