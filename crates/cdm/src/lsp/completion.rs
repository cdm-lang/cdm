//! Code completion features
//!
//! This module provides intelligent code completion for CDM files:
//! - Built-in types (string, number, boolean, JSON)
//! - User-defined type aliases
//! - Model names
//! - Plugin names from @plugin directives
//! - Snippet templates for common patterns

use tower_lsp::lsp_types::*;
use tree_sitter::{Node, Parser};
use super::position::lsp_position_to_byte_offset;
use super::plugin_schema_cache::PluginSchemaCache;

/// Compute completion items for a position in a CDM document
///
/// # Arguments
/// * `text` - The document text
/// * `position` - The cursor position
/// * `plugin_cache` - Optional cache for plugin schemas (for plugin config completions)
/// * `document_uri` - Optional document URI (for resolving plugin paths)
pub fn compute_completions(
    text: &str,
    position: Position,
    plugin_cache: Option<&PluginSchemaCache>,
    document_uri: Option<&Url>,
) -> Option<Vec<CompletionItem>> {
    let byte_offset = lsp_position_to_byte_offset(text, position);

    // Parse the document
    let mut parser = Parser::new();
    parser.set_language(&grammar::LANGUAGE.into()).ok()?;
    let tree = parser.parse(text, None)?;

    let root = tree.root_node();

    // Determine the completion context
    let context = determine_completion_context(root, text, byte_offset)?;

    // Generate completions based on context
    let mut items = Vec::new();

    match context {
        CompletionContext::TypeExpression => {
            // Suggest built-in types
            items.extend(builtin_type_completions());

            // Suggest user-defined types and models
            items.extend(user_defined_type_completions(root, text));
        }
        CompletionContext::ExtendsClause => {
            // Suggest model names only
            items.extend(model_name_completions(root, text));
        }
        CompletionContext::PluginDirective => {
            // Suggest known plugin names
            items.extend(plugin_name_completions(root, text));
        }
        CompletionContext::TopLevel => {
            // Suggest snippets for new models and type aliases
            items.extend(snippet_completions());
        }
        CompletionContext::PluginConfigField { plugin_name, config_level } => {
            // Suggest plugin config field names
            if let (Some(cache), Some(uri)) = (plugin_cache, document_uri) {
                if let Some(schema) = cache.get_or_load(&plugin_name, uri, text) {
                    // Get cursor node for finding already-defined fields
                    if let Some(node) = find_node_at_offset(root, byte_offset) {
                        let already_defined = extract_already_defined_fields(node, text, byte_offset);
                        items.extend(plugin_field_completions(&schema, &config_level, &already_defined));
                    }
                }
            }
        }
        CompletionContext::PluginConfigValue { plugin_name, config_level, field_name } => {
            // Suggest plugin config field values
            if let (Some(cache), Some(uri)) = (plugin_cache, document_uri) {
                if let Some(schema) = cache.get_or_load(&plugin_name, uri, text) {
                    items.extend(plugin_value_completions(&schema, &config_level, &field_name));
                }
            }
        }
        CompletionContext::SuppressCompletions => {
            // Don't show any completions (e.g., immediately after comma in plugin config)
            return None;
        }
        CompletionContext::Unknown => {
            // Provide generic completions
            items.extend(builtin_type_completions());
            items.extend(snippet_completions());
        }
    }

    if items.is_empty() {
        None
    } else {
        Some(items)
    }
}

/// The context in which completion was triggered
#[derive(Debug, Clone, PartialEq)]
enum CompletionContext {
    /// Inside a type expression (field type, type alias)
    TypeExpression,
    /// Inside an extends clause
    ExtendsClause,
    /// Inside a @plugin directive
    PluginDirective,
    /// At the top level of the document
    TopLevel,
    /// Inside a plugin config object, suggesting field names
    /// e.g., @sql { | } or @sql { dialect: "postgres", | }
    PluginConfigField {
        plugin_name: String,
        config_level: PluginConfigLevel,
    },
    /// After a field name inside plugin config, suggesting values
    /// e.g., @sql { dialect: | }
    PluginConfigValue {
        plugin_name: String,
        config_level: PluginConfigLevel,
        field_name: String,
    },
    /// Suppress all completions (e.g., immediately after comma in plugin config)
    SuppressCompletions,
    /// Unknown context
    Unknown,
}

