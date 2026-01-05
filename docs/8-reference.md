# 8. Reference

This section provides authoritative reference material for CDM. It is intended for lookup and clarification, not as a learning guide.

Where possible, earlier sections ([Core Concepts](1-core-concepts.md), [Context System](3-context-system.md), [Plugins](4-plugins-and-code-generation.md)) explain *why* things work the way they do. This section documents *what* exists and *how it behaves*.

---

## 8.1 Language Reference

### Source Files

* CDM files use the `.cdm` extension
* Files must be UTF-8 encoded
* Schemas may consist of multiple files linked via [`@extends`](3-context-system.md#33-creating-a-context-with-extends)

---

### Top-Level Elements

A CDM file may contain, in order:

1. `@extends` directives (optional)
2. Plugin imports and configuration
3. Type alias definitions
4. Model definitions
5. Model or type removals (in contexts)

Plugin imports must appear before any type or model definitions.

---

### Identifiers

* Must begin with a letter or `_`
* May contain letters, digits, and `_`
* Are case-sensitive
* No reserved keywords (though shadowing built-ins is discouraged)

---

### Comments

* Single-line comments using `//`
* Inline comments are allowed
* Block comments are not supported

---

## 8.2 Type System Reference

### Built-in Types

CDM provides the following built-in primitive types:

* `string`
* `number`
* `boolean`
* `JSON`

---

### Type Expressions

Supported type expressions include:

* Identifier references
* Arrays (`T[]`)
* Union types (`A | B`)
* String literal unions (`"a" | "b"`)

Only single-dimensional arrays are supported.

---

### Optional Fields

Fields may be marked optional using `?`:

```cdm
email?: string
```

Optionality means the field may be omitted entirely.

---

### Default Values

Fields may define default values using `=`.

Defaults must be literals:

* String
* Number
* Boolean
* Array
* Object

Function calls are not permitted as defaults.

---

## 8.3 Type Aliases

### Definition

```cdm
Email: string
```

Type aliases:

* Are resolved at build time
* May reference other aliases
* May carry plugin configuration
* Cannot be circular

---

### Union Aliases

```cdm
Status: "active" | "pending" | "disabled"
```

Union aliases may include string literals or type references.

---

## 8.4 Models

### Definition

```cdm
User {
  id: string
  email: string
}
```

Each field must appear on its own line.

Empty models are allowed:

```cdm
Empty {}
```

---

### Field Definition Syntax

A field may include:

* Name
* Optional marker
* Type
* Default value
* Plugin configuration
* Entity ID

All components are optional except the name.

---

### Relationships

Fields may reference other models, including circular references.

```cdm
Post {
  author: User
}
```

---

## 8.5 Inheritance

### Extending Models

```cdm
Admin extends User {
  role: string
}
```

Multiple inheritance is supported.

When conflicts occur, the **last parent listed wins**.

---

### Field Removal

Inherited fields may be removed using `-`:

```cdm
User {
  -password_hash
}
```

Removing non-existent fields is an error.

---

## 8.6 [Context System](3-context-system.md) Reference

### `@extends`

```cdm
@extends ./base.cdm
```

* Must appear at the top of the file
* Paths are resolved relative to the file
* Circular chains are disallowed

For detailed context usage, see [Section 3: Context System](3-context-system.md).

---

### Context Capabilities

Contexts may:

* Add models and types
* Remove models and types
* Modify models
* Override type aliases
* Change plugin configuration

Contexts apply changes incrementally along the extends chain.

---

## 8.7 [Plugin](4-plugins-and-code-generation.md) Reference

### Plugin Declaration

```cdm
@typescript {
  build_output: "./generated"
}
```

Plugins may be:

* Registry plugins
* Git plugins
* Local plugins

---

### Configuration Levels

Configuration may be defined at:

* Global
* Type alias
* Model
* Field

Lower levels override higher levels.

---

### Execution Capabilities

Plugins may implement:

* Validation
* Build
* Migration

Missing required configuration causes validation failure.

For detailed plugin usage, see [Section 4: Plugins & Code Generation](4-plugins-and-code-generation.md). For writing custom plugins, see [Section 7: Plugin Development](7-plugin-development.md).

---

## 8.8 CLI Reference

### Core Commands

* [`cdm validate`](5-cli-usage-and-workflows.md#51-validation-workflow)
* [`cdm build`](5-cli-usage-and-workflows.md#52-build-workflow)
* [`cdm migrate`](5-cli-usage-and-workflows.md#53-migration-workflow)
* [`cdm format`](5-cli-usage-and-workflows.md#54-formatting-and-entity-ids)
* [`cdm plugin`](5-cli-usage-and-workflows.md#55-plugin-management)

Each command supports `--help` for full option listings. For detailed workflows, see [Section 5: CLI Usage & Workflows](5-cli-usage-and-workflows.md).

---

### Exit Codes

* `0` — success
* `1` — validation or runtime error
* `2` — file or configuration error

---

## 8.9 Error Codes

Errors are reported with stable identifiers (e.g. `E201`).

Error categories include:

* File structure errors
* Type resolution errors
* Context errors
* Plugin errors
* Entity ID errors

Warnings are reported separately and do not fail validation.

---

## 8.10 Data Formats

CDM internally represents schemas and changes using structured JSON formats.

These formats are used for:

* Plugin input
* Migration comparison
* Schema snapshots

The exact formats are documented in the Appendix.

---

## Closing Notes

This reference section completes the core CDM documentation.

At this point, you should have:

* A working understanding of CDM's mental model — see [Core Concepts](1-core-concepts.md)
* Practical experience with schemas, [contexts](3-context-system.md), and [plugins](4-plugins-and-code-generation.md)
* Familiarity with the [CLI](5-cli-usage-and-workflows.md) and [tooling](6-tooling-and-editor-support.md)
* A reference for language behavior and commands

As CDM evolves, additional material—such as expanded references, examples, and best practices—may be added incrementally.

For the most up-to-date information, including plugin documentation and implementation details, refer to the project repository and individual plugin READMEs.
