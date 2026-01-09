# 4. Plugins & Code Generation

CDM itself does not generate SQL, TypeScript, documentation, or schemas.

All output in CDM is produced by **plugins**.

Plugins are responsible for transforming a fully resolved CDM schema into concrete artifacts such as code, schemas, migrations, or documentation.

---

## 4.1 What Is a Plugin?

A plugin is a sandboxed module that:

* Receives a fully resolved CDM schema
* Validates plugin-specific configuration
* Generates output files
* Optionally generates migration files

Plugins run only after CDM has:

1. Parsed the schema
2. Resolved all contexts
3. Validated structural correctness

This guarantees that plugins always operate on a complete and consistent model.

---

## 4.2 Why Plugins Are Separate from the Language

CDM is intentionally focused on **data modeling**, not code generation.

Separating plugins from the language allows CDM to:

* Keep the core language small and stable
* Support many output targets without coupling
* Allow independent evolution of generators
* Enable third-party and internal tooling

As a result, CDM schemas remain portable and long-lived, while plugins can change as tooling needs evolve.

---

## 4.3 Plugin Capabilities

Plugins may support one or more of the following capabilities:

* **Validation** — enforce additional schema rules
* **Build** — generate output files
* **Migrate** — generate migration files

CDM enforces required configuration based on a plugin's declared capabilities.

For example:

* Plugins with **build** capability require `build_output`
* Plugins with **migrate** capability require `migrations_output`

If required configuration is missing, CDM fails validation before generation begins.

> **Note:** `build_output` and `migrations_output` are CDM-level settings, not plugin settings. CDM processes these values to determine where to write files, then filters them out before passing configuration to plugins. Plugins never see these values and should not include them in their configuration schemas.

---

## 4.4 Plugin Sources

Plugins can be loaded from multiple sources.

### Registry Plugins

Plugins without a `from` clause are resolved from the CDM registry:

```cdm
@typescript
@sql
@docs
@jsonschema
```

These are the officially supported plugins maintained by the CDM project.

#### Version Pinning

Registry plugins support version pinning using the `version` config key with full semver range support:

```cdm
@sql {
  version: "1.2.3",
  build_output: "./db/schema"
}
```

Supported version constraint formats:

| Constraint       | Meaning                                         |
| ---------------- | ----------------------------------------------- |
| `"1.2.3"`        | Exact version                                   |
| `"^1.2.3"`       | Compatible with 1.x.x (>=1.2.3 <2.0.0)          |
| `"~1.2.3"`       | Patch-level changes only (>=1.2.3 <1.3.0)       |
| `">=1.0.0 <2.0.0"` | Explicit range                                |
| (omitted)        | Latest available version                        |

If no version is specified, the latest available version is used.

---

### Git Plugins

Plugins can be loaded directly from Git repositories:

```cdm
@analytics from git:https://github.com/my-org/cdm-analytics.git
```

This supports private repositories, pinned versions, and custom tooling.

#### Git Reference Pinning

Git plugins can specify a git reference using the `git_ref` config key:

```cdm
@sql from git:https://github.com/cdm-lang/cdm-plugin-sql.git {
  git_ref: "v1.2.3"
}
```

The `git_ref` can be:

* A tag (e.g., `"v1.2.3"`)
* A branch name (e.g., `"main"`, `"develop"`)
* A commit SHA (e.g., `"a1b2c3d"`)

If no `git_ref` is specified, the `main` branch is used by default.

#### Subdirectory Paths

For monorepos or repositories where the plugin is nested within a subdirectory, use the `git_path` config key:

```cdm
@myplugin from git:https://github.com/my-org/monorepo.git {
  git_path: "packages/cdm-plugin"
}
```

The `git_path` specifies the directory containing the `cdm-plugin.json` manifest file.

You can combine git reference pinning with subdirectory paths:

```cdm
@myplugin from git:https://github.com/my-org/monorepo.git {
  git_ref: "v2.0.0",
  git_path: "packages/cdm-plugin"
}
```

---

### Local Plugins

Plugins can be loaded from the local filesystem:

```cdm
@custom from "./plugins/my-plugin"
```

This is useful for development, experimentation, and internal generators.

---

## 4.5 Configuring Plugins

Plugins are configured using JSON-like configuration blocks:

```cdm
@typescript {
  build_output: "./generated"
}
```

Configuration can appear at four levels:

| Level      | Applies To       |
| ---------- | ---------------- |
| Global     | Entire schema    |
| Type Alias | A specific type  |
| Model      | A specific model |
| Field      | A specific field |

Lower-level configuration overrides or extends higher-level configuration.

---

## 4.6 Configuration Inheritance and [Contexts](3-context-system.md)

When a [context](3-context-system.md) extends another schema, plugin configuration merges across context boundaries.

Merge rules:

* Objects are deep-merged
* Arrays are replaced entirely
* Primitive values override parent values

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

The API context inherits all TypeScript settings except where explicitly overridden.

---

## 4.7 Plugin Execution Order

Plugins execute in the order they appear in the schema:

```cdm
@validation
@sql
@typescript
```

This allows validation plugins to fail early and generation plugins to operate on validated input.

Plugins run independently and do not share state.

---

## 4.8 Build vs Migrate

CDM distinguishes between **building** and **migrating** schemas.

### Build

`cdm build`:

* Validates the schema
* Executes each plugin’s build step
* Writes generated files to disk

Build output is derived solely from the current schema.

---

### Migrate

[`cdm migrate`](5-cli-usage-and-workflows.md#53-migration-workflow):

* Compares the current schema to a previously saved version
* Computes structural differences (deltas)
* Uses [entity IDs](1-core-concepts.md#14-entity-ids-and-schema-evolution) to reliably detect renames
* Invokes plugins' migration steps

Migration output describes how to evolve an existing system to match the new schema.

---

## 4.9 Official Plugins

CDM currently provides the following official plugins:

* **TypeScript** — generate TypeScript types and models
* **SQL** — generate database schemas and migrations
* **Docs** — generate human-readable documentation
* **JSON Schema** — generate JSON Schema definitions

Additional official plugins may be added in the future.

Each plugin is documented independently and may be configured at global, model, or field level.

---

## 4.10 Plugin Safety and Isolation

Plugins run in a sandboxed environment with strict limits:

* No network access
* No filesystem access outside configured output directories
* Execution time and memory limits
* Maximum output size limits

These constraints ensure plugins are safe to run in local and CI environments.

---

## 4.11 Using Multiple Plugins Together

It is common to enable multiple plugins in a single schema:

```cdm
@sql {
  build_output: "./db/schema"
  migrations_output: "./db/migrations"
}

@typescript {
  build_output: "./types"
}

@jsonschema {
  build_output: "./schemas"
}
```

All plugins operate on the same resolved schema and generate independent outputs.

---

## 4.12 What Plugins Cannot Do

Plugins cannot:

* Modify the schema
* Affect type resolution
* Change context behavior
* Communicate with other plugins
* Perform runtime logic

Plugins are pure transformations from schema to artifacts.

---

## 4.13 Plugin Development

Writing custom plugins is a first-class use case in CDM, but it is intentionally documented separately.

A dedicated section covers:

* When to write a plugin
* Plugin architecture and lifecycle
* Configuration schemas
* Build and migration APIs
* Testing and publishing plugins

If you're interested in extending CDM itself, see **[Section 7: Plugin Development](7-plugin-development.md)**.

---

## What's Next?

With plugins understood, the next section focuses on **day-to-day usage**.

Proceed to **[Section 5: CLI Usage & Workflows](5-cli-usage-and-workflows.md)** to learn how validation, builds, migrations, and formatting fit into real-world development workflows.
