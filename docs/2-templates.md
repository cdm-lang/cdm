# Templates

Templates are reusable CDM schema packages that can be imported into any project. They enable sharing common patterns like authentication systems, type libraries, and organizational standards.

## Quick Start

```cdm
// Import SQL types under a namespace
import sql from "sql/postgres-types"

// Use namespaced types in your models
User {
  id: sql.UUID #1
  email: sql.Varchar #2
  created_at: sql.Timestamp #3
} #10
```

## Import vs Extends

CDM provides two ways to use templates:

### Namespaced Import (`import`)

Brings template definitions into scope under a namespace. Imported definitions are **not** included in your schema output - they're only available for type references.

```cdm
import sql from "sql/postgres-types"

User {
  id: sql.UUID #1           // Reference type from namespace
  name: sql.Varchar #2
}
```

### Merged Import (`extends`)

Merges template definitions directly into your schema. All models and types become part of your output, and you can modify them.

```cdm
extends "cdm/auth"

// User model from auth template is now in your schema
// You can extend or modify it
User {
  avatar_url: string #100   // Add new field
  -internal_notes           // Remove inherited field
}
```

### Combined Usage

Both can be used together:

```cdm
import sql from "sql/postgres-types"    // For type references
extends "cdm/auth"                       // Merge auth models

User {
  id: sql.UUID #100                   // Use SQL type
  avatar_url: string #101             // Extend merged model
}

Post {
  id: sql.UUID #1
  author: User #2                     // Reference merged User model
}
```

## Template Sources

### Registry Templates

Published templates from the CDM registry:

```cdm
import sql from "sql/postgres-types"
import auth from "cdm/auth"
```

With version constraints:

```cdm
import sql from "sql/postgres-types" { version: "^1.0.0" }
extends "cdm/auth" { version: "~2.1.0" }
```

Version constraint formats:
- `"1.0.0"` - Exact version
- `"^1.0.0"` - Compatible with 1.x.x (>=1.0.0 <2.0.0)
- `"~1.0.0"` - Patch updates only (>=1.0.0 <1.1.0)
- `">=1.0.0"` - At least this version

### Git Templates

Templates from git repositories:

```cdm
import custom from "git:https://github.com/org/cdm-types.git"
```

With git reference pinning:

```cdm
import custom from "git:https://github.com/org/cdm-types.git" {
  git_ref: "v2.0.0"
}
```

For monorepos, specify a subdirectory:

```cdm
import types from "git:https://github.com/org/monorepo.git" {
  git_ref: "main",
  git_path: "packages/cdm-types"
}
```

### Local Templates

Templates from the local filesystem:

```cdm
// Reference a template directory (requires cdm-template.json manifest)
import shared from "./templates/shared"
import common from "../common-schemas"

// Reference a CDM file directly (no manifest required)
import pg from "../templates/sql-types/postgres.cdm"
```

Paths are resolved relative to the importing file. Direct `.cdm` file references are useful for development or when you don't need the full template manifest structure.

## Namespace Access

Access namespaced types using dot notation:

```cdm
import sql from "sql/postgres-types"

User {
  id: sql.UUID #1
  name: sql.Varchar #2
  bio: sql.Text #3
  age: sql.SmallInt #4
}
```

### Field-Level Configuration

Add configuration when using namespaced types:

```cdm
import sql from "sql/postgres-types"

User {
  name: sql.Varchar {
    @sql { length: 100 }
  } #1

  id: sql.UUID {
    @sql { default: "uuid_generate_v4()" }
  } #2
}
```

## Creating Templates

### Directory Structure

```
my-template/
├── cdm-template.json     # Manifest (required)
├── index.cdm             # Main entry point (required)
├── types.cdm             # Additional files (optional)
└── README.md             # Documentation (optional)
```

### Manifest File

`cdm-template.json`:

```json
{
  "name": "myorg/common-types",
  "version": "1.0.0",
  "description": "Common type definitions for MyOrg projects",
  "entry": "./index.cdm"
}
```

### Entry File

`index.cdm`:

```cdm
import sql from "sql/postgres-types"

// Re-export commonly used types
UUID: sql.UUID #1
Timestamp: sql.Timestamp #2

// Define reusable models
Auditable {
  created_at: Timestamp #1
  updated_at: Timestamp #2
  created_by: string #3
} #10

BaseModel extends Auditable {
  id: UUID #5
} #11
```

## Best Practices

### Version Pinning

Pin versions in production schemas:

```cdm
// Good - pinned version
extends "cdm/auth" { version: "2.1.0" }

// Risky - uses latest
extends "cdm/auth"
```

### Namespace Naming

Choose clear, descriptive namespaces:

```cdm
// Good - clear what each namespace contains
import pg from "sql/postgres-types"
import mysql from "sql/mysql-types"
import auth from "cdm/auth"

// Bad - ambiguous
import t1 from "sql/postgres-types"
import t2 from "sql/mysql-types"
```

### Avoid Conflicts

Each namespace must be unique:

```cdm
// Error: duplicate namespace 'sql'
import sql from "sql/postgres-types"
import sql from "sql/mysql-types"

// Fixed: use different namespaces
import pg from "sql/postgres-types"
import mysql from "sql/mysql-types"
```

## Directive Ordering

All `extends`, `import`, and plugin directives must appear at the top of a file, before any definitions:

```cdm
// Directives first (order among them is flexible)
extends "./base.cdm"
extends "cdm/auth" { version: "^2.0.0" }
import sql from "sql/postgres-types"
@sql { dialect: "postgres" }
@typescript { build_output: "./src/types" }

// Then definitions
User {
  id: sql.UUID #1
}
```

## Example: Multi-Tenant SaaS

```cdm
extends "cdm/multi-tenant" { version: "^1.0.0" }
extends "cdm/auth" { version: "^2.1.0" }
import sql from "sql/postgres-types"

@sql { dialect: "postgres", build_output: "./db" }

// Extend inherited Organization with billing
Organization {
  stripe_customer_id: sql.Varchar { @sql { length: 255 } } #50
  plan: "free" | "pro" | "enterprise" = "free" #51
}

// Add organization to inherited User
User {
  organization: Organization #100
}

// Custom domain models
Project {
  id: sql.UUID #1
  organization: Organization #2
  name: sql.Varchar { @sql { length: 100 } } #3
  created_by: User #4

  @sql { table: "projects" }
} #30
```

## Related Documentation

- [Core Concepts](1-core-concepts.md) - CDM fundamentals
- [Context System](3-context-system.md) - Local file inheritance with `extends`
- [Plugins and Code Generation](4-plugins-and-code-generation.md) - Using plugins with templates
- [Templates Specification](../specs/templates.md) - Full technical specification