// Re-export ConfigLevel from plugin_validation for use in completions
pub use crate::plugin_validation::ConfigLevel as PluginConfigLevel;

/// Determine what kind of completion is appropriate at the cursor position
fn determine_completion_context(root: Node, text: &str, offset: usize) -> Option<CompletionContext> {
    // Find the node at the cursor position
    let node = find_node_at_offset(root, offset)?;

    // Check for plugin config context first (highest priority)
    if let Some(plugin_context) = detect_plugin_config_context(node, text, offset) {
        return Some(plugin_context);
    }

    // IMPORTANT: Check if we're after a colon first, before checking tree structure
    // This ensures that "name: " correctly detects as TypeExpression
    if is_after_colon(text, offset) {
        return Some(CompletionContext::TypeExpression);
    }

    // Walk up the tree to find the context
    let mut current = node;
    loop {
        match current.kind() {
            "type_expression" | "union_type" | "array_type" | "optional_type" => {
                return Some(CompletionContext::TypeExpression);
            }
            "extends_clause" => {
                return Some(CompletionContext::ExtendsClause);
            }
            "plugin_directive" => {
                return Some(CompletionContext::PluginDirective);
            }
            "source_file" => {
                // Check if we're at the top level (not inside any definition)
                if is_at_top_level(node, text) {
                    return Some(CompletionContext::TopLevel);
                }
                break;
            }
            _ => {}
        }

        if let Some(parent) = current.parent() {
            current = parent;
        } else {
            break;
        }
    }

    Some(CompletionContext::Unknown)
}

/// Check if the cursor is at the top level (not inside a model or field)
fn is_at_top_level(node: Node, text: &str) -> bool {
    // Check if there's only whitespace or comments before this position
    let start_byte = node.start_byte();
    let before_text = &text[..start_byte];

    // Simple heuristic: if we're not inside braces, we're at top level
    let open_braces = before_text.matches('{').count();
    let close_braces = before_text.matches('}').count();

    open_braces == close_braces
}

/// Check if the cursor is immediately after a colon (field type position)
fn is_after_colon(text: &str, offset: usize) -> bool {
    if offset == 0 {
        return false;
    }

    // Look backward for a colon, skipping whitespace
    let before = &text[..offset];
    let trimmed = before.trim_end();

    trimmed.ends_with(':')
}

/// Check if plugin config field completions should be shown at this position.
/// Returns false if cursor is immediately after a comma without a space/newline.
fn should_show_plugin_field_completions(text: &str, offset: usize) -> bool {
    if offset == 0 {
        return true;
    }

    // Look at the text before the cursor
    let before = &text[..offset];

    // Find the last non-whitespace character before cursor
    let trimmed = before.trim_end();
    if trimmed.is_empty() {
        return true;
    }

    // If the last non-whitespace char is a comma, check if there's whitespace after it
    if trimmed.ends_with(',') {
        // There must be at least one space or newline between the comma and cursor
        let comma_pos = trimmed.len() - 1;
        let after_comma = &before[comma_pos + 1..];
        // Need at least one whitespace character (space or newline) after comma
        return after_comma.chars().any(|c| c == ' ' || c == '\n' || c == '\r' || c == '\t');
    }

    true
}

