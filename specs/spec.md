# CDM Language Specification

**Version**: 1.1.0-draft  
**Status**: Draft

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Lexical Structure](#2-lexical-structure)
3. [Type System](#3-type-system)
4. [Type Aliases](#4-type-aliases)
5. [Models](#5-models)
6. [Inheritance](#6-inheritance)
7. [Context System](#7-context-system)
8. [Plugin System](#8-plugin-system)
9. [Semantic Validation](#9-semantic-validation)
10. [File Structure and Resolution](#10-file-structure-and-resolution)
11. [CLI Interface](#11-cli-interface)
12. [Plugin Development](#12-plugin-development)
13. [Appendix A: Grammar](#appendix-a-grammar)
14. [Appendix B: Error Catalog](#appendix-b-error-catalog)
15. [Appendix C: Registry Format](#appendix-c-registry-format)
16. [Appendix D: Data Exchange Format](#appendix-d-data-exchange-format)

---

## 1. Introduction

### 1.1 Purpose

CDM (Contextual Data Model) is a schema definition language designed to serve as a single source of truth for data models across an entire technology stack. From a single set of CDM definitions, developers can generate:

- SQL schemas and migrations
- TypeScript/JavaScript types
- Protocol Buffer definitions
- Validation code
- API documentation
- Any other output via the plugin system

### 1.2 Design Goals

1. **Single Source of Truth**: Define your schema once, generate everything else
2. **Context-Aware**: Different views of the same schema for different environments (database, API, client)
3. **Extensible**: Plugin system for custom code generation and validation
4. **Familiar Syntax**: TypeScript-inspired syntax for low learning curve
5. **Type Safe**: Strong static validation before any code generation
6. **Migration-Friendly**: Optional entity IDs enable reliable rename tracking across schema versions

### 1.3 Core Concepts

**Type Alias**: A named type that can reference built-in types, other aliases, or union types.

**Model**: A structured type with named fields, similar to a class or struct.

**Context**: A CDM file that extends another, providing environment-specific modifications to the schema.

**Plugin**: A WebAssembly module that transforms CDM schemas into output files (code, SQL, documentation).

**Entity ID**: An optional stable identifier (`#N`) that enables reliable rename tracking for migrations.

### 1.4 Example

```cdm
// Import plugins
@sql {
  dialect: "postgres",
  schema: "public",
  build_output: "./db/schema",
  migrations_output: "./db/migrations"
}
@typescript {
  build_output: "./types"
}

// Type aliases
Email: string {
  @validation { format: "email", max_length: 320 }
  @sql { type: "VARCHAR(320)" }
} #1

Status: "active" | "pending" | "suspended" #2

// Models
User {
  id: string #1
  email: Email #2
  name: string #3
  status: Status = "pending" #4
  posts: Post[] #5
  created_at: string #6

  @sql { table: "users", indexes: [{ fields: ["email"], unique: true }] }
} #10

Post {
  id: string #1
  author: User #2
  title: string #3
  content: string #4
  published: boolean = false #5

  @sql { table: "posts" }
} #11
```

---

## 2. Lexical Structure

### 2.1 Character Set

CDM source files are encoded in UTF-8.

### 2.2 Whitespace

Horizontal whitespace (spaces, tabs) is ignored except as a token separator. Indentation is not significant.

**Newlines** are significant in the following contexts:

- **Model bodies**: Each field, field removal, or plugin configuration must appear on its own line. Single-line model definitions are not supported (except for empty models).
- **Top-level definitions**: Each type alias, model, or directive should appear on its own line.

Within object literals, array literals, and plugin configuration blocks, newlines are optional since these constructs use commas as separators.

```cdm
// Valid: fields on separate lines
User {
  id: string #1
  name: string #2
  email: string #3
}

// Valid: empty model on one line
EmptyModel {}

// Invalid: fields on same line (will not parse)
User { id: string #1 name: string #2 }
```

### 2.3 Comments

CDM supports single-line comments beginning with `//`:

```cdm
// This is a comment
User {
  name: string  // Inline comment
}
```

Block comments are not supported.

### 2.4 Identifiers

Identifiers must begin with a letter (a-z, A-Z) or underscore, followed by any combination of letters, digits, or underscores:

```
identifier = [a-zA-Z_][a-zA-Z0-9_]*
```

**Valid identifiers**: `User`, `user_name`, `_private`, `Post2`

**Invalid identifiers**: `2fast`, `user-name`, `@special`

**Reserved words**: None. Built-in type names (`string`, `number`, `boolean`, `JSON`) can be shadowed by user definitions, though this is not recommended.

### 2.5 Literals

#### String Literals

Strings are enclosed in double quotes with backslash escape sequences:

```cdm
"hello world"
"line1\nline2"
"quote: \"value\""
```

**Escape sequences**:

- `\"` - double quote
- `\\` - backslash
- `\/` - forward slash
- `\b` - backspace
- `\f` - form feed
- `\n` - newline
- `\r` - carriage return
- `\t` - tab
- `\uXXXX` - Unicode code point (4 hex digits)

#### Number Literals

Numbers can be integers or decimals, optionally negative:

```cdm
42
-17
3.14159
-0.5
```

Scientific notation is not supported.

#### Boolean Literals

```cdm
true
false
```

### 2.6 Punctuation and Operators

| Symbol  | Usage                                       |
| ------- | ------------------------------------------- |
| `{` `}` | Block delimiters (models, config objects)   |
| `[` `]` | Array type suffix, array literals           |
| `(` `)` | Reserved for future use                     |
| `:`     | Type annotation, object key-value separator |
| `=`     | Default value assignment                    |
| `?`     | Optional field marker                       |
| `\|`    | Union type separator                        |
| `,`     | Object/array element separator              |
| `-`     | Field/model removal prefix                  |
| `@`     | Plugin/directive prefix                     |
| `#`     | Entity ID prefix                            |

### 2.7 Entity IDs

Entity IDs are optional stable identifiers that enable reliable rename tracking across schema versions. IDs appear at the end of type alias, model, and field definitions.

#### Syntax

```
#N
```

Where `N` is a positive integer (1 or greater).

#### Examples

```cdm
Email: string #1                    // Type alias with ID
Status: "active" | "pending" #2     // Union type alias with ID

User {
  name: string #1                   // Field with ID
  email: string #2                  // Field with ID
} #10                               // Model with ID
```

#### Purpose

Entity IDs solve the ambiguity between renames and remove+add operations when computing migrations:

```cdm
// Before
User {
  name: string #1
} #10

// After - ID #1 proves this is a rename, not remove+add
User {
  displayName: string #1
} #10
```

Without IDs, the migration system cannot distinguish between:

- Renaming `name` to `displayName` (preserves data)
- Removing `name` and adding `displayName` (loses data)

#### Scope

- **Model and Type Alias IDs**: Global within the project. No two models or type aliases can share the same ID.
- **Field IDs**: Scoped to their containing model. Field `#1` in `User` is distinct from field `#1` in `Post`.

#### Rules

1. **Uniqueness**: No duplicate IDs within scope
2. **Immutability**: Once assigned, IDs should never change
3. **No Reuse**: Deleted entity IDs should not be reused for new entities
4. **Optionality**: IDs are optional; entities without IDs use heuristic matching for migrations

---

## 3. Type System

### 3.1 Built-in Types

CDM provides four built-in primitive types:

| Type      | Description                               | Example Values  |
| --------- | ----------------------------------------- | --------------- |
| `string`  | Unicode text                              | `"hello"`, `""` |
| `number`  | Numeric value (integer or floating-point) | `42`, `-3.14`   |
| `boolean` | Logical value                             | `true`, `false` |
| `JSON`    | Arbitrary JSON data                       | Any valid JSON  |

### 3.2 Type Expressions

A type expression defines the type of a field or alias. Type expressions can be:

#### Simple Type Reference

A reference to a built-in type, type alias, or model:

```cdm
name: string
email: Email
author: User
```

#### Array Type

An array of another type, denoted with `[]` suffix:

```cdm
tags: string[]
posts: Post[]
matrix: number[][]  // Not supported - single dimension only
```

**Note**: Only single-dimensional arrays are supported. For multi-dimensional data, use a model or JSON type.

#### Union Type

A union of string literals and/or type references:

```cdm
// String literal union
status: "active" | "pending" | "deleted"

// Type reference union
content: TextBlock | ImageBlock | VideoBlock

// Mixed union
result: "error" | SuccessPayload
```

**Note**: Union types of models are currently supported in field definitions. Union types for models themselves (discriminated unions) are planned for a future version and would enable patterns like:

```cdm
// Future feature - not yet supported
ConfigValue: StringConfig | NumberConfig | BooleanConfig

StringConfig {
  type: "string"
  value: string
}

NumberConfig {
  type: "number"
  value: number
}
```

### 3.3 Optional Types

Fields can be marked optional with the `?` suffix on the field name:

```cdm
User {
  name: string       // Required
  nickname?: string  // Optional
}
```

Optional fields may be omitted entirely. This is distinct from a field that allows null values (which would require a union type if supported).

### 3.4 Type Compatibility

Types are compatible according to these rules:

1. A type is compatible with itself
2. A type alias is compatible with its underlying type
3. Array types are compatible if their element types are compatible
4. Union types are compatible if all members are compatible with corresponding members

---

## 4. Type Aliases

### 4.1 Basic Type Alias

A type alias creates a named reference to another type:

```cdm
Email: string
UserId: string
Count: number
```

With entity IDs for migration tracking:

```cdm
Email: string #1
UserId: string #2
Count: number #3
```

### 4.2 Type Alias with Plugin Configuration

Type aliases can include plugin-specific configuration:

```cdm
Email: string {
  @validation { format: "email", max_length: 320 }
  @sql { type: "VARCHAR(320)" }
} #1

UUID: string {
  @sql { type: "UUID", default: "gen_random_uuid()" }
} #2
```

### 4.3 Union Type Aliases

Union types can be named via type alias:

```cdm
Status: "active" | "pending" | "suspended" | "deleted" #1

Priority: "low" | "medium" | "high" | "critical" #2

ContentBlock: TextBlock | ImageBlock | CodeBlock #3
```

Union type aliases can also have plugin configuration:

```cdm
AccountType: "free" | "premium" | "enterprise" {
  @sql { type: "ENUM", name: "account_type_enum" }
} #4
```

### 4.4 Type Alias Semantics

- Type aliases are resolved at build time
- Aliases can reference other aliases (but not circularly)
- When a type alias is used in a field, the field inherits the alias's plugin configuration
- Field-level plugin configuration merges with (and can override) alias-level configuration

---

## 5. Models

### 5.1 Basic Model Definition

A model defines a structured type with named fields:

```cdm
User {
  id: string
  name: string
  email: string
}
```

With entity IDs:

```cdm
User {
  id: string #1
  name: string #2
  email: string #3
} #10
```

**Syntax Note**: Each model member (field definition, field removal, or plugin configuration) must appear on its own line. Single-line model definitions like `User { id: string, name: string }` are not supported. Empty models may appear on a single line: `EmptyModel {}`.

### 5.2 Field Definitions

Fields are defined within a model body. A field has:

- **Name**: Required identifier
- **Optional marker**: Optional `?` suffix on name
- **Type**: Optional type expression (defaults to `string` if omitted)
- **Default value**: Optional default using `=`
- **Plugin configuration**: Optional plugin block
- **Entity ID**: Optional `#N` suffix for migration tracking

#### Untyped Fields

If no type is specified, the field defaults to `string`:

```cdm
BasicUser {
  name #1
  email #2
  bio #3
} #10
```

#### Typed Fields

```cdm
TypedUser {
  id: string #1
  age: number #2
  active: boolean #3
  metadata: JSON #4
} #11
```

#### Optional Fields

```cdm
User {
  name: string #1
  nickname?: string #2
  bio? #3
} #12
```

#### Fields with Default Values

Default values must be literals (string, number, boolean, array, or object):

```cdm
Settings {
  theme: string = "dark" #1
  max_items: number = 100 #2
  enabled: boolean = true #3
  tags: string[] = ["default"] #4
  options: JSON = { "verbose": false } #5
} #13
```

**Note**: Function calls (like `now()`) are not supported as default values. Time-based defaults should be handled by plugins or application code.

#### Fields with Plugin Configuration

```cdm
Post {
  content: string {
    @sql { type: "TEXT" }
    @validation { min_length: 10, max_length: 50000 }
  } #1
} #20
```

### 5.3 Model-Level Plugin Configuration

Models can have plugin configuration blocks after all fields:

```cdm
User {
  id: string #1
  email: string #2
  name: string #3

  @sql {
    table: "users",
    indexes: [{ fields: ["email"], unique: true }]
  }
  @api { expose: ["id", "name", "email"] }
} #10
```

### 5.4 Field Relationships

Fields can reference other models, creating relationships:

```cdm
User {
  id: string #1
  posts: Post[] #2
} #10

Post {
  id: string #1
  author: User #2
  tags: Tag[] #3
} #11

Tag {
  id: string #1
  name: string #2
  posts: Post[] #3
} #12
```

Circular references between models are allowed and common for bidirectional relationships.

---

## 6. Inheritance

### 6.1 Single Inheritance

A model can extend another model using the `extends` keyword:

```cdm
Timestamped {
  created_at: string #1
  updated_at: string #2
} #10

Article extends Timestamped {
  id: string #3
  title: string #4
  content: string #5
} #11
```

The child model inherits all fields from the parent. The effective definition of `Article` is:

```cdm
Article {
  created_at: string  // Inherited
  updated_at: string  // Inherited
  id: string
  title: string
  content: string
}
```

### 6.2 Multiple Inheritance

A model can extend multiple parents:

```cdm
Timestamped {
  created_at: string #1
  updated_at: string #2
} #10

Auditable {
  created_by: User #1
  updated_by: User #2
} #11

Document extends Timestamped, Auditable {
  id: string #3
  title: string #4
  content: string #5
} #12
```

#### Field Conflict Resolution

When multiple parents define the same field, the **last parent listed wins**:

```cdm
Parent1 {
  status: "active" | "inactive" #1
} #10

Parent2 {
  status: "enabled" | "disabled" #1
} #11

Child extends Parent1, Parent2 {
  // status is "enabled" | "disabled" (from Parent2)
} #12
```

The child can always override explicitly for clarity:

```cdm
Child extends Parent1, Parent2 {
  status: "on" | "off" #2
} #12
```

### 6.3 Field Removal

Child models can remove inherited fields using the `-` prefix:

```cdm
BaseUser {
  id: string #1
  username: string #2
  email: string #3
  password_hash: string #4
  salt: string #5
} #10

PublicUser extends BaseUser {
  -password_hash
  -salt

  display_name: string #6
  avatar_url?: string #7
} #11
```

Field removal only applies to inherited fields. Attempting to remove a field that doesn't exist in a parent is an error.

**Note**: Field removals reference the parent's field by name. The ID remains with the original field definition in the parent.

### 6.4 Field Override

Child models can override inherited fields in two ways:

#### Redefining the Field

Provide a complete new definition:

```cdm
Parent {
  status: "active" | "inactive" #1
} #10

Child extends Parent {
  status: "active" | "inactive" | "pending" = "pending" #1
} #11
```

#### Adding Plugin Configuration

Add or override plugin configuration for an inherited field without redefining its type:

```cdm
Parent {
  email: string #1
} #10

Child extends Parent {
  email {
    @validation { format: "email" }
    @sql { unique: true }
  }
} #11
```

### 6.5 Inheritance of Plugin Configuration

When a child extends a parent:

1. **Model-level config**: Child's config merges with parent's config (see Section 7.4 for merge rules)
2. **Field-level config**: Inherited fields retain their plugin config; child can add or override
3. **Type alias config**: Fields using type aliases inherit the alias's plugin config

---

## 7. Context System

### 7.1 Overview

The context system allows multiple "views" of the same schema for different environments. A context file extends a base schema and can:

- Add new models and type aliases
- Remove models and type aliases
- Modify inherited models (add fields, remove fields, change types)
- Override type aliases
- Add or modify plugin configuration

### 7.2 Extends Directive

A context file begins with the `@extends` directive:

```cdm
// api.cdm
@extends ./base.cdm

// Modifications follow...
```

The path is relative to the current file.

### 7.3 Context Capabilities

#### Adding New Definitions

Context files can define new types and models:

```cdm
// api.cdm
@extends ./base.cdm

// New type alias
ApiToken: string {
  @validation { pattern: "^[a-zA-Z0-9]{32}$" }
} #20

// New model
ApiRequest {
  token: ApiToken #1
  timestamp: string #2
  endpoint: string #3
} #30
```

#### Removing Definitions

Use the `-` prefix to remove models or type aliases:

```cdm
// api.cdm
@extends ./base.cdm

-InternalAuditLog    // Remove model
-SystemConfig        // Remove model
-InternalCode        // Remove type alias (if no fields reference it)
```

Removing a type alias that is still referenced by any model (inherited or defined) is an error.

#### Modifying Models

Use the model name with a block to modify an inherited model:

```cdm
// api.cdm
@extends ./base.cdm

User {
  -password_hash      // Remove inherited field
  -salt               // Remove inherited field

  avatar_url: string #10  // Add new field
  is_online: boolean = false #11

  @api { expose: ["id", "name", "email", "avatar_url"] }
}
```

This syntax is the same as model definition. The system distinguishes between modification and new definition based on whether a model with that name exists in the ancestor chain.

#### Overriding Type Aliases

Type aliases can be redefined, automatically affecting all fields that use them:

```cdm
// base.cdm
Email: string {
  @validation { format: "email", max_length: 320 }
  @sql { type: "VARCHAR(320)" }
} #1

User { email: Email #1 } #10
Admin { contact_email: Email #1 } #11
```

```cdm
// api.cdm
@extends ./base.cdm

Email: string {
  @validation { format: "email" }  // Simpler validation for API
  // No SQL config needed for API context
} #1
```

In `api.cdm`, both `User.email` and `Admin.contact_email` use the redefined `Email` type.

### 7.4 Configuration Merging

When a context file extends another, plugin configurations are merged.

#### Merge Rules

1. **Objects**: Deep merge (recursive)
2. **Arrays**: Replace entirely
3. **Primitives**: Replace entirely

#### Example

```cdm
// base.cdm
@sql {
  dialect: "postgres",
  naming: { tables: "snake_case", columns: "snake_case" },
  indexes: [{ fields: ["id"] }]
}
```

```cdm
// child.cdm
@extends ./base.cdm

@sql {
  schema: "api",
  naming: { columns: "camelCase" },
  indexes: [{ fields: ["created_at"] }]
}
```

**Result in child.cdm:**

```cdm
@sql {
  dialect: "postgres",              // Inherited
  schema: "api",                    // Added
  naming: {
    tables: "snake_case",           // Inherited (deep merge)
    columns: "camelCase"            // Overridden
  },
  indexes: [{ fields: ["created_at"] }]  // Replaced (arrays replace entirely)
}
```

### 7.5 Context Chains

Contexts can extend other contexts, forming a chain:

```cdm
// base.cdm
User {
  id: string #1
  email: string #2
  password_hash: string #3
} #10

// client.cdm
@extends ./base.cdm
User { -password_hash }

// mobile.cdm
@extends ./client.cdm
User { device_token?: string #4 }
```

In `mobile.cdm`:

- `User` has `id`, `email` (from base via client)
- `password_hash` is removed (from client)
- `device_token` is added (from mobile)

Child contexts have access to all types from all ancestors.

### 7.6 Type Resolution in Contexts

When building a context, types are resolved as follows:

1. Collect all type aliases from the ancestor chain, with child definitions overriding parents
2. Collect all models from the ancestor chain, applying modifications from each context level
3. Resolve all type references using the collected definitions
4. Validate that all referenced types exist

Example:

```cdm
// base.cdm
Status: "active" | "inactive" #1
User { status: Status #1 } #10

// api.cdm
@extends ./base.cdm
Status: "active" | "inactive" | "pending" #1  // Override
```

When building `api.cdm`, `User.status` has type `"active" | "inactive" | "pending"`.

### 7.7 Restrictions

1. **No circular extends**: A file cannot extend itself or create a cycle
2. **No upward references**: A parent context cannot reference types defined only in a child
3. **Extends at top**: All `@extends` directives must appear at the top of the file, before plugin imports

---

## 8. Plugin System

### 8.1 Overview

Plugins extend CDM with custom code generation and validation. Plugins are WebAssembly modules that run in a sandboxed environment without filesystem or network access.

### 8.2 Plugin Import Syntax

Plugins are imported at the top of CDM files, before any type or model definitions:

```cdm
// Registry plugin (validation only - no build/migrate, so no output dirs needed)
@validation

// Registry plugin with build and migrate capabilities
@sql {
  dialect: "postgres",
  schema: "public",
  build_output: "./db/schema",
  migrations_output: "./db/migrations"
}

// Git plugin with build capability
@analytics from git:https://github.com/myorg/cdm-analytics.git {
  version: "1.0.0",
  endpoint: "https://analytics.example.com",
  build_output: "./analytics"
}

// Local plugin for development with build capability
@custom from ./plugins/my-plugin {
  debug: true,
  build_output: "./generated/custom"
}
```

### 8.3 Plugin Sources

#### Registry Plugins

Plugins without a `from` clause are resolved via the CDM plugin registry:

```cdm
@sql
@typescript
@validation
```

The registry is a curated JSON index hosted in the CDM repository.

#### Git Plugins

Plugins can be loaded from any git repository:

```cdm
// HTTPS
@plugin from git:https://github.com/user/repo.git

// SSH (private repos)
@plugin from git:git@github.com:org/private-repo.git
```

#### Local Plugins

Plugins can be loaded from the local filesystem:

```cdm
@custom from ./plugins/my-plugin
@shared from ../shared-plugins/common
```

### 8.4 Plugin Configuration

Plugin configuration uses JSON object syntax:

```cdm
@sql {
  dialect: "postgres",
  schema: "public",
  naming_convention: "snake_case",
  indexes: [
    { fields: ["email"], unique: true },
    { fields: ["created_at"], order: "DESC" }
  ]
}
```

Keys can be unquoted identifiers or quoted strings. Values can be any JSON value.

#### Reserved Configuration Keys

CDM extracts these keys before passing config to plugins:

| Key                 | Type     | Applies To       | Required                       | Description                                                  |
| ------------------- | -------- | ---------------- | ------------------------------ | ------------------------------------------------------------ |
| `version`           | `string` | Registry plugins | No                             | Semver version constraint (see Appendix C.2 for syntax)      |
| `git_ref`           | `string` | Git plugins      | No (defaults to `"main"`)      | Git reference: tag, branch name, or commit SHA               |
| `git_path`          | `string` | Git plugins      | No                             | Subdirectory path containing `cdm-plugin.json` manifest      |
| `build_output`      | `string` | All plugins      | Yes, if plugin has `build()`   | Output directory for generated files                         |
| `migrations_output` | `string` | All plugins      | Yes, if plugin has `migrate()` | Output directory for migration files                         |

**Validation Rules:**

- If a plugin exports a `build()` function, the global config **must** include `build_output`
- If a plugin exports a `migrate()` function, the global config **must** include `migrations_output`
- CDM validates these requirements after loading the plugin and reports error E406 if required keys are missing

### 8.5 Configuration Levels

Plugins receive configuration at four levels:

| Level      | Location         | Example                                           |
| ---------- | ---------------- | ------------------------------------------------- |
| Global     | Plugin import    | `@sql { dialect: "postgres" }`                    |
| TypeAlias  | Type alias block | `Email: string { @sql { type: "VARCHAR(320)" } }` |
| Model      | Model block      | `User { @sql { table: "users" } }`                |
| Field      | Field block      | `name: string { @sql { type: "TEXT" } }`          |

### 8.6 Plugin Execution Order

When multiple plugins are imported, they are executed in the order they appear in the file:

```cdm
@validation  // Executed first
@sql         // Executed second
@typescript  // Executed third
```

### 8.7 Plugin Configuration in Context Chains

When a context file extends another, plugin configurations merge:

```cdm
// base.cdm
@sql { dialect: "postgres", schema: "public" }

// api.cdm
@extends ./base.cdm
@sql { schema: "api" }  // Merges with parent config
```

See Section 7.4 for merge rules.

---

## 9. Semantic Validation

### 9.1 Validation Phases

CDM validation occurs in multiple phases:

1. **Lexical Analysis**: Tokenization
2. **Syntactic Analysis**: Parse tree construction (tree-sitter)
3. **Symbol Resolution**: Build symbol table, resolve references
4. **Semantic Validation**: Type checking, constraint validation
5. **Plugin Validation**: Plugin-specific configuration validation

### 9.2 Validation Rules

#### File Structure

| Rule                                        | Error |
| ------------------------------------------- | ----- |
| Plugin imports must come before definitions | E001  |
| `@extends` must come before plugin imports  | E002  |
| (Reserved for future use)                   | E003  |

#### Type Definitions

| Rule                              | Error |
| --------------------------------- | ----- |
| Duplicate type alias in same file | E101  |
| Circular type alias reference     | E102  |
| Unknown type reference            | E103  |

#### Model Definitions

| Rule                                  | Error |
| ------------------------------------- | ----- |
| Duplicate model in same file          | E201  |
| Duplicate field in same model         | E202  |
| Unknown parent in extends clause      | E203  |
| Removing non-existent field           | E204  |
| Field override on non-inherited field | E205  |

#### Context System

| Rule                             | Error |
| -------------------------------- | ----- |
| Circular extends chain           | E301  |
| Removing type alias still in use | E302  |
| Removing model still referenced  | E303  |
| Extends file not found           | E304  |

#### Plugin System

| Rule                           | Error |
| ------------------------------ | ----- |
| Plugin not found               | E401  |
| Invalid plugin configuration   | E402  |
| Missing required plugin export | E403  |
| Plugin execution failed        | E404  |
| Plugin output too large        | E405  |
| Missing required output config | E406  |

#### Entity IDs

| Rule                                       | Error |
| ------------------------------------------ | ----- |
| Duplicate model/type alias ID              | E501  |
| Duplicate field ID within model            | E502  |
| Reused ID (was deleted in previous schema) | E503  |

### 9.3 Forward References

Within a single file, forward references are allowed:

```cdm
// Valid: Post references User before User is defined
Post {
  author: User #1
} #10

User {
  posts: Post[] #1
} #11
```

Forward references across files are resolved through the ancestor chainâ€”a child context can reference types from ancestors, but not vice versa.

### 9.4 Circular Model References

Circular references between models are allowed and common:

```cdm
User {
  posts: Post[] #1
} #10

Post {
  author: User #1
} #11
```

### 9.5 Error Recovery

The parser should recover from errors when possible to report multiple issues:

```cdm
User {
  name: string #1
  email: UnknownType #2    // Error: Unknown type
  age: number #3           // Continue parsing
} #10

Post {
  title: string #1
  author: AlsoUnknown #2   // Error: Unknown type
} #11
```

Both errors should be reported in a single validation pass.

---

## 10. File Structure and Resolution

### 10.1 File Extension

CDM files use the `.cdm` extension.

### 10.2 File Encoding

All CDM files must be UTF-8 encoded.

### 10.3 Project Structure

A typical CDM project structure:

```
my-project/
â”œâ”€â”€ cdm/
â”‚   â”œâ”€â”€ base.cdm           # Base schema
â”‚   â”œâ”€â”€ api.cdm            # API context
â”‚   â”œâ”€â”€ client.cdm         # Client context
â”‚   â””â”€â”€ admin.cdm          # Admin context
â”œâ”€â”€ .cdm/
â”‚   â”œâ”€â”€ cache/
â”‚   â”‚   â””â”€â”€ plugins/       # Downloaded plugin WASM files
â”‚   â”œâ”€â”€ previous_schema.json
â”‚   â””â”€â”€ registry.json      # Cached plugin registry
â”œâ”€â”€ db/
â”‚   â”œâ”€â”€ schema/            # Generated SQL
â”‚   â””â”€â”€ migrations/        # Generated migrations
â””â”€â”€ src/
    â””â”€â”€ types/             # Generated TypeScript
```

### 10.4 Path Resolution

Paths in `@extends` directives and local plugin references are resolved relative to the containing file:

```cdm
// In /project/cdm/contexts/api.cdm
@extends ../base.cdm           // Resolves to /project/cdm/base.cdm
@custom from ../../plugins/my-plugin  // Resolves to /project/plugins/my-plugin
```

### 10.5 Build Outputs

When building a context, CDM:

1. Resolves the full ancestor chain
2. Merges all type aliases and models
3. Merges all plugin configurations
4. Validates the complete schema
5. Invokes each plugin's `build` function
6. Writes output files to configured directories
7. Warns about entities without IDs (if `--check-ids` specified)

---

## 11. CLI Interface

### 11.1 Commands Overview

```
cdm <command> [options]

Commands:
  validate    Validate CDM files
  build       Generate output files
  migrate     Generate migration files
  format      Format CDM files and assign entity IDs
  plugin      Plugin management

Options:
  --help      Show help
  --version   Show version
```

### 11.2 Validate Command

Validates CDM files without generating output.

```bash
cdm validate [files...]
cdm validate                    # Validate all .cdm files in current directory
cdm validate schema.cdm         # Validate specific file
cdm validate cdm/*.cdm          # Validate multiple files
```

**Options:**

- `--quiet`, `-q`: Only output errors
- `--format <fmt>`: Output format (text, json)
- `--check-ids`: Validate entity ID usage (uniqueness, no reuse)

**Exit Codes:**

- 0: Validation successful
- 1: Validation errors found
- 2: File not found or other error

### 11.3 Build Command

Generates output files by running all plugin `build` functions.

```bash
cdm build [files...]
cdm build                       # Build all .cdm files
cdm build api.cdm               # Build specific context
```

**Options:**

- `--output`, `-o <dir>`: Override output directory
- `--plugin <name>`: Only run specific plugin
- `--dry-run`: Show what would be generated without writing
- `--check-ids`: Warn about entities without IDs

**Behavior:**

1. Validate all files
2. For each context file (or specified files):
   - Resolve full schema
   - Call each plugin's `build` function
   - Write output files to configured directories

### 11.4 Migrate Command

Generates migration files by comparing current schema to previous schema.

```bash
cdm migrate [files...]
cdm migrate                     # Generate migrations for all contexts
cdm migrate base.cdm            # Generate migrations for specific context
cdm migrate --name "add_avatar" # Custom migration name
```

**Options:**

- `--name`, `-n <name>`: Custom migration name
- `--output`, `-o <dir>`: Override migrations output directory
- `--dry-run`: Show deltas without generating files

**Behavior:**

1. Load previous schema from `.cdm/previous_schema.json`
2. Build current schema
3. Compute deltas between previous and current (using entity IDs when available)
4. Call each plugin's `migrate` function with deltas
5. Write migration files
6. Save current schema as new previous schema

**Rename Detection:**

- Entities with IDs: 100% reliable rename detection
- Entities without IDs: Heuristic detection with user confirmation for ambiguous cases

### 11.5 Format Command

Formats CDM files and optionally assigns entity IDs.

```bash
cdm format [files...]
cdm format                          # Format all .cdm files
cdm format schema.cdm               # Format specific file
cdm format --assign-ids             # Auto-assign missing IDs
cdm format --check                  # Check formatting without writing
```

**Options:**

- `--assign-ids`: Automatically assign IDs to entities without them
- `--check`: Verify formatting without modifying files (exit code 1 if changes needed)
- `--write`, `-w`: Write changes to files (default)

**ID Assignment Behavior:**

When `--assign-ids` is used:

1. Finds all entities (models, type aliases, fields) without IDs
2. Determines the next available ID for each scope
3. Assigns sequential IDs to unidentified entities
4. Reports assignments made

```bash
$ cdm format --assign-ids schema.cdm
Assigned IDs:
  Model 'NewModel' -> #15
  Field 'NewModel.email' -> #1
  Field 'NewModel.name' -> #2
  Type alias 'NewStatus' -> #5
```

### 11.6 Plugin Commands

#### List Plugins

```bash
cdm plugin list                 # List registry plugins
cdm plugin list --cached        # List cached plugins
```

#### Plugin Info

```bash
cdm plugin info sql             # Show plugin details
cdm plugin info sql --versions  # Show available versions
```

#### Create New Plugin

```bash
cdm plugin new my-plugin                    # Create Rust plugin
cdm plugin new my-plugin --output ./plugins # Custom directory
```

#### Cache Management

```bash
cdm plugin cache sql            # Pre-download specific plugin
cdm plugin cache --all          # Cache all plugins used in project
cdm plugin clear-cache          # Clear plugin cache
cdm plugin clear-cache sql      # Clear specific plugin
```

---

## 12. Plugin Development

### 12.1 Plugin Structure

A CDM plugin repository contains:

```
cdm-plugin-example/
â”œâ”€â”€ cdm-plugin.json       # Manifest (required)
â”œâ”€â”€ schema.cdm            # Settings schema (required)
â”œâ”€â”€ Cargo.toml            # Rust project config
â”œâ”€â”€ src/
â”‚   â”œâ”€â”€ lib.rs            # Plugin entry point
â”‚   â”œâ”€â”€ validate.rs       # Config validation
â”‚   â”œâ”€â”€ build.rs          # Code generation
â”‚   â””â”€â”€ migrate.rs        # Migration generation
â”œâ”€â”€ .gitignore
â””â”€â”€ README.md
```

### 12.2 Manifest Format

`cdm-plugin.json`:

```json
{
  "name": "example",
  "version": "1.0.0",
  "description": "An example CDM plugin",
  "wasm": {
    "file": "target/wasm32-wasip1/release/cdm_plugin_example.wasm",
    "release_url": "https://github.com/.../releases/download/v{version}/plugin.wasm"
  },
  "capabilities": ["build", "migrate"]
}
```

**Fields:**

| Field              | Required | Description                              |
| ------------------ | -------- | ---------------------------------------- |
| `name`             | Yes      | Plugin identifier                        |
| `version`          | Yes      | Semantic version                         |
| `description`      | Yes      | Human-readable description               |
| `wasm.file`        | Yes      | Path to WASM file (relative to manifest) |
| `wasm.release_url` | No       | URL template for downloading releases    |
| `capabilities`     | Yes      | Array of: `"generate"`, `"migrate"`      |

### 12.3 Settings Schema

Plugins expose their settings schema via the `_schema()` WASM export (see Section 12.4). The schema defines valid configuration using CDM syntax:

```cdm
// Example schema returned by _schema() for sql plugin

GlobalSettings {
  dialect: "postgres" | "mysql" | "sqlite" = "postgres"
  schema?: string
  naming_convention: "snake_case" | "camelCase" = "snake_case"
}

TypeAliasSettings {
  type?: string
  default?: string
  comment?: string
}

ModelSettings {
  table?: string
  indexes?: Index[]
  primary_key?: string
}

FieldSettings {
  type?: string
  column?: string
  default?: string
  index?: boolean
  unique?: boolean
}

Index {
  fields: string[]
  unique?: boolean
  type?: "btree" | "hash" | "gin" | "gist"
  where?: string
}
```

The schema must define at least `GlobalSettings`. `TypeAliasSettings`, `ModelSettings`, and `FieldSettings` are optional.

#### Field Optionality and Defaults

Plugin schema fields follow these rules for optionality:

- **Fields marked with `?`**: Optional and may be omitted entirely (e.g., `schema?: string`)
- **Fields with default values**: Implicitly optional; when omitted, the default value is automatically applied (e.g., `dialect: "postgres" | "mysql" | "sqlite" = "postgres"`)
- **Fields without `?` or a default value**: Required and must be provided by the user (e.g., `build_output: string`)

When validating user configuration, CDM automatically fills in default values for any omitted fields before passing the configuration to the plugin's `validate_config()` function. This means plugins always receive complete configuration objects with all defaults applied.

**Example:**

```cdm
// Plugin schema
GlobalSettings {
  dialect: "postgres" | "mysql" | "sqlite" = "postgres"  // Optional (has default)
  schema?: string                                         // Optional (explicitly marked)
  build_output: string                                    // Required (no default, no ?)
  max_connections: number = 10                            // Optional (has default)
}

// User configuration
@sql {
  build_output: "./db/schema"       // Required, must provide
  schema: "public"                  // Optional, explicitly set
  // dialect omitted - defaults to "postgres"
  // max_connections omitted - defaults to 10
}

// Configuration received by plugin (after defaults applied)
{
  "dialect": "postgres",
  "schema": "public",
  "build_output": "./db/schema",
  "max_connections": 10
}
```

CDM validates user-provided plugin configurations against this schema before calling `validate_config`.

### 12.4 Plugin API

Plugins must implement `schema()`. The `validate_config()`, `build()`, and `migrate()` functions are optional.

#### schema (Required)

Returns the plugin's configuration schema as a CDM string.

```rust
fn schema() -> String
```

This function should return a CDM schema defining the structure of valid configurations at global, model, and field levels (see Section 12.3).

CDM uses this schema to validate user-provided plugin configurations before calling `validate_config()`.

#### validate_config (Optional)

Validates user configuration at each level.

```rust
fn validate_config(
    level: ConfigLevel,
    config: JSON,
    utils: Utils,
) -> Vec<ValidationError>
```

**Types:**

```rust
enum ConfigLevel {
    Global,
    Model { name: String },
    Field { model: String, field: String },
}

struct PathSegment {
    kind: String,   // "global", "model", "field", "config", etc.
    name: String,
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

#### build (Optional)

Transforms schema into output files.

```rust
fn build(
    schema: Schema,
    config: JSON,
    utils: Utils,
) -> Vec<OutputFile>
```

**Types:**

```rust
struct Schema {
    type_aliases: Vec<TypeAliasDefinition>,
    models: Vec<ModelDefinition>,
}

struct OutputFile {
    path: String,      // Relative path
    content: String,
}
```

#### migrate (Optional)

Generates migration files from schema changes.

```rust
fn migrate(
    schema: Schema,
    deltas: Vec<Delta>,
    config: JSON,
    utils: Utils,
) -> Vec<OutputFile>
```

### 12.5 Delta Types

Deltas represent schema changes for migration generation:

```rust
enum Delta {
    // Models
    ModelAdded { name: String, after: ModelDefinition },
    ModelRemoved { name: String, before: ModelDefinition },
    ModelRenamed {
        old_name: String,
        new_name: String,
        id: Option<u32>,  // ID that links the rename
        before: ModelDefinition,
        after: ModelDefinition
    },

    // Fields
    FieldAdded { model: String, field: String, after: FieldDefinition },
    FieldRemoved { model: String, field: String, before: FieldDefinition },
    FieldRenamed {
        model: String,
        old_name: String,
        new_name: String,
        id: Option<u32>,  // ID that links the rename
        before: FieldDefinition,
        after: FieldDefinition
    },
    FieldTypeChanged {
        model: String,
        field: String,
        before: TypeExpression,
        after: TypeExpression
    },
    FieldOptionalityChanged {
        model: String,
        field: String,
        before: bool,   // was optional
        after: bool     // is now optional
    },
    FieldDefaultChanged {
        model: String,
        field: String,
        before: Option<Value>,
        after: Option<Value>
    },

    // Type Aliases
    TypeAliasAdded { name: String, after: TypeAliasDefinition },
    TypeAliasRemoved { name: String, before: TypeAliasDefinition },
    TypeAliasRenamed {
        old_name: String,
        new_name: String,
        id: Option<u32>,  // ID that links the rename
        before: TypeAliasDefinition,
        after: TypeAliasDefinition
    },
    TypeAliasTypeChanged {
        name: String,
        before: TypeExpression,
        after: TypeExpression
    },

    // Inheritance
    InheritanceAdded { model: String, parent: String },
    InheritanceRemoved { model: String, parent: String },

    // Config Changes
    GlobalConfigChanged { before: JSON, after: JSON },
    ModelConfigChanged { model: String, before: JSON, after: JSON },
    FieldConfigChanged { model: String, field: String, before: JSON, after: JSON },
}
```

**Rename Detection:**

- Entities **with IDs**: 100% reliable rename detection. If an entity's ID exists in both schemas but the name differs, it's definitively a rename.
- Entities **without IDs**: Heuristic detection based on type compatibility, name similarity, and structural matching. Ambiguous cases prompt for user confirmation.

### 12.6 Supporting Types

```rust
struct ModelDefinition {
    name: String,
    id: Option<u32>,              // Entity ID for rename tracking
    parents: Vec<String>,
    fields: Vec<FieldDefinition>,
    config: HashMap<String, JSON>,  // plugin name -> config
}

struct FieldDefinition {
    name: String,
    id: Option<u32>,              // Entity ID for rename tracking
    field_type: TypeExpression,
    optional: bool,
    default: Option<Value>,
    config: HashMap<String, JSON>,  // plugin name -> config
}

struct TypeAliasDefinition {
    name: String,
    id: Option<u32>,              // Entity ID for rename tracking
    alias_type: TypeExpression,
    config: HashMap<String, JSON>,  // plugin name -> config
}

enum TypeExpression {
    Identifier(String),
    Array(Box<TypeExpression>),
    Union(Vec<TypeExpression>),
    StringLiteral(String),
}

enum Value {
    String(String),
    Number(f64),
    Boolean(bool),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}
```

### 12.7 Utility Functions

CDM provides utility functions to plugins:

```rust
struct Utils {
    // String case conversion
    fn change_case(&self, input: &str, format: CaseFormat) -> String;

    // English pluralization
    fn pluralize(&self, input: &str) -> String;
}

enum CaseFormat {
    Snake,      // user_profile
    Camel,      // userProfile
    Pascal,     // UserProfile
    Kebab,      // user-profile
    Constant,   // USER_PROFILE
    Title,      // User Profile
}
```

#### Pluralization

The `pluralize` function converts singular English words to their plural form following standard English pluralization rules:

```rust
utils.pluralize("user")      // "users"
utils.pluralize("category")  // "categories"
utils.pluralize("person")    // "people"
utils.pluralize("child")     // "children"
utils.pluralize("box")       // "boxes"
utils.pluralize("leaf")      // "leaves"
```

**Pluralization Rules:**

1. **Regular plurals**: Adds 's' (cat â†’ cats)
2. **Sibilants** (s, x, z, ch, sh): Adds 'es' (box â†’ boxes)
3. **Consonant + y**: Changes to 'ies' (city â†’ cities)
4. **Vowel + y**: Adds 's' (boy â†’ boys)
5. **Words ending in f/fe**: Changes to 'ves' (leaf â†’ leaves)
6. **Consonant + o**: Adds 'es' (hero â†’ heroes)
7. **Irregular plurals**: Handles common irregular forms (person â†’ people, child â†’ children, etc.)
8. **Unchanging words**: Returns original (sheep â†’ sheep, fish â†’ fish)
9. **Case preservation**: Maintains capitalization (Person â†’ People)

### 12.8 Building Plugins

```bash
# Install WASM target
rustup target add wasm32-wasip1

# Build the plugin
cargo build --release --target wasm32-wasip1
```

### 12.9 Testing Locally

Reference a local plugin during development:

```cdm
@my-plugin from ./path/to/cdm-plugin-my-plugin {
  debug: true
}
```

### 12.10 Publishing

1. Create a GitHub repository for your plugin
2. Build the WASM file
3. Create a GitHub release with the `.wasm` file attached
4. (Optional) Submit a PR to add your plugin to the CDM registry

### 12.11 Sandbox Limits

Plugins run with resource limits:

| Limit     | Default    | Description                     |
| --------- | ---------- | ------------------------------- |
| Memory    | 256 MB     | Maximum WASM memory             |
| Execution | 30 seconds | Maximum execution time per call |
| Output    | 10 MB      | Maximum total output size       |

---

## Appendix A: Grammar

### A.1 EBNF Grammar

```ebnf
(* Top-level structure *)
source_file = { newlines }, { extends_directive , newlines }, { plugin_import , newlines }, { definition , [ newlines ] } ;
definition = model_removal | type_alias | model_definition ;

(* Whitespace *)
newline = "\r\n" | "\n" ;
newlines = newline , { newline } ;

(* Comments *)
comment = "//" , { any_char - newline } ;

(* Plugin imports *)
plugin_import = "@" , identifier , [ "from" , plugin_source ] , [ object_literal ] ;
plugin_source = git_reference | plugin_path ;
git_reference = "git:" , url ;
plugin_path = ( "./" | "../" ) , path_segment , { "/" , path_segment } ;

(* Directives *)
extends_directive = "@extends" , file_path ;
model_removal = "-" , identifier ;

(* Entity IDs *)
entity_id = "#" , positive_integer ;
positive_integer = nonzero_digit , { digit } ;
nonzero_digit = "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" ;

(* Type aliases *)
type_alias = identifier , ":" , type_expression , [ plugin_block ] , [ entity_id ] ;

(* Models *)
model_definition = identifier , [ extends_clause ] , model_body , [ entity_id ] ;
extends_clause = "extends" , identifier , { "," , identifier } ;
(* Note: model members are separated by newlines, not commas *)
model_body = "{" , [ newlines ] , [ model_member , { newlines , model_member } , [ newlines ] ] , "}" ;
model_member = field_removal | plugin_config | field_override | field_definition ;

(* Fields *)
field_removal = "-" , identifier ;
field_override = identifier , plugin_block ;
field_definition = identifier , [ "?" ] , [ ":" , type_expression , [ "=" , value ] , [ plugin_block ] ] , [ entity_id ] ;

(* Types *)
type_expression = union_type | array_type | type_identifier ;
union_type = union_member , "|" , union_member , { "|" , union_member } ;
union_member = string_literal | array_type | type_identifier ;
array_type = type_identifier , "[" , "]" ;
type_identifier = identifier ;

(* Plugins *)
plugin_block = "{" , [ newlines ] , [ plugin_config , { [ newlines ] , plugin_config } , [ newlines ] ] , "}" ;
plugin_config = "@" , identifier , object_literal ;

(* Values - commas separate elements, newlines optional *)
value = string_literal | number_literal | boolean_literal | array_literal | object_literal ;
array_literal = "[" , [ newlines ] , [ value , { "," , [ newlines ] , value } , [ "," ] , [ newlines ] ] , "]" ;
object_literal = "{" , [ newlines ] , [ object_entry , { "," , [ newlines ] , object_entry } , [ "," ] , [ newlines ] ] , "}" ;
object_entry = ( identifier | string_literal ) , ":" , value ;

(* Literals *)
string_literal = '"' , { string_char | escape_sequence } , '"' ;
string_char = any_char - ( '"' | '\' ) ;
escape_sequence = '\' , ( '"' | '\' | '/' | 'b' | 'f' | 'n' | 'r' | 't' | unicode_escape ) ;
unicode_escape = 'u' , hex_digit , hex_digit , hex_digit , hex_digit ;
number_literal = [ "-" ] , digit , { digit } , [ "." , digit , { digit } ] ;
boolean_literal = "true" | "false" ;

(* Basic elements *)
identifier = ( letter | "_" ) , { letter | digit | "_" } ;
letter = "a" | ... | "z" | "A" | ... | "Z" ;
digit = "0" | ... | "9" ;
hex_digit = digit | "a" | ... | "f" | "A" | ... | "F" ;
```

### A.2 Tree-sitter Grammar

See `grammar.js` in the CDM repository for the complete tree-sitter grammar implementation.

---

## Appendix B: Error Catalog

### B.1 File Structure Errors

| Code | Message                        | Description                                            |
| ---- | ------------------------------ | ------------------------------------------------------ |
| E001 | Plugin import after definition | Plugin imports must come before type/model definitions |
| E002 | Extends after plugin import    | @extends directives must come before plugin imports    |
| E003 | (Reserved)                     | Reserved for future use                                |

### B.2 Type Errors

| Code | Message                       | Description                                         |
| ---- | ----------------------------- | --------------------------------------------------- |
| E101 | Duplicate type alias '{name}' | Type alias defined multiple times in same file      |
| E102 | Circular type alias reference | Type alias references itself directly or indirectly |
| E103 | Unknown type '{name}'         | Reference to undefined type                         |

### B.3 Model Errors

| Code | Message                                         | Description                                |
| ---- | ----------------------------------------------- | ------------------------------------------ |
| E201 | Duplicate model '{name}'                        | Model defined multiple times in same file  |
| E202 | Duplicate field '{field}' in model '{model}'    | Field name used multiple times             |
| E203 | Unknown parent model '{name}'                   | Extends clause references undefined model  |
| E204 | Cannot remove non-existent field '{field}'      | Field removal on field not in parent       |
| E205 | Field override on non-inherited field '{field}' | Field override syntax used for local field |

### B.4 Context Errors

| Code | Message                                                       | Description                                  |
| ---- | ------------------------------------------------------------- | -------------------------------------------- |
| E301 | Circular extends chain                                        | File extends itself directly or indirectly   |
| E302 | Cannot remove type '{name}': still referenced by {locations}  | Type removal when type is still used         |
| E303 | Cannot remove model '{name}': still referenced by {locations} | Model removal when model is still referenced |
| E304 | Extends file not found: '{path}'                              | Extended file does not exist                 |

### B.5 Plugin Errors

| Code | Message                                                      | Description                                             |
| ---- | ------------------------------------------------------------ | ------------------------------------------------------- |
| E401 | Plugin not found: '{name}'                                   | Could not resolve plugin                                |
| E402 | Invalid plugin configuration: {details}                      | Plugin config validation failed                         |
| E403 | Plugin missing required export: '{function}'                 | WASM module doesn't export required function (\_schema) |
| E404 | Plugin execution failed: {details}                           | Plugin function threw error or timed out                |
| E405 | Plugin output too large: {size} exceeds {limit}              | Output size limit exceeded                              |
| E406 | Plugin '{name}' requires '{key}' in global config            | Plugin has capability but missing required output path  |

### B.6 Entity ID Errors

| Code | Message                                     | Description                                        |
| ---- | ------------------------------------------- | -------------------------------------------------- |
| E501 | Duplicate entity ID #{id}                   | Same ID used for multiple models or type aliases   |
| E502 | Duplicate field ID #{id} in model '{model}' | Same ID used for multiple fields in one model      |
| E503 | Reused entity ID #{id}                      | ID was used by a deleted entity in previous schema |

### B.7 Warnings

| Code | Message                              | Description                                  |
| ---- | ------------------------------------ | -------------------------------------------- |
| W001 | Unused type alias '{name}'           | Type alias defined but never referenced      |
| W002 | Unused model '{name}'                | Model defined but never referenced           |
| W003 | Field shadows parent field '{field}' | Child field completely replaces parent field |
| W004 | Empty model '{name}'                 | Model has no fields                          |
| W005 | Entity '{name}' has no ID            | Entity lacks ID for migration tracking       |
| W006 | Field '{model}.{field}' has no ID    | Field lacks ID for migration tracking        |

---

## Appendix C: Registry Format

### C.1 Registry JSON Schema

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
        },
        "1.1.0": {
          "wasm_url": "https://github.com/.../releases/download/v1.1.0/plugin.wasm",
          "checksum": "sha256:e5f6g7h8..."
        }
      },
      "latest": "1.1.0"
    }
  }
}
```

### C.2 Version Resolution

#### Version Constraint Syntax

Version constraints follow semantic versioning:

| Constraint  | Meaning                                        |
| ----------- | ---------------------------------------------- |
| `"1.0.0"`   | Exact version                                  |
| `"^1.0.0"`  | Compatible with 1.x.x (>=1.0.0 <2.0.0)         |
| `"~1.0.0"`  | Patch-level changes only (>=1.0.0 <1.1.0)      |
| `">=1.0.0"` | At least this version                          |
| `"*"`       | Any version (equivalent to omitting version)   |

**Note:** The `version` config key applies only to registry plugins. For git plugins, use `git_ref` instead (see Reserved Configuration Keys in Section 8.4).

#### Resolution Rules

When resolving a plugin version:

1. **Registry plugins**: If `version` specified with semver range, find the highest matching version from registry
2. **Registry plugins**: If `version` specified as exact version, use that exact version
3. **Registry plugins**: If no version specified, use `latest` from registry
4. **Git plugins**: Use `git_ref` to specify a branch, tag, or commit SHA (defaults to `"main"`)

---

## Appendix D: Data Exchange Format

### D.1 Schema JSON Format

When passing schemas to plugins or storing for diffing:

```json
{
  "type_aliases": [
    {
      "name": "Email",
      "id": 1,
      "alias_type": { "kind": "identifier", "name": "string" },
      "config": {
        "validation": { "format": "email" },
        "sql": { "type": "VARCHAR(320)" }
      }
    }
  ],
  "models": [
    {
      "name": "User",
      "id": 10,
      "parents": [],
      "fields": [
        {
          "name": "id",
          "id": 1,
          "field_type": { "kind": "identifier", "name": "string" },
          "optional": false,
          "default": null,
          "config": {}
        },
        {
          "name": "email",
          "id": 2,
          "field_type": { "kind": "identifier", "name": "Email" },
          "optional": false,
          "default": null,
          "config": {}
        }
      ],
      "config": {
        "sql": { "table": "users" }
      }
    }
  ]
}
```

**Note**: The `id` field is `null` when no entity ID is assigned.

### D.2 Type Expression JSON

```json
// Identifier
{ "kind": "identifier", "name": "string" }

// Array
{ "kind": "array", "element": { "kind": "identifier", "name": "Post" } }

// Union
{
  "kind": "union",
  "members": [
    { "kind": "string_literal", "value": "active" },
    { "kind": "string_literal", "value": "inactive" }
  ]
}

// String literal
{ "kind": "string_literal", "value": "active" }
```

---

_End of Specification_