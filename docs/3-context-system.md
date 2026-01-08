# 3. Context System

The **context system** is what allows CDM to model real-world systems without copying schemas.

A context represents a **specific view of a schema**—for example, a database schema, an API schema, or a client-facing schema—while sharing a common base definition.

Instead of duplicating models, contexts **extend and modify** existing schemas in a controlled, explicit way.

---

## 3.1 What Is a Context?

A context is a CDM file that **extends another CDM file** and applies changes to it.

```cdm
extends "./base.cdm"
```

A context can:

* Add new models and types
* Remove models and types
* Modify existing models
* Override type aliases
* Change plugin configuration

Contexts are resolved by applying changes **layer by layer**, starting from a base schema and moving outward.

---

## 3.2 Why Contexts Exist

Most systems need different representations of the same data:

* The database needs internal fields and indexes
* APIs should hide sensitive fields
* Clients need a simplified shape
* Admin tools may expose additional data

Without contexts, teams typically:

* Copy schemas into multiple files
* Manually keep them in sync
* Introduce subtle inconsistencies

Contexts solve this by making differences explicit and reviewable.

---

## 3.3 Creating a Context with `extends`

A context file begins with an `extends` directive:

```cdm
// api.cdm
extends "./base.cdm"
```

Rules:

* `extends` must appear at the top of the file
* Paths are resolved relative to the current file
* A context can extend another context, forming a chain
* Circular extends are not allowed

After the `extends` directive, the file may contain additions and modifications.

---

## 3.4 Modifying Models in a Context

To modify an existing model, redeclare it with a block.

```cdm
User {
  -password_hash
  avatar_url?: string
}
```

This does **not** redefine the model—it applies changes to the inherited definition.

### Common Modifications

Contexts can:

* Add fields
* Remove inherited fields
* Change defaults
* Add or override plugin configuration

All modifications are validated against the inherited schema.

---

## 3.5 Removing Fields

Fields inherited from a parent schema can be removed using the `-` prefix:

```cdm
User {
  -password_hash
  -salt
}
```

Rules:

* Only inherited fields may be removed
* Removing a field that does not exist is an error
* The field’s entity ID remains associated with the original definition

Field removal is commonly used to create API- or client-safe views of models.

---

## 3.6 Adding Fields in a Context

Contexts can introduce new fields:

```cdm
User {
  last_login_at?: string
}
```

New fields:

* Behave like normal fields
* Can have defaults and plugin config
* Should be assigned entity IDs if they may persist

These fields exist only in the context where they are defined.

---

## 3.7 Overriding Type Aliases

Type aliases can be overridden in a context to change behavior globally.

```cdm
// base.cdm
Email: string {
  @validation { format: "email", max_length: 320 }
}
```

```cdm
// api.cdm
extends "./base.cdm"

Email: string {
  @validation { format: "email" }
}
```

All fields referencing `Email` automatically use the overridden definition in the API context.

This is especially useful for:

* Adjusting validation rules
* Removing database-specific config
* Customizing client-side behavior

---

## 3.8 Adding and Removing Models or Types

Contexts can add entirely new definitions:

```cdm
ApiToken {
  value: string
  expires_at: string
}
```

They can also remove inherited models or types:

```cdm
-InternalAuditLog
```

Rules:

* A model or type cannot be removed if it is still referenced
* Removal applies only within the context
* Base schemas remain unchanged

---

## 3.9 Context Chains

Contexts can extend other contexts:

```cdm
// base.cdm
User {
  id: string
  email: string
  password_hash: string
}
```

```cdm
// api.cdm
extends "./base.cdm"

User {
  -password_hash
}
```

```cdm
// mobile.cdm
extends "./api.cdm"

User {
  device_token?: string
}
```

The final schema is built by applying changes in order:

1. `base.cdm`
2. `api.cdm`
3. `mobile.cdm`

Each layer only expresses what changes.

---

## 3.10 [Plugin](4-plugins-and-code-generation.md) Configuration in Contexts

[Plugin](4-plugins-and-code-generation.md) configuration merges across context boundaries.

```cdm
// base.cdm
@typescript {
  strict_nulls: true
}
```

```cdm
// api.cdm
extends "./base.cdm"

@typescript {
  strict_nulls: false
}
```

Merge rules:

* Objects are deep-merged
* Arrays replace entirely
* Primitive values override parent values

This allows context-specific output without duplicating configuration.

---

## 3.11 Validation and Safety Guarantees

CDM validates context usage aggressively:

* Removing referenced entities is disallowed
* Removing non-existent fields is an error
* Type references must resolve after all overrides
* Context chains must be acyclic

These checks ensure that every resolved context produces a valid, self-consistent schema.

---

## 3.12 When to Use Contexts (and When Not To)

Contexts are ideal when:

* Multiple consumers need different views of the same data
* You want to avoid schema duplication
* Changes must be explicit and reviewable

Avoid contexts when:

* Differences are purely stylistic
* A separate schema truly represents a different domain
* You’re trying to encode runtime behavior

Contexts model **structural differences**, not logic.

---

## What's Next?

With contexts understood, the next section explores how CDM turns schemas into real artifacts.

Proceed to **[Section 4: Plugins & Code Generation](4-plugins-and-code-generation.md)** to see how CDM generates SQL, TypeScript, validation logic, and more.
