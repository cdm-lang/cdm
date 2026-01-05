# 6. Tooling & Editor Support

CDM is designed to be edited frequently and collaboratively. To support this, CDM provides tooling that improves correctness, feedback, and ergonomics while authoring schemas.

This section covers the current editor and formatting support.

---

## 6.1 Language Server (LSP)

CDM includes a Language Server Protocol (LSP) implementation.

The LSP provides editor features such as:

* Syntax diagnostics
* Semantic validation errors
* Go-to-definition for models and type aliases
* Reference tracking
* Hover information

All diagnostics are derived from the same validation logic used by the [CLI](5-cli-usage-and-workflows.md), ensuring consistency between editor feedback and build-time errors.

---

### What the LSP Understands

The language server is aware of:

* CDM syntax and grammar
* [Type aliases and models](1-core-concepts.md#12-types-models-and-fields)
* [Context](3-context-system.md) inheritance and overrides
* [Entity IDs](1-core-concepts.md#14-entity-ids-and-schema-evolution)
* [Plugin](4-plugins-and-code-generation.md) configuration structure

This allows errors to surface while editing, before running CLI commands.

---

## 6.2 Editor Extensions

CDM provides official editor extensions built on top of the language server.

These extensions embed the LSP and handle editor-specific integration such as file detection, formatting hooks, and diagnostics display.

---

### VS Code Extension

The VS Code extension adds:

* Syntax highlighting for `.cdm` files
* Inline validation diagnostics
* Autocompletion for keywords and identifiers
* Jump-to-definition across files
* Formatting support

When installed, the language server starts automatically whenever a `.cdm` file is opened.

---

### Cursor Extension

CDM also provides an official **Cursor** extension, distributed via the **Open VSX registry**.

The Cursor extension offers the same core functionality as the VS Code extension, including:

* Syntax highlighting
* Inline diagnostics
* Language server–powered features
* Formatting support

Because Cursor is compatible with VS Code extensions, the experience is consistent across editors.

---

## 6.3 Installing Editor Support

### VS Code

The VS Code extension can be installed from the VS Code Marketplace.

Once installed, `.cdm` files are recognized automatically.

---

### Cursor

The Cursor extension can be installed from the Open VSX registry.

Once installed, Cursor will automatically activate CDM support for `.cdm` files.

---

## 6.4 Diagnostics and Error Feedback

CDM tooling emphasizes **clear, actionable errors**.

Diagnostics include:

* A stable error code
* A precise source location
* A human-readable explanation

Warnings are also surfaced for non-fatal issues such as:

* Missing entity IDs
* Unused models or type aliases
* Empty models

Editor diagnostics match CLI diagnostics exactly.

---

## 6.5 Formatting Support

CDM includes a formatter that enforces a canonical style.

### Formatting Files

```bash
cdm format schema.cdm
```

Formatting:

* Normalizes whitespace and indentation
* Orders definitions consistently
* Makes diffs easier to review

The formatter is intentionally opinionated to minimize stylistic inconsistency.

---

### Formatting in Editors

When supported by the editor, formatting can be triggered on save or manually using standard formatting commands.

Editor formatting uses the same formatter as the CLI.

---

## 6.6 Tooling Philosophy

CDM tooling follows a small set of guiding principles:

* **Single source of truth** — CLI and editor tooling share the same validation logic
* **Early feedback** — errors should surface while editing
* **Deterministic behavior** — no editor-only semantics
* **Low ceremony** — tooling should stay out of the way

Tooling exists to support the language, not redefine it.

---

## 6.7 Current Limitations

As CDM continues to evolve, tooling support is intentionally conservative.

Some features may improve over time, including:

* Advanced refactoring assistance
* Deeper plugin-aware editor hints
* Cross-project indexing

These limitations affect ergonomics, not schema correctness.

---

## What's Next?

With tooling covered, the remaining sections focus on **advanced and reference material**, including:

* [Plugin development](7-plugin-development.md)
* [Language reference](8-reference.md#81-language-reference)
* [CLI reference](8-reference.md#88-cli-reference)
* [Error codes](8-reference.md#89-error-codes)

Proceed to **[Section 7: Plugin Development](7-plugin-development.md)** if you want to extend CDM with custom generators or validators.
