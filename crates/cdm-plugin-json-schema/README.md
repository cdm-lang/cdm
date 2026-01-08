# CDM JSON Schema Plugin

Generate JSON Schema definitions from your CDM models and type aliases for validation, documentation, and API contracts.

## Overview

The JSON Schema plugin transforms CDM models into standards-compliant JSON Schema files. It supports multiple JSON Schema draft versions and provides flexible output modes for single-file bundled schemas or separate schema files per model.

## Installation

The JSON Schema plugin is available in the CDM registry:

```cdm
@jsonschema {
  build_output: "./schemas"
}
```

## Quick Start

```cdm
@jsonschema {
  draft: "draft7",
  output_mode: "single-file",
  build_output: "./schemas"
}

Email: string {
  @jsonschema {
    description: "Valid email address",
    format: "email",
    max_length: 320
  }
} #1

Status: "active" | "pending" | "suspended" {
  @jsonschema {
    description: "User account status"
  }
} #2

User {
  id: string #1
  email: Email #2
  name: string #3
  status: Status = "pending" #4
  age?: number #5

  @jsonschema {
    title: "User",
    description: "A user account in the system",
    additional_properties: false
  }
} #10
```

**Generated JSON Schema (draft7):**

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "title": "User",
  "description": "A user account in the system",
  "properties": {
    "id": {
      "type": "string"
    },
    "email": {
      "$ref": "#/$defs/Email"
    },
    "name": {
      "type": "string"
    },
    "status": {
      "$ref": "#/$defs/Status"
    },
    "age": {
      "type": "number"
    }
  },
  "required": ["id", "email", "name", "status"],
  "additionalProperties": false,
  "$defs": {
    "Email": {
      "type": "string",
      "description": "Valid email address",
      "format": "email",
      "maxLength": 320
    },
    "Status": {
      "type": "string",
      "description": "User account status",
      "enum": ["active", "pending", "suspended"]
    }
  }
}
```

## Global Settings

Configure the plugin at import time to set defaults for all generated schemas.

### `draft`

- **Type:** `"draft4" | "draft6" | "draft7" | "draft2019-09" | "draft2020-12"`
- **Default:** `"draft7"`
- **Description:** JSON Schema specification version to target.

```cdm
@jsonschema { draft: "draft7" }
@jsonschema { draft: "draft2020-12" }
```

Each draft version uses the corresponding `$schema` URL:
- `draft4`: `http://json-schema.org/draft-04/schema#`
- `draft6`: `http://json-schema.org/draft-06/schema#`
- `draft7`: `http://json-schema.org/draft-07/schema#`
- `draft2019-09`: `https://json-schema.org/draft/2019-09/schema`
- `draft2020-12`: `https://json-schema.org/draft/2020-12/schema`

### `include_schema_property`

- **Type:** `boolean`
- **Default:** `true`
- **Description:** Include the `$schema` property in generated schemas.

```cdm
@jsonschema { include_schema_property: true }
```

Set to `false` to omit the `$schema` property from output.

### `include_examples`

- **Type:** `boolean`
- **Default:** `false`
- **Description:** Include example values in generated schemas.

```cdm
@jsonschema { include_examples: true }

User {
  email: string {
    @jsonschema {
      examples: ["user@example.com", "admin@example.com"]
    }
  } #1
} #10
```

### `output_mode`

- **Type:** `"single-file" | "multi-file"`
- **Default:** `"single-file"`
- **Description:** Generate one bundled schema file or separate files per model.

```cdm
@jsonschema { output_mode: "single-file" }  // schema.json
@jsonschema { output_mode: "multi-file" }   // User.schema.json, Post.schema.json, etc.
```

**single-file mode:** All models are bundled into `schema.json` with one root model and others in `$defs`.

**multi-file mode:** Each model gets its own `{ModelName}.schema.json` file.

### `schema_id`

