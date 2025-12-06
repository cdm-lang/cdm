use crate::{Definition, DefinitionKind, Diagnostic, Position, Severity, Span, SymbolTable};

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct ValidationResult {
    pub diagnostics: Vec<Diagnostic>,
    // Include the tree for callers that need it (e.g., code generation)
    pub tree: Option<tree_sitter::Tree>,
}

impl ValidationResult {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity == Severity::Error)
    }
}

/// Validate CDM source code and return all diagnostics.
/// 
/// This performs both syntactic (parse errors) and semantic 
/// (undefined types, etc.) validation.
pub fn validate(source: &str) -> ValidationResult {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    // Parse
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&grammar::LANGUAGE.into())
        .expect("Failed to load grammar");

    let tree = match parser.parse(source, None) {
        Some(tree) => tree,
        None => {
            diagnostics.push(Diagnostic {
                message: "Failed to parse file".to_string(),
                severity: Severity::Error,
                span: Span {
                    start: Position { line: 0, column: 0 },
                    end: Position { line: 0, column: 0 },
                },
            });
            return ValidationResult { diagnostics, tree: None };
        }
    };

    // Collect syntax errors from tree-sitter
    collect_syntax_errors(tree.root_node(), source, &mut diagnostics);

    // Semantic validation
    collect_semantic_errors(tree.root_node(), source, &mut diagnostics);

    ValidationResult {
        diagnostics,
        tree: Some(tree),
    }
}

fn collect_syntax_errors(
    node: tree_sitter::Node,
    source: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if node.is_error() || node.is_missing() {
        let start = node.start_position();
        let end = node.end_position();
        let text = node.utf8_text(source.as_bytes()).unwrap_or("<invalid>");
        
        diagnostics.push(Diagnostic {
            message: format!("Syntax error: unexpected '{}'", text),
            severity: Severity::Error,
            span: Span {
                start: Position { line: start.row, column: start.column },
                end: Position { line: end.row, column: end.column },
            },
        });
    }

    for child in node.children(&mut node.walk()) {
        collect_syntax_errors(child, source, diagnostics);
    }
}

fn get_node_text<'a>(node: tree_sitter::Node, source: &'a str) -> &'a str {
    node.utf8_text(source.as_bytes()).unwrap_or("")
}

fn node_span(node: tree_sitter::Node) -> Span {
    let start = node.start_position();
    let end = node.end_position();
    Span {
        start: Position { line: start.row, column: start.column },
        end: Position { line: end.row, column: end.column },
    }
}

