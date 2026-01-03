//! Navigation features: hover, go-to-definition, and find references
//!
//! This module provides LSP navigation capabilities by analyzing the parsed CDM tree.

use tower_lsp::lsp_types::*;
use tree_sitter::{Node, Parser};

use super::position::{lsp_position_to_byte_offset, byte_span_to_lsp_range};

/// Find the symbol (identifier) at the given position in the document
pub fn find_symbol_at_position(text: &str, position: Position) -> Option<(String, Range)> {
    let byte_offset = lsp_position_to_byte_offset(text, position);
    
    // Parse the document
    let mut parser = Parser::new();
    parser.set_language(&grammar::LANGUAGE.into()).ok()?;
    let tree = parser.parse(text, None)?;
    
    // Find the node at the cursor position
    let root = tree.root_node();
    let node = find_node_at_offset(root, byte_offset)?;
    
    // We're interested in identifiers
    if node.kind() == "identifier" {
        let symbol_name = node.utf8_text(text.as_bytes()).ok()?.to_string();
        let range = byte_span_to_lsp_range(text, node.start_byte(), node.end_byte());
        return Some((symbol_name, range));
    }
    
    None
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

/// Extract all type and model definitions from a CDM document
pub fn extract_definitions(text: &str) -> Vec<(String, DefinitionInfo)> {
    let mut parser = Parser::new();
    if parser.set_language(&grammar::LANGUAGE.into()).is_err() {
        return Vec::new();
    }

    let tree = match parser.parse(text, None) {
        Some(t) => t,
        None => return Vec::new(),
    };

    let mut definitions = Vec::new();
    let root = tree.root_node();

    // Walk the tree to find type aliases and models
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "type_alias" => {
                if let Some(def) = extract_type_alias(child, text) {
                    definitions.push(def);
                }
            }
            "model_definition" => {
                if let Some(def) = extract_model(child, text) {
                    definitions.push(def);
                }
            }
            _ => {}
        }
    }

    definitions
}

#[derive(Debug, Clone)]
pub struct DefinitionInfo {
    #[allow(dead_code)] // Used in tests
    pub kind: DefinitionKind,
    pub range: Range,
    pub hover_text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DefinitionKind {
    TypeAlias,
    Model,
}

/// Extract type alias information from a type_alias_declaration node
fn extract_type_alias(node: Node, text: &str) -> Option<(String, DefinitionInfo)> {
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
            "type_expression" => {
                type_expr = child.utf8_text(text.as_bytes()).ok().map(|s| s.to_string());
            }
            _ => {}
        }
    }
    
    let name = name?;
    let type_expr = type_expr.unwrap_or_else(|| "string".to_string());
    let range = byte_span_to_lsp_range(text, node.start_byte(), node.end_byte());
    
    Some((name.clone(), DefinitionInfo {
        kind: DefinitionKind::TypeAlias,
        range,
        hover_text: format!("```cdm\n{}: {}\n```\n\nType alias", name, type_expr),
    }))
}

/// Extract model information from a model_declaration node
fn extract_model(node: Node, text: &str) -> Option<(String, DefinitionInfo)> {
    let mut cursor = node.walk();
    let mut name = None;
    let mut extends = Vec::new();
    let mut fields = Vec::new();
    
    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                if name.is_none() {
                    name = child.utf8_text(text.as_bytes()).ok().map(|s| s.to_string());
                }
            }
            "extends_clause" => {
                extends = extract_extends(child, text);
            }
            "model_body" => {
                fields = extract_fields(child, text);
            }
            _ => {}
        }
    }
    
    let name = name?;
    let range = byte_span_to_lsp_range(text, node.start_byte(), node.end_byte());
    
    // Build hover text
    let mut hover = format!("```cdm\n{}", name);
    if !extends.is_empty() {
        hover.push_str(" extends ");
        hover.push_str(&extends.join(", "));
    }
    hover.push_str(" {\n");
    for field in &fields {
        hover.push_str(&format!("  {}\n", field));
    }
    hover.push_str("}\n```\n\nModel definition");
    
    Some((name, DefinitionInfo {
        kind: DefinitionKind::Model,
        range,
        hover_text: hover,
    }))
}

/// Extract extends clause identifiers
fn extract_extends(node: Node, text: &str) -> Vec<String> {
    let mut extends = Vec::new();
    let mut cursor = node.walk();
    
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            if let Ok(name) = child.utf8_text(text.as_bytes()) {
                extends.push(name.to_string());
            }
        }
    }
    
    extends
}

/// Extract field definitions from model body
fn extract_fields(node: Node, text: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut cursor = node.walk();
    
    for child in node.children(&mut cursor) {
        if child.kind() == "field_declaration" {
            if let Ok(field_text) = child.utf8_text(text.as_bytes()) {
                // Clean up the field text (remove extra whitespace)
                let cleaned = field_text.trim();
                fields.push(cleaned.to_string());
            }
        }
    }
    
    fields
}

/// Find all references to a symbol in the document
pub fn find_all_references(text: &str, symbol: &str) -> Vec<Range> {
    let mut parser = Parser::new();
    if parser.set_language(&grammar::LANGUAGE.into()).is_err() {
        return Vec::new();
    }
    
    let tree = match parser.parse(text, None) {
        Some(t) => t,
        None => return Vec::new(),
    };
    
    let mut references = Vec::new();
    let root = tree.root_node();
    
    // Recursively search for identifiers matching the symbol
    find_identifiers_recursive(root, text, symbol, &mut references);
    
    references
}

fn find_identifiers_recursive(node: Node, text: &str, symbol: &str, references: &mut Vec<Range>) {
    if node.kind() == "identifier" {
        if let Ok(name) = node.utf8_text(text.as_bytes()) {
            if name == symbol {
                let range = byte_span_to_lsp_range(text, node.start_byte(), node.end_byte());
                references.push(range);
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        find_identifiers_recursive(child, text, symbol, references);
    }
}

#[cfg(test)]
#[path = "navigation/navigation_tests.rs"]
mod navigation_tests;