- **Type:** `string` (optional)
- **Description:** Root schema ID for referencing (adds `$id` property).

```cdm
@jsonschema {
  schema_id: "https://example.com/schemas/user.json"
}
```

Generates:
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://example.com/schemas/user.json",
  ...
}
```

### `include_descriptions`

- **Type:** `boolean`
- **Default:** `true`
- **Description:** Include description fields from CDM configuration in schemas.

```cdm
@jsonschema { include_descriptions: true }

User {
  name: string {
    @jsonschema { description: "Full name of the user" }
  } #1
} #10
```

Set to `false` to omit all descriptions.

### `relationship_mode`

- **Type:** `"reference" | "inline"`
- **Default:** `"reference"`
- **Description:** How to handle model references (for future use).

```cdm
@jsonschema { relationship_mode: "reference" }
```

**reference mode:** Model references use `$ref` to point to definitions.

**inline mode:** Reserved for future inline expansion of referenced models.

### `union_mode`

- **Type:** `"enum" | "oneOf"`
- **Default:** `"enum"`
- **Description:** How to represent string literal unions in JSON Schema.

```cdm
@jsonschema { union_mode: "enum" }

Status: "active" | "pending" | "suspended" #1
```

**enum mode (default):**
```json
{
  "type": "string",
  "enum": ["active", "pending", "suspended"]
}
```

**oneOf mode:**
```json
{
  "oneOf": [
    { "const": "active" },
    { "const": "pending" },
    { "const": "suspended" }
  ]
}
```

### `root_model`

- **Type:** `string` (optional)
- **Description:** In single-file mode, specify which model is the root schema (others go in `$defs`).

```cdm
@jsonschema {
  output_mode: "single-file",
  root_model: "User"
}

User { ... } #10
Post { ... } #11
```

Without `root_model`, the first non-skipped model (or model with `is_root: true`) becomes the root.

## Type Alias Settings

Configure JSON Schema generation for specific type aliases.

### `description`

- **Type:** `string` (optional)
- **Description:** Documentation for this type alias.

```cdm
Email: string {
  @jsonschema {
    description: "RFC 5322 compliant email address"
  }
} #1
```

### `union_mode`

- **Type:** `"enum" | "oneOf"` (optional)
- **Description:** Override global union mode for this specific type alias.

```cdm
@jsonschema { union_mode: "enum" }  // Global default

Status: "active" | "pending" {
  @jsonschema {
    union_mode: "oneOf"  // Override for this type
  }
} #1
```

### `skip`

- **Type:** `boolean`
- **Default:** `false`
- **Description:** Exclude this type alias from generated schemas.

```cdm
InternalCode: "A" | "B" | "C" {
  @jsonschema { skip: true }
} #1
```

This type will not appear in `$defs`.

## Model Settings

Configure JSON Schema generation for specific models.

### `title`

- **Type:** `string` (optional)
- **Description:** Human-readable title for the schema.

```cdm
User {
  id: string #1
  @jsonschema {
    title: "User Account"
  }
} #10
```

Generates:
```json
{
  "type": "object",
  "title": "User Account",
  ...
}
```

### `description`

- **Type:** `string` (optional)
- **Description:** Documentation for this model.

```cdm
User {
  id: string #1
  @jsonschema {
    title: "User",
    description: "Represents a registered user in the system"
  }
} #10
```

### `additional_properties`

- **Type:** `boolean` (optional)
- **Default:** `false` (strict mode)
- **Description:** Whether to allow properties not defined in the schema.

```cdm
User {
  name: string #1
  @jsonschema {
    additional_properties: true  // Allow extra properties
  }
} #10
```

**Default behavior (false):**
```json
{
  "additionalProperties": false
}
```

**With true:**
```json
{
  "additionalProperties": true
}
```

### `skip`

- **Type:** `boolean`
- **Default:** `false`
- **Description:** Exclude this model from generated schemas.

```cdm
InternalCache {
  key: string #1
  value: JSON #2
  @jsonschema { skip: true }
} #10
```

### `relationship_mode`

- **Type:** `"reference" | "inline"` (optional)
- **Description:** Override global relationship mode for this model's references (for future use).

```cdm
User {
  posts: Post[] #1
  @jsonschema {
    relationship_mode: "reference"
  }
} #10
```

### `is_root`

- **Type:** `boolean`
- **Default:** `false`
- **Description:** In single-file mode, mark this model as the root schema instead of placing it in `$defs`.

```cdm
@jsonschema {
  output_mode: "single-file"
}