/// Pass 1: Collect all definitions into a symbol table
fn collect_definitions(
    root: tree_sitter::Node,
    source: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> SymbolTable {
    let mut symbol_table = SymbolTable::new();
    let mut cursor = root.walk();

    for node in root.children(&mut cursor) {
        match node.kind() {
            "type_alias" => {
                collect_type_alias(node, source, &mut symbol_table, diagnostics);
            }
            "model_definition" => {
                collect_model(node, source, &mut symbol_table, diagnostics);
            }
            _ => {}
        }
    }

    symbol_table
}

fn collect_type_alias(
    node: tree_sitter::Node,
    source: &str,
    symbol_table: &mut SymbolTable,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(name_node) = node.child_by_field_name("name") else {
        return;
    };

    let name = get_node_text(name_node, source);
    let span = node_span(name_node);

    // Check for duplicate definition
    if let Some(existing) = symbol_table.definitions.get(name) {
        diagnostics.push(Diagnostic {
            message: format!(
                "'{}' is already defined at line {}",
                name,
                existing.span.start.line + 1
            ),
            severity: Severity::Error,
            span,
        });
        return;
    }

    symbol_table.definitions.insert(
        name.to_string(),
        Definition {
            kind: DefinitionKind::TypeAlias,
            span,
        },
    );
}

fn collect_model(
    node: tree_sitter::Node,
    source: &str,
    symbol_table: &mut SymbolTable,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(name_node) = node.child_by_field_name("name") else {
        return;
    };

    let name = get_node_text(name_node, source);
    let span = node_span(name_node);

    // Check for duplicate definition
    if let Some(existing) = symbol_table.definitions.get(name) {
        diagnostics.push(Diagnostic {
            message: format!(
                "'{}' is already defined at line {}",
                name,
                existing.span.start.line + 1
            ),
            severity: Severity::Error,
            span,
        });
        return;
    }

    // Collect extends parents
    let extends = collect_extends_parents(node, source);

    symbol_table.definitions.insert(
        name.to_string(),
        Definition {
            kind: DefinitionKind::Model { extends },
            span,
        },
    );
}

fn collect_extends_parents(node: tree_sitter::Node, source: &str) -> Vec<String> {
    let mut parents = Vec::new();

    let Some(extends_node) = node.child_by_field_name("extends") else {
        return parents;
    };

    // extends_clause can have multiple "parent" fields
    let mut cursor = extends_node.walk();
    for child in extends_node.children_by_field_name("parent", &mut cursor) {
        let parent_name = get_node_text(child, source);
        parents.push(parent_name.to_string());
    }

    parents
}

fn collect_semantic_errors(
    root: tree_sitter::Node,
    source: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Pass 1: Build symbol table
    let symbol_table = collect_definitions(root, source, diagnostics);

    // Pass 2: Validate references (TODO)
    validate_references(root, source, &symbol_table, diagnostics);
}

fn validate_references(
    root: tree_sitter::Node,
    source: &str,
    symbol_table: &SymbolTable,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut cursor = root.walk();

    for node in root.children(&mut cursor) {
        match node.kind() {
            "type_alias" => {
                validate_type_alias(node, source, symbol_table, diagnostics);
            }
            "model_definition" => {
                validate_model(node, source, symbol_table, diagnostics);
            }
            _ => {}
        }
    }
}

fn validate_type_alias(
    node: tree_sitter::Node,
    source: &str,
    symbol_table: &SymbolTable,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Check the type expression on the right side of the alias
    let Some(type_node) = node.child_by_field_name("type") else {
        return;
    };

    validate_type_expression(type_node, source, symbol_table, diagnostics);
}

fn validate_model(
    node: tree_sitter::Node,
    source: &str,
    symbol_table: &SymbolTable,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Validate extends clause
    if let Some(extends_node) = node.child_by_field_name("extends") {
        validate_extends(extends_node, source, symbol_table, diagnostics);
    }

    // Check field types in the model body
    let Some(body_node) = node.child_by_field_name("body") else {
        return;
    };

    let mut cursor = body_node.walk();

    for child in body_node.children(&mut cursor) {
        if child.kind() == "field_definition" {
            validate_field(child, source, symbol_table, diagnostics);
        }
    }
}

fn validate_extends(
    extends_node: tree_sitter::Node,
    source: &str,
    symbol_table: &SymbolTable,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut cursor = extends_node.walk();

    for parent_node in extends_node.children_by_field_name("parent", &mut cursor) {
        let parent_name = get_node_text(parent_node, source);

        match symbol_table.get(parent_name) {
            None => {
                diagnostics.push(Diagnostic {
                    message: format!("Undefined type '{}' in extends clause", parent_name),
                    severity: Severity::Error,
                    span: node_span(parent_node),
                });
            }
            Some(def) => {
                if matches!(def.kind, DefinitionKind::TypeAlias) {
                    diagnostics.push(Diagnostic {
                        message: format!(
                            "Cannot extend '{}': it is a type alias, not a model",
                            parent_name
                        ),
                        severity: Severity::Error,
                        span: node_span(parent_node),
                    });
                }
            }
        }
    }
}

fn validate_field(
    node: tree_sitter::Node,
    source: &str,
    symbol_table: &SymbolTable,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Field might not have a type (untyped fields default to string)
    let Some(type_node) = node.child_by_field_name("type") else {
        return;
    };

    validate_type_expression(type_node, source, symbol_table, diagnostics);
}

fn validate_type_expression(
    node: tree_sitter::Node,
    source: &str,
    symbol_table: &SymbolTable,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match node.kind() {
        "type_identifier" => {
            let type_name = get_node_text(node, source);
            if !symbol_table.is_defined(type_name) {
                diagnostics.push(Diagnostic {
                    message: format!("Undefined type '{}'", type_name),
                    severity: Severity::Error,
                    span: node_span(node),
                });
            }
        }
        "array_type" => {
            // Array type has a type_identifier child
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "type_identifier" {
                    validate_type_expression(child, source, symbol_table, diagnostics);
                }
            }
        }
        "union_type" => {
            // Union can have type_identifiers, string_literals, and array_types
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "type_identifier" | "array_type" => {
                        validate_type_expression(child, source, symbol_table, diagnostics);
                    }
                    "string_literal" => {
                        // String literals in unions are valid (e.g., "active" | "pending")
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}