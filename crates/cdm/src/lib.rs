mod validate;
mod diagnostics;
mod symbol_table;

pub use diagnostics::{Diagnostic, Position, Span, Severity};
pub use symbol_table::{Ancestor,SymbolTable, Definition, DefinitionKind, FieldInfo, field_exists_in_parents, is_builtin_type, is_type_defined, resolve_definition};
pub use validate::{validate, extract_extends_paths, ValidationResult};