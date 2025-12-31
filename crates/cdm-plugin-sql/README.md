# CDM SQL Plugin

Generate SQL DDL schemas and migrations for PostgreSQL and SQLite from your CDM models.

## Overview

The SQL plugin transforms CDM models into SQL `CREATE TABLE` statements and generates migration files when your schema changes. It supports both PostgreSQL and SQLite dialects with dialect-specific features like JSONB, array types, and advanced indexing.

## Installation

The SQL plugin is available in the CDM registry:

```cdm
@sql {
  dialect: "postgresql",
  build_output: "./db/schema",
  migrations_output: "./db/migrations"
}
```

## Quick Start

```cdm
@sql {
  dialect: "postgresql",
  schema: "public",
  build_output: "./db/schema",
  migrations_output: "./db/migrations"
}

User {
  id: string #1
  email: string #2
  name: string #3
  created_at: string #4

  @sql {
    table_name: "users",
    indexes: [
      { fields: ["id"], primary: true },
      { fields: ["email"], unique: true }
    ]
  }
} #10

Post {
  id: string #1
  author_id: string #2
  title: string #3
  content: string #4
  published: boolean = false #5

  @sql {
    table_name: "posts",
    indexes: [
      { fields: ["id"], primary: true },
      { fields: ["author_id"] }
    ]
  }
} #11
```

**Generated SQL (PostgreSQL):**

```sql
CREATE TABLE "users" (
  "id" VARCHAR(255) NOT NULL,
  "email" VARCHAR(255) NOT NULL,
  "name" VARCHAR(255) NOT NULL,
  "created_at" VARCHAR(255) NOT NULL,
  PRIMARY KEY ("id"),
  UNIQUE ("email")
);

CREATE TABLE "posts" (
  "id" VARCHAR(255) NOT NULL,
  "author_id" VARCHAR(255) NOT NULL,
  "title" VARCHAR(255) NOT NULL,
  "content" VARCHAR(255) NOT NULL,
  "published" BOOLEAN NOT NULL DEFAULT FALSE,
  PRIMARY KEY ("id")
);

CREATE INDEX "idx_posts_1" ON "posts" ("author_id");
```

## Global Settings

Configure the plugin at import time to set defaults for all tables.

### `dialect`

- **Type:** `"postgresql" | "sqlite"`
- **Default:** `"postgresql"`
- **Description:** Database dialect to generate SQL for.

```cdm
@sql { dialect: "postgresql" }
@sql { dialect: "sqlite" }
```

### `schema`

- **Type:** `string` (optional)
- **PostgreSQL only**
- **Description:** Database schema/namespace for all tables.

```cdm
@sql {
  dialect: "postgresql",
  schema: "public"
}
```

Generates: `CREATE TABLE "public"."users" (...)`

### `table_name_format`

- **Type:** `"snake_case" | "preserve" | "camel_case" | "pascal_case"`
- **Default:** `"snake_case"`
- **Description:** How to format table names from model names.

```cdm
@sql { table_name_format: "snake_case" }

UserProfile {} #1  // → user_profile
```

### `column_name_format`

- **Type:** `"snake_case" | "preserve" | "camel_case" | "pascal_case"`
- **Default:** `"snake_case"`
- **Description:** How to format column names from field names.

```cdm
@sql { column_name_format: "snake_case" }

User {
  firstName: string #1  // → first_name
} #10
```

### `pluralize_table_names`

- **Type:** `boolean`
- **Default:** `true`
- **Description:** Automatically pluralize table names.

```cdm
@sql { pluralize_table_names: true }

User {} #1   // → users
Post {} #2   // → posts
```

### `default_string_length`

- **Type:** `number`
- **Default:** `255`
- **Description:** Default VARCHAR length for string types (PostgreSQL only; SQLite uses TEXT).

```cdm
@sql { default_string_length: 500 }

User {
  name: string #1  // → VARCHAR(500) for PostgreSQL
} #10
```

### `number_type`

- **Type:** `"real" | "double" | "numeric"`
- **Default:** `"double"`
- **Description:** SQL type for CDM `number` type.

