# AGENTS.md

## Project overview
CDM (Contextual Data Models) is a schema language and toolchain for defining a single data model and generating context-specific outputs (SQL, TypeScript, docs, etc.) via plugins. The core reference is the language spec in `specs/spec.md`, and user-facing docs live under `docs/`.

## Key areas in this repo
- **CLI + core engine + LSP**: `crates/cdm/` (includes `cdm lsp` subcommand)
- **Grammar + parser**: `crates/grammar/` (Tree-sitter grammar and generated parser)
- **Plugin interface**: `crates/cdm-plugin-interface/`
- **Official plugins**: `crates/cdm-plugin-*` (SQL, TypeScript, docs, JSON schema, etc.)
- **JSON validator**: `crates/cdm-json-validator/`
- **Utilities**: `crates/cdm-utils/`
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
- For rust based unit tests, they should live in their own files. For example crates/cdm/src/git_plugin.rs has tests in the file crates/cdm/src/git_plugin/git_plugin_tests.rs and is linked with the code below. All rust unit tests should follow this format.

    ```rs
    #[cfg(test)]
    #[path = "git_plugin/git_plugin_tests.rs"]
    mod git_plugin_tests;
    ```

- When making any code changes ensure that the full code still builds without errors/warnings. If there are any, fix them.
- When making code changes be sure that all relevant tests run. `just test` may not be able to fully run inside the docker container, but you should run any specific tests related to the changed code and ensure that they still pass.
- When making code changes, start by adding tests that assert the bug is fixed or feature is working. Then make the changes. Then ensure that those tests (along with others) pass.
- When running tests in editors/cdm-extension, you may need to run npm install before running the tests.
- When making code changes to public facing api's be sure to update the appropriate documentation (plugin README's, cdm docs, specs, etc)
- When you consider implementing a new function, first check if the codebase has any other similar functions that could be re-used, or refactored to support multiple use cases and prefer doing that rather than re-implementing something similar.