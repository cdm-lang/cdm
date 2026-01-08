# 5. CLI Usage & Workflows

CDM is primarily interacted with through its command-line interface. The CLI is designed to support both local development and automated workflows such as CI and migrations.

This section describes the most common commands and how they are used together in practice.

---

## 5.1 Validation Workflow

Validation is the foundation of all CDM workflows.

### Validating Schemas

Use `cdm validate` to check schemas without generating output:

```bash
cdm validate schema.cdm
```

Validation checks:

* Syntax correctness
* Type resolution
* Context correctness
* Entity ID usage
* Plugin configuration validity

If validation fails, CDM reports all detected errors and exits with a non-zero status.

---

### Validation in CI

`cdm validate` is designed to run in CI:

```bash
cdm validate cdm/**/*.cdm
```

Typical usage:

* Run on every pull request
* Fail the build on schema errors
* Enforce consistency before generation

Validation is fast and does not require output directories to exist.

---

## 5.2 Build Workflow

Building is how CDM generates artifacts from schemas.

### Building a Schema or Context

```bash
cdm build schema.cdm
```

This command:

1. Validates the supplied file
2. Resolves that file and its ancestor chain (via `extends`, if present)
3. Produces a fully resolved schema
4. Executes each plugin’s build step
5. Writes generated files to disk

If a single file is supplied, only that schema or context is resolved and built.

---

### Building Multiple Files

You may build multiple schema or context files at once:

```bash
cdm build cdm/*.cdm
```

Each file is resolved independently, including its own `extends` chain, and built separately.

This is commonly used when generating outputs for multiple environments (e.g. base, API, client).

---

## 5.3 Migration Workflow

Migrations describe how to evolve an existing system **when it changes**.

### Generating Migrations

```bash
cdm migrate schema.cdm
```

This command:

1. Loads the previously saved schema
2. Builds the current schema
3. Computes structural differences between the two
4. Uses entity IDs to detect renames
5. Invokes plugins’ migration steps
6. Writes migration files
7. Saves the current schema for future comparisons

---

### Rename Detection

CDM uses [entity IDs](1-core-concepts.md#14-entity-ids-and-schema-evolution) to distinguish renames from removals:

* With entity IDs, renames are detected reliably
* Without entity IDs, heuristics are used and ambiguous cases may require confirmation

This makes schema refactors significantly safer.

---

### Reviewing Migrations

Migration files should be:

* Reviewed like code
* Committed to version control
* Applied using your existing tooling

CDM generates migrations, but does not apply them automatically.

---

## 5.4 Formatting and Entity IDs

CDM includes tooling to help maintain consistent formatting and correct ID usage.

### Formatting Schemas

```bash
cdm format schema.cdm
```

This:

* Normalizes formatting
* Orders definitions consistently
* Improves diff readability

---

### Assigning Entity IDs

```bash
cdm format --assign-ids schema.cdm
```

This automatically assigns IDs to:

* Models
* Type aliases
* Fields

ID assignment respects scope rules and avoids collisions.

---

### Checking ID Usage

```bash
cdm validate --check-ids
```

This reports:

* Missing IDs
* Duplicate IDs
* Reused IDs

Entity IDs are optional, but strongly recommended for evolving schemas.

---

## 5.5 [Plugin](4-plugins-and-code-generation.md) Management

The CLI includes commands for managing [plugins](4-plugins-and-code-generation.md).

### Listing Plugins

```bash
cdm plugin list
```

Lists available registry plugins.

---

### Plugin Information

```bash
cdm plugin info typescript
```

Shows details about a specific plugin, including available versions and capabilities.

---

### Caching Plugins

```bash
cdm plugin cache typescript
```

Pre-downloads plugin binaries for offline or CI usage.

---

### Clearing the Plugin Cache

```bash
cdm plugin clear-cache
```

Removes cached plugin artifacts.

---

## 5.6 Typical Team Workflow

A common workflow looks like this:

1. Modify CDM schemas
2. Run `cdm validate`
3. Review schema changes
4. Run `cdm build`
5. Run `cdm migrate` (if applicable)
6. Review generated output
7. Commit schemas and generated artifacts

CDM is designed to make schema changes explicit, reviewable, and safe.

---

## 5.7 Common Mistakes

* Skipping validation before building
* Reusing or changing [entity IDs](1-core-concepts.md#14-entity-ids-and-schema-evolution)
* Treating generated code as the source of truth
* Using [contexts](3-context-system.md) to encode runtime behavior

CDM works best when schemas are treated as first-class artifacts.

---

## What's Next?

With the core workflows covered, the next section focuses on **developer tooling and editor support**.

Proceed to **[Section 6: Tooling & Editor Support](6-tooling-and-editor-support.md)** to learn how CDM integrates with editors and improves the authoring experience.
