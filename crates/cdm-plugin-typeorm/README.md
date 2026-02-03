# TypeORM Plugin

Generate TypeORM entities from CDM schemas.

## Installation

```bash
cdm plugin install typeorm
```

## Configuration

### Global Settings

Configure in your `cdm.json`:

```json
{
  "plugins": {
    "typeorm": {
      "entity_file_strategy": "per_model",
      "table_name_format": "snake_case",
      "column_name_format": "snake_case",
      "pluralize_table_names": true,
      "typeorm_import_path": "typeorm"
    }
  }
}
```

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `entity_file_strategy` | `"single"` \| `"per_model"` | `"per_model"` | Output one file per model or all in one file |
| `entities_file_name` | `string` | `"entities.ts"` | Filename when using `"single"` strategy |
| `table_name_format` | `"snake_case"` \| `"preserve"` \| `"camel_case"` \| `"pascal_case"` | `"snake_case"` | Table naming convention |
| `column_name_format` | `"snake_case"` \| `"preserve"` \| `"camel_case"` \| `"pascal_case"` | `"snake_case"` | Column naming convention |
| `pluralize_table_names` | `boolean` | `true` | Pluralize table names (User → users) |
| `typeorm_import_path` | `string` | `"typeorm"` | Custom TypeORM import path |
| `definite_assignment` | `boolean` | `true` | Add `!` to non-optional properties for TypeScript's `strictPropertyInitialization` |

## Field-Level Relations

The TypeORM plugin supports all four relation types through field configuration.

### ManyToOne Relation

A Post belongs to one User (author):

```cdm
Post {
  id: string {
    @typeorm { primary: { generation: "uuid" } }
  } #1
  title: string #2

  author: User {
    @typeorm {
      relation: {
        type: "many_to_one",
        inverse_side: "posts"
      },
      join_column: { name: "author_id" }
    }
  } #3
} #10
```

**Generated TypeScript:**

```typescript
import { Entity, Column, PrimaryGeneratedColumn, ManyToOne, JoinColumn } from "typeorm"
import { User } from "./User"

@Entity({ name: "posts" })
export class Post {
    @PrimaryGeneratedColumn("uuid")
    id!: string

    @Column()
    title!: string

    @ManyToOne(() => User, (user) => user.posts)
    @JoinColumn({ name: "author_id" })
    author!: User
}
```

### OneToMany Relation

A User has many Posts:

```cdm
User {
  id: string {
    @typeorm { primary: { generation: "uuid" } }
  } #1
  name: string #2

  posts: Post[] {
    @typeorm {
      relation: {
        type: "one_to_many",
        inverse_side: "author"
      }
    }
  } #3
} #10
```

**Generated TypeScript:**

```typescript
import { Entity, Column, PrimaryGeneratedColumn, OneToMany } from "typeorm"
import { Post } from "./Post"

@Entity({ name: "users" })
export class User {
    @PrimaryGeneratedColumn("uuid")
    id!: string

    @Column()
    name!: string

    @OneToMany(() => Post, (post) => post.author)
    posts!: Post[]
}
```

### OneToOne Relation

A User has one Profile:

```cdm
User {
  id: string {
    @typeorm { primary: { generation: "uuid" } }
  } #1

  profile: Profile {
    @typeorm {
      relation: {
        type: "one_to_one",
        inverse_side: "user",
        cascade: true
      },
      join_column: { name: "profile_id" }
    }
  } #2
} #10

Profile {
  id: string {
    @typeorm { primary: { generation: "uuid" } }
  } #1
  bio: string #2

  user: User {
    @typeorm {
      relation: {
        type: "one_to_one",
        inverse_side: "profile"
      }
    }
  } #3
} #11
```

### ManyToMany Relation

Posts have many Tags:

```cdm
Post {
  id: string {
    @typeorm { primary: { generation: "uuid" } }
  } #1
  title: string #2

  tags: Tag[] {
    @typeorm {
      relation: {
        type: "many_to_many",
        inverse_side: "posts"
      },
      join_table: {
        name: "post_tags",
        join_column: { name: "post_id" },
        inverse_join_column: { name: "tag_id" }
      }
    }
  } #3
} #10

Tag {
  id: string {
    @typeorm { primary: { generation: "uuid" } }
  } #1
  name: string #2

  posts: Post[] {
    @typeorm {
      relation: {
        type: "many_to_many",
        inverse_side: "tags"
      }
    }
  } #3
} #11
```

### Relation Options

All relation types support these options:

| Option | Type | Description |
|--------|------|-------------|
| `type` | `"one_to_one"` \| `"one_to_many"` \| `"many_to_one"` \| `"many_to_many"` | Required. Relation type |
| `inverse_side` | `string` | Property name on target entity for bidirectional relations |
| `cascade` | `boolean` | Enable cascade operations |
| `eager` | `boolean` | Automatically load relation |
| `lazy` | `boolean` | Return Promise for lazy loading |
| `nullable` | `boolean` | Allow null values |
| `on_delete` | `"CASCADE"` \| `"SET NULL"` \| `"RESTRICT"` \| `"NO ACTION"` \| `"DEFAULT"` | ON DELETE action |
| `on_update` | `"CASCADE"` \| `"SET NULL"` \| `"RESTRICT"` \| `"NO ACTION"` \| `"DEFAULT"` | ON UPDATE action |

### Join Configuration

**JoinColumn** (for ManyToOne, owning side of OneToOne):

```cdm
@typeorm {
  join_column: {
    name: "author_id",           // Foreign key column name
    referenced_column: "id"      // Column on target entity (default: "id")
  }
}
```

**JoinTable** (for ManyToMany):

```cdm
@typeorm {
  join_table: {
    name: "post_tags",                              // Junction table name
    join_column: { name: "post_id" },               // This entity's FK
    inverse_join_column: { name: "tag_id" }         // Target entity's FK
  }
}
```

## Entity Lifecycle Hooks

Define methods that execute at specific points in the entity lifecycle. Hooks can be defined in two ways:

### Simple Hooks (Stub Methods)

For hooks where you'll implement the logic directly in the generated entity:

```cdm
User {
  id: string {
    @typeorm { primary: { generation: "uuid" } }
  } #1
  created_at?: string #2

  @typeorm {
    hooks: {
      before_insert: "setCreatedAt"
    }
  }
} #10
```

**Generated TypeScript:**

```typescript
import { Entity, Column, PrimaryGeneratedColumn, BeforeInsert } from "typeorm"

@Entity({ name: "users" })
export class User {
    @PrimaryGeneratedColumn("uuid")
    id!: string

    @Column({ nullable: true })
    created_at?: string

    @BeforeInsert()
    setCreatedAt() {
        // Implementation required
    }
}
```

### Hooks with Imports

For hooks that delegate to external functions, specify both the method name and import path:

```cdm
User {
  id: string {
    @typeorm { primary: { generation: "uuid" } }
  } #1
  email: string #2
  password_hash: string #3

  @typeorm {
    hooks: {
      before_insert: {
        method: "hashPassword",
        import: "./hooks/userHooks"
      },
      after_load: {
        method: "initializeTransientFields",
        import: "./hooks/userHooks"
      }
    }
  }
} #10
```

**Generated TypeScript:**

```typescript
import { Entity, Column, PrimaryGeneratedColumn, BeforeInsert, AfterLoad } from "typeorm"
import { hashPassword, initializeTransientFields } from "./hooks/userHooks"

@Entity({ name: "users" })
export class User {
    @PrimaryGeneratedColumn("uuid")
    id!: string

    @Column()
    email!: string

    @Column()
    password_hash!: string

    @BeforeInsert()
    hashPassword() {
        hashPassword.call(this)
    }

    @AfterLoad()
    initializeTransientFields() {
        initializeTransientFields.call(this)
    }
}
```

**Example hook implementation** (`./hooks/userHooks.ts`):

```typescript
import { User } from "../User"
import * as bcrypt from "bcrypt"

export function hashPassword(this: User) {
    if (this.password_hash) {
        this.password_hash = bcrypt.hashSync(this.password_hash, 10)
    }
}

export function initializeTransientFields(this: User) {
    // Initialize computed properties, etc.
}
```

### Mixed Formats

You can mix both formats in the same entity:

```cdm
User {
  @typeorm {
    hooks: {
      before_insert: "simpleStub",
      after_load: {
        method: "computeFields",
        import: "./hooks/compute"
      }
    }
  }
}
```

### Available Hooks

| Hook | TypeORM Decorator | When Called |
|------|-------------------|-------------|
| `before_insert` | `@BeforeInsert()` | Before entity is inserted |
| `after_insert` | `@AfterInsert()` | After entity is inserted |
| `before_update` | `@BeforeUpdate()` | Before entity is updated |
| `after_update` | `@AfterUpdate()` | After entity is updated |
| `before_remove` | `@BeforeRemove()` | Before entity is removed |
| `after_remove` | `@AfterRemove()` | After entity is removed |
| `after_load` | `@AfterLoad()` | After entity is loaded from database |
| `before_soft_remove` | `@BeforeSoftRemove()` | Before soft removal |
| `after_soft_remove` | `@AfterSoftRemove()` | After soft removal |
| `after_recover` | `@AfterRecover()` | After entity is recovered |

### Hook Configuration

Each hook accepts either:

| Format | Description |
|--------|-------------|
| `string` | Method name only - generates a stub method |
| `{ method, import }` | Object with method name and import path - imports and delegates to external function |

## Model Settings

Configure at the model level:

```cdm
User {
  // fields...

  @typeorm {
    table: "app_users",           // Override table name
    schema: "public",             // PostgreSQL schema
    indexes: {
      user_email_unique: { fields: ["email"], unique: true },
      user_created_at_idx: { fields: ["created_at"] }
    },
    skip: false,                  // Skip entity generation
    definite_assignment: false    // Disable ! for this model
  }
} #10
```

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `table` | `string` | model name | Override table name |
| `schema` | `string` | - | PostgreSQL schema |
| `indexes` | `Index[string]` | - | Indexes on this model (keyed by index name for proper inheritance) |
| `skip` | `boolean` | `false` | Skip entity generation |
| `hooks` | `object` | - | Entity lifecycle hooks |
| `definite_assignment` | `boolean` | global setting | Override definite assignment for all fields in this model |

## Field Settings

Configure individual fields:

```cdm
User {
  email: string {
    @typeorm {
      column: "email_address",    // Override column name
      type: "varchar",            // SQL type override
      unique: true,
      nullable: false,
      length: 255,
      default: "'unknown'",       // SQL default expression
      comment: "User email",
      definite_assignment: false  // Disable ! for this field
    }
  } #1
} #10
```

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `column` | `string` | field name | Override column name |
| `type` | `string` | - | SQL type override |
| `ts_type` | `string` \| `object` | - | TypeScript type override |
| `primary` | `object` | - | Primary key configuration |
| `relation` | `object` | - | Relation configuration |
| `join_column` | `object` | - | Join column configuration |
| `join_table` | `object` | - | Join table configuration |
| `unique` | `boolean` | `false` | Column is unique |
| `default` | `string` | - | Default value (SQL expression) |
| `nullable` | `boolean` | CDM optional | Override nullability |
| `skip` | `boolean` | `false` | Skip this field |
| `length` | `number` | - | Column length |
| `array` | `boolean` | `false` | Array column |
| `comment` | `string` | - | Column comment |
| `definite_assignment` | `boolean` | model/global setting | Override definite assignment for this field |

## TypeScript Type Override (ts_type)

Override the generated TypeScript type for fields and type aliases. This is useful when you want to use custom types instead of the default mappings.

### String Format (Built-in Types)

For types that don't require imports:

```cdm
User {
  id: string {
    @typeorm { primary: { generation: "uuid" } }
  } #1

  metadata: JSON {
    @typeorm {
      ts_type: "Record<string, string>"
    }
  } #2
} #10
```

**Generated TypeScript:**

```typescript
@Entity({ name: "users" })
export class User {
    @PrimaryGeneratedColumn("uuid")
    id!: string

    @Column()
    metadata!: Record<string, string>
}
```

### Object Format (With Imports)

For custom types that need to be imported:

```cdm
User {
  id: string {
    @typeorm { primary: { generation: "uuid" } }
  } #1

  profile: JSON {
    @typeorm {
      ts_type: {
        type: "UserProfile",
        import: "./types/user"
      }
    }
  } #2
} #10
```

**Generated TypeScript:**

```typescript
import { Entity, Column, PrimaryGeneratedColumn } from "typeorm"
import { UserProfile } from "./types/user"

@Entity({ name: "users" })
export class User {
    @PrimaryGeneratedColumn("uuid")
    id!: string

    @Column()
    profile!: UserProfile
}
```

### Default Imports

For default exports, set `default: true`:

```cdm
User {
  id: string {
    @typeorm { primary: { generation: "uuid" } }
  } #1

  config: JSON {
    @typeorm {
      ts_type: {
        type: "AppConfig",
        import: "./config",
        default: true
      }
    }
  } #2
} #10
```

**Generated TypeScript:**

```typescript
import { Entity, Column, PrimaryGeneratedColumn } from "typeorm"
import AppConfig from "./config"

@Entity({ name: "users" })
export class User {
    @PrimaryGeneratedColumn("uuid")
    id!: string

    @Column()
    config!: AppConfig
}
```

### Type Alias Level ts_type

Apply `ts_type` to a type alias to affect all fields using that type:

```cdm
type Metadata = JSON {
  @typeorm {
    column_type: "jsonb",
    ts_type: {
      type: "MetadataType",
      import: "./types/metadata"
    }
  }
}

User {
  id: string {
    @typeorm { primary: { generation: "uuid" } }
  } #1

  // Uses MetadataType from type alias config
  metadata: Metadata #2
} #10
```

**Generated TypeScript:**

