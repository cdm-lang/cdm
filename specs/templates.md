# CDM Templates Specification

**Version**: 1.0.0-draft
**Status**: Draft

---

## Implementation Note

This specification introduces semver range syntax for version constraints (e.g., `^1.0.0`, `~1.0.0`). The existing plugin system currently supports only exact versions and git refs. To maintain consistency between plugins and templates, the plugin version resolution system should be updated to support the same semver range syntax documented here. See Appendix C.2 of the main specification for the updated version resolution rules that apply to both plugins and templates.

---

## 1. Overview

Templates are versioned, reusable CDM schema packages that can be imported into any CDM project. They enable:

- **Plugin type libraries**: Plugins like `@sql` can maintain companion templates with pre-defined type aliases (e.g., `UUID`, `Text`, `Varchar`)
- **Feature packages**: Complete feature implementations (authentication, multi-tenancy, billing) with models, types, and plugin configuration
- **Shared schemas**: Organization-wide base schemas that multiple projects can build upon

Templates complement the existing plugin system:
- **Plugins** = Code (WASM) that transforms schemas into outputs
- **Templates** = Data (CDM schemas) that provide reusable definitions

---

## 2. Template Sources

Templates can be loaded from the same sources as plugins:

### 2.1 Registry Templates

Templates published to the CDM template registry:

```cdm
import sql from sql/postgres-types
import auth from cdm/auth
```

Registry template names can optionally include `/` characters for organizational scoping:
- `sql/postgres-types` - scoped under `sql`
- `cdm/auth` - scoped under `cdm`
- `mytemplate` - unscoped, top-level name

Scoping is optional but recommended for clarity and to avoid naming conflicts.

### 2.2 Git Templates

Templates loaded from git repositories:

```cdm
import auth from git:https://github.com/cdm-lang/cdm-template-auth.git
import internal from git:git@github.com:myorg/cdm-schemas.git
```

#### Git Reference Pinning

Git templates can specify a git reference using the `git_ref` config key:

```cdm
import auth from git:https://github.com/cdm-lang/cdm-template-auth.git {
  git_ref: "v2.1.0"
}
```

The `git_ref` can be:
- A tag (e.g., `"v2.1.0"`)
- A branch name (e.g., `"main"`, `"develop"`)
- A commit SHA (e.g., `"a1b2c3d"`)

If no `git_ref` is specified, the `main` branch is used by default.

#### Subdirectory Paths

For monorepos or repositories where the template is nested within a subdirectory, use the `git_path` config key:

```cdm
import types from git:https://github.com/my-org/monorepo.git {
  git_path: "packages/cdm-types"
}
```

The `git_path` specifies the directory containing the `cdm-template.json` manifest file.

You can combine git reference pinning with subdirectory paths:

```cdm
import types from git:https://github.com/my-org/monorepo.git {
  git_ref: "v2.0.0",
  git_path: "packages/cdm-types"
}
```

### 2.3 Local Templates

Templates loaded from the local filesystem:

```cdm
import shared from ./templates/shared
import common from ../common-schemas
```

Paths are resolved relative to the importing file.

---

## 3. Import Modes

CDM provides two distinct ways to use templates, matching the semantic difference between referencing and inheriting:

### 3.1 Namespaced Import (`import`)

The `import` keyword brings template definitions into scope under a namespace, without merging them into your schema output.

**Syntax:**

```
import <namespace> from <source> [{ <config> }]
```

**Example:**

```cdm
import sql from sql/postgres-types
import auth from cdm/auth

User {
  id: sql.UUID #1
  email: sql.Varchar { @sql { length: 255 } } #2
  role: auth.Role #3
}
```

**Behavior:**
- Template types are accessible via `namespace.TypeName` syntax
- Template models are accessible via `namespace.ModelName` syntax
- Imported definitions are NOT included in your schema output
- Useful for referencing types without including all template models in your build

### 3.2 Merged Import (`extends`)

The `extends` keyword merges template definitions into your schema, making them part of your output.

**Syntax (expanded to support all sources):**

```
extends <source> [{ <config> }]
```

**Examples:**

```cdm
// Local file (current behavior)
extends ./base.cdm

// Registry template
extends cdm/auth

// Git template
extends git:https://github.com/cdm-lang/cdm-template-auth.git

// With version pinning
extends cdm/auth { version: "2.1.0" }
```

