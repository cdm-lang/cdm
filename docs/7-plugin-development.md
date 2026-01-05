# 7. Plugin Development

Plugins are how CDM turns schemas into concrete outputs such as code, schemas, migrations, or documentation.

This section explains **when and why to write a plugin**, and how to get started using the CDM CLI. Detailed APIs and data structures are covered later in the reference sections.

---

## 7.1 When to Write a Plugin

You should consider writing a custom plugin when:

* You need to generate artifacts not covered by official plugins
* You want organization-specific code generation
* You need domain-specific validation rules
* You want CDM to drive non-code outputs (configs, metadata, documentation)

You do **not** need a plugin to:

* Change schema behavior
* Add runtime logic
* Customize existing plugins (most customization is configuration-based)

If an existing plugin can be configured to meet your needs, prefer that.

---

## 7.2 What Plugins Can Do

Plugins operate on the **fully resolved schema** and may:

* Validate plugin-specific configuration
* Generate output files during `cdm build`
* Generate migration files during `cdm migrate`
* Inspect models, fields, types, and relationships
* React to schema changes via deltas

Conceptually, plugins are pure transformations:

```
(schema + config) → output files
```

---

## 7.3 What Plugins Cannot Do

Plugins cannot:

* Modify the schema
* Affect type resolution or context inheritance
* Communicate with other plugins
* Perform runtime logic
* Access the network
* Access the filesystem outside configured output directories

These constraints ensure plugins are safe, deterministic, and composable.

---

## 7.4 Plugin Architecture Overview

CDM plugins are:

* Compiled to **WebAssembly**
* Executed in a sandboxed environment
* Loaded from a registry, Git repository, or local filesystem
* Invoked by the [CDM CLI](5-cli-usage-and-workflows.md)

Each plugin declares:

* Its capabilities (`build`, `migrate`, `validate`)
* A configuration schema
* Optional build and migration entry points

CDM validates all plugin configuration **before** invoking plugin logic.

---

## 7.5 Creating a Plugin with `cdm plugin new`

The easiest way to create a plugin is using the CDM CLI.

### Creating a New Plugin

```bash
cdm plugin new my-plugin --lang rust
```

At present, **Rust is the only supported language** for CDM plugins, and the `--lang rust` flag is required.

This command scaffolds a new plugin project with a standard structure, including:

* A plugin manifest
* A configuration schema
* A Rust project targeting WebAssembly
* Stub implementations for validation, build, and migration hooks

By default, the plugin is created in the current directory.

---

### Output Structure

A newly created plugin looks like:

```text
my-plugin/
├── cdm-plugin.json        # Plugin manifest
├── schema.cdm             # Plugin configuration schema
├── Cargo.toml             # Rust project configuration
├── src/
│   ├── lib.rs             # Plugin entry point
│   ├── validate.rs        # Config validation (optional)
│   ├── build.rs           # Build logic (optional)
│   └── migrate.rs         # Migration logic (optional)
└── README.md
```

This structure reflects how CDM loads and interacts with plugins.

---

### Choosing Capabilities

You may implement only a subset of plugin capabilities:

* **Validation-only plugins** enforce rules but generate no output
* **Build plugins** generate artifacts during `cdm build`
* **Migration plugins** generate migration files during `cdm migrate`

Unused hooks can be omitted.

---

## 7.6 Using a Local Plugin

During development, plugins are typically referenced locally.

```cdm
@my-plugin from ./my-plugin {
  build_output: "./generated"
}
```

This allows rapid iteration without publishing or versioning the plugin.

Local plugins behave exactly like registry or Git plugins.

---

## 7.7 Plugin Lifecycle

A plugin participates in CDM workflows as follows:

### Validation Phase

* CDM loads the plugin
* The plugin provides its configuration schema
* User configuration is validated and defaults are applied

### Build Phase ([`cdm build`](5-cli-usage-and-workflows.md#52-build-workflow))

* The resolved schema and configuration are passed to the plugin
* Output files are generated
* Files are written to the configured output directory

### Migration Phase ([`cdm migrate`](5-cli-usage-and-workflows.md#53-migration-workflow))

* CDM computes schema deltas
* The plugin receives both schema and deltas
* Migration files are generated

Plugins may participate in any subset of these phases.

---

## 7.8 Official vs Custom Plugins

### Official Plugins

Maintained by the CDM project:

* TypeScript
* SQL
* Docs
* JSON Schema

These are documented and versioned as part of the core ecosystem.

---

### Custom Plugins

Written and maintained by users or organizations:

* Loaded from Git or local paths
* Versioned independently
* Ideal for internal tooling and experimentation

Both types are treated identically by CDM.

---

## 7.9 Testing and Iteration

Plugins should be tested against:

* Minimal schemas
* Context chains
* Schema changes that produce deltas
* Invalid configurations

Local plugin loading makes it easy to iterate quickly and test edge cases.

---

## 7.10 Publishing Plugins

Plugins can be distributed by:

* Publishing releases on GitHub
* Referencing Git URLs directly in schemas
* Optionally submitting to the CDM registry

Versioning and compatibility are the responsibility of the plugin author.

---

## 7.11 Philosophy of Plugin Development

Plugins should aim to be:

* Deterministic
* Side-effect free
* Explicit in configuration
* Conservative in assumptions

In CDM, **schema authors express intent**, and plugins execute that intent faithfully.

---

## What's Next?

With plugin development covered, the remaining section provides **reference material**.

Proceed to **[Section 8: Reference](8-reference.md)** for detailed language syntax, CLI commands, and plugin APIs.
