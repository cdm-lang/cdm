# CDM Rust Plugin

Generate Rust types from CDM schemas.

## Overview

The Rust plugin converts CDM schema definitions into Rust code. It generates structs, enums, and type aliases with configurable derive macros, serde support, visibility, and naming conventions.

## How It Works

The plugin processes your CDM schema and generates Rust output files based on your configuration:

1. **Schema Parsing** - Reads CDM models and type aliases from your schema
2. **Type Mapping** - Converts CDM types to their Rust equivalents
3. **Code Generation** - Generates Rust structs, enums, and type aliases
4. **File Organization** - Outputs to a single file or separate files per model

### Type Mapping

CDM types are mapped to Rust as follows:

| CDM Type | Rust Type |
|----------|-----------|
| `string` | `String` |
| `number` | `f64` (configurable) |
| `boolean` | `bool` |
| `JSON` | `serde_json::Value` |
| `T[]` | `Vec<T>` |
| `V[K]` (Map) | `HashMap<K, V>` (or `BTreeMap`) |
| Optional `?` | `Option<T>` |
| String literal unions | Rust enum with `#[serde(rename)]` |
| Type reference unions | Rust enum with `#[serde(untagged)]` |
| User-defined types | Pass through as-is |

## Configuration

### Global Settings

Configure the plugin at the global level in your CDM schema:

```cdm
@rust {
  build_output: "./src/generated",
  serde_support: true,
  number_type: "i64"
}
```

#### `build_output`

**Type:** `string`
**Default:** `"."`

Specifies the directory where the generated Rust files will be saved. This setting is automatically available for all CDM plugins.

#### `file_strategy`

**Type:** `"single" | "per_model"`
**Default:** `"single"`

Controls how output files are organized:

- **`"single"`** - All types in one file (specified by `single_file_name`)
- **`"per_model"`** - Each model in a separate file, shared type aliases in `types.rs`, with a `mod.rs` for re-exports

#### `single_file_name`

**Type:** `string`
**Default:** `"types.rs"`

The filename to use when `file_strategy` is `"single"`.

#### `derive_macros`

**Type:** `string` (comma-separated)
**Default:** `"Debug, Clone, Serialize, Deserialize"`

Comma-separated list of derive macros to add to generated types.

```cdm
@rust {
  derive_macros: "Debug, Clone, PartialEq, Serialize, Deserialize"
}
```

#### `serde_support`

**Type:** `boolean`
**Default:** `true`

When `true`, generates `use serde::{Serialize, Deserialize};` and adds `#[serde(rename = "...")]` attributes when field names differ from their CDM source names.

#### `number_type`

**Type:** `"f64" | "f32" | "i32" | "i64" | "u32" | "u64"`
**Default:** `"f64"`

The Rust type to use for CDM `number` fields.

#### `map_type`

**Type:** `"HashMap" | "BTreeMap"`
**Default:** `"HashMap"`

The Rust map type to use for CDM map fields.

#### `visibility`

**Type:** `"pub" | "pub_crate" | "private"`
**Default:** `"pub"`

Default visibility for generated types and fields.

#### `type_name_format`

**Type:** `"preserve" | "pascal" | "camel" | "snake" | "kebab" | "constant"`
**Default:** `"preserve"`

Formatting to apply to type and struct names.

#### `field_name_format`

**Type:** `"preserve" | "pascal" | "camel" | "snake" | "kebab" | "constant"`
**Default:** `"snake"`

Formatting to apply to field names. Defaults to `"snake"` for idiomatic Rust.

### Type Alias Settings

Configure individual type aliases:

```cdm
Email: string {
  @rust {
    export_name: "EmailAddress"
  }
}
```

#### `type_override`

**Type:** `string`
**Default:** None

Override the generated Rust type completely.

#### `export_name`

**Type:** `string`
**Default:** None

Customize the name for this type alias.

#### `skip`

**Type:** `boolean`
**Default:** `false`

When `true`, this type alias will not be generated in the output.

### Model Settings

Configure individual models:

```cdm
User {
  id: string
  name: string

  @rust {
    struct_name: "UserModel",
    visibility: "pub_crate"
  }
}
```

#### `struct_name`

**Type:** `string`
**Default:** None

Override the generated struct name.

#### `derive_macros`

**Type:** `string` (comma-separated)
**Default:** Inherits from global setting

