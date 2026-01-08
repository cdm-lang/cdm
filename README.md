# CDM — Contextual Data Models

**CDM** is a schema language for defining your application’s data model **once** and generating **everything else** from it.

From a single CDM schema, you can generate:

* SQL schemas and migrations
* TypeScript / JavaScript types
* Validation logic
* API-specific views of your models
* Documentation
* Other custom output via plugins

CDM is designed to be the **source of truth** for your data — across databases, APIs, services, and clients.

---

## Why CDM Exists

Most teams define the same data model *multiple times*:

* SQL tables
* ORM models
* API contracts
* Client types
* Validation schemas
* Migration logic

These definitions can be difficult to keep in sync and inevitably drift.

**CDM eliminates that drift.**

You define your data model *once*, in a language designed specifically for modeling, evolution, and code generation — and CDM takes care of the rest.

---

## What Makes CDM Different

### 1. One Model, Many [Contexts](docs/3-context-system.md)

CDM lets you describe **different views of the same schema** without duplication.

For example:

* A **database** context with internal fields and indexes
* An **API** context that hides sensitive fields
* A **client** context with only what the frontend needs

Each context extends a shared base model and applies targeted changes — safely and explicitly.

```cdm
// base.cdm
User {
  id: string
  name: string
  email: string
}
```

```cdm
// database.cdm
extends "./base.cdm"

User {
  birth_date: string
  password_hash: string
}
```

```cdm
// client.cdm
extends "./base.cdm"

User {
  age: number
}
```

---

### 2. Migration-Safe [Schema Evolution](docs/1-core-concepts.md#14-entity-ids-and-schema-evolution)

CDM optionally supports **stable entity IDs** that allow it to *reliably detect renames*.

That means:

* Renaming a field does **not** drop data
* Migrations are deterministic
* Refactors are safe

```cdm
User {
  display_name: string #2  // renamed from "name"
}
```

---

### 3. Context-Aware Code Generation

CDM doesn’t just generate code — it generates **the right code for the environment**.

The same field can:

* Be a `VARCHAR(320)` in SQL
* A named type in TypeScript
* Have strict validation rules
* Be hidden entirely in some contexts

All of this is configured *at the schema level*, not scattered across tools.

---

### 4. Plugin-Driven, Language-Agnostic

CDM itself is intentionally focused.

Everything else — SQL, TypeScript, docs, validation — is handled by **[plugins](docs/4-plugins-and-code-generation.md)**.

Plugins:

* Are sandboxed WebAssembly modules
* Can generate any output
* Can validate schema rules
* Can be loaded from a registry, GitHub, or locally

This makes CDM extensible without locking you into a specific stack.

---

### 5. Familiar, Readable Syntax

CDM uses a **TypeScript-inspired syntax** designed to feel obvious to most developers:

```cdm
Email: string {
  @validation { format: "email" }
} #1

User {
  id: string #1
  email: Email #2
  status: "active" | "pending" = "pending" #3
} #10
```

---

## Who CDM Is For

CDM is a good fit if you:

* Maintain schemas across multiple layers (DB, API, client)
* Care about safe schema evolution and migrations
* Want strong guarantees around refactors
* Are tired of duplicating models
* Want code generation without losing control

It’s especially useful for:

* Backend-heavy applications
* APIs with multiple consumers
* Long-lived systems where schemas evolve
* Teams that value explicitness and correctness

---

## What CDM Is *Not*

CDM is **not**:

* An ORM
* A runtime validation library
* A replacement for your database
* A general-purpose programming language

CDM operates *before* runtime — at design, validation, and generation time.

---

## Installation

Once you’re ready to try CDM, installation is straightforward.

### Quick Install

#### macOS / Linux

```bash
curl -fsSL https://raw.githubusercontent.com/cdm-lang/cdm/main/install.sh | sh
```

#### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/cdm-lang/cdm/main/install.ps1 | iex
```

The installer:

* Downloads the latest CDM binary
* Adds it to your PATH
* Installs shell completions

---

### Alternative Installation Methods

* **npm**: `npm install -g @cdm-lang/cli`
* **Manual download**: Prebuilt binaries from GitHub releases
* **Build from source**: Rust (latest stable)

(See the full installation section below for details.)

---

## Using CDM

Verify installation:

```bash
cdm --version
cdm --help
```

Core commands:

* `cdm validate` — validate schemas
* `cdm build` — generate code
* `cdm migrate` — generate migrations
* `cdm format` — format schemas and assign IDs
* `cdm plugin` — manage plugins

---

## Learning More

* 📘 **[Getting Started](docs/0-getting-started.md)** — quick start guide and installation
* 📚 **[Core Concepts](docs/1-core-concepts.md)** — contexts, IDs, plugins, and schema evolution
* 🔀 **[Context System](docs/3-context-system.md)** — different views of the same schema
* 🧩 **[Plugins](docs/4-plugins-and-code-generation.md)** — SQL, TypeScript, validation, and custom generators
* ⌨️ **[CLI Usage](docs/5-cli-usage-and-workflows.md)** — build, migrate, and format commands
* 🛠 **[Editor Support](docs/6-tooling-and-editor-support.md)** — VS Code extension and LSP
* 📖 **[Reference](docs/8-reference.md)** — language and CLI reference

---

## License

MPL-2.0