User {
  id: string #1
  @jsonschema { is_root: true }  // This becomes the root
} #10

Post {
  id: string #1  // This goes in $defs
} #11
```

## Field Settings

Configure JSON Schema generation for specific fields.

### `description`

- **Type:** `string` (optional)
- **Description:** Documentation for this field.

```cdm
User {
  email: string {
    @jsonschema {
      description: "Primary email address for account notifications"
    }
  } #1
} #10
```

### String Constraints

#### `pattern`

- **Type:** `string` (optional)
- **Description:** Regex pattern for string validation.

```cdm
User {
  username: string {
    @jsonschema {
      pattern: "^[a-zA-Z0-9_]{3,20}$",
      description: "Alphanumeric username, 3-20 characters"
    }
  } #1
} #10
```

Generates:
```json
{
  "type": "string",
  "pattern": "^[a-zA-Z0-9_]{3,20}$",
  "description": "Alphanumeric username, 3-20 characters"
}
```

#### `min_length` and `max_length`

- **Type:** `number` (optional)
- **Description:** String length constraints.

```cdm
User {
  bio: string {
    @jsonschema {
      min_length: 10,
      max_length: 500
    }
  } #1
} #10
```

Generates:
```json
{
  "type": "string",
  "minLength": 10,
  "maxLength": 500
}
```

#### `format`

- **Type:** `string` (optional)
- **Description:** Semantic format hint (email, uri, uuid, date-time, etc.).

```cdm
User {
  email: string {
    @jsonschema { format: "email" }
  } #1

  website?: string {
    @jsonschema { format: "uri" }
  } #2

  created_at: string {
    @jsonschema { format: "date-time" }
  } #3

  id: string {
    @jsonschema { format: "uuid" }
  } #4
} #10
```

**Common formats:**
- `email` - Email address
- `uri`, `uri-reference` - URIs
- `uuid` - UUID
- `date`, `time`, `date-time` - Timestamps
- `ipv4`, `ipv6` - IP addresses
- `hostname` - Hostnames
- `json-pointer` - JSON Pointer

### Number Constraints

#### `minimum` and `maximum`

- **Type:** `number` (optional)
- **Description:** Inclusive numeric range constraints.

```cdm
Product {
  price: number {
    @jsonschema {
      minimum: 0,
      maximum: 999999.99
    }
  } #1

  rating: number {
    @jsonschema {
      minimum: 0,
      maximum: 5
    }
  } #2
} #10
```

#### `exclusive_minimum` and `exclusive_maximum`

- **Type:** `number` (optional)
- **Description:** Exclusive numeric range constraints.

```cdm
Product {
  discount_percent: number {
    @jsonschema {
      exclusive_minimum: 0,   // Greater than 0
      exclusive_maximum: 100  // Less than 100
    }
  } #1
} #10
```

### `custom_type`

- **Type:** `string` (optional)
- **Description:** Override the JSON Schema type for this field.

```cdm
User {
  metadata: JSON {
    @jsonschema {
      custom_type: "object"  // Instead of no type restriction
    }
  } #1
} #10
```

### `examples`

- **Type:** `JSON[]` (optional)
- **Description:** Example values for this field (only included if `include_examples: true`).

```cdm
@jsonschema { include_examples: true }

