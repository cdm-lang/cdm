# CDM Plugin: Docs

A CDM plugin that generates comprehensive documentation from your schema definitions in multiple output formats (Markdown, HTML, or JSON).

## Overview

The `docs` plugin transforms your CDM schema into human-readable documentation. It extracts type aliases, models, fields, and their relationships to create structured documentation that can be used for API references, developer guides, or internal specifications.

**Key Features:**

- **Multiple output formats**: Generate Markdown, HTML, or JSON documentation
- **Hierarchical documentation**: Automatically creates table of contents with anchor links
- **Inheritance support**: Shows parent-child relationships between models
- **Example embedding**: Include usage examples at type, model, and field levels
- **Visibility control**: Hide internal types and models with the `hidden` flag
- **Deprecation marking**: Mark fields as deprecated with visual indicators
- **Customizable**: Configure global settings and override at any level

## What is CDM?

CDM (Contextual Data Model) is a schema definition language that serves as a single source of truth for data models across your entire technology stack. From CDM definitions, you can generate SQL schemas, TypeScript types, API documentation, and more through a plugin system.

**Plugins** are WebAssembly modules that transform CDM schemas into output files. They run in a sandboxed environment and receive your schema as structured data.

## Installation

Add the plugin to your CDM file with an import statement and global configuration:

```cdm
@docs {
    format: "markdown"
    include_examples: true
    include_inheritance: true
    title: "My API Documentation"
    build_output: "./docs"
}
```

**Note**: The `build_output` key is required and specifies where generated documentation files will be written.

## How It Works

The docs plugin processes your CDM schema in the following steps:

1. **Schema Loading**: CDM parses your `.cdm` files and resolves all type aliases, models, and inheritance relationships
2. **Configuration Validation**: The plugin validates your `@docs` configuration blocks against the settings schema
3. **Documentation Generation**: Based on the configured format, the plugin:
   - Iterates through all type aliases and models
   - Extracts descriptions, examples, and metadata from `@docs` config blocks
   - Formats the information according to the output format (Markdown/HTML/JSON)
   - Respects visibility settings (hides types/models marked as `hidden`)
   - Includes inheritance information if `include_inheritance` is enabled
   - Embeds examples if `include_examples` is enabled
4. **File Output**: Writes the generated documentation to the `build_output` directory

### Output Files

- **Markdown format**: Generates `schema.md`
- **HTML format**: Generates `schema.html` with embedded styles
- **JSON format**: Generates `schema.json` with the raw schema structure

## Configuration Reference

The docs plugin supports configuration at four levels: global (plugin import), type alias, model, and field.

### Global Settings

Applied at the plugin import level and affects the entire documentation output.

```cdm
@docs {
    format: "markdown"
    include_examples: true
    include_inheritance: true
    title: "My Project Documentation"
    build_output: "./docs"
}
```

| Setting | Type | Default | Required | Description |
|---------|------|---------|----------|-------------|
| `format` | `"markdown" \| "html" \| "json"` | `"markdown"` | No | Output format for generated documentation. Markdown is ideal for version control and GitHub, HTML for static sites, JSON for custom processing. |
| `include_examples` | `boolean` | `undefined` | No | When `true`, includes example code blocks from `@docs` configurations in the output. Examples are wrapped in code fences for Markdown/HTML. |
| `include_inheritance` | `boolean` | `undefined` | No | When `true`, shows which models extend which parents in an "Extends" section for each model. Helps visualize the inheritance hierarchy. |
| `title` | `string` | `"Schema Documentation"` | No | The main heading/title for the generated documentation. Appears as an H1 in Markdown/HTML output. |
| `build_output` | `string` | - | Yes | Directory path where generated documentation files will be written (relative to CDM file location). Required by CDM for all plugins with build capability. |

### Type Alias Settings

Applied to individual type alias definitions to document custom types.

```cdm
EmailAddress: string {
    @docs {
        description: "A valid email address in RFC 5322 format"
        example: "user@example.com"
        hidden: false
    }
}
```

| Setting | Type | Default | Required | Description |
|---------|------|---------|----------|-------------|
| `description` | `string` | `undefined` | No | Human-readable explanation of what this type represents and when to use it. Appears directly under the type name in documentation. |
| `example` | `string` | `undefined` | No | Example value for this type. Only appears if `include_examples` is enabled globally. Should be a string representation of a valid value. |
| `hidden` | `boolean` | `false` | No | When `true`, completely excludes this type alias from the generated documentation. Useful for internal implementation types that shouldn't be exposed in public documentation. |

### Model Settings

Applied to model definitions to document data structures.

