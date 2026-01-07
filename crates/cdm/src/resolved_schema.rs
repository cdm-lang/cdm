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
use std::fs;
use std::path::PathBuf;
use anyhow::{Context, Result};
use crate::{Ancestor, DefinitionKind, FieldInfo, SymbolTable};
use cdm_utils::Span;
use cdm_plugin_interface::Schema;

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
                            plugin_configs: def.plugin_configs.clone(),
                            source_file: ancestor.path.clone(),
                            source_span: def.span,
                            cached_parsed_type: RefCell::new(None),
                            entity_id: def.entity_id.clone(),
                        },
                    );
                }
                DefinitionKind::Model { extends } => {
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
                                        default_value: f.default_value.clone(),
                                        plugin_configs: f.plugin_configs.clone(),
                                        source_file: ancestor.path.clone(),
                                        source_span: f.span,
                                        cached_parsed_type: RefCell::new(None),
                                        entity_id: f.entity_id.clone(),
                                    })
                                    .collect(),
                                parents: extends.clone(),
                                plugin_configs: def.plugin_configs.clone(),
                                source_file: ancestor.path.clone(),
                                source_span: def.span,
                                entity_id: def.entity_id.clone(),
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
                        plugin_configs: def.plugin_configs.clone(),
                        source_file: "current file".to_string(),
                        source_span: def.span,
                        cached_parsed_type: RefCell::new(None),
                        entity_id: def.entity_id.clone(),
                    },
                );
            }
            DefinitionKind::Model { extends } => {
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
                                    default_value: f.default_value.clone(),
                                    plugin_configs: f.plugin_configs.clone(),
                                    source_file: "current file".to_string(),
                                    source_span: f.span,
                                    cached_parsed_type: RefCell::new(None),
                                    entity_id: f.entity_id.clone(),
                                })
                                .collect(),
                            parents: extends.clone(),
                            plugin_configs: def.plugin_configs.clone(),
                            entity_id: def.entity_id.clone(),
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

/// Build the Schema structure from validation result for a specific plugin.
///
/// This function creates a plugin-compatible schema by:
/// 1. Building ancestors from their file paths
/// 2. Creating a resolved schema that merges inheritance
/// 3. Converting to the plugin API format
///
/// # Arguments
/// * `validation_result` - The validation result containing symbol table and fields
/// * `ancestor_paths` - Paths to ancestor CDM files
/// * `plugin_name` - Name of the plugin to extract configs for (empty string for all configs)
///
/// # Returns
/// A Schema in the plugin API format
pub fn build_cdm_schema_for_plugin(
    validation_result: &crate::validate::ValidationResult,
    ancestor_paths: &[PathBuf],
    plugin_name: &str,
) -> Result<Schema> {
    // Build ancestors for resolved schema
    let mut ancestors = Vec::new();
    for ancestor_path in ancestor_paths {
        let source = fs::read_to_string(ancestor_path)
            .with_context(|| format!("Failed to read ancestor file: {}", ancestor_path.display()))?;
        let ancestor_result = crate::validate(&source, &ancestors);
        if ancestor_result.has_errors() {
            anyhow::bail!("Ancestor file has validation errors: {}", ancestor_path.display());
        }
        ancestors.push(ancestor_result.into_ancestor(ancestor_path.display().to_string()));
    }

    // Build resolved schema (merging inheritance)
    let resolved = build_resolved_schema(
        &validation_result.symbol_table,
        &validation_result.model_fields,
        &ancestors,
        &[],
    );

    // Convert to plugin API Schema format
    let mut models = HashMap::new();
    for (name, model) in resolved.models {
        models.insert(name.clone(), cdm_plugin_interface::ModelDefinition {
            name: name.clone(),
            parents: model.parents,
            fields: model.fields.iter().map(|f| {
                // Parse the type expression
                let parsed_type = f.parsed_type().unwrap_or_else(|_| {
                    // Default to string if parsing fails
                    crate::ParsedType::Primitive(crate::PrimitiveType::String)
                });

                cdm_plugin_interface::FieldDefinition {
                    name: f.name.clone(),
                    field_type: convert_type_expression(&parsed_type),
                    optional: f.optional,
                    default: f.default_value.as_ref().map(|v| v.into()),
                    config: if plugin_name.is_empty() {
                        // For schema storage, include all plugin configs as a JSON object
                        serde_json::to_value(&f.plugin_configs).unwrap_or(serde_json::json!({}))
                    } else {
                        // For plugin execution, get this plugin's config
                        f.plugin_configs.get(plugin_name).cloned().unwrap_or(serde_json::json!({}))
                    },
                    entity_id: f.entity_id.clone(),
                }
            }).collect(),
            config: if plugin_name.is_empty() {
                serde_json::to_value(&model.plugin_configs).unwrap_or(serde_json::json!({}))
            } else {
                model.plugin_configs.get(plugin_name).cloned().unwrap_or(serde_json::json!({}))
            },
            entity_id: model.entity_id.clone(),
        });
    }

    let mut type_aliases = HashMap::new();
    for (name, alias) in resolved.type_aliases {
        // Parse the type expression
        let parsed_type = alias.parsed_type().unwrap_or_else(|_| {
            // Default to string if parsing fails
            crate::ParsedType::Primitive(crate::PrimitiveType::String)
        });

        type_aliases.insert(name.clone(), cdm_plugin_interface::TypeAliasDefinition {
            name: name.clone(),
            alias_type: convert_type_expression(&parsed_type),
            config: if plugin_name.is_empty() {
                serde_json::to_value(&alias.plugin_configs).unwrap_or(serde_json::json!({}))
            } else {
                alias.plugin_configs.get(plugin_name).cloned().unwrap_or(serde_json::json!({}))
            },
            entity_id: alias.entity_id.clone(),
        });
    }

    Ok(Schema {
        models,
        type_aliases,
    })
}

/// Convert internal ParsedType to plugin API TypeExpression
pub fn convert_type_expression(parsed_type: &crate::ParsedType) -> cdm_plugin_interface::TypeExpression {
    use crate::{ParsedType, PrimitiveType};

    match parsed_type {
        ParsedType::Primitive(prim) => {
            let name = match prim {
                PrimitiveType::String => "string",
                PrimitiveType::Number => "number",
                PrimitiveType::Boolean => "boolean",
            };
            cdm_plugin_interface::TypeExpression::Identifier {
                name: name.to_string()
            }
        }
        ParsedType::Reference(name) => {
            cdm_plugin_interface::TypeExpression::Identifier {
                name: name.clone()
            }
        }
        ParsedType::Array(inner) => {
            cdm_plugin_interface::TypeExpression::Array {
                element_type: Box::new(convert_type_expression(inner))
            }
        }
        ParsedType::Union(members) => {
            cdm_plugin_interface::TypeExpression::Union {
                types: members.iter().map(convert_type_expression).collect()
            }
        }
        ParsedType::Literal(value) => {
            cdm_plugin_interface::TypeExpression::StringLiteral {
                value: value.clone()
            }
        }
        ParsedType::Null => {
            cdm_plugin_interface::TypeExpression::Identifier {
                name: "null".to_string()
            }
        }
    }
}

#[cfg(test)]
#[path = "resolved_schema/resolved_schema_tests.rs"]
mod resolved_schema_tests;