Override derive macros for this specific model.

#### `skip`

**Type:** `boolean`
**Default:** `false`

When `true`, this model will not be generated in the output.

#### `visibility`

**Type:** `"pub" | "pub_crate" | "private"`
**Default:** Inherits from global setting

Override visibility for this specific model and its fields.

#### `file_name`

**Type:** `string`
**Default:** None

When using `file_strategy: "per_model"`, specify a custom filename for this model.

### Field Settings

Configure individual fields within models:

```cdm
User {
  id: string {
    @rust { visibility: "private" }
  }

  created_at: string {
    @rust { type_override: "chrono::DateTime<chrono::Utc>" }
  }

  firstName: string {
    @rust { serde_rename: "first_name" }
  }
}
```

#### `type_override`

**Type:** `string`
**Default:** None

Override the Rust type for this specific field.

#### `field_name`

**Type:** `string`
**Default:** None

Rename this field in the generated output (overrides `field_name_format`).

#### `skip`

**Type:** `boolean`
**Default:** `false`

When `true`, this field will not be included in the generated output.

#### `serde_rename`

**Type:** `string`
**Default:** None

Explicitly set the `#[serde(rename = "...")]` value for this field.

#### `visibility`

**Type:** `"pub" | "pub_crate" | "private"`
**Default:** Inherits from model setting

Override visibility for this specific field.

## Examples

### Basic Usage

```cdm
@rust {
  build_output: "./src/generated"
}

Email: string
Status: "active" | "inactive" | "pending"

User {
  id: string
  name: string
  email: Email
  status: Status
  age?: number
}
```

Generates `types.rs`:
```rust
use serde::{Serialize, Deserialize};

pub type Email = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Status {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "inactive")]
    Inactive,
    #[serde(rename = "pending")]
    Pending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: Email,
    pub status: Status,
    pub age: Option<f64>,
}
```

### Custom Number Type

```cdm
@rust {
  build_output: "./src/generated",
  number_type: "i64"
}

User {
  id: string
  age: number
  score?: number
}
```

Generates:
```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub age: i64,
    pub score: Option<i64>,
}
```

### Per-Model Files

```cdm
@rust {
  build_output: "./src/generated",
  file_strategy: "per_model"
}

Email: string

User {
  id: string
  email: Email
}

Post {
  id: string
  title: string
  author: User
}
```

Generates:
- `types.rs` - Contains the `Email` type alias
- `user.rs` - Contains the `User` struct
- `post.rs` - Contains the `Post` struct
- `mod.rs` - Module declarations and re-exports

`mod.rs`:
```rust
use serde::{Serialize, Deserialize};

mod types;
mod post;
mod user;

pub use types::*;
pub use post::*;
pub use user::*;
```

`user.rs`:
```rust
use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: Email,
}
```

### Union Types

String literal unions become Rust enums:

```cdm
Status: "active" | "inactive"
```

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Status {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "inactive")]
    Inactive,
}
```

Type reference unions become enums with newtype variants:

```cdm
Content: TextBlock | ImageBlock
```

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Content {
    TextBlock(TextBlock),
    ImageBlock(ImageBlock),
}
```

Inline unions on fields generate named enums:

```cdm
User {
  role: "admin" | "member"
}
```

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserRole {
    #[serde(rename = "admin")]
    Admin,
    #[serde(rename = "member")]
    Member,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub role: UserRole,
}
```

### Field-Level Customization

```cdm
User {
  id: string {
    @rust { visibility: "private" }
  }

  created_at: string {
    @rust { type_override: "chrono::DateTime<chrono::Utc>" }
  }

  internal_field: string {
    @rust { skip: true }
  }
}
```

Generates:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
```

### Without Serde

```cdm
@rust {
  build_output: "./src/types",
  serde_support: false,
  derive_macros: "Debug, Clone, PartialEq"
}

User {
  id: string
  name: string
}
```

Generates:
```rust
#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: String,
    pub name: String,
}
```

## Validation

The plugin validates configuration at build time:

- **Rust identifiers** - Ensures field and type names are valid Rust identifiers
- **Reserved keywords** - Warns if names conflict with Rust reserved keywords
- **Configuration values** - Validates enum values match allowed options
- **File extensions** - Ensures `single_file_name` has a `.rs` extension

## License

MPL-2.0
