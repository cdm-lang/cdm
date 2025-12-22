//! Resolved schema - the final merged view after inheritance and removals
//!
//! This module provides a "resolved view" of a CDM schema that combines:
//! - Type aliases and models from ancestor files
//! - Overrides from the current file
//! - Removals specified in the current file
//!
//! This is distinct from the per-file SymbolTable, which stays file-scoped
//! with file-relative spans for error reporting.

use std::collections::{HashMap, HashSet};
use crate::{Ancestor, DefinitionKind, FieldInfo, Span, SymbolTable};

/// A fully resolved schema after applying inheritance and removals.
///
/// This represents the final state of definitions available in a file,
/// including inherited definitions from ancestors.
#[derive(Debug)]
pub struct ResolvedSchema {
    /// All available type aliases (name → resolved definition)
    pub type_aliases: HashMap<String, ResolvedTypeAlias>,
    /// All available models (name → resolved model)
    pub models: HashMap<String, ResolvedModel>,
}

impl ResolvedSchema {
    pub fn new() -> Self {
        Self {
            type_aliases: HashMap::new(),
            models: HashMap::new(),
        }
    }

    /// Check if a definition (type alias or model) exists
    pub fn contains(&self, name: &str) -> bool {
        self.type_aliases.contains_key(name) || self.models.contains_key(name)
    }
}

/// A resolved type alias with source tracking
#[derive(Debug, Clone)]
pub struct ResolvedTypeAlias {
    pub name: String,
    /// The type expression as a string
    pub type_expr: String,
    /// Type identifiers referenced by this type alias
    pub references: Vec<String>,
    /// Which file this definition came from (for error reporting)
    pub source_file: String,
    /// Span in the source file
    pub source_span: Span,
}

/// A resolved model with source tracking
#[derive(Debug, Clone)]
pub struct ResolvedModel {
    pub name: String,
    /// All fields in this model (including inherited fields)
    pub fields: Vec<ResolvedField>,
    /// Which file this model was defined in
    pub source_file: String,
    /// Span in the source file
    pub source_span: Span,
}

/// A resolved field with source tracking
#[derive(Debug, Clone)]
pub struct ResolvedField {
    pub name: String,
    /// The type expression as a string (None for untyped fields defaulting to string)
    pub type_expr: Option<String>,
    pub optional: bool,
    /// Which file this field came from (original definition or inheritance)
    pub source_file: String,
    /// Span in the source file
    pub source_span: Span,
}

