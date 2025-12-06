mod validate;
mod diagnostics;
mod symbol_table;

pub use diagnostics::{Diagnostic, Position, Span, Severity};
pub use symbol_table::{SymbolTable, Definition, DefinitionKind};
pub use validate::{validate, ValidationResult};