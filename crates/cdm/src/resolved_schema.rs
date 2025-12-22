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
use std::cell::RefCell;
use crate::{Ancestor, DefinitionKind, FieldInfo, SymbolTable};
use cdm_utils::Span;

// Re-export types from cdm-utils
pub use cdm_utils::{
    ParsedType, PrimitiveType, ResolvedSchema, ResolvedTypeAlias,
    ResolvedModel, ResolvedField, find_references_in_resolved
};

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
                            cached_parsed_type: RefCell::new(None),
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
                                        cached_parsed_type: RefCell::new(None),
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
                        cached_parsed_type: RefCell::new(None),
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
                                    cached_parsed_type: RefCell::new(None),
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