User {
  email: string {
    @jsonschema {
      examples: ["user@example.com", "admin@company.org"]
    }
  } #1

  age: number {
    @jsonschema {
      examples: [25, 30, 42]
    }
  } #2
} #10
```

### `relationship_mode`

- **Type:** `"reference" | "inline"` (optional)
- **Description:** Override relationship handling for this specific field (for future use).

```cdm
Post {
  author: User {
    @jsonschema {
      relationship_mode: "reference"
    }
  } #1
} #10
```

### `skip`

- **Type:** `boolean`
- **Default:** `false`
- **Description:** Exclude this field from the generated schema.

```cdm
User {
  password_hash: string {
    @jsonschema { skip: true }  // Don't include in schema
  } #1
} #10
```

## Type Mapping

### CDM to JSON Schema Type Mapping

| CDM Type            | JSON Schema Output                          |
| ------------------- | ------------------------------------------- |
| `string`            | `{ "type": "string" }`                      |
| `number`            | `{ "type": "number" }`                      |
| `boolean`           | `{ "type": "boolean" }`                     |
| `JSON`              | `{}` (no type restriction)                  |
| `string[]`          | `{ "type": "array", "items": { "type": "string" } }` |
| `Model`             | `{ "$ref": "#/$defs/Model" }`               |
| `"a" \| "b" \| "c"` | `{ "type": "string", "enum": ["a", "b", "c"] }` (enum mode) |
| `"a" \| "b" \| "c"` | `{ "oneOf": [{ "const": "a" }, { "const": "b" }, { "const": "c" }] }` (oneOf mode) |

### Optional Fields

CDM optional fields (marked with `?`) are excluded from the `required` array:

```cdm
User {
  name: string #1      // Required
  nickname?: string #2 // Optional
} #10
```

Generates:
```json
{
  "properties": {
    "name": { "type": "string" },
    "nickname": { "type": "string" }
  },
  "required": ["name"]
}
```

### Union Types

String literal unions are represented based on the `union_mode` setting:

```cdm
@jsonschema { union_mode: "enum" }

Status: "active" | "pending" | "suspended" #1
```

**enum mode (default):**
```json
{
  "type": "string",
  "enum": ["active", "pending", "suspended"]
}
```

**oneOf mode:**
```json
{
  "oneOf": [
    { "const": "active" },
    { "const": "pending" },
    { "const": "suspended" }
  ]
}
```

## Output Modes

### Single-File Mode

All models bundled into one `schema.json` with a root schema and `$defs`:

```cdm
@jsonschema {
  output_mode: "single-file",
  root_model: "User"
}

User {
  id: string #1
  posts: Post[] #2
} #10

Post {
  id: string #1
  author: User #2
} #11
```

**Output: `schema.json`**
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "id": { "type": "string" },
    "posts": {
      "type": "array",
      "items": { "$ref": "#/$defs/Post" }
    }
  },
  "required": ["id", "posts"],
  "additionalProperties": false,
  "$defs": {
    "Post": {
      "type": "object",
      "properties": {
        "id": { "type": "string" },
        "author": { "$ref": "#/$defs/User" }
      },
      "required": ["id", "author"],
      "additionalProperties": false
    }
  }
}
```

### Multi-File Mode

Each model gets its own schema file:

```cdm
@jsonschema {
  output_mode: "multi-file"
}

User { ... } #10
Post { ... } #11
```

**Output:**
- `User.schema.json`
- `Post.schema.json`

## Examples

### Complete API Schema

