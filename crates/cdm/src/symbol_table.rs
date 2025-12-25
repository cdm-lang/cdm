// symbol_table.rs
use std::collections::HashMap;
use std::fmt;
use cdm_utils::Span;

/// The kind of definition, with data needed for validation
#[derive(Debug, Clone)]
pub enum DefinitionKind {
    /// A type alias like `Email: string` or `Status: "a" | "b"`
    /// 
    /// `references` contains all type identifiers referenced in the type expression.
    /// For `ValidatedEmail: string`, references = ["string"]
    /// For `Result: Ok | Err`, references = ["Ok", "Err"]  
    /// For `Items: Item[]`, references = ["Item"]
    /// 
    /// `type_expr` contains the original type expression text (for union validation)
    TypeAlias {
        references: Vec<String>,
        type_expr: String,
    },
    
    /// A model definition like `User { name: string }`
    /// 
    /// `extends` contains the names of parent models.
    /// For `AdminUser extends BaseUser, Timestamped`, extends = ["BaseUser", "Timestamped"]
    Model {
        extends: Vec<String>,
    },
}

#[derive(Debug, Clone)]
pub struct Definition {
    pub kind: DefinitionKind,
    pub span: Span,
    /// Plugin-specific configurations (plugin_name → config)
    pub plugin_configs: std::collections::HashMap<String, serde_json::Value>,
}

/// Information about a field in a model.
///
/// Used for cross-file validation of field removals and overrides.
#[derive(Debug, Clone)]
pub struct FieldInfo {
    /// The field name
    pub name: String,
    /// The type expression as a string, None for untyped fields (which default to string)
    pub type_expr: Option<String>,
    /// Whether the field is optional (has `?` marker)
    pub optional: bool,
    /// Source location of the field definition
    pub span: Span,
    /// Plugin-specific configurations (plugin_name → config)
    pub plugin_configs: std::collections::HashMap<String, serde_json::Value>,
    /// Default value for this field
    pub default_value: Option<serde_json::Value>,
}

/// A resolved ancestor file with its symbol table and field information.
/// 
/// Built by the caller (CLI, LSP, etc.) by parsing and analyzing parent files
/// in the `@extends` chain. The caller is responsible for:
/// 1. Parsing the source to find `@extends` directives
/// 2. Resolving file paths
/// 3. Recursively loading/parsing ancestor files
/// 4. Building `Ancestor` structs for each
/// 
/// Ancestors should be ordered from immediate parent to most distant ancestor.
#[derive(Debug, Clone)]
pub struct Ancestor {
    /// File path (for error messages)
    pub path: String,
    /// All type and model definitions from this file
    pub symbol_table: SymbolTable,
    /// Field information for each model: model_name -> fields
    pub model_fields: HashMap<String, Vec<FieldInfo>>,
}

#[derive(Debug, Clone)]
pub struct SymbolTable {
    pub definitions: HashMap<String, Definition>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            definitions: HashMap::new(),
        }
    }

    /// Check if a type name is defined in this symbol table (user-defined or built-in)
    pub fn is_defined(&self, name: &str) -> bool {
        self.definitions.contains_key(name) || is_builtin_type(name)
    }

    /// Get a definition by name from this symbol table only
    pub fn get(&self, name: &str) -> Option<&Definition> {
        self.definitions.get(name)
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a type name is defined in the local symbol table or any ancestor.
pub fn is_type_defined(name: &str, local: &SymbolTable, ancestors: &[Ancestor]) -> bool {
    if local.is_defined(name) {
        return true;
    }
    
    ancestors.iter().any(|a| a.symbol_table.is_defined(name))
}

/// Get a definition by name, checking local symbol table first, then ancestors.
/// Returns the definition and optionally which ancestor it came from.
pub fn resolve_definition<'a>(
    name: &str,
    local: &'a SymbolTable,
    ancestors: &'a [Ancestor],
) -> Option<(&'a Definition, Option<&'a str>)> {
    if let Some(def) = local.get(name) {
        return Some((def, None));
    }
    
    for ancestor in ancestors {
        if let Some(def) = ancestor.symbol_table.get(name) {
            return Some((def, Some(&ancestor.path)));
        }
    }
    
    None
}