**Behavior:**
- All template definitions are merged into your schema
- You can modify, extend, or remove merged definitions
- Merged types are available without namespace prefix
- This is the current `extends` behavior, extended to support remote sources

### 3.3 Combined Usage

Both keywords can be used together:

```cdm
// Import SQL types as a namespace (for type references)
import sql from sql/postgres-types

// Merge auth models into our schema (we want to extend them)
extends cdm/auth

// Auth's User model is now in our schema, we can extend it
User {
  id: sql.UUID #100           // Use SQL type from namespace
  avatar_url: string #101     // Add new field
  -internal_notes             // Remove inherited field
}

// Auth's Session model is also available
Session {
  device_info: string #50     // Extend it too
}

// Our own model can reference both
Post {
  id: sql.UUID #1
  author: User #2             // User from merged auth template
  created_at: sql.Timestamp #3
}
```

---

## 4. Directive Ordering

All `extends`, `import`, and plugin directives must appear at the top of a CDM file, before any type alias or model definitions. The ordering among these directives is flexible.

**Example:**

```cdm
// These can appear in any order, as long as they're before definitions
extends ./base.cdm
extends cdm/auth { version: "^2.0.0" }
import sql from sql/postgres-types
import analytics from git:https://github.com/org/analytics.git
@sql { dialect: "postgres", build_output: "./db" }
@typescript { build_output: "./src/types" }

// Definitions must come after all directives
User {
  id: sql.UUID #1
}
```

---

## 5. Namespace Access

### 5.1 Dot Notation

Namespaced definitions are accessed using dot notation:

```cdm
import sql from sql/postgres-types

User {
  id: sql.UUID #1
  name: sql.Varchar #2
  bio: sql.Text #3
  age: sql.SmallInt #4
}
```

### 5.2 Nested Namespaces

If a template re-exports another template, nested access is supported:

```cdm
// If cdm/auth internally imports and re-exports sql/postgres-types as "types"
import auth from cdm/auth

User {
  id: auth.types.UUID #1  // Access nested namespace
}
```

However, templates SHOULD re-export commonly used types at the top level for convenience:

```cdm
// Better: auth re-exports UUID directly
import auth from cdm/auth

User {
  id: auth.UUID #1  // Cleaner access
}
```

### 5.3 Field-Level Configuration on Namespaced Types

When using a namespaced type, you can add field-level plugin configuration:

```cdm
import sql from sql/postgres-types

User {
  // sql.Varchar provides @sql { type: "VARCHAR" }
  // Field adds length specification
  name: sql.Varchar {
    @sql { length: 100 }
  } #1

  // sql.UUID provides @sql { type: "UUID" }
  // Field overrides default
  id: sql.UUID {
    @sql { default: "uuid_generate_v4()" }
  } #2
}
```

Configuration merges following existing rules (Section 7.4 of main spec):
- Objects: Deep merge
- Arrays: Replace entirely
- Primitives: Replace entirely

---

## 6. Template Structure

### 6.1 Directory Layout

A template is a directory containing:

```
cdm-template-auth/
├── cdm-template.json     # Manifest (required)
├── index.cdm             # Main entry point (required)
├── types.cdm             # Type definitions (optional)
├── models.cdm            # Model definitions (optional)
└── README.md             # Documentation (optional)
```

### 6.2 Manifest Format

`cdm-template.json`:

```json
{
  "name": "cdm/auth",
  "version": "2.1.0",
  "description": "Complete authentication system with User, Session, and Role models",
  "entry": "./index.cdm",
  "exports": {
    ".": "./index.cdm",
    "./types": "./types.cdm",
    "./models": "./models.cdm"
  }
}
```

**Fields:**

| Field          | Required | Description                                           |
| -------------- | -------- | ----------------------------------------------------- |
| `name`         | Yes      | Template identifier (e.g., `cdm/auth`)                |
| `version`      | Yes      | Semantic version                                      |
| `description`  | Yes      | Human-readable description                            |
| `entry`        | Yes      | Path to main CDM file (relative to manifest)          |
| `exports`      | No       | Named export paths for selective importing            |

Template dependencies are resolved from the `import` and `extends` statements in the CDM files themselves, not declared in the manifest. This keeps the manifest simple and avoids duplication.

### 6.3 Subpath Exports

The `exports` field allows templates to expose multiple entry points. Consumers can import from specific subpaths:

```cdm
// Import everything from the main entry
import auth from cdm/auth

// Import only types (if template exports "./types")
import authTypes from cdm/auth/types

// Import only models (if template exports "./models")
import authModels from cdm/auth/models
```

This is useful when:
- You only need type aliases, not models
- You want to avoid importing unused definitions
- The template organizes code into logical submodules

### 6.4 Entry File

The entry file (`index.cdm`) defines what the template exports:

```cdm
// index.cdm for sql/postgres-types

// Type aliases for SQL types
UUID: string {
  @sql { type: "UUID" }
} #1

Text: string {
  @sql { type: "TEXT" }
} #2

Varchar: string {
  @sql { type: "VARCHAR" }
} #3

SmallInt: number {
  @sql { type: "SMALLINT" }
} #4

Integer: number {
  @sql { type: "INTEGER" }
} #5

BigInt: number {
  @sql { type: "BIGINT" }
} #6

Timestamp: string {
  @sql { type: "TIMESTAMPTZ" }
} #7

Boolean: boolean {
  @sql { type: "BOOLEAN" }
} #8
```

### 6.5 Template with Models

```cdm
// index.cdm for cdm/auth

// Import SQL types for use in our models
import sql from sql/postgres-types

// Re-export commonly used types
UUID: sql.UUID #1

// Models
User {
  id: sql.UUID #1
  email: sql.Varchar { @sql { length: 255, unique: true } } #2
  password_hash: sql.Varchar { @sql { length: 255 } } #3
  created_at: sql.Timestamp #4
  updated_at: sql.Timestamp #5

  @sql { table: "users" }
} #10

Session {
  id: sql.UUID #1
  user: User #2
  token: sql.Varchar { @sql { length: 64, unique: true } } #3
  expires_at: sql.Timestamp #4
  created_at: sql.Timestamp #5

  @sql { table: "sessions" }
} #11

Role: "admin" | "user" | "guest" {
  @sql { type: "VARCHAR(20)" }
} #12

UserRole {
  user: User #1
  role: Role #2

  @sql {
    table: "user_roles",
    primary_key: ["user_id", "role"]
  }
} #13
```

---

## 7. Conflict Resolution

### 7.1 Namespace Conflicts

When two imports use the same namespace, it's an error:

```cdm
import sql from sql/postgres-types
import sql from sql/mysql-types  // Error: duplicate namespace 'sql'
```

**Solution:** Use different namespaces:

```cdm
import pg from sql/postgres-types
import mysql from sql/mysql-types
```

### 7.2 Merged Definition Conflicts

When extending multiple templates that define the same model or type alias, the **last template wins** (consistent with multiple inheritance):

```cdm
extends cdm/auth             // Defines User
extends myorg/extended-auth  // Also defines User - this one wins
```

You can always override explicitly in your schema:

```cdm
extends cdm/auth
extends myorg/extended-auth

// Explicit override takes precedence
User {
  // Your definition
}
```

### 7.3 Import vs Extend Conflicts

If the same template is both imported and extended, both are valid:

```cdm
extends cdm/auth             // Merge User, Session, Role into schema
import auth from cdm/auth    // Also available as auth.User, auth.Session

// Both work:
author: User #1        // From merged extends
author: auth.User #2   // From namespaced import (same thing)
```

This is redundant but not an error. The namespaced version references the same merged definition.

---

## 8. Dependency Resolution

### 8.1 Template Dependencies

Template dependencies are declared via `import` and `extends` statements in the CDM files, not in the manifest. When loading a template, CDM:

1. Parses the template's CDM files
2. Discovers `import` and `extends` statements
3. Recursively resolves all dependencies
4. Loads dependencies in topological order
5. Loads the template itself

### 8.2 Version Resolution

Version constraints follow semantic versioning and can be specified in the `import` or `extends` config block:

```cdm
import sql from sql/postgres-types { version: "^1.0.0" }
extends cdm/auth { version: "~2.1.0" }
```

| Constraint | Meaning                                        |
| ---------- | ---------------------------------------------- |
| `"1.0.0"`  | Exact version                                  |
| `"^1.0.0"` | Compatible with 1.x.x (>=1.0.0 <2.0.0)         |
| `"~1.0.0"` | Patch-level changes only (>=1.0.0 <1.1.0)      |
| `">=1.0.0"`| At least this version                          |
| `"*"`      | Any version (or omit version for latest)       |

**Note:** The `version` config key applies only to registry templates. For git templates, use `git_ref` instead:

```cdm
import custom from git:https://github.com/org/repo.git { git_ref: "main" }
import custom from git:https://github.com/org/repo.git { git_ref: "a1b2c3d" }
```

### 8.3 Version Conflicts

When different templates require incompatible versions of the same dependency:

```
cdm/auth requires sql/postgres-types ^1.0.0
cdm/billing requires sql/postgres-types ^2.0.0
```

CDM reports an error with the conflicting requirements. Resolution options:

1. Update one template to use a compatible version
2. Use template versions that have compatible dependencies
3. Contact template maintainers to update dependencies

---

## 9. Template Registry

### 9.1 Registry Format

The template registry is a JSON file similar to the plugin registry:

```json
{
  "version": 1,
  "updated_at": "2024-01-15T10:30:00Z",
  "templates": {
    "sql/postgres-types": {
      "description": "PostgreSQL type aliases for CDM",
      "repository": "git:https://github.com/cdm-lang/cdm-template-sql-postgres.git",
      "official": true,
      "versions": {
        "1.0.0": {
          "archive_url": "https://github.com/.../releases/download/v1.0.0/template.tar.gz",
          "checksum": "sha256:a1b2c3d4..."
        }
      },
      "latest": "1.0.0"
    },
    "cdm/auth": {
      "description": "Authentication system with User, Session, and Role models",
      "repository": "git:https://github.com/cdm-lang/cdm-template-auth.git",
      "official": true,
      "versions": {
        "2.1.0": {
          "archive_url": "https://github.com/.../releases/download/v2.1.0/template.tar.gz",
          "checksum": "sha256:e5f6g7h8..."
        }
      },
      "latest": "2.1.0"
    }
  }
}
```

### 9.2 Template Caching

Templates are cached in `.cdm/cache/templates/`:

```
.cdm/
└── cache/
    ├── plugins/           # WASM files
    └── templates/         # Template packages
        ├── sql/
        │   └── postgres-types/
        │       └── 1.0.0/
        │           ├── cdm-template.json
        │           └── index.cdm
        └── cdm/
            └── auth/
                └── 2.1.0/
                    ├── cdm-template.json
                    ├── index.cdm
                    └── models.cdm
```

---

## 10. CLI Commands

### 10.1 Template Commands

```
cdm template <subcommand>

Subcommands:
  list              List available templates from registry
  info <name>       Show template details
  cache <name>      Pre-download a template
  clear-cache       Clear template cache
```

### 10.2 Examples

```bash
# List registry templates
cdm template list

# Show template info
cdm template info cdm/auth
cdm template info cdm/auth --versions

# Cache templates for offline use
cdm template cache sql/postgres-types
cdm template cache --all  # Cache all templates used in project

# Clear cache
cdm template clear-cache
cdm template clear-cache cdm/auth
```

---

## 11. Error Catalog

### 11.1 Template Errors

| Code | Message                                            | Description                                      |
| ---- | -------------------------------------------------- | ------------------------------------------------ |
| E601 | Template not found: '{name}'                       | Could not resolve template from any source       |
| E602 | Invalid template manifest: {details}               | Template manifest is malformed                   |
| E603 | Template entry file not found: '{path}'            | Entry file specified in manifest doesn't exist   |
| E604 | Circular template dependency: {chain}              | Templates depend on each other circularly        |
| E605 | Duplicate namespace '{name}'                       | Two imports use the same namespace               |
| E606 | Unknown namespace '{name}'                         | Reference to namespace that wasn't imported      |
| E607 | Template version conflict: {details}               | Incompatible version requirements                |
| E608 | Template requires plugin '{name}' not imported     | Template needs a plugin that isn't imported      |

### 11.2 Warnings

| Code | Message                                            | Description                                      |
| ---- | -------------------------------------------------- | ------------------------------------------------ |
| W101 | Template '{name}' imported but never used          | Namespace imported but no references to it       |
| W102 | Template plugin requirement not satisfied          | Template wants a plugin that isn't imported      |

---

## 12. Examples

### 12.1 Using SQL Types

```cdm
import sql from sql/postgres-types

@sql { dialect: "postgres", build_output: "./db" }

User {
  id: sql.UUID #1
  email: sql.Varchar { @sql { length: 255 } } #2
  bio: sql.Text #3
  age: sql.SmallInt #4
  balance: sql.BigInt #5
  created_at: sql.Timestamp #6
  is_active: sql.Boolean #7

  @sql {
    table: "users",
    indexes: [{ fields: ["email"], unique: true }]
  }
} #10
```