```cdm
@jsonschema {
  draft: "draft7",
  output_mode: "single-file",
  include_descriptions: true,
  include_examples: false,
  schema_id: "https://api.example.com/schemas/v1.json",
  build_output: "./schemas"
}

Email: string {
  @jsonschema {
    description: "RFC 5322 email address",
    format: "email",
    max_length: 320
  }
} #1

UUID: string {
  @jsonschema {
    description: "RFC 4122 UUID",
    format: "uuid",
    pattern: "^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"
  }
} #2

Status: "active" | "pending" | "suspended" | "deleted" {
  @jsonschema {
    description: "User account status"
  }
} #3

User {
  id: UUID #1
  email: Email #2
  name: string #3
  age?: number #4
  status: Status = "pending" #5
  created_at: string #6

  @jsonschema {
    title: "User",
    description: "User account representation",
    additional_properties: false,
    is_root: true
  }
} #10

Post {
  id: UUID #1
  author_id: UUID #2
  title: string #3
  content: string #4
  published: boolean = false #5
  view_count: number #6

  @jsonschema {
    title: "Post",
    description: "Blog post or article"
  }
} #11
```

### Validation-Heavy Schema

```cdm
@jsonschema {
  draft: "draft2020-12",
  build_output: "./schemas"
}

User {
  username: string {
    @jsonschema {
      description: "Unique username",
      pattern: "^[a-zA-Z0-9_]{3,20}$",
      min_length: 3,
      max_length: 20
    }
  } #1

  email: string {
    @jsonschema {
      description: "Primary email",
      format: "email",
      max_length: 320
    }
  } #2

  age: number {
    @jsonschema {
      description: "User age in years",
      minimum: 13,
      maximum: 120
    }
  } #3

  website?: string {
    @jsonschema {
      description: "Personal website URL",
      format: "uri"
    }
  } #4

  bio?: string {
    @jsonschema {
      description: "Short biography",
      max_length: 500
    }
  } #5

  @jsonschema {
    title: "User Registration",
    description: "User account with validation constraints"
  }
} #10
```

### Multi-File Output

```cdm
@jsonschema {
  output_mode: "multi-file",
  build_output: "./schemas"
}

User {
  id: string #1
  name: string #2
  @jsonschema {
    title: "User",
    description: "User model"
  }
} #10

Post {
  id: string #1
  title: string #2
  @jsonschema {
    title: "Post",
    description: "Post model"
  }
} #11

Comment {
  id: string #1
  text: string #2
  @jsonschema {
    title: "Comment",
    description: "Comment model"
  }
} #12
```

**Output:**
- `schemas/User.schema.json`
- `schemas/Post.schema.json`
- `schemas/Comment.schema.json`

## CLI Commands

```bash
# Validate JSON Schema configuration
cdm validate schema.cdm

# Generate schema files
cdm build schema.cdm

# Use with specific output directory
cdm build schema.cdm --output ./custom-schemas
```

## Output Files

### Single-File Mode

- **File:** `schema.json`
- **Location:** Specified by `build_output` setting

### Multi-File Mode

- **Files:** `{ModelName}.schema.json` for each model
- **Location:** Specified by `build_output` setting

## Best Practices

1. **Use type aliases** for common validated types (Email, UUID, URL) to ensure consistency
2. **Set `additional_properties: false`** for strict validation in APIs
3. **Add descriptions** to improve documentation and developer experience
4. **Use `format` hints** for better validation and tooling support
5. **Leverage `skip: true`** for internal-only models or fields
6. **Choose appropriate draft version** based on your validator's support
7. **Use `min_length`/`max_length`** to prevent abuse and ensure data quality
8. **Apply `pattern` constraints** for format validation (usernames, codes, etc.)
9. **Document with `title`** and `description` for API documentation generation
10. **Use `examples`** (with `include_examples: true`) for API documentation tools

---

## Development

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

### Testing

```bash
# Run all tests
make test

# Run unit tests only (faster, no WASM build)
make test-unit

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

Use this plugin from any CDM file with a relative path:

```cdm
@jsonschema from "./path/to/cdm-plugin-json-schema" {
  build_output: "./schemas"
}

User {
  id: string #1
  name: string #2
} #10
```

Then run:

```bash
cdm build schema.cdm
```

## License

See the main CDM repository for license information.
