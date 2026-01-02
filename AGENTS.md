# AGENTS.md

## Project overview
CDM (Contextual Data Models) is a schema language and toolchain for defining a single data model and generating context-specific outputs (SQL, TypeScript, docs, etc.) via plugins. The core reference is the language spec in `specs/spec.md`, and user-facing docs live under `docs/`.

## Key areas in this repo
- **CLI + core engine**: `crates/cdm/`
- **Grammar + parser**: `crates/grammar/` (Tree-sitter grammar and generated parser)
- **Plugin interface**: `crates/cdm-plugin-interface/`
- **Official plugins**: `crates/cdm-plugin-*` (SQL, TypeScript, docs, JSON schema, etc.)
- **JSON validator**: `crates/cdm-json-validator/`
- **Utilities**: `crates/cdm-utils/`
- **LSP**: `crates/cdm-lsp/`
- **Editor tooling**: `editors/` (Editor extension in `editors/cdm-extension/`)
- **Docs & specs**: `docs/`, `specs/`
- **Examples**: `examples/`

## References
- **README**: `README.md` for high-level positioning and CLI entry points.
- **Language spec**: `specs/spec.md` for authoritative syntax/semantics.
- **Docs**: `docs/` for user workflows, CLI usage, plugins, and reference material.

## Common commands
- **Install deps**: `just setup`
- **Generate parser**: `just generate` (runs tree-sitter generation in `crates/grammar`)
- **Build all**: `just build` (generates grammar then `cargo build`)
- **Run CLI**: `just run -- <args>`
- **Tests**: `just test` (runs `cargo test` and VS Code extension tests)

## Notes
- If you modify grammar files under `crates/grammar`, regenerate the parser (`just generate`) before building or testing.
- Plugin release workflows are defined in `Justfile` (see `release-plugin` and `release-cli`).
