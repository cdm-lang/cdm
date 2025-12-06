use std::collections::{HashMap, HashSet};
use std::fmt;
use crate::diagnostics::Span;

#[derive(Debug, Clone)]
pub enum DefinitionKind {
    TypeAlias,
    Model { extends: Vec<String> },
}

#[derive(Debug, Clone)]
pub struct Definition {
    pub kind: DefinitionKind,
    pub span: Span,
}

#[derive(Debug)]
pub struct SymbolTable {
    pub definitions: HashMap<String, Definition>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            definitions: HashMap::new(),
        }
    }

    pub fn built_in_types() -> HashSet<&'static str> {
        HashSet::from([
            "string", "number", "boolean", "decimal",
            "DateTime", "JSON", "UUID",
        ])
    }

    pub fn is_defined(&self, name: &str) -> bool {
        self.definitions.contains_key(name) || Self::built_in_types().contains(name)
    }

    pub fn get(&self, name: &str) -> Option<&Definition> {
        self.definitions.get(name)
    }
}

impl fmt::Display for SymbolTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Symbol Table ({} definitions):", self.definitions.len())?;
        writeln!(f, "{}", "-".repeat(40))?;
        
        for (name, def) in &self.definitions {
            match &def.kind {
                DefinitionKind::TypeAlias => {
                    writeln!(f, "  {} (type alias) - line {}", name, def.span.start.line + 1)?;
                }
                DefinitionKind::Model { extends } => {
                    if extends.is_empty() {
                        writeln!(f, "  {} (model) - line {}", name, def.span.start.line + 1)?;
                    } else {
                        writeln!(
                            f,
                            "  {} (model extends {}) - line {}",
                            name,
                            extends.join(", "),
                            def.span.start.line + 1
                        )?;
                    }
                }
            }
        }
        
        Ok(())
    }
}