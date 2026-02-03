//! Document symbols for outline view
//!
//! This module provides document symbols for the LSP outline/breadcrumb view,
//! showing the structure of CDM files with models, fields, and type aliases.

use tower_lsp::lsp_types::*;
use tree_sitter::{Node, Parser};

use super::position::byte_span_to_lsp_range;

/// Compute document symbols for the given CDM document
pub fn compute_document_symbols(text: &str) -> Option<Vec<DocumentSymbol>> {
    let mut parser = Parser::new();
    parser.set_language(&grammar::LANGUAGE.into()).ok()?;
    let tree = parser.parse(text, None)?;

    let mut symbols = Vec::new();
    let root = tree.root_node();

    // Walk the tree to find type aliases and models
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "type_alias" => {
                if let Some(symbol) = extract_type_alias_symbol(child, text) {
                    symbols.push(symbol);
                }
            }
            "model_definition" => {
                if let Some(symbol) = extract_model_symbol(child, text) {
                    symbols.push(symbol);
                }
            }
            _ => {}
        }
    }

    Some(symbols)
}

/// Extract a DocumentSymbol from a type_alias node
fn extract_type_alias_symbol(node: Node, text: &str) -> Option<DocumentSymbol> {
    let mut cursor = node.walk();
    let mut name = None;
    let mut type_expr = None;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                if name.is_none() {
                    name = child.utf8_text(text.as_bytes()).ok().map(|s| s.to_string());
                }
            }
            _ if child.is_named() && type_expr.is_none() && name.is_some() => {
                // Get the type expression text
                type_expr = child.utf8_text(text.as_bytes()).ok().map(|s| s.to_string());
            }
            _ => {}
        }
    }

    let name = name?;
    let detail = type_expr.map(|t| format!(": {}", t));

    let range = byte_span_to_lsp_range(text, node.start_byte(), node.end_byte());
    let selection_range = range; // For type aliases, the whole line is the selection

    #[allow(deprecated)]
    Some(DocumentSymbol {
        name,
        detail,
        kind: SymbolKind::TYPE_PARAMETER, // Use TypeParameter for type aliases
        tags: None,
        deprecated: None,
        range,
        selection_range,
        children: None,
    })
}

/// Extract a DocumentSymbol from a model_definition node
fn extract_model_symbol(node: Node, text: &str) -> Option<DocumentSymbol> {
    let mut cursor = node.walk();
    let mut name = None;
    let mut name_node = None;
    let mut model_body = None;
    let mut extends = Vec::new();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                if name.is_none() {
                    name = child.utf8_text(text.as_bytes()).ok().map(|s| s.to_string());
                    name_node = Some(child);
                }
            }
            "extends_clause" => {
                extends = extract_extends(child, text);
            }
            "model_body" => {
                model_body = Some(child);
            }
            _ => {}
        }
    }

    let name = name?;
    let name_node = name_node?;

    // Build detail string with extends clause
    let detail = if extends.is_empty() {
        None
    } else {
        Some(format!("extends {}", extends.join(", ")))
    };

    let range = byte_span_to_lsp_range(text, node.start_byte(), node.end_byte());
    let selection_range = byte_span_to_lsp_range(text, name_node.start_byte(), name_node.end_byte());

    // Extract fields as children
    let children = model_body.and_then(|body| {
        let fields = extract_field_symbols(body, text);
        if fields.is_empty() {
            None
        } else {
            Some(fields)
        }
    });

    #[allow(deprecated)]
    Some(DocumentSymbol {
        name,
        detail,
        kind: SymbolKind::CLASS, // Use Class for models
        tags: None,
        deprecated: None,
        range,
        selection_range,
        children,
    })
}

/// Extract parent model names from extends_clause
fn extract_extends(node: Node, text: &str) -> Vec<String> {
    let mut parents = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            if let Ok(parent) = child.utf8_text(text.as_bytes()) {
                parents.push(parent.to_string());
            }
        }
    }

    parents
}

/// Extract field symbols from a model_body node
fn extract_field_symbols(body_node: Node, text: &str) -> Vec<DocumentSymbol> {
    let mut fields = Vec::new();
    let mut cursor = body_node.walk();

    for child in body_node.children(&mut cursor) {
        match child.kind() {
            "field_definition" => {
                if let Some(field) = extract_field_symbol(child, text) {
                    fields.push(field);
                }
            }
            "field_removal" => {
                if let Some(field) = extract_field_removal_symbol(child, text) {
                    fields.push(field);
                }
            }
            _ => {}
        }
    }

    fields
}

/// Extract a field symbol from a field_definition node
fn extract_field_symbol(node: Node, text: &str) -> Option<DocumentSymbol> {
    let mut cursor = node.walk();
    let mut name = None;
    let mut name_node = None;
    let mut type_expr = None;
    let mut is_optional = false;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                if name.is_none() {
                    name = child.utf8_text(text.as_bytes()).ok().map(|s| s.to_string());
                    name_node = Some(child);
                }
            }
            "?" => {
                is_optional = true;
            }
            "type_identifier" | "array_type" | "union_type" | "map_type" => {
                if type_expr.is_none() {
                    type_expr = child.utf8_text(text.as_bytes()).ok().map(|s| s.to_string());
                }
            }
            _ => {}
        }
    }

    let name = name?;
    let name_node = name_node?;

    // Build detail string with type and optional marker
    let detail = match (type_expr, is_optional) {
        (Some(t), true) => Some(format!("?: {}", t)),
        (Some(t), false) => Some(format!(": {}", t)),
        (None, true) => Some("?".to_string()),
        (None, false) => None,
    };

    let range = byte_span_to_lsp_range(text, node.start_byte(), node.end_byte());
    let selection_range = byte_span_to_lsp_range(text, name_node.start_byte(), name_node.end_byte());

    #[allow(deprecated)]
    Some(DocumentSymbol {
        name,
        detail,
        kind: SymbolKind::FIELD,
        tags: None,
        deprecated: None,
        range,
        selection_range,
        children: None,
    })
}

/// Extract a field removal symbol from a field_removal node
fn extract_field_removal_symbol(node: Node, text: &str) -> Option<DocumentSymbol> {
    let mut cursor = node.walk();
    let mut name = None;
    let mut name_node = None;

    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            name = child.utf8_text(text.as_bytes()).ok().map(|s| s.to_string());
            name_node = Some(child);
            break;
        }
    }

    let name = format!("-{}", name?);
    let name_node = name_node?;

    let range = byte_span_to_lsp_range(text, node.start_byte(), node.end_byte());
    let selection_range = byte_span_to_lsp_range(text, name_node.start_byte(), name_node.end_byte());

    #[allow(deprecated)]
    Some(DocumentSymbol {
        name,
        detail: Some("(removed)".to_string()),
        kind: SymbolKind::FIELD,
        tags: Some(vec![SymbolTag::DEPRECATED]),
        deprecated: Some(true),
        range,
        selection_range,
        children: None,
    })
}

#[cfg(test)]
#[path = "symbols/symbols_tests.rs"]
mod symbols_tests;
