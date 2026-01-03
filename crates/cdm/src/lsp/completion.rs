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

/// Compute completion items for a position in a CDM document
pub fn compute_completions(text: &str, position: Position) -> Option<Vec<CompletionItem>> {
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
    /// Unknown context
    Unknown,
}

/// Determine what kind of completion is appropriate at the cursor position
fn determine_completion_context(root: Node, text: &str, offset: usize) -> Option<CompletionContext> {
    // Find the node at the cursor position
    let node = find_node_at_offset(root, offset)?;

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

#[cfg(test)]
#[path = "completion/completion_tests.rs"]
mod completion_tests;