/// Get the fields for a model, checking local model_fields first, then ancestors.
/// Returns all fields from the inheritance chain (accumulated).
pub fn get_inherited_fields<'a>(
    model_name: &str,
    local_fields: &'a HashMap<String, Vec<FieldInfo>>,
    local_symbol_table: &'a SymbolTable,
    ancestors: &'a [Ancestor],
) -> Vec<&'a FieldInfo> {
    let mut fields = Vec::new();
    
    // Get fields from ancestors first (so child fields can override)
    // We need to follow the extends chain for the model
    if let Some(def) = local_symbol_table.get(model_name) {
        if let DefinitionKind::Model { extends } = &def.kind {
            for parent_name in extends {
                // Recursively get parent's inherited fields
                let parent_fields = get_inherited_fields(
                    parent_name,
                    local_fields,
                    local_symbol_table,
                    ancestors,
                );
                fields.extend(parent_fields);
            }
        }
    } else {
        // Model might be in ancestors
        for ancestor in ancestors {
            if let Some(def) = ancestor.symbol_table.get(model_name) {
                if let DefinitionKind::Model { extends } = &def.kind {
                    for parent_name in extends {
                        let parent_fields = get_inherited_fields(
                            parent_name,
                            &ancestor.model_fields,
                            &ancestor.symbol_table,
                            ancestors,
                        );
                        fields.extend(parent_fields);
                    }
                }
                // Add this ancestor model's own fields
                if let Some(model_fields) = ancestor.model_fields.get(model_name) {
                    fields.extend(model_fields.iter());
                }
                break;
            }
        }
    }
    
    // Add local fields last
    if let Some(model_fields) = local_fields.get(model_name) {
        fields.extend(model_fields.iter());
    }
    
    fields
}

/// Check if a field exists in a model's parent chain (not including the model itself).
/// Used to validate field removals and field overrides.
pub fn field_exists_in_parents(
    model_name: &str,
    field_name: &str,
    local_fields: &HashMap<String, Vec<FieldInfo>>,
    local_symbol_table: &SymbolTable,
    ancestors: &[Ancestor],
) -> bool {
    // Get the extends list for this model
    let extends = if let Some(def) = local_symbol_table.get(model_name) {
        if let DefinitionKind::Model { extends } = &def.kind {
            extends.clone()
        } else {
            return false;
        }
    } else {
        // Model might be in ancestors (shouldn't happen for the model being validated)
        return false;
    };

    // If there are explicit extends, check those
    if !extends.is_empty() {
        for parent_name in &extends {
            let parent_fields = get_inherited_fields(
                parent_name,
                local_fields,
                local_symbol_table,
                ancestors,
            );

            if parent_fields.iter().any(|f| f.name == field_name) {
                return true;
            }
        }
    } else {
        // No explicit extends - check if a model with the same name exists in ancestors
        // (implicit extension/modification pattern from spec section 7.3)
        for ancestor in ancestors {
            if let Some(_def) = ancestor.symbol_table.get(model_name) {
                // Found a model with the same name in an ancestor
                if let Some(ancestor_fields) = ancestor.model_fields.get(model_name) {
                    if ancestor_fields.iter().any(|f| f.name == field_name) {
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// Built-in primitive types that don't need to be declared
pub fn is_builtin_type(name: &str) -> bool {
    matches!(
        name,
        "string" | "number" | "boolean" | "JSON"
    )
}

impl fmt::Display for SymbolTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Symbol Table ({} definitions):", self.definitions.len())?;
        writeln!(f, "{}", "-".repeat(40))?;

        // Sort by name for consistent output
        let mut entries: Vec<_> = self.definitions.iter().collect();
        entries.sort_by_key(|(name, _)| *name);

        for (name, def) in entries {
            match &def.kind {
                DefinitionKind::TypeAlias { references, type_expr } => {
                    if references.is_empty() {
                        writeln!(
                            f,
                            "  {} (type alias: {}) - line {}",
                            name,
                            type_expr,
                            def.span.start.line + 1
                        )?;
                    } else {
                        writeln!(
                            f,
                            "  {} (type alias -> {}) - line {}",
                            name,
                            references.join(", "),
                            def.span.start.line + 1
                        )?;
                    }
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