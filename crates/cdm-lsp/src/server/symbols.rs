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
            "type_identifier" | "array_type" | "union_type" => {
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
mod tests {
    use super::*;

    #[test]
    fn test_type_alias_symbol() {
        let text = "Email: string #1";
        let symbols = compute_document_symbols(text).unwrap();

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "Email");
        assert_eq!(symbols[0].kind, SymbolKind::TYPE_PARAMETER);
        assert!(symbols[0].detail.is_some());
    }

    #[test]
    fn test_model_symbol_with_fields() {
        let text = r#"User {
  name: string #1
  email: Email #2
  age?: number #3
} #10"#;

        let symbols = compute_document_symbols(text).unwrap();

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "User");
        assert_eq!(symbols[0].kind, SymbolKind::CLASS);

        let children = symbols[0].children.as_ref().unwrap();
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].name, "name");
        assert_eq!(children[1].name, "email");
        assert_eq!(children[2].name, "age");

        // Check optional field detail
        assert!(children[2].detail.as_ref().unwrap().starts_with('?'));
    }

    #[test]
    fn test_model_with_extends() {
        let text = r#"AdminUser extends User, Timestamped {
  level: number #1
} #20"#;

        let symbols = compute_document_symbols(text).unwrap();

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "AdminUser");
        assert!(symbols[0].detail.as_ref().unwrap().contains("User"));
        assert!(symbols[0].detail.as_ref().unwrap().contains("Timestamped"));
    }

    #[test]
    fn test_field_removal() {
        let text = r#"AdminUser extends User {
  -password
  admin_level: number #1
} #20"#;

        let symbols = compute_document_symbols(text).unwrap();

        let children = symbols[0].children.as_ref().unwrap();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].name, "-password");
        assert_eq!(children[0].tags, Some(vec![SymbolTag::DEPRECATED]));
    }

    #[test]
    fn test_multiple_definitions() {
        let text = r#"Email: string #1

User {
  name: string #1
  email: Email #2
} #10

Admin extends User {
  level: number #1
} #11"#;

        let symbols = compute_document_symbols(text).unwrap();

        assert_eq!(symbols.len(), 3);
        assert_eq!(symbols[0].name, "Email");
        assert_eq!(symbols[0].kind, SymbolKind::TYPE_PARAMETER);
        assert_eq!(symbols[1].name, "User");
        assert_eq!(symbols[1].kind, SymbolKind::CLASS);
        assert_eq!(symbols[2].name, "Admin");
        assert_eq!(symbols[2].kind, SymbolKind::CLASS);
    }
}