### 12.2 Extending an Auth Template

```cdm
extends cdm/auth { version: "^2.1.0" }
import sql from sql/postgres-types

@sql { dialect: "postgres", build_output: "./db" }

// Extend the inherited User model
User {
  -password_hash            // Remove for API context
  avatar_url: sql.Varchar { @sql { length: 500 } } #100
  display_name: sql.Varchar { @sql { length: 100 } } #101
}

// Add our own models that reference auth models
Post {
  id: sql.UUID #1
  author: User #2           // References extended User
  title: sql.Varchar { @sql { length: 200 } } #3
  content: sql.Text #4
  created_at: sql.Timestamp #5

  @sql { table: "posts" }
} #20
```

### 12.3 Multi-Tenant SaaS Starter

```cdm
extends cdm/multi-tenant { version: "^1.0.0" }
extends cdm/auth { version: "^2.1.0" }
import sql from sql/postgres-types

@sql { dialect: "postgres", build_output: "./db" }

// Both templates provide models, we can use and extend them
Organization {
  // Add billing fields to the multi-tenant Organization
  stripe_customer_id: sql.Varchar { @sql { length: 255 } } #50
  plan: "free" | "pro" | "enterprise" = "free" #51
}

User {
  // Add organization relationship to auth User
  organization: Organization #100
}

// Our domain models
Project {
  id: sql.UUID #1
  organization: Organization #2
  name: sql.Varchar { @sql { length: 100 } } #3
  created_by: User #4
  created_at: sql.Timestamp #5

  @sql { table: "projects" }
} #30
```

### 12.4 Creating a Custom Template

**Directory structure:**

```
my-company-schemas/
├── cdm-template.json
├── index.cdm
├── types.cdm
└── models/
    ├── audit.cdm
    └── common.cdm
```

**cdm-template.json:**

```json
{
  "name": "mycompany/base",
  "version": "1.0.0",
  "description": "Base schema for MyCompany projects",
  "entry": "./index.cdm"
}
```

**index.cdm:**

```cdm
import sql from sql/postgres-types

// Re-export SQL types we commonly use
UUID: sql.UUID #1
Timestamp: sql.Timestamp #2
Text: sql.Text #3

// Common base model with audit fields
Auditable {
  created_at: Timestamp #1
  updated_at: Timestamp #2
  created_by: string #3
  updated_by: string #4
} #10

// Standard ID + audit fields
BaseModel extends Auditable {
  id: UUID #5
} #11
```

**Using the custom template:**

```cdm
extends mycompany/base
import sql from sql/postgres-types

@sql { dialect: "postgres", build_output: "./db" }

// All our models extend BaseModel, getting ID and audit fields
User extends BaseModel {
  email: sql.Varchar { @sql { length: 255 } } #10
  name: sql.Varchar { @sql { length: 100 } } #11

  @sql { table: "users" }
} #20

Post extends BaseModel {
  author: User #10
  title: sql.Varchar { @sql { length: 200 } } #11
  content: sql.Text #12

  @sql { table: "posts" }
} #21
```

---

## 13. Migration Considerations

### 13.1 Template Updates

When a template version changes, consuming schemas may need migration:

1. **Additive changes** (new types/models): Generally safe, no migration needed
2. **Field additions to template models**: May require database migration if extended
3. **Breaking changes**: May require schema updates in consuming projects

### 13.2 Entity ID Stability

Templates should use stable entity IDs to enable reliable migrations in consuming projects:

- Template type aliases and models should have IDs
- When consumers extend template models and add fields, those fields get their own IDs
- Template updates that preserve IDs enable clean migrations

### 13.3 Version Pinning

Production schemas should pin template versions:

```cdm
extends cdm/auth { version: "2.1.0" }  // Pinned to exact version
extends cdm/auth { version: "^2.1.0" } // Pinned to compatible range
```

Rather than using unpinned (latest):

```cdm
extends cdm/auth  // Uses latest - risky for production
```

---

## 14. Entity ID Collision Prevention

### 14.1 The Problem

When multiple templates use the same numeric entity IDs independently, extending both into a single schema would cause collisions:

```cdm
// cdm/auth template defines:
User { id: string #1 } #10

// cdm/billing template defines:
Invoice { id: string #1 } #10

// Consumer schema:
extends cdm/auth
extends cdm/billing
// Without collision prevention: #10 refers to both User and Invoice!
```

### 14.2 Composite Entity IDs

Entity IDs are composite values consisting of a **source** and a **local ID**:

```
EntityId = (Source, LocalId)
```

Two entity IDs only collide if they have the **same source AND same local ID**. IDs from different sources never collide.

### 14.3 Entity ID Sources

| Source Type | Description | Identity |
|-------------|-------------|----------|
| `Local` | Definitions in the current schema (including `extends` file inheritance) | N/A |
| `Registry` | From a registry template | Registry name (e.g., `cdm/auth`) |
| `Git` | From a git template | URL + optional path |
| `LocalTemplate` | From a local filesystem template (with `cdm-template.json`) | Path relative to project root |

**Important:** Files without a `cdm-template.json` manifest are NOT templates. They are treated as part of the local schema when used with `extends` file inheritance.

### 14.4 Source Identity Rules

- **Registry templates**: Identity is the registry name (guaranteed unique by registry)
- **Git templates**: Identity is the git URL plus optional `git_path` for monorepos
- **Local templates**: Identity is the canonicalized path relative to project root (NOT the manifest `name` field, to avoid collisions between unrelated templates)
- **Version is NOT part of identity**: Template IDs remain stable across version upgrades

### 14.5 Field ID Ownership

Field entity IDs belong to the model where they are defined:

```cdm
extends cdm/auth
// cdm/auth defines: User { id: string #1, email: string #2 } #10

User {
  avatar: string #100  // This field ID is Local, not cdm/auth
}
```

Result:
- `User.id #1` → Source: `cdm/auth`
- `User.email #2` → Source: `cdm/auth`
- `User.avatar #100` → Source: `Local`

### 14.6 Re-exports

Templates may re-export types from other templates. Re-exported definitions get **new entity IDs** in the re-exporting template's namespace:

```cdm
// cdm/auth/index.cdm
import sql from sql/postgres-types

// Re-export creates a NEW type alias with cdm/auth as source
UUID: sql.UUID #1  // This #1 belongs to cdm/auth, not sql/postgres-types
```

### 14.7 Constraints

| Constraint | Behavior |
|------------|----------|
| Circular template dependencies | **Error** (E604) |
| Duplicate `extends` of same template | **Error** |
| `import` + `extends` same template | Allowed (redundant but valid) |
| Redefining a template's model | Must use `-ModelName` to remove first |

### 14.8 Collision Detection

Entity ID validation checks for collisions **within the same source**:

```cdm
// These DON'T collide (different sources):
extends cdm/auth      // User #10
extends cdm/billing   // Invoice #10
LocalModel { } #10    // Local #10

// These DO collide (same source - Local):
TypeA: string #1
TypeB: string #1  // Error E501: Duplicate entity ID #1
```

### 14.9 Serialization Format

Composite entity IDs are serialized in `previous_schema.json` for migration tracking:

```json
{
  "entity_id": {
    "type": "local",
    "local_id": 10
  }
}
```

```json
{
  "entity_id": {
    "type": "registry",
    "name": "cdm/auth",
    "local_id": 10
  }
}
```

```json
{
  "entity_id": {
    "type": "git",
    "url": "https://github.com/org/repo.git",
    "path": "packages/auth",
    "local_id": 10
  }
}
```

```json
{
  "entity_id": {
    "type": "local_template",
    "path": "templates/shared",
    "local_id": 10
  }
}
```

When `entity_id` is not specified in the CDM source, it is omitted from serialization.

### 14.10 Migration and Rename Detection

The migration system uses composite entity IDs to detect renames:

```cdm
// Previous schema:
User { name: string #1 } #10

// Current schema:
User { fullName: string #1 } #10  // Same composite ID
```

With composite IDs, the migration system correctly identifies this as a field rename (not delete + add), enabling:
- `RENAME COLUMN name TO fullName` instead of `DROP COLUMN` + `ADD COLUMN`
- `RENAME TABLE` instead of `DROP TABLE` + `CREATE TABLE`

### 14.11 Future Considerations

**Peer dependencies** are not currently supported. If future use cases require templates to share the same instance of a dependency (e.g., for type identity across template boundaries), peer dependency support may be added.

---

_End of Templates Specification_