| Option     | PostgreSQL         | SQLite  |
| ---------- | ------------------ | ------- |
| `"real"`   | `REAL`             | `REAL`  |
| `"double"` | `DOUBLE PRECISION` | `REAL`  |
| `"numeric"`| `NUMERIC`          | `NUMERIC` |

```cdm
@sql { number_type: "numeric" }

Product {
  price: number #1  // → NUMERIC
} #10
```

### `infer_not_null`

- **Type:** `boolean`
- **Default:** `true`
- **Description:** Automatically add `NOT NULL` constraints to required fields (fields without `?`).

```cdm
@sql { infer_not_null: true }

User {
  name: string #1      // → NOT NULL
  nickname?: string #2 // → nullable
} #10
```

### `apply_cdm_defaults`

- **Type:** `boolean`
- **Default:** `true`
- **Description:** Apply CDM default values as SQL `DEFAULT` constraints.

```cdm
@sql { apply_cdm_defaults: true }

Settings {
  theme: string = "dark" #1  // → DEFAULT 'dark'
  max_items: number = 100 #2 // → DEFAULT 100
} #10
```

## Type Alias Settings

Configure SQL generation for specific type aliases.

### `type`

- **Type:** `string` (optional)
- **Description:** Override the SQL type for this type alias.

```cdm
Email: string {
  @sql { type: "VARCHAR(320)" }
} #1

UUID: string {
  @sql { type: "UUID" }
} #2

User {
  id: UUID #1
  email: Email #2
} #10
```

**Generated:**
```sql
CREATE TABLE "users" (
  "id" UUID NOT NULL,
  "email" VARCHAR(320) NOT NULL,
  ...
);
```

### `default`

- **Type:** `string` (optional)
- **Description:** SQL expression for default value (e.g., function calls).

```cdm
UUID: string {
  @sql {
    type: "UUID",
    default: "gen_random_uuid()"
  }
} #1

Timestamp: string {
  @sql {
    type: "TIMESTAMP",
    default: "CURRENT_TIMESTAMP"
  }
} #2
```

### `comment`

- **Type:** `string` (optional)
- **Description:** Documentation comment for the type alias (not currently used in SQL output).

## Model Settings

Configure SQL generation for specific models.

### `table_name`

- **Type:** `string` (optional)
- **Description:** Override the table name for this model.

```cdm
User {
  id: string #1
  @sql { table_name: "app_users" }
} #10
```

Generates: `CREATE TABLE "app_users" (...)`

### `schema`

- **Type:** `string` (optional)
- **PostgreSQL only**
- **Description:** Override the schema/namespace for this model.

```cdm
@sql { schema: "public" }

AdminUser {
  id: string #1
  @sql { schema: "admin" }
} #10
```

Generates: `CREATE TABLE "admin"."admin_users" (...)`

### `indexes`

- **Type:** `Index[]` (optional)
- **Description:** Define indexes, primary keys, and unique constraints.

```cdm
User {
  id: string #1
  email: string #2
  created_at: string #3

  @sql {
    indexes: [
      { fields: ["id"], primary: true },
      { fields: ["email"], unique: true },
      { fields: ["created_at"], method: "btree" },
      { fields: ["email"], where: "deleted_at IS NULL", name: "active_users_email" }
    ]
  }
} #10
```

**Index Options:**

| Field      | Type                                      | Description                              |
| ---------- | ----------------------------------------- | ---------------------------------------- |
| `fields`   | `string[]` (required)                     | Field names to index                     |
| `primary`  | `boolean` (optional)                      | Mark as PRIMARY KEY                      |
| `unique`   | `boolean` (optional)                      | Mark as UNIQUE constraint                |
| `method`   | `"btree" \| "hash" \| "gin" \| "gist" \| "spgist" \| "brin"` | Index method (PostgreSQL only)           |
| `name`     | `string` (optional)                       | Custom index name (auto-generated if omitted) |
| `where`    | `string` (optional)                       | Partial index condition (PostgreSQL only) |

**Primary Keys:**
- Only one primary key allowed per table
- Can be composite: `{ fields: ["org_id", "user_id"], primary: true }`

**Unique Constraints:**
- Can have multiple unique constraints
- Can be composite: `{ fields: ["email", "tenant_id"], unique: true }`

**Regular Indexes:**
- Omit both `primary` and `unique` for regular indexes
- Auto-generated name format: `idx_{table}_{index_number}`

