Got it — thanks for the correction. You’re right: this should be **Section 0 of the docs**, not the project README.

Below is **only Section 0: Getting Started**, written as a self-contained docs chapter that assumes the README already exists and links here.

This version:

* Follows your exact ordering
* Uses the **simplified TypeScript plugin config**
* Is written as *documentation*, not marketing copy
* Avoids repeating long install variants
* Is structured for a docs site / sidebar

---

# 0. Getting Started

This section introduces CDM and walks you through a minimal, end-to-end example. By the end, you’ll have a working CDM schema and generated TypeScript output.

---

## 0.1 What Is CDM?

**CDM (Contextual Data Models)** is a schema language for defining your application’s data model once and generating everything else from it.

From a single CDM schema, you can generate:

* SQL schemas and migrations
* TypeScript / JavaScript types
* Validation logic
* API-specific views of your models
* Documentation
* Custom outputs via plugins

CDM is designed to act as the **single source of truth** for data across your stack — database, backend, API, and client.

### Why CDM Exists

In most systems, the same data model is defined multiple times:

* Database schemas
* ORM models
* API DTOs
* Client-side types
* Validation schemas
* Migration logic

These definitions drift over time.

CDM eliminates this drift by centralizing the data model in a purpose-built language and generating all downstream representations from it.

### Key Ideas

* **[Contexts](3-context-system.md)** allow different views of the same schema (e.g. DB vs API).
* **[Entity IDs](1-core-concepts.md#14-entity-ids-and-schema-evolution)** make schema evolution and renames safe.
* **[Plugins](4-plugins-and-code-generation.md)** generate code, schemas, and other artifacts.
* **Explicit modeling** avoids runtime magic.

CDM operates before runtime — during design, validation, and generation.

---

## 0.2 Quick Start

This is the fastest way to see CDM working.

### Step 1: Create a Schema

Create a file called `schema.cdm`:

```cdm
User {
  id: string #1
  email: string #2
  name: string #3
} #10
```

This defines a single model and assigns stable entity IDs for future migrations.

---

### Step 2: Validate the Schema

```bash
cdm validate schema.cdm
```

If validation succeeds, your schema is syntactically and semantically correct.

---

### Step 3: Build Outputs

To see CDM generate real output, enable the TypeScript plugin.

Update `schema.cdm`:

```cdm
@typescript {
  build_output: "./generated"
}

User {
  id: string #1
  email: string #2
  name: string #3
} #10
```

No additional configuration is required.

---

### Step 4: Run the Build

```bash
cdm build schema.cdm
```

CDM will:

1. Validate the schema
2. Load the TypeScript plugin
3. Generate output files

---

### Step 5: Inspect the Output

You should now have generated files in:

```text
./generated
```

Containing TypeScript similar to:

```ts
export interface User {
  id: string;
  email: string;
  name: string;
}
```

You’ve now generated TypeScript types from a CDM schema.

---

## 0.3 Installation

Before continuing with the rest of the documentation, make sure CDM is installed.

### Quick Install

#### macOS / Linux

```bash
curl -fsSL https://raw.githubusercontent.com/cdm-lang/cdm/main/install.sh | sh
```

#### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/cdm-lang/cdm/main/install.ps1 | iex
```

Verify installation:

```bash
cdm --version
```

---

### Alternative Installation Methods

* **npm**: `npm install -g @cdm-lang/cli`
* **Manual download**: Prebuilt binaries from GitHub releases
* **Build from source**: Clone the repo and build with Rust

Detailed installation instructions are covered in the CLI documentation.

---

## What's Next?

From here, you can continue with:

* **[Core Concepts](1-core-concepts.md)** — contexts, IDs, plugins, and schema evolution
* **[Context System](3-context-system.md)** — different views of the same schema
* **[Plugins](4-plugins-and-code-generation.md)** — SQL, TypeScript, validation, and custom generators
* **[CLI Workflows](5-cli-usage-and-workflows.md)** — build, migrate, and format commands

[Section 1: Core Concepts](1-core-concepts.md) starts by building a deeper mental model of how CDM works.