/// Build a resolved schema from current file symbols and ancestors.
///
/// This merges definitions from ancestors (oldest first) and then applies
/// current file definitions and removals to create the final view.
///
/// # Arguments
/// * `current_symbols` - Symbol table from the current file
/// * `current_fields` - Model fields from the current file
/// * `ancestors` - Ancestor files (in order: immediate parent first)
/// * `removals` - List of (name, span, kind) tuples for items to remove
///
/// # Returns
/// A ResolvedSchema representing the final merged state
pub fn build_resolved_schema(
    current_symbols: &SymbolTable,
    current_fields: &HashMap<String, Vec<FieldInfo>>,
    ancestors: &[Ancestor],
    removals: &[(String, Span, &str)],
) -> ResolvedSchema {
    let mut resolved = ResolvedSchema::new();

    // Build set of removed names for quick lookup
    let removed_names: HashSet<&String> = removals.iter().map(|(name, _, _)| name).collect();

    // Add definitions from ancestors (process in reverse order: furthest ancestor first)
    // This way closer ancestors override more distant ones
    for ancestor in ancestors.iter().rev() {
        // Add type aliases from this ancestor
        for (name, def) in &ancestor.symbol_table.definitions {
            // Skip if removed or already added by a closer ancestor
            if removed_names.contains(name) || resolved.contains(name) {
                continue;
            }

            match &def.kind {
                DefinitionKind::TypeAlias { references, type_expr } => {
                    resolved.type_aliases.insert(
                        name.clone(),
                        ResolvedTypeAlias {
                            name: name.clone(),
                            type_expr: type_expr.clone(),
                            references: references.clone(),
                            source_file: ancestor.path.clone(),
                            source_span: def.span,
                        },
                    );
                }
                DefinitionKind::Model { .. } => {
                    // Add model if it has fields in ancestor.model_fields
                    if let Some(fields) = ancestor.model_fields.get(name) {
                        resolved.models.insert(
                            name.clone(),
                            ResolvedModel {
                                name: name.clone(),
                                fields: fields
                                    .iter()
                                    .map(|f| ResolvedField {
                                        name: f.name.clone(),
                                        type_expr: f.type_expr.clone(),
                                        optional: f.optional,
                                        source_file: ancestor.path.clone(),
                                        source_span: f.span,
                                    })
                                    .collect(),
                                source_file: ancestor.path.clone(),
                                source_span: def.span,
                            },
                        );
                    }
                }
            }
        }
    }

    // Override with current file definitions (these take precedence)
    for (name, def) in &current_symbols.definitions {
        // Skip if removed
        if removed_names.contains(name) {
            continue;
        }

        match &def.kind {
            DefinitionKind::TypeAlias { references, type_expr } => {
                resolved.type_aliases.insert(
                    name.clone(),
                    ResolvedTypeAlias {
                        name: name.clone(),
                        type_expr: type_expr.clone(),
                        references: references.clone(),
                        source_file: "current file".to_string(),
                        source_span: def.span,
                    },
                );
            }
            DefinitionKind::Model { .. } => {
                // Add model from current file
                if let Some(fields) = current_fields.get(name) {
                    resolved.models.insert(
                        name.clone(),
                        ResolvedModel {
                            name: name.clone(),
                            fields: fields
                                .iter()
                                .map(|f| ResolvedField {
                                    name: f.name.clone(),
                                    type_expr: f.type_expr.clone(),
                                    optional: f.optional,
                                    source_file: "current file".to_string(),
                                    source_span: f.span,
                                })
                                .collect(),
                            source_file: "current file".to_string(),
                            source_span: def.span,
                        },
                    );
                }
            }
        }
    }

    resolved
}

/// Find all references to a specific definition name in the resolved schema.
///
/// Returns a list of reference locations in the format:
/// - "type alias 'Name'" for type aliases that reference it
/// - "Model.field" for model fields that reference it
///
/// Includes source file information for inherited references.
pub fn find_references_in_resolved(
    resolved: &ResolvedSchema,
    target_name: &str,
) -> Vec<String> {
    let mut references = Vec::new();

    // Check type aliases that reference the target
    for (alias_name, alias) in &resolved.type_aliases {
        if alias.references.contains(&target_name.to_string()) {
            if alias.source_file == "current file" {
                references.push(format!("type alias '{}'", alias_name));
            } else {
                references.push(format!(
                    "type alias '{}' (inherited from {})",
                    alias_name, alias.source_file
                ));
            }
        }
    }

    // Check model fields that reference the target
    for (model_name, model) in &resolved.models {
        for field in &model.fields {
            if let Some(type_expr) = &field.type_expr {
                if field_type_references_definition(type_expr, target_name) {
                    if field.source_file == "current file" {
                        references.push(format!("{}.{}", model_name, field.name));
                    } else {
                        references.push(format!(
                            "{}.{} (inherited from {})",
                            model_name, field.name, field.source_file
                        ));
                    }
                }
            }
        }
    }

    references
}

/// Check if a field's type expression references a specific definition
fn field_type_references_definition(type_expr: &str, definition_name: &str) -> bool {
    // Split on non-identifier characters and check for exact match
    // This handles: TypeName, TypeName[], "literal" | TypeName, etc.
    type_expr
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .any(|word| word == definition_name)
}