### `constraints`

- **Type:** `Constraint[]` (optional)
- **Description:** Define advanced constraints (CHECK, EXCLUDE, custom).

```cdm
Product {
  id: string #1
  price: number #2
  sale_price?: number #3

  @sql {
    constraints: [
      {
        type: "check",
        fields: ["price"],
        expression: "price > 0",
        name: "positive_price"
      },
      {
        type: "check",
        fields: ["sale_price", "price"],
        expression: "sale_price < price"
      }
    ]
  }
} #10
```

**Constraint Types:**

| Type          | Description                              | Required Fields      |
| ------------- | ---------------------------------------- | -------------------- |
| `not_null`    | NOT NULL constraint                      | `fields`             |
| `null`        | NULL constraint                          | `fields`             |
| `default`     | DEFAULT constraint                       | `fields`, `expression` |
| `check`       | CHECK constraint                         | `fields`, `expression` |
| `foreign_key` | FOREIGN KEY constraint                   | `fields`, `reference`  |
| `exclude`     | EXCLUDE constraint (PostgreSQL only)     | `fields`, `expression` |
| `custom`      | Custom constraint SQL                    | `fields`, `expression` |

**Constraint Options:**

| Field        | Type                | Description                  |
| ------------ | ------------------- | ---------------------------- |
| `type`       | `string` (required) | Constraint type              |
| `fields`     | `string[]` (required) | Fields involved              |
| `expression` | `string` (optional) | SQL expression               |
| `reference`  | `Reference` (optional) | Foreign key reference        |
| `name`       | `string` (optional) | Custom constraint name       |

### `skip`

- **Type:** `boolean` (optional)
- **Default:** `false`
- **Description:** Skip table generation for this model.

```cdm
InternalCache {
  key: string #1
  value: JSON #2
  @sql { skip: true }
} #10
```

This model will not generate a SQL table.

## Field Settings

Configure SQL generation for specific fields.

### `column_name`

- **Type:** `string` (optional)
- **Description:** Override the column name for this field.

```cdm
User {
  displayName: string {
    @sql { column_name: "display_name" }
  } #1
} #10
```

### `type`

- **Type:** `string` (optional)
- **Description:** Override the SQL type for this field.

```cdm
Post {
  content: string {
    @sql { type: "TEXT" }
  } #1

  metadata: JSON {
    @sql { type: "JSONB" }  // PostgreSQL only
  } #2

  tags: string[] {
    @sql { type: "TEXT[]" }  // PostgreSQL only
  } #3
} #10
```

### `references`

- **Type:** `Reference` (optional)
- **Description:** Define a foreign key reference.

```cdm
Post {
  author_id: string {
    @sql {
      references: {
        table: "User",
        column: "id",
        on_delete: "cascade"
      }
    }
  } #1
} #10
```

**Reference Options:**

| Field       | Type                                          | Description                   |
| ----------- | --------------------------------------------- | ----------------------------- |
| `table`     | `string` (required)                           | Referenced model name         |
| `column`    | `string` (optional, default: `"id"`)          | Referenced column name        |
| `on_delete` | `"cascade" \| "set_null" \| "restrict" \| "no_action" \| "set_default"` | ON DELETE action              |
| `on_update` | `"cascade" \| "set_null" \| "restrict" \| "no_action" \| "set_default"` | ON UPDATE action              |

### `relationship`

- **Type:** `Relationship` (optional)
- **Description:** Document relationship type (for future ORM generation).

```cdm
User {
  posts: Post[] {
    @sql {
      relationship: {
        type: "one_to_many",
        foreign_key: "author_id"
      }
    }
  } #1
} #10

Student {
  courses: Course[] {
    @sql {
      relationship: {
        type: "many_to_many",
        through: "enrollments"
      }
    }
  } #1
} #11
```

**Relationship Options:**

| Field         | Type                                 | Description                  |
| ------------- | ------------------------------------ | ---------------------------- |
| `type`        | `"one_to_one" \| "one_to_many" \| "many_to_many"` (required) | Relationship type            |
| `through`     | `string` (required for many_to_many) | Junction table name          |
| `foreign_key` | `string` (optional)                  | Foreign key column override  |
| `required`    | `boolean` (optional)                 | Require relationship (NOT NULL) |