/// Detect if cursor is inside a plugin config block and determine the context
fn detect_plugin_config_context(node: Node, text: &str, offset: usize) -> Option<CompletionContext> {
    // Walk up the tree to find if we're inside an object_literal within a plugin config
    let mut current = node;
    let mut in_object_literal = false;
    let mut object_literal_node: Option<Node> = None;
    let mut field_name_if_value_position: Option<String> = None;

    loop {
        match current.kind() {
            "object_literal" => {
                in_object_literal = true;
                object_literal_node = Some(current);
            }
            "object_entry" => {
                // Check if we're in the value position (after the colon)
                if let Some(key_node) = current.child_by_field_name("key") {
                    let key_end = key_node.end_byte();
                    if offset > key_end {
                        // Check if there's a colon between key and cursor
                        let text_between = &text[key_end..offset.min(current.end_byte())];
                        if text_between.contains(':') {
                            // We're in value position - extract the field name
                            if let Ok(key_text) = key_node.utf8_text(text.as_bytes()) {
                                field_name_if_value_position = Some(key_text.to_string());
                            }
                        }
                    }
                }
            }
            "plugin_config" => {
                // We found a plugin_config ancestor - this is what we're looking for
                if in_object_literal {
                    let plugin_name = extract_plugin_name_from_config(current, text)?;
                    let config_level = determine_config_level(current, text)?;

                    if let Some(field_name) = field_name_if_value_position {
                        return Some(CompletionContext::PluginConfigValue {
                            plugin_name,
                            config_level,
                            field_name,
                        });
                    } else {
                        // Only show field completions after comma+space/newline, not immediately after comma
                        if !should_show_plugin_field_completions(text, offset) {
                            return Some(CompletionContext::SuppressCompletions);
                        }
                        return Some(CompletionContext::PluginConfigField {
                            plugin_name,
                            config_level,
                        });
                    }
                }
            }
            "plugin_import" => {
                // Global plugin config in @plugin { ... } directive
                if in_object_literal {
                    let plugin_name = extract_plugin_name_from_import(current, text)?;

                    if let Some(field_name) = field_name_if_value_position {
                        return Some(CompletionContext::PluginConfigValue {
                            plugin_name,
                            config_level: PluginConfigLevel::Global,
                            field_name,
                        });
                    } else {
                        // Only show field completions after comma+space/newline, not immediately after comma
                        if !should_show_plugin_field_completions(text, offset) {
                            return Some(CompletionContext::SuppressCompletions);
                        }
                        return Some(CompletionContext::PluginConfigField {
                            plugin_name,
                            config_level: PluginConfigLevel::Global,
                        });
                    }
                }
            }
            "source_file" => break,
            _ => {}
        }

        if let Some(parent) = current.parent() {
            current = parent;
        } else {
            break;
        }
    }

    // Also check if we're right after an opening brace of a plugin config
    // e.g., @sql {| where cursor is right after {
    if let Some(obj_node) = object_literal_node {
        // Check if this object_literal's parent is plugin_config or plugin_import
        if let Some(parent) = obj_node.parent() {
            match parent.kind() {
                "plugin_config" => {
                    // Only show field completions after comma+space/newline, not immediately after comma
                    if !should_show_plugin_field_completions(text, offset) {
                        return Some(CompletionContext::SuppressCompletions);
                    }
                    let plugin_name = extract_plugin_name_from_config(parent, text)?;
                    let config_level = determine_config_level(parent, text)?;
                    return Some(CompletionContext::PluginConfigField {
                        plugin_name,
                        config_level,
                    });
                }
                "plugin_import" => {
                    // Only show field completions after comma+space/newline, not immediately after comma
                    if !should_show_plugin_field_completions(text, offset) {
                        return Some(CompletionContext::SuppressCompletions);
                    }
                    let plugin_name = extract_plugin_name_from_import(parent, text)?;
                    return Some(CompletionContext::PluginConfigField {
                        plugin_name,
                        config_level: PluginConfigLevel::Global,
                    });
                }
                _ => {}
            }
        }
    }

    None
}

/// Extract plugin name from a plugin_config node (e.g., @sql { ... })
fn extract_plugin_name_from_config(node: Node, text: &str) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(text.as_bytes()).ok())
        .map(|s| s.to_string())
}

/// Extract plugin name from a plugin_import node (e.g., @sql from "..." { ... })
fn extract_plugin_name_from_import(node: Node, text: &str) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(text.as_bytes()).ok())
        .map(|s| s.to_string())
}