```cdm
User {
    id: string
    email: string
    name: string

    @docs {
        description: "Represents a user account in the system"
        example: "{\"id\": \"123\", \"email\": \"alice@example.com\", \"name\": \"Alice\"}"
        hidden: false
    }
}
```

| Setting | Type | Default | Required | Description |
|---------|------|---------|----------|-------------|
| `description` | `string` | `undefined` | No | High-level overview of what this model represents, its purpose, and how it's used in the system. Appears at the top of the model's documentation section. |
| `example` | `string` | `undefined` | No | Complete JSON example showing this model with realistic data. Only appears if `include_examples` is enabled. Should be valid JSON as a string (escape quotes). |
| `hidden` | `boolean` | `false` | No | When `true`, excludes this model entirely from documentation output. Useful for internal base models or implementation details. Models marked as `hidden` won't appear in the table of contents or model sections. |

### Field Settings

Applied to individual fields within models to document their purpose and usage.

```cdm
Post {
    title: string {
        @docs {
            description: "The post title (max 200 characters)"
            example: "My First Blog Post"
            deprecated: false
        }
    }

    legacyField?: string {
        @docs {
            description: "Old field, will be removed in v2.0"
            deprecated: true
        }
    }
}
```

| Setting | Type | Default | Required | Description |
|---------|------|---------|----------|-------------|
| `description` | `string` | `undefined` | No | Explanation of the field's purpose, constraints, and usage notes. Appears in the field table's "Description" column. |
| `example` | `string` | `undefined` | No | Example value for this specific field. Only shown if `include_examples` is enabled globally. Useful for showing format expectations (dates, UUIDs, etc.). |
| `deprecated` | `boolean` | `false` | No | When `true`, marks this field as deprecated in the documentation. In Markdown/HTML output, the field name appears with strikethrough formatting (~~fieldName~~) to indicate it should not be used in new code. |

## Complete Example

Here's a comprehensive example showing all configuration levels:

```cdm
// Global configuration
@docs {
    format: "markdown"
    include_examples: true
    include_inheritance: true
    title: "Blog API Documentation"
    build_output: "./docs"
}

// Type alias with documentation
EmailAddress: string {
    @docs {
        description: "A valid email address in RFC 5322 format. Used for user authentication and notifications."
        example: "user@example.com"
    }
}

UserStatus: "active" | "inactive" | "suspended" {
    @docs {
        description: "Current state of a user account. Active users can log in and create content."
    }
}

// Base model (hidden from public docs)
BaseEntity {
    id: string
    createdAt: string
    updatedAt: string

    @docs {
        description: "Internal base model with common timestamp fields"
        hidden: true
    }
}

// User model extending base
User extends BaseEntity {
    email: EmailAddress {
        @docs {
            description: "Primary email address for login and notifications"
            example: "alice@example.com"
        }
    }

    username: string {
        @docs {
            description: "Unique username for the account (3-20 characters, alphanumeric)"
            example: "alice_smith"
        }
    }

    displayName?: string {
        @docs {
            description: "Optional display name shown in the UI"
            example: "Alice Smith"
        }
    }

    status: UserStatus = "active" {
        @docs {
            description: "Current account status"
        }
    }

    bio?: string {
        @docs {
            description: "Optional user biography (max 500 characters)"
            example: "Software developer and blogger"
        }
    }

    @docs {
        description: "Represents a user account in the blogging system. Users can create posts and comments."
        example: "{\"id\": \"550e8400-e29b-41d4-a716-446655440000\", \"email\": \"alice@example.com\", \"username\": \"alice_smith\", \"status\": \"active\"}"
    }
}

// Post model with field-level docs
Post extends BaseEntity {
    authorId: string {
        @docs {
            description: "Reference to the User who created this post"
        }
    }

    title: string {
        @docs {
            description: "Post title, displayed in listings and at the top of the post (max 200 chars)"
            example: "Getting Started with CDM"
        }
    }

    content: string {
        @docs {
            description: "Post content in Markdown format"
            example: "# Hello World\n\nThis is my first post!"
        }
    }

    published: boolean = false {
        @docs {
            description: "Whether the post is publicly visible. Unpublished posts are only visible to the author."
        }
    }

    tags: string[] {
        @docs {
            description: "List of tags for categorizing and searching posts"
            example: "[\"typescript\", \"web-development\", \"tutorial\"]"
        }
    }

    viewCount: number {
        @docs {
            description: "Number of times this post has been viewed"
            deprecated: true
        }
    }

    @docs {
        description: "Represents a blog post or article. Posts belong to a single author and can have multiple comments."
    }
}
```

### Generated Markdown Output