### `comment`

- **Type:** `string` (optional)
- **Description:** Documentation comment for the field (not currently used in SQL output).

### `skip`

- **Type:** `boolean` (optional)
- **Default:** `false`
- **Description:** Skip column generation for this field.

```cdm
User {
  computed_field: string {
    @sql { skip: true }
  } #1
} #10
```

This field will not generate a SQL column.

## Type Mapping

### CDM to SQL Type Mapping

| CDM Type     | PostgreSQL (default)         | SQLite   |
| ------------ | ---------------------------- | -------- |
| `string`     | `VARCHAR(255)`               | `TEXT`   |
| `number`     | `DOUBLE PRECISION`           | `REAL`   |
| `boolean`    | `BOOLEAN`                    | `INTEGER` (0/1) |
| `JSON`       | `JSONB`                      | `TEXT`   |
| `string[]`   | `VARCHAR(255)[]`             | `TEXT`   |
| `Model`      | `JSONB`                      | `TEXT`   |
| `"a" \| "b"` | `VARCHAR(255)`               | `TEXT`   |

**Custom type overrides:**

```cdm
@sql {
  default_string_length: 500,  // VARCHAR(500)
  number_type: "numeric"       // NUMERIC instead of DOUBLE PRECISION
}
```

## Migrations

The SQL plugin generates migration files when your schema changes.

### Usage

```bash
cdm migrate base.cdm
```

This generates:
- `001_migration.up.postgres.sql` (or `.sqlite.sql`)
- `001_migration.down.postgres.sql` (or `.sqlite.sql`)

### Supported Changes

| Change                  | PostgreSQL                        | SQLite                        |
| ----------------------- | --------------------------------- | ----------------------------- |
| Add table               | `CREATE TABLE`                    | `CREATE TABLE`                |
| Remove table            | `DROP TABLE`                      | `DROP TABLE`                  |
| Rename table            | `ALTER TABLE ... RENAME TO`       | `ALTER TABLE ... RENAME TO`   |
| Add column              | `ALTER TABLE ... ADD COLUMN`      | `ALTER TABLE ... ADD COLUMN`  |
| Remove column           | `ALTER TABLE ... DROP COLUMN`     | `ALTER TABLE ... DROP COLUMN` |
| Rename column           | `ALTER TABLE ... RENAME COLUMN`   | `ALTER TABLE ... RENAME COLUMN` (3.25.0+) |
| Change column type      | `ALTER TABLE ... ALTER COLUMN TYPE` | Manual migration required     |
| Change optionality      | `ALTER COLUMN SET/DROP NOT NULL`  | Manual migration required     |
| Change default          | `ALTER COLUMN SET/DROP DEFAULT`   | Manual migration required     |

### SQLite Limitations

SQLite has limited `ALTER TABLE` support. The following changes require manual migration:

- Changing column types
- Changing NOT NULL constraints
- Changing DEFAULT values

The plugin will generate comments in migration files indicating manual intervention is needed.

### Entity IDs for Reliable Renames

Use entity IDs to ensure renames are detected correctly:

```cdm
User {
  name: string #1
} #10

// After rename - ID #1 proves this is a rename, not remove+add
User {
  displayName: string #1  // Renamed from 'name'
} #10
```

Without IDs, the migration system cannot distinguish between:
- Renaming `name` to `displayName` (preserves data)
- Removing `name` and adding `displayName` (loses data)

## Examples

### Complete E-Commerce Schema

```cdm
@sql {
  dialect: "postgresql",
  schema: "ecommerce",
  pluralize_table_names: true,
  build_output: "./db/schema",
  migrations_output: "./db/migrations"
}

UUID: string {
  @sql {
    type: "UUID",
    default: "gen_random_uuid()"
  }
} #1

Timestamp: string {
  @sql {
    type: "TIMESTAMP",
    default: "CURRENT_TIMESTAMP"
  }
} #2

Email: string {
  @sql { type: "VARCHAR(320)" }
} #3

User {
  id: UUID #1
  email: Email #2
  name: string #3
  created_at: Timestamp #4
  updated_at: Timestamp #5

  @sql {
    indexes: [
      { fields: ["id"], primary: true },
      { fields: ["email"], unique: true },
      { fields: ["created_at"] }
    ]
  }
} #10

Product {
  id: UUID #1
  name: string #2
  description: string #3
  price: number #4
  sale_price?: number #5
  stock: number #6
  created_at: Timestamp #7

  @sql {
    indexes: [
      { fields: ["id"], primary: true },
      { fields: ["name"], method: "gin" }
    ],
    constraints: [
      { type: "check", fields: ["price"], expression: "price >= 0" },
      { type: "check", fields: ["stock"], expression: "stock >= 0" },
      { type: "check", fields: ["sale_price", "price"], expression: "sale_price IS NULL OR sale_price < price" }
    ]
  }
} #11

Order {
  id: UUID #1
  user_id: UUID #2
  status: string = "pending" #3
  total: number #4
  created_at: Timestamp #5

  @sql {
    indexes: [
      { fields: ["id"], primary: true },
      { fields: ["user_id"] },
      { fields: ["status"] },
      { fields: ["created_at"] }
    ]
  }
} #12

OrderItem {
  id: UUID #1
  order_id: UUID #2
  product_id: UUID #3
  quantity: number #4
  price: number #5

  @sql {
    indexes: [
      { fields: ["id"], primary: true },
      { fields: ["order_id"] },
      { fields: ["product_id"] }
    ],
    constraints: [
      { type: "check", fields: ["quantity"], expression: "quantity > 0" },
      { type: "check", fields: ["price"], expression: "price >= 0" }
    ]
  }
} #13
```

### Multi-Tenant Schema

```cdm
@sql {
  dialect: "postgresql",
  schema: "saas",
  build_output: "./db/schema",
  migrations_output: "./db/migrations"
}

Tenant {
  id: string #1
  name: string #2
  subdomain: string #3

  @sql {
    indexes: [
      { fields: ["id"], primary: true },
      { fields: ["subdomain"], unique: true }
    ]
  }
} #10

User {
  id: string #1
  tenant_id: string #2
  email: string #3
  name: string #4

  @sql {
    indexes: [
      { fields: ["id"], primary: true },
      { fields: ["email", "tenant_id"], unique: true },
      { fields: ["tenant_id"] }
    ]
  }
} #11

Document {
  id: string #1
  tenant_id: string #2
  owner_id: string #3
  title: string #4
  content: string #5

  @sql {
    indexes: [
      { fields: ["id"], primary: true },
      { fields: ["tenant_id"] },
      { fields: ["owner_id"] }
    ]
  }
} #12
```

## CLI Commands

```bash
# Validate SQL configuration
cdm validate base.cdm

# Generate schema files
cdm build base.cdm

# Generate migrations
cdm migrate base.cdm

# Generate with custom migration name
cdm migrate base.cdm --name "add_user_avatars"
```

## Output Files

### Schema Files

- **PostgreSQL:** `schema.postgres.sql`
- **SQLite:** `schema.sqlite.sql`

Location: Specified by `build_output` setting.

### Migration Files

- **PostgreSQL:**
  - `001_migration.up.postgres.sql`
  - `001_migration.down.postgres.sql`
- **SQLite:**
  - `001_migration.up.sqlite.sql`
  - `001_migration.down.sqlite.sql`

Location: Specified by `migrations_output` setting.

## Best Practices

1. **Use Entity IDs** for reliable rename detection in migrations
2. **Set `apply_cdm_defaults: false`** if you handle defaults at the application layer
3. **Use type aliases** for common types (UUID, Email, Timestamp) to ensure consistency
4. **Leverage `skip: true`** for computed fields or API-only fields
5. **Always specify primary keys** explicitly via indexes
6. **Use CHECK constraints** for data validation at the database level
7. **Consider dialect limitations** when targeting SQLite (limited ALTER TABLE support)
8. **Use partial indexes** (PostgreSQL) for efficient queries on filtered data
9. **Specify foreign keys** via `references` for referential integrity
10. **Document relationships** via `relationship` for future ORM generation

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

The compiled WASM file will be at `target/wasm32-wasip1/release/cdm_plugin_sql.wasm`

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
@sql from ./path/to/cdm-plugin-sql {
  dialect: "postgresql",
  build_output: "./generated"
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
