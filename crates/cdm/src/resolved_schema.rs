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
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use crate::{Ancestor, DefinitionKind, FieldInfo, SymbolTable};
use crate::template_resolver::extract_template_imports;
use crate::template_validation::validate_template_imports;
use crate::validate::validate_with_templates;
use cdm_utils::Span;
use cdm_plugin_interface::Schema;

// Re-export types from cdm-utils
pub use cdm_utils::{
    ParsedType, PrimitiveType, ResolvedSchema, ResolvedTypeAlias,
    ResolvedModel, ResolvedField, find_references_in_resolved
};

/// Load templates for a source file and return the namespaces.
fn load_templates_for_source(
    source: &str,
    source_path: &Path,
) -> Result<Vec<crate::ImportedNamespace>> {
    // Parse to extract template imports
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&grammar::LANGUAGE.into())
        .expect("Failed to load grammar");

    let parse_tree = match parser.parse(source, None) {
        Some(tree) => tree,
        None => return Ok(Vec::new()),
    };

    let template_imports = extract_template_imports(parse_tree.root_node(), source, source_path);

    // Validate template imports
    let template_diagnostics = validate_template_imports(&template_imports);
    if template_diagnostics.iter().any(|d| d.severity == crate::Severity::Error) {
        let errors: Vec<_> = template_diagnostics
            .iter()
            .filter(|d| d.severity == crate::Severity::Error)
            .map(|d| d.message.clone())
            .collect();
        anyhow::bail!("Template import errors: {}", errors.join("; "));
    }

    // Load template namespaces
    let project_root = source_path.parent().unwrap_or_else(|| Path::new("."));
    let mut load_diagnostics = Vec::new();
    let namespaces = crate::validate::load_template_namespaces(
        &template_imports,
        project_root,
        &mut load_diagnostics,
    );

    if load_diagnostics.iter().any(|d| d.severity == crate::Severity::Error) {
        let errors: Vec<_> = load_diagnostics
            .iter()
            .filter(|d| d.severity == crate::Severity::Error)
            .map(|d| d.message.clone())
            .collect();
        anyhow::bail!("Template loading errors: {}", errors.join("; "));
    }

    Ok(namespaces)
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
/// * `field_removals` - Map of model name to set of field names to remove
///
/// # Returns
/// A ResolvedSchema representing the final merged state
pub fn build_resolved_schema(
    current_symbols: &SymbolTable,
    current_fields: &HashMap<String, Vec<FieldInfo>>,
    ancestors: &[Ancestor],
    removals: &[(String, Span, &str)],
    field_removals: &HashMap<String, HashSet<String>>,
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

    // Add type aliases from imported template namespaces with qualified names
    // This allows qualified references like sql.UUID to resolve properly
    for (ns_name, namespace) in &current_symbols.namespaces {
        for (type_name, def) in &namespace.symbol_table.definitions {
            // Skip if this definition is removed
            let qualified_name = format!("{}.{}", ns_name, type_name);

            match &def.kind {
                DefinitionKind::TypeAlias { references, type_expr } => {
                    // Only add if not already present (local definitions take precedence)
                    if !resolved.type_aliases.contains_key(&qualified_name) {
                        resolved.type_aliases.insert(
                            qualified_name,
                            ResolvedTypeAlias {
                                name: type_name.clone(),
                                type_expr: type_expr.clone(),
                                references: references.clone(),
                                plugin_configs: def.plugin_configs.clone(),
                                source_file: namespace.template_path.display().to_string(),
                                source_span: def.span,
                                cached_parsed_type: RefCell::new(None),
                                entity_id: def.entity_id.clone(),
                            },
                        );
                    }
                }
                DefinitionKind::Model { .. } => {
                    // Models from templates are not typically used directly
                    // They would need to be referenced with qualified names
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
                // Check if this model already exists in resolved (from an ancestor)
                // Per spec Section 7.3, if a model exists in ancestors, the current file
                // MODIFIES it (merging fields and configs) rather than REPLACING it.
                let current_file_fields = current_fields.get(name).cloned().unwrap_or_default();
                let removed_fields = field_removals.get(name);

                if let Some(existing_model) = resolved.models.get(name) {
                    // Model exists in ancestor - MERGE modifications
                    let mut merged_fields = Vec::new();
                    let current_field_names: HashSet<_> = current_file_fields.iter().map(|f| &f.name).collect();

                    // Add ancestor fields (unless overridden or removed by current file)
                    for ancestor_field in &existing_model.fields {
                        // Skip if field is overridden by current file
                        if current_field_names.contains(&ancestor_field.name) {
                            continue;
                        }
                        // Skip if field is removed by current file
                        if let Some(removals) = removed_fields {
                            if removals.contains(&ancestor_field.name) {
                                continue;
                            }
                        }
                        // Keep ancestor field as-is
                        merged_fields.push(ancestor_field.clone());
                    }

                    // Add/override with current file fields
                    for f in &current_file_fields {
                        merged_fields.push(ResolvedField {
                            name: f.name.clone(),
                            type_expr: f.type_expr.clone(),
                            optional: f.optional,
                            default_value: f.default_value.clone(),
                            plugin_configs: f.plugin_configs.clone(),
                            source_file: "current file".to_string(),
                            source_span: f.span,
                            cached_parsed_type: RefCell::new(None),
                            entity_id: f.entity_id.clone(),
                        });
                    }

                    // Merge plugin configs: ancestor configs + current file configs (current overrides)
                    let mut merged_configs = existing_model.plugin_configs.clone();
                    for (key, value) in &def.plugin_configs {
                        merged_configs.insert(key.clone(), value.clone());
                    }

                    // Merge parents: combine unique parents from both
                    let mut merged_parents = existing_model.parents.clone();
                    for parent in extends {
                        if !merged_parents.contains(parent) {
                            merged_parents.push(parent.clone());
                        }
                    }

                    resolved.models.insert(
                        name.clone(),
                        ResolvedModel {
                            name: name.clone(),
                            fields: merged_fields,
                            parents: merged_parents,
                            plugin_configs: merged_configs,
                            // Use current file's entity_id if present, otherwise keep ancestor's
                            entity_id: def.entity_id.clone().or(existing_model.entity_id.clone()),
                            source_file: "current file".to_string(),
                            source_span: def.span,
                        },
                    );
                } else {
                    // Model doesn't exist in ancestors - new definition
                    resolved.models.insert(
                        name.clone(),
                        ResolvedModel {
                            name: name.clone(),
                            fields: current_file_fields
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
/// 3. Converting to the plugin API format with inherited fields flattened
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

        // Load templates for this ancestor
        let namespaces = load_templates_for_source(&source, ancestor_path)?;

        // Validate with templates
        let ancestor_result = validate_with_templates(&source, &ancestors, namespaces);
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
        &HashMap::new(), // No field removals when building schema for plugins
    );

    // Convert to plugin API Schema format
    let mut models = HashMap::new();
    for (name, model) in &resolved.models {
        // Use get_inherited_fields to collect all fields including inherited ones.
        // This reuses the existing inheritance resolution logic from symbol_table.
        let all_fields = crate::symbol_table::get_inherited_fields(
            name,
            &validation_result.model_fields,
            &validation_result.symbol_table,
            &ancestors,
        );

        // Deduplicate fields - child fields override parent fields with same name
        // Process in order and keep only the last occurrence of each field name
        let mut field_map: HashMap<String, &crate::FieldInfo> = HashMap::new();
        for field in &all_fields {
            field_map.insert(field.name.clone(), field);
        }

        // Convert fields to plugin API format, preserving order:
        // inherited fields first (in order), then own fields
        let mut seen_names = HashSet::new();
        let mut ordered_fields = Vec::new();

        for field in &all_fields {
            // Only add each field name once (last occurrence wins for properties,
            // but we want first occurrence for ordering)
            if !seen_names.contains(&field.name) {
                seen_names.insert(field.name.clone());

                // Use the final field from the map (which has correct overridden properties)
                if let Some(final_field) = field_map.get(&field.name) {
                    // Parse the type expression
                    let parsed_type = match &final_field.type_expr {
                        Some(type_str) => cdm_utils::parse_type_string(type_str).unwrap_or_else(|_| {
                            crate::ParsedType::Primitive(crate::PrimitiveType::String)
                        }),
                        None => crate::ParsedType::Primitive(crate::PrimitiveType::String),
                    };

                    ordered_fields.push(cdm_plugin_interface::FieldDefinition {
                        name: final_field.name.clone(),
                        field_type: convert_type_expression(&parsed_type),
                        optional: final_field.optional,
                        default: final_field.default_value.as_ref().map(|v| v.into()),
                        config: if plugin_name.is_empty() {
                            serde_json::to_value(&final_field.plugin_configs).unwrap_or(serde_json::json!({}))
                        } else {
                            final_field.plugin_configs.get(plugin_name).cloned().unwrap_or(serde_json::json!({}))
                        },
                        entity_id: final_field.entity_id.clone(),
                    });
                }
            }
        }

        models.insert(name.clone(), cdm_plugin_interface::ModelDefinition {
            name: name.clone(),
            parents: model.parents.clone(),
            fields: ordered_fields,
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