The above schema would generate documentation like this:

```markdown
# Blog API Documentation

## Table of Contents

### Type Aliases

- [EmailAddress](#type-emailaddress)
- [UserStatus](#type-userstatus)

### Models

- [User](#model-user)
- [Post](#model-post)

## Type Aliases

### Type: EmailAddress

A valid email address in RFC 5322 format. Used for user authentication and notifications.

**Type:** `string`

**Example:**

```
user@example.com
```

---

### Type: UserStatus

Current state of a user account. Active users can log in and create content.

**Type:** `"active" | "inactive" | "suspended"`

---

## Models

### Model: User

Represents a user account in the blogging system. Users can create posts and comments.

**Extends:** BaseEntity

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| email | `EmailAddress` | Yes | Primary email address for login and notifications |
| username | `string` | Yes | Unique username for the account (3-20 characters, alphanumeric) |
| displayName | `string` | No | Optional display name shown in the UI |
| status | `UserStatus` | Yes | Current account status |
| bio | `string` | No | Optional user biography (max 500 characters) |

**Example:**

```json
{"id": "550e8400-e29b-41d4-a716-446655440000", "email": "alice@example.com", "username": "alice_smith", "status": "active"}
```

---

### Model: Post

Represents a blog post or article. Posts belong to a single author and can have multiple comments.

**Extends:** BaseEntity

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| authorId | `string` | Yes | Reference to the User who created this post |
| title | `string` | Yes | Post title, displayed in listings and at the top of the post (max 200 chars) |
| content | `string` | Yes | Post content in Markdown format |
| published | `boolean` | Yes | Whether the post is publicly visible. Unpublished posts are only visible to the author. |
| tags | `string[]` | Yes | List of tags for categorizing and searching posts |
| ~~viewCount~~ | `number` | Yes | Number of times this post has been viewed |

---
```

Note how `BaseEntity` is excluded (hidden), inheritance is shown, examples are included, and `viewCount` appears with strikethrough formatting due to deprecation.

## Building Documentation

To generate documentation from your CDM schema:

```bash
# Validate your schema
cdm validate schema.cdm

# Build documentation (and any other configured plugins)
cdm build schema.cdm
```

The generated documentation will be written to the directory specified in `build_output`.

## Output Formats

### Markdown

Best for version control, GitHub repositories, and documentation sites that use Markdown processors. Creates a single `schema.md` file with:

- Hierarchical structure with H1/H2/H3 headings
- Markdown tables for fields
- Code fences for examples
- Anchor links in table of contents

### HTML

Best for standalone documentation or static site hosting. Creates a single `schema.html` file with:

- Embedded CSS styles for basic formatting
- Same structure as Markdown but rendered as HTML
- Self-contained (no external dependencies)

### JSON

Best for programmatic processing or building custom documentation tools. Creates `schema.json` with:

- Complete schema structure as JSON
- All type aliases and models with their configurations
- Can be consumed by custom documentation generators or API tools

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

### Local Development

Reference the local plugin during development:

```cdm
@docs from ./path/to/cdm-plugin-docs {
    format: "markdown"
    build_output: "./docs"
}
```

Then run:

```bash
cdm validate    # Check configuration
cdm build       # Generate documentation
```

## Plugin Architecture

This plugin implements two of the three optional CDM plugin functions:

1. **`schema()`** (required): Returns the configuration schema defined in [schema.cdm](schema.cdm)
2. **`validate_config()`** (optional): Validates user-provided `@docs` configurations beyond basic schema checks
3. **`build()`** (optional): Generates documentation files from the schema

The plugin does NOT implement `migrate()` since documentation generation doesn't involve schema migrations.

### Files and Structure

```
cdm-plugin-docs/
├── cdm-plugin.json          # Plugin manifest
├── schema.cdm               # Configuration schema (defines valid settings)
├── Makefile                 # Development tasks
├── setup.sh                 # Dependency setup script
├── README.md                # This file (user documentation)
├── TEMPLATE_README.md       # Plugin development guide
├── src/
│   ├── lib.rs              # Plugin entry point, WASM exports
│   ├── validate.rs         # Configuration validation logic
│   └── build.rs            # Documentation generation logic
├── tests/
│   └── integration_test.rs # Integration tests
└── example/
    └── schema.cdm          # Example usage demonstrating all features
```

## Using as a Template

This plugin serves as a complete template for creating your own CDM plugins. See [TEMPLATE_README.md](TEMPLATE_README.md) for detailed instructions on:

- Plugin structure and architecture
- Implementing validation and code generation
- Testing strategies
- Publishing your plugin

## License

MPL-2.0
