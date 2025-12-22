mod validate;
mod diagnostics;
mod symbol_table;
mod plugin_runner;
mod file_resolver;
mod grammar_parser;
mod resolved_schema;

pub use diagnostics::{Diagnostic, Position, Span, Severity};
pub use symbol_table::{Ancestor,SymbolTable, Definition, DefinitionKind, FieldInfo, field_exists_in_parents, is_builtin_type, is_type_defined, resolve_definition};
pub use validate::{validate, validate_tree, ValidationResult};
pub use plugin_runner::PluginRunner;
pub use file_resolver::{FileResolver, LoadedFile, LoadedFileTree};
pub use grammar_parser::GrammarParser;
pub use resolved_schema::{ResolvedSchema, ResolvedTypeAlias, ResolvedModel, ResolvedField, build_resolved_schema, find_references_in_resolved};