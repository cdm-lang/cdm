# CDM TypeScript Plugin

Generate TypeScript types from CDM schemas.

## Overview

The TypeScript plugin converts CDM schema definitions into TypeScript code. It supports generating interfaces, classes, or type aliases with extensive customization options for file organization, naming conventions, and type handling.

## How It Works

The plugin processes your CDM schema and generates TypeScript output files based on your configuration:

1. **Schema Parsing** - Reads CDM models and type aliases from your schema
2. **Type Mapping** - Converts CDM types to their TypeScript equivalents
3. **Code Generation** - Generates TypeScript interfaces, classes, or type aliases
4. **File Organization** - Outputs to a single file or separate files per model

### Type Mapping

CDM types are mapped to TypeScript as follows:

| CDM Type | TypeScript Type |
|----------|----------------|
| `string` | `string` |
| `number` | `number` |
| `boolean` | `boolean` |
| `JSON` | `Record<string, unknown> \| unknown[]` (strict) or `any` (non-strict) |
| `Array<T>` | `T[]` |
| String literals | `"literal"` |
| Union types | `Type1 \| Type2 \| ...` |
| User-defined types | Pass through as-is |

## Configuration

### Global Settings

Configure the plugin at the global level in your CDM schema:

```cdm
@typescript {
  build_output: "./src/generated",
  output_format: "interface",
  file_strategy: "single",
  strict_nulls: true
}
```

#### `build_output`

**Type:** `string`
**Default:** `"."`

Specifies the directory where the generated TypeScript files will be saved. This setting is automatically available for all CDM plugins.

The path can be absolute or relative to your project root:

```cdm
@typescript {
  build_output: "./src/types"
}
```

When using `file_strategy: "single"`, the file specified by `single_file_name` will be created in this directory. When using `file_strategy: "per_model"`, all generated model files will be placed in this directory.

#### `output_format`

**Type:** `"interface" | "class" | "type"`
**Default:** `"interface"`

Determines how models are generated:

- **`"interface"`** - Generates TypeScript interfaces (default):
  ```typescript
  export interface User {
    id: string;
    name: string;
    email?: string;
  }
  ```

- **`"class"`** - Generates TypeScript classes with a constructor:
  ```typescript
  export class User {
    id: string;
    name: string;
    email?: string;

    constructor(data: Partial<User>) {
      Object.assign(this, data);
    }
  }
  ```

- **`"type"`** - Generates type aliases:
  ```typescript
  export type User = {
    id: string;
    name: string;
    email?: string;
  };
  ```

#### `file_strategy`

**Type:** `"single" | "per_model"`
**Default:** `"single"`

Controls how output files are organized:

- **`"single"`** - All types in one file (specified by `single_file_name`)
- **`"per_model"`** - Each model in a separate file, shared type aliases in `types.ts`

#### `single_file_name`

**Type:** `string`
**Default:** `"types.ts"`

The filename to use when `file_strategy` is `"single"`.

#### `optional_strategy`

**Type:** `"native" | "union_undefined"`
**Default:** `"native"`

How optional fields are represented:

- **`"native"`** - Uses TypeScript's `?` marker:
  ```typescript
  interface User {
    email?: string;
  }
  ```

- **`"union_undefined"`** - Uses union types without `?`:
  ```typescript
  interface User {
    email: string | undefined;
  }
  ```

#### `strict_nulls`

**Type:** `boolean`
**Default:** `true`

When `true`, the `JSON` type is mapped to `Record<string, unknown> | unknown[]`. When `false`, it's mapped to `any`.

#### `generate_zod`

**Type:** `boolean`
**Default:** `false`

When `true`, generates [Zod](https://zod.dev/) schemas alongside TypeScript types. Each model and type alias gets a corresponding schema (e.g., `User` gets `UserSchema`). The Zod import is automatically added when needed.

```cdm
@typescript {
  build_output: "./src/types",
  generate_zod: true
}
```

#### `export_all`

**Type:** `boolean`
**Default:** `true`

When `true`, all generated types are exported. When `false`, types are not exported.

#### `type_name_format`

**Type:** `"preserve" | "pascal" | "camel" | "snake" | "kebab" | "constant"`
**Default:** `"preserve"`

Formatting to apply to type and model names:

- **`"preserve"`** - Keep names as defined in the schema
- **`"pascal"`** - Convert to PascalCase (e.g., `UserProfile`)
- **`"camel"`** - Convert to camelCase (e.g., `userProfile`)
- **`"snake"`** - Convert to snake_case (e.g., `user_profile`)
- **`"kebab"`** - Convert to kebab-case (e.g., `user-profile`)
- **`"constant"`** - Convert to CONSTANT_CASE (e.g., `USER_PROFILE`)

#### `field_name_format`

**Type:** `"preserve" | "pascal" | "camel" | "snake" | "kebab" | "constant"`
**Default:** `"preserve"`

Formatting to apply to field names. Same options as `type_name_format`.

### Type Alias Settings

Configure individual type aliases:

```cdm
Email: string {
  @typescript {
    export_name: "EmailAddress",
    type_override: "string & { __brand: 'email' }"
  }
}
```

#### `type_override`

**Type:** `string`
**Default:** None

Override the generated TypeScript type completely. Useful for branded types or custom type definitions.

#### `export_name`

**Type:** `string`
**Default:** None

Customize the exported name for this type alias (overrides `type_name_format`).

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

  @typescript {
    output_format: "class",
    readonly: true,
    export_name: "UserModel"
  }
}
```

#### `output_format`

**Type:** `"interface" | "class" | "type"`
**Default:** Inherits from global setting

Override the output format for this specific model.

#### `file_name`

**Type:** `string`
**Default:** None

When using `file_strategy: "per_model"`, specify a custom filename for this model. Otherwise, the model name is used.

#### `export_name`

**Type:** `string`
**Default:** None

Customize the exported name for this model (overrides `type_name_format`).

#### `skip`

**Type:** `boolean`
**Default:** `false`

When `true`, this model will not be generated in the output.

#### `readonly`

**Type:** `boolean`
**Default:** `false`

When `true`, all fields in this model are marked as `readonly`.

#### `generate_zod`

**Type:** `boolean`
**Default:** Inherits from global setting

Override whether to generate a Zod schema for this specific model. Useful for enabling Zod on specific models when globally disabled, or excluding models when globally enabled.

```cdm
User {
  id: string
  name: string

  @typescript {
    generate_zod: true
  }
}
```

### Field Settings

Configure individual fields within models:

```cdm
User {
  id: string {
    @typescript {
      field_name: "userId",
      readonly: true
    }
  }

  created_at: string {
    @typescript {
      type_override: "Date"
    }
  }
}
```

#### `type_override`

**Type:** `string`
**Default:** None

Override the TypeScript type for this specific field.

#### `field_name`

**Type:** `string`
**Default:** None

Rename this field in the generated output (overrides `field_name_format`).

#### `readonly`

**Type:** `boolean`
**Default:** `false`

When `true`, this field is marked as `readonly`.

#### `skip`

**Type:** `boolean`
**Default:** `false`

When `true`, this field will not be included in the generated output.

## Examples

### Basic Usage

```cdm
@typescript {
  build_output: "./src/types",
  output_format: "interface",
  file_strategy: "single",
  single_file_name: "models.ts"
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

Generates `models.ts`:
```typescript
export type Email = string;

export type Status = "active" | "inactive" | "pending";

export interface User {
  id: string;
  name: string;
  email: Email;
  status: Status;
  age?: number;
}
```

### Using Classes

```cdm
@typescript {
  build_output: "./src/types",
  output_format: "class"
}

User {
  id: string
  name: string
  email?: string
}
```

Generates:
```typescript
export class User {
  id: string;
  name: string;
  email?: string;

  constructor(data: Partial<User>) {
    Object.assign(this, data);
  }
}
```

### Per-Model Files

```cdm
@typescript {
  build_output: "./src/types",
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
- `types.ts` - Contains the `Email` type alias
- `User.ts` - Contains the `User` interface with imports for referenced type aliases
- `Post.ts` - Contains the `Post` interface with imports for referenced models

When using `per_model` file strategy, the plugin automatically generates import statements for:
- **Model references** - When a model field references another model, an import is generated (e.g., `import { User } from "./User"`)
- **Type alias references** - When a model uses a type alias, an import from `types.ts` is generated (e.g., `import { Email } from "./types"`)

For example, `User.ts` would contain:
```typescript
import { Email } from "./types"

export interface User {
  id: string;
  email: Email;
}
```

And `Post.ts` would contain:
```typescript
import { User } from "./User"

export interface Post {
  id: string;
  title: string;
  author: User;
}
```

### Field Name Formatting

```cdm
@typescript {
  build_output: "./src/types",
  field_name_format: "camel"
}

User {
  user_id: string
  first_name: string
  last_name: string
  email_address: string
}
```

Generates:
```typescript
export interface User {
  userId: string;
  firstName: string;
  lastName: string;
  emailAddress: string;
}
```

### Field-Level Customization

```cdm
User {
  id: string

  name: string {
    @typescript { field_name: "fullName" }
  }

  created_at: string {
    @typescript { type_override: "Date" }
  }

  email: string {
    @typescript { readonly: true }
  }

  internal_field: string {
    @typescript { skip: true }
  }
}
```

Generates:
```typescript
export interface User {
  id: string;
  fullName: string;
  created_at: Date;
  readonly email: string;
}
```

### Readonly Models

```cdm
User {
  id: string
  name: string
  email: string

  @typescript { readonly: true }
}
```

Generates:
```typescript
export interface User {
  readonly id: string;
  readonly name: string;
  readonly email: string;
}
```

### Zod Schema Generation

```cdm
@typescript {
  build_output: "./src/types",
  generate_zod: true
}

Status: "active" | "inactive"

User {
  id: string
  name: string
  email?: string
  status: Status
}
```

Generates:
```typescript
import { z } from 'zod';

export type Status = "active" | "inactive";

export const StatusSchema = z.union([z.literal("active"), z.literal("inactive")]);

export interface User {
  id: string;
  name: string;
  email?: string;
  status: Status;
}

export const UserSchema: z.ZodType<User> = z.object({
  id: z.string(),
  name: z.string(),
  email: z.string().optional(),
  status: StatusSchema,
});
```

Zod schemas enable runtime validation of data:
```typescript
const data = await fetch('/api/user').then(r => r.json());
const user = UserSchema.parse(data); // Throws if invalid
```

#### Circular References

The plugin automatically handles circular references between models by using Zod's `z.lazy()` for deferred evaluation:

```cdm
User {
  id: string
  posts: Post[]
}

Post {
  id: string
  author: User
}
```

Generates schemas that properly handle the circular dependency:
```typescript
export const UserSchema: z.ZodType<User> = z.object({
  id: z.string(),
  posts: z.array(z.lazy(() => PostSchema)),
});

export const PostSchema: z.ZodType<Post> = z.object({
  id: z.string(),
  author: z.lazy(() => UserSchema),
});
```

This also works for self-referential types (e.g., tree structures where a `Node` has `children: Node[]`).

### Type Alias Configuration

```cdm
// Branded type example
Email: string {
  @typescript {
    type_override: "string & { readonly __brand: 'Email' }"
  }
}

// Custom export name
UserId: string {
  @typescript { export_name: "UserIdentifier" }
}

// Skip generation
InternalCode: string {
  @typescript { skip: true }
}
```

Generates:
```typescript
export type Email = string & { readonly __brand: 'Email' };

export type UserIdentifier = string;

// InternalCode is not generated
```

## Validation

The plugin validates configuration at build time:

- **TypeScript identifiers** - Ensures field and type names are valid TypeScript identifiers
- **Reserved keywords** - Warns if names conflict with TypeScript reserved keywords
- **Configuration values** - Validates enum values match allowed options
- **File extensions** - Ensures `single_file_name` has a `.ts` extension

## License

MPL-2.0