/// Determine the config level by walking up from a plugin_config node
fn determine_config_level(plugin_config_node: Node, text: &str) -> Option<PluginConfigLevel> {
    let mut current = plugin_config_node.parent()?;
    let mut field_name: Option<String> = None;
    let mut model_name: Option<String> = None;

    loop {
        match current.kind() {
            "plugin_block" => {
                // Continue up to find what contains the plugin_block
                current = current.parent()?;
            }
            "type_alias" => {
                let name = current.child_by_field_name("name")
                    .and_then(|n| n.utf8_text(text.as_bytes()).ok())
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                return Some(PluginConfigLevel::TypeAlias { name });
            }
            "model_body" => {
                // Need to get the model name from the parent model_definition
                if let Some(model_def) = current.parent() {
                    if model_def.kind() == "model_definition" {
                        model_name = model_def.child_by_field_name("name")
                            .and_then(|n| n.utf8_text(text.as_bytes()).ok())
                            .map(|s| s.to_string());
                    }
                }
                // Direct child of model_body means model-level config
                let name = model_name.unwrap_or_default();
                return Some(PluginConfigLevel::Model { name });
            }
            "field_definition" | "field_override" => {
                // Get the field name
                field_name = current.child_by_field_name("name")
                    .and_then(|n| n.utf8_text(text.as_bytes()).ok())
                    .map(|s| s.to_string());
                // Continue up to find the model
                current = current.parent()?;
            }
            "model_definition" => {
                // We got here from a field - return field level
                if let Some(field) = field_name {
                    let model = current.child_by_field_name("name")
                        .and_then(|n| n.utf8_text(text.as_bytes()).ok())
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    return Some(PluginConfigLevel::Field { model, field });
                }
                current = current.parent()?;
            }
            "source_file" => {
                // Reached top without finding a specific context
                // This shouldn't happen for plugin_config, but handle gracefully
                break;
            }
            _ => {
                current = current.parent()?;
            }
        }
    }

    None
}

/// Extract already-defined field names in the current object_literal
fn extract_already_defined_fields(node: Node, text: &str, _offset: usize) -> std::collections::HashSet<String> {
    let mut defined = std::collections::HashSet::new();

    // Walk up to find the enclosing object_literal
    let mut current = node;
    let mut object_literal: Option<Node> = None;

    loop {
        if current.kind() == "object_literal" {
            object_literal = Some(current);
            break;
        }
        if let Some(parent) = current.parent() {
            current = parent;
        } else {
            break;
        }
    }

    // Extract field names from object_entry children
    if let Some(obj) = object_literal {
        let mut cursor = obj.walk();
        for child in obj.children(&mut cursor) {
            if child.kind() == "object_entry" {
                if let Some(key_node) = child.child_by_field_name("key") {
                    if let Ok(key_text) = key_node.utf8_text(text.as_bytes()) {
                        defined.insert(key_text.to_string());
                    }
                }
            }
        }
    }

    defined
}

/// Find the smallest node that contains the given byte offset
fn find_node_at_offset(node: Node, offset: usize) -> Option<Node> {
    // Check if this node contains the offset
    if offset < node.start_byte() || offset > node.end_byte() {
        return None;
    }

    // Try to find a child node that contains the offset
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = find_node_at_offset(child, offset) {
            return Some(found);
        }
    }

    // No child found, return this node
    Some(node)
}

/// Generate completion items for built-in types
fn builtin_type_completions() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "string".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Built-in string type".to_string()),
            documentation: Some(Documentation::String(
                "A string value".to_string()
            )),
            ..Default::default()
        },
        CompletionItem {
            label: "number".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Built-in number type".to_string()),
            documentation: Some(Documentation::String(
                "A numeric value (integer or float)".to_string()
            )),
            ..Default::default()
        },
        CompletionItem {
            label: "boolean".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Built-in boolean type".to_string()),
            documentation: Some(Documentation::String(
                "A boolean value (true or false)".to_string()
            )),
            ..Default::default()
        },
        CompletionItem {
            label: "JSON".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Built-in JSON type".to_string()),
            documentation: Some(Documentation::String(
                "Any valid JSON value".to_string()
            )),
            ..Default::default()
        },
    ]
}

/// Generate completion items for user-defined types and models
fn user_defined_type_completions(root: Node, text: &str) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        match child.kind() {
            "type_alias" => {
                if let Some(item) = extract_type_alias_completion(child, text) {
                    items.push(item);
                }
            }
            "model_definition" => {
                if let Some(item) = extract_model_completion(child, text) {
                    items.push(item);
                }
            }
            _ => {}
        }
    }

    items
}

/// Extract completion item for a type alias
fn extract_type_alias_completion(node: Node, text: &str) -> Option<CompletionItem> {
    let name = node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(text.as_bytes()).ok())?;

    let type_expr = node.child_by_field_name("type")
        .and_then(|n| n.utf8_text(text.as_bytes()).ok())
        .unwrap_or("string");

    Some(CompletionItem {
        label: name.to_string(),
        kind: Some(CompletionItemKind::TYPE_PARAMETER),
        detail: Some(format!("Type alias: {}", type_expr)),
        documentation: Some(Documentation::String(
            format!("{}: {}", name, type_expr)
        )),
        ..Default::default()
    })
}

/// Extract completion item for a model
fn extract_model_completion(node: Node, text: &str) -> Option<CompletionItem> {
    let name = node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(text.as_bytes()).ok())?;

    // Get field count for documentation
    let field_count = if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        body.children(&mut cursor)
            .filter(|n| n.kind() == "field_definition")
            .count()
    } else {
        0
    };

    Some(CompletionItem {
        label: name.to_string(),
        kind: Some(CompletionItemKind::CLASS),
        detail: Some(format!("Model with {} field(s)", field_count)),
        documentation: Some(Documentation::String(
            format!("Model: {}", name)
        )),
        ..Default::default()
    })
}

/// Generate completion items for model names (for extends clause)
fn model_name_completions(root: Node, text: &str) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        if child.kind() == "model_definition" {
            if let Some(item) = extract_model_completion(child, text) {
                items.push(item);
            }
        }
    }

    items
}

/// Generate completion items for plugin names
fn plugin_name_completions(root: Node, text: &str) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut cursor = root.walk();

    // Extract plugin names from @plugin directives
    for child in root.children(&mut cursor) {
        if child.kind() == "plugin_directive" {
            if let Some(name_node) = child.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(text.as_bytes()) {
                    if seen.insert(name.to_string()) {
                        items.push(CompletionItem {
                            label: name.to_string(),
                            kind: Some(CompletionItemKind::MODULE),
                            detail: Some("Plugin".to_string()),
                            ..Default::default()
                        });
                    }
                }
            }
        }
    }

    items
}

/// Generate snippet completions for common patterns
fn snippet_completions() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "model".to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("New model definition".to_string()),
            insert_text: Some("${1:ModelName} {\n  ${2:field}: ${3:string} #1\n} #10".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            documentation: Some(Documentation::String(
                "Create a new model with a field".to_string()
            )),
            ..Default::default()
        },
        CompletionItem {
            label: "type".to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("New type alias".to_string()),
            insert_text: Some("${1:TypeName}: ${2:string} #1".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            documentation: Some(Documentation::String(
                "Create a new type alias".to_string()
            )),
            ..Default::default()
        },
        CompletionItem {
            label: "extends".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("Extend a model".to_string()),
            insert_text: Some("extends ${1:BaseModel}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            documentation: Some(Documentation::String(
                "Extend another model".to_string()
            )),
            ..Default::default()
        },
    ]
}

/// Generate completion items for plugin config fields
///
/// # Arguments
/// * `schema` - The plugin's settings schema (containing GlobalSettings, ModelSettings, etc.)
/// * `config_level` - Which settings model to use (Global, TypeAlias, Model, Field)
/// * `already_defined` - Field names already present in the current config block
pub fn plugin_field_completions(
    schema: &super::plugin_schema_cache::PluginSettingsSchema,
    config_level: &PluginConfigLevel,
    already_defined: &std::collections::HashSet<String>,
) -> Vec<CompletionItem> {
    let mut items: Vec<CompletionItem> = Vec::new();

    // For Global level, add reserved settings (version, build_output, migrations_output)
    if matches!(config_level, PluginConfigLevel::Global) {
        items.extend(reserved_global_settings_completions(schema, already_defined));
    }

    // Add plugin-defined fields
    let fields = schema.fields_for_level(config_level);

    items.extend(fields
        .iter()
        .filter(|field| !already_defined.contains(&field.name))
        .map(|field| {
            let detail = format_field_detail(field);
            let documentation = format_field_documentation(field);
            let insert_text = format_field_insert_text(field);

            CompletionItem {
                label: field.name.clone(),
                kind: Some(CompletionItemKind::PROPERTY),
                detail: Some(detail),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: documentation,
                })),
                insert_text: Some(insert_text),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            }
        }));

    items
}