```typescript
import { Entity, Column, PrimaryGeneratedColumn } from "typeorm"
import { MetadataType } from "./types/metadata"

@Entity({ name: "users" })
export class User {
    @PrimaryGeneratedColumn("uuid")
    id!: string

    @Column()
    metadata!: MetadataType
}
```

### Precedence Rules

Field-level `ts_type` takes precedence over type alias-level `ts_type`:

```cdm
type Metadata = JSON {
  @typeorm {
    ts_type: {
      type: "DefaultMetadata",
      import: "./types/default"
    }
  }
}

User {
  id: string {
    @typeorm { primary: { generation: "uuid" } }
  } #1

  // Field-level overrides type alias
  metadata: Metadata {
    @typeorm {
      ts_type: {
        type: "UserMetadata",
        import: "./types/user"
      }
    }
  } #2
} #10
```

**Generated TypeScript uses `UserMetadata`, not `DefaultMetadata`:**

```typescript
import { Entity, Column, PrimaryGeneratedColumn } from "typeorm"
import { UserMetadata } from "./types/user"

@Entity({ name: "users" })
export class User {
    @PrimaryGeneratedColumn("uuid")
    id!: string

    @Column()
    metadata!: UserMetadata
}
```

### Import Grouping

Multiple imports from the same path are automatically grouped:

```cdm
User {
  id: string {
    @typeorm { primary: { generation: "uuid" } }
  } #1

  profile: JSON {
    @typeorm {
      ts_type: { type: "UserProfile", import: "./types" }
    }
  } #2

  settings: JSON {
    @typeorm {
      ts_type: { type: "UserSettings", import: "./types" }
    }
  } #3
} #10
```

**Generated TypeScript:**

```typescript
import { Entity, Column, PrimaryGeneratedColumn } from "typeorm"
import { UserProfile, UserSettings } from "./types"

@Entity({ name: "users" })
export class User {
    @PrimaryGeneratedColumn("uuid")
    id!: string

    @Column()
    profile!: UserProfile

    @Column()
    settings!: UserSettings
}
```

### ts_type Configuration Reference

| Format | Description |
|--------|-------------|
| `string` | Type name only (no import) |
| `{ type, import }` | Named import from path (`default: false` is implied) |
| `{ type, import, default: true }` | Default import from path |

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `type` | `string` | required | The TypeScript type name |
| `import` | `string` | required | Import path for the type |
| `default` | `boolean` | `false` | If true, generates default import; if false, generates named import |

## Primary Keys

Configure primary key generation:

```cdm
User {
  id: string {
    @typeorm {
      primary: {
        generation: "uuid"        // "uuid" | "increment" | "identity" | "rowid"
      }
    }
  } #1
} #10
```

For non-generated primary keys:

```cdm
User {
  id: string {
    @typeorm {
      primary: {}                 // No generation strategy = @PrimaryColumn
    }
  } #1
} #10
```

### Primary Key Type Override

You can override the SQL column type for primary keys using the `type` field setting:

```cdm
User {
  id: number {
    @typeorm {
      primary: { generation: "increment" },
      type: "bigint"
    }
  } #1
} #10
```

**Generated TypeScript:**

```typescript
@Entity({ name: "users" })
export class User {
    @PrimaryGeneratedColumn("increment", { type: "bigint" })
    id!: number
}
```

For non-generated primary keys:

```cdm
User {
  id: string {
    @typeorm {
      primary: {},
      type: "uuid"
    }
  } #1
} #10
```

**Generated TypeScript:**

```typescript
@Entity({ name: "users" })
export class User {
    @PrimaryColumn({ type: "uuid" })
    id!: string
}
```

## Definite Assignment Assertion

TypeScript's `strictPropertyInitialization` requires class properties to be initialized in the constructor or have a definite assignment assertion (`!`). Since TypeORM initializes entity properties via the ORM rather than constructors, this plugin adds `!` to non-optional properties by default.

### Default Behavior

```cdm
User {
  id: string {
    @typeorm { primary: { generation: "uuid" } }
  } #1
  email: string #2
  nickname?: string #3
} #10
```

**Generated TypeScript:**

```typescript
@Entity({ name: "users" })
export class User {
    @PrimaryGeneratedColumn("uuid")
    id!: string

    @Column()
    email!: string

    @Column({ nullable: true })
    nickname?: string
}
```

### Disabling Globally

```json
{
  "plugins": {
    "typeorm": {
      "definite_assignment": false
    }
  }
}
```

### Model-Level Override

```cdm
User {
  email: string #1

  @typeorm {
    definite_assignment: false
  }
} #10
```

### Field-Level Override

```cdm
User {
  email: string {
    @typeorm {
      definite_assignment: false
    }
  } #1
} #10
```

### Precedence

Field setting → Model setting → Global setting → `true` (default)