/// Generate completion items for reserved global settings
/// These are settings handled by CDM itself, not passed to the plugin:
/// - version: always shown
/// - build_output: shown if plugin has _build function
/// - migrations_output: shown if plugin has _migrate function
fn reserved_global_settings_completions(
    schema: &super::plugin_schema_cache::PluginSettingsSchema,
    already_defined: &std::collections::HashSet<String>,
) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    // version - always available
    if !already_defined.contains("version") {
        items.push(CompletionItem {
            label: "version".to_string(),
            kind: Some(CompletionItemKind::PROPERTY),
            detail: Some("version?: string".to_string()),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "**Type:** `string`\n\n*Optional*\n\nVersion constraint for plugin resolution (e.g., \"1.0.0\", \"^1.2.3\")".to_string(),
            })),
            insert_text: Some("version: \"$1\"".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
    }

    // build_output - only if plugin has _build function
    if schema.has_build && !already_defined.contains("build_output") {
        items.push(CompletionItem {
            label: "build_output".to_string(),
            kind: Some(CompletionItemKind::PROPERTY),
            detail: Some("build_output?: string".to_string()),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "**Type:** `string`\n\n*Optional*\n\nOutput directory for generated files from `cdm build`".to_string(),
            })),
            insert_text: Some("build_output: \"$1\"".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
    }

    // migrations_output - only if plugin has _migrate function
    if schema.has_migrate && !already_defined.contains("migrations_output") {
        items.push(CompletionItem {
            label: "migrations_output".to_string(),
            kind: Some(CompletionItemKind::PROPERTY),
            detail: Some("migrations_output?: string".to_string()),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "**Type:** `string`\n\n*Optional*\n\nOutput directory for migration files from `cdm migrate`".to_string(),
            })),
            insert_text: Some("migrations_output: \"$1\"".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
    }

    items
}

/// Generate completion items for plugin config field values
///
/// # Arguments
/// * `schema` - The plugin's settings schema
/// * `config_level` - Which settings model to use
/// * `field_name` - The field name we're providing values for
pub fn plugin_value_completions(
    schema: &super::plugin_schema_cache::PluginSettingsSchema,
    config_level: &PluginConfigLevel,
    field_name: &str,
) -> Vec<CompletionItem> {
    let fields = schema.fields_for_level(config_level);

    let field = match fields.iter().find(|f| f.name == field_name) {
        Some(f) => f,
        None => return Vec::new(),
    };

    let mut items = Vec::new();

    // Add enum values from pre-parsed literal values
    for value in &field.literal_values {
        items.push(CompletionItem {
            label: format!("\"{}\"", value),
            kind: Some(CompletionItemKind::ENUM_MEMBER),
            detail: Some(format!("Value for {}", field_name)),
            insert_text: Some(format!("\"{}\"", value)),
            ..Default::default()
        });
    }

    // Add boolean completions if field is boolean type
    if field.is_boolean {
        items.push(CompletionItem {
            label: "true".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some(format!("Boolean value for {}", field_name)),
            ..Default::default()
        });
        items.push(CompletionItem {
            label: "false".to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some(format!("Boolean value for {}", field_name)),
            ..Default::default()
        });
    }

    items
}

/// Format field detail string (type info)
fn format_field_detail(field: &super::plugin_schema_cache::SettingsField) -> String {
    let type_str = field.type_expr.as_deref().unwrap_or("string");
    let optional_marker = if field.optional { "?" } else { "" };
    format!("{}{}: {}", field.name, optional_marker, type_str)
}

/// Format field documentation (markdown)
fn format_field_documentation(field: &super::plugin_schema_cache::SettingsField) -> String {
    let mut doc = String::new();

    // Type info
    let type_str = field.type_expr.as_deref().unwrap_or("string");
    doc.push_str(&format!("**Type:** `{}`\n\n", type_str));

    // Optionality
    if field.optional {
        doc.push_str("*Optional*\n\n");
    } else {
        doc.push_str("*Required*\n\n");
    }

    // Default value
    if let Some(default) = &field.default_value {
        doc.push_str(&format!("**Default:** `{}`", default));
    }

    doc
}

/// Format field insert text as a snippet
fn format_field_insert_text(field: &super::plugin_schema_cache::SettingsField) -> String {
    // Provide a snippet with a placeholder for the value
    format!("{}: $1", field.name)
}

#[cfg(test)]
#[path = "completion/completion_tests.rs"]
mod completion_tests;
