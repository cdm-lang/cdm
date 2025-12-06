// validate.rs
use std::collections::{HashMap, HashSet};

use crate::{
    Ancestor, Definition, DefinitionKind, Diagnostic, FieldInfo, Position, Severity, Span,
    SymbolTable, field_exists_in_parents, is_builtin_type, is_type_defined, resolve_definition,
};

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct ValidationResult {
    pub diagnostics: Vec<Diagnostic>,
    /// The parsed tree (for callers that need it, e.g., code generation)
    pub tree: Option<tree_sitter::Tree>,
    /// Symbol table built from this file (useful for building Ancestor structs)
    pub symbol_table: SymbolTable,
    /// Field information for models in this file (useful for building Ancestor structs)
    pub model_fields: HashMap<String, Vec<FieldInfo>>,
}

impl ValidationResult {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity == Severity::Error)
    }

    /// Convert this result into an Ancestor for use when validating files that extend this one.
    pub fn into_ancestor(self, path: String) -> Ancestor {
        Ancestor {
            path,
            symbol_table: self.symbol_table,
            model_fields: self.model_fields,
        }
    }
}

/// Validate a CDM source file with full cross-file context.
///
/// # Parameters
///
/// - `source`: The CDM source code to validate
/// - `ancestors`: Resolved ancestor files from the `@extends` chain, ordered from
///   immediate parent to most distant ancestor. Pass an empty slice if this file
///   has no `@extends` directive.
///
/// # Validation Scope
///
/// **Syntax validation:**
/// - All parse errors from tree-sitter
///
/// **Definition validation:**
/// - Duplicate type/model definitions within this file
/// - Shadowing of ancestor definitions (warning)
/// - Shadowing of built-in types (warning)
///
/// **Type reference validation:**
/// - Undefined type references (checks this file and all ancestors)
/// - Kind mismatches in extends (extending a type alias instead of a model)
/// - Undefined models in extends clause (checks this file and all ancestors)
///
/// **Inheritance validation:**
/// - Circular inheritance (within this file and across ancestors)
/// - Circular type alias references
///
/// **Field validation:**
/// - Duplicate field definitions within a single model
/// - Invalid field overrides (overriding a field defined in the same model)
/// - Invalid field removals (`-field` where field doesn't exist in any parent)
/// - Invalid field overrides targeting non-existent inherited fields
///
/// # Building Ancestors
///
/// The caller is responsible for resolving `@extends` directives and building
/// the ancestor chain. A typical workflow:
///
/// 1. Parse the source to find `@extends` directive (use `extract_extends_path()`)
/// 2. Resolve the file path
/// 3. Recursively validate ancestor files (with their own ancestors)
/// 4. Use `ValidationResult::into_ancestor()` to convert results
///
/// # Example
///
/// ```ignore
/// // For a file with no @extends
/// let result = validate(source, &[]);
///
/// // For a file that extends base.cdm
/// let base_result = validate(&base_source, &[]);
/// let base_ancestor = base_result.into_ancestor("base.cdm".to_string());
/// let result = validate(source, &[base_ancestor]);
/// ```
pub fn validate(source: &str, ancestors: &[Ancestor]) -> ValidationResult {
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
            return ValidationResult {
                diagnostics,
                tree: None,
                symbol_table: SymbolTable::new(),
                model_fields: HashMap::new(),
            };
        }
    };

    // Collect syntax errors from tree-sitter
    collect_syntax_errors(tree.root_node(), source, &mut diagnostics);

    // Semantic validation
    let (symbol_table, model_fields) =
        collect_semantic_errors(tree.root_node(), source, ancestors, &mut diagnostics);

    ValidationResult {
        diagnostics,
        tree: Some(tree),
        symbol_table,
        model_fields,
    }
}

/// Extract all @extends paths from a source file.
/// 
/// Returns paths in the order they appear in the file.
/// This is a helper for callers who need to resolve the extends chain.
pub fn extract_extends_paths(source: &str) -> Vec<String> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&grammar::LANGUAGE.into())
        .expect("Failed to load grammar");

    let Some(tree) = parser.parse(source, None) else {
        return Vec::new();
    };
    
    let root = tree.root_node();
    let mut cursor = root.walk();
    let mut paths = Vec::new();

    for node in root.children(&mut cursor) {
        if node.kind() == "extends_directive" {
            if let Some(path_node) = node.child_by_field_name("path") {
                paths.push(get_node_text(path_node, source).to_string());
            }
        }
    }

    paths
}

fn collect_syntax_errors(node: tree_sitter::Node, source: &str, diagnostics: &mut Vec<Diagnostic>) {
    if node.is_error() || node.is_missing() {
        let start = node.start_position();
        let end = node.end_position();
        let text = node.utf8_text(source.as_bytes()).unwrap_or("<invalid>");

        diagnostics.push(Diagnostic {
            message: format!("Syntax error: unexpected '{}'", text),
            severity: Severity::Error,
            span: Span {
                start: Position {
                    line: start.row,
                    column: start.column,
                },
                end: Position {
                    line: end.row,
                    column: end.column,
                },
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
        start: Position {
            line: start.row,
            column: start.column,
        },
        end: Position {
            line: end.row,
            column: end.column,
        },
    }
}

// =============================================================================
// Pass 1: Collect definitions into symbol table
// =============================================================================

fn collect_definitions(
    root: tree_sitter::Node,
    source: &str,
    ancestors: &[Ancestor],
    diagnostics: &mut Vec<Diagnostic>,
) -> (SymbolTable, HashMap<String, Vec<FieldInfo>>) {
    let mut symbol_table = SymbolTable::new();
    let mut model_fields: HashMap<String, Vec<FieldInfo>> = HashMap::new();
    let mut cursor = root.walk();

    for node in root.children(&mut cursor) {
        match node.kind() {
            "type_alias" => {
                collect_type_alias(node, source, ancestors, &mut symbol_table, diagnostics);
            }
            "model_definition" => {
                collect_model(
                    node,
                    source,
                    ancestors,
                    &mut symbol_table,
                    &mut model_fields,
                    diagnostics,
                );
            }
            _ => {}
        }
    }

    (symbol_table, model_fields)
}

fn collect_type_alias(
    node: tree_sitter::Node,
    source: &str,
    ancestors: &[Ancestor],
    symbol_table: &mut SymbolTable,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(name_node) = node.child_by_field_name("name") else {
        return;
    };

    let name = get_node_text(name_node, source);
    let span = node_span(name_node);

    // Check for duplicate definition in this file
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

    // Check for shadowing ancestor definitions (warning)
    for ancestor in ancestors {
        if ancestor.symbol_table.definitions.contains_key(name) {
            diagnostics.push(Diagnostic {
                message: format!(
                    "'{}' shadows definition from '{}'",
                    name, ancestor.path
                ),
                severity: Severity::Warning,
                span,
            });
            break;
        }
    }

    // Check for shadowing built-in types (warning)
    if is_builtin_type(name) {
        diagnostics.push(Diagnostic {
            message: format!("'{}' shadows built-in type", name),
            severity: Severity::Warning,
            span,
        });
    }

    // Extract type references from the type expression
    let references = if let Some(type_node) = node.child_by_field_name("type") {
        extract_type_references(type_node, source)
    } else {
        Vec::new()
    };

    symbol_table.definitions.insert(
        name.to_string(),
        Definition {
            kind: DefinitionKind::TypeAlias { references },
            span,
        },
    );
}

/// Extract all type identifier references from a type expression.
/// Handles simple types, arrays, and unions.
fn extract_type_references(node: tree_sitter::Node, source: &str) -> Vec<String> {
    let mut references = Vec::new();
    collect_type_references_recursive(node, source, &mut references);
    references
}

fn collect_type_references_recursive(
    node: tree_sitter::Node,
    source: &str,
    references: &mut Vec<String>,
) {
    match node.kind() {
        "type_identifier" => {
            let type_name = get_node_text(node, source);
            references.push(type_name.to_string());
        }
        "array_type" => {
            // Array type has a type_identifier child
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "type_identifier" {
                    let type_name = get_node_text(child, source);
                    references.push(type_name.to_string());
                }
            }
        }
        "union_type" => {
            // Union can have type_identifiers, string_literals, and array_types
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "type_identifier" | "array_type" => {
                        collect_type_references_recursive(child, source, references);
                    }
                    // string_literal members don't create type references
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

fn collect_model(
    node: tree_sitter::Node,
    source: &str,
    ancestors: &[Ancestor],
    symbol_table: &mut SymbolTable,
    model_fields: &mut HashMap<String, Vec<FieldInfo>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(name_node) = node.child_by_field_name("name") else {
        return;
    };

    let name = get_node_text(name_node, source);
    let span = node_span(name_node);

    // Check for duplicate definition in this file
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

    // Check for shadowing ancestor definitions (warning)
    for ancestor in ancestors {
        if ancestor.symbol_table.definitions.contains_key(name) {
            diagnostics.push(Diagnostic {
                message: format!("'{}' shadows definition from '{}'", name, ancestor.path),
                severity: Severity::Warning,
                span,
            });
            break;
        }
    }

    // Check for shadowing built-in types (warning)
    if is_builtin_type(name) {
        diagnostics.push(Diagnostic {
            message: format!("'{}' shadows built-in type", name),
            severity: Severity::Warning,
            span,
        });
    }

    // Collect extends parents
    let extends = collect_extends_parents(node, source);

    // Collect field information
    let fields = collect_field_info(node, source);
    model_fields.insert(name.to_string(), fields);

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

/// Collect field information from a model definition.
fn collect_field_info(node: tree_sitter::Node, source: &str) -> Vec<FieldInfo> {
    let mut fields = Vec::new();

    let Some(body_node) = node.child_by_field_name("body") else {
        return fields;
    };

    let mut cursor = body_node.walk();
    for child in body_node.children(&mut cursor) {
        if child.kind() == "field_definition" {
            if let Some(name_node) = child.child_by_field_name("name") {
                let name = get_node_text(name_node, source).to_string();
                let span = node_span(name_node);

                // Check for optional marker
                let optional = child.child_by_field_name("optional").is_some();

                // Get type expression as string
                let type_expr = child
                    .child_by_field_name("type")
                    .map(|t| get_node_text(t, source).to_string());

                fields.push(FieldInfo {
                    name,
                    type_expr,
                    optional,
                    span,
                });
            }
        }
    }

    fields
}

// =============================================================================
// Pass 2: Semantic validation
// =============================================================================

fn collect_semantic_errors(
    root: tree_sitter::Node,
    source: &str,
    ancestors: &[Ancestor],
    diagnostics: &mut Vec<Diagnostic>,
) -> (SymbolTable, HashMap<String, Vec<FieldInfo>>) {
    // Pass 1: Build symbol table (also collects type alias references and model fields)
    let (symbol_table, model_fields) = collect_definitions(root, source, ancestors, diagnostics);

    // Pass 2a: Detect inheritance cycles
    detect_inheritance_cycles(&symbol_table, ancestors, diagnostics);

    // Pass 2b: Detect type alias cycles
    detect_type_alias_cycles(&symbol_table, diagnostics);

    // Pass 2c: Validate references and fields
    validate_references(root, source, &symbol_table, &model_fields, ancestors, diagnostics);

    (symbol_table, model_fields)
}

// =============================================================================
// Circular Inheritance Detection
// =============================================================================

/// Detect circular inheritance in model definitions.
///
/// Uses DFS with path tracking - if we encounter a model that's already
/// in our current traversal path, we have a cycle.
fn detect_inheritance_cycles(
    symbol_table: &SymbolTable,
    ancestors: &[Ancestor],
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Track globally visited nodes to avoid redundant work
    let mut fully_visited: HashSet<String> = HashSet::new();

    for (name, def) in &symbol_table.definitions {
        if let DefinitionKind::Model { .. } = &def.kind {
            if !fully_visited.contains(name) {
                let mut path: Vec<String> = Vec::new();
                let mut path_set: HashSet<String> = HashSet::new();

                check_inheritance_cycle(
                    name,
                    symbol_table,
                    ancestors,
                    &mut path,
                    &mut path_set,
                    &mut fully_visited,
                    diagnostics,
                );
            }
        }
    }
}

/// Recursive DFS to detect cycles in inheritance chain.
///
/// Returns true if a cycle was found starting from this node.
fn check_inheritance_cycle(
    name: &str,
    symbol_table: &SymbolTable,
    ancestors: &[Ancestor],
    path: &mut Vec<String>,
    path_set: &mut HashSet<String>,
    fully_visited: &mut HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    // If already in current path, we found a cycle
    if path_set.contains(name) {
        // Find where the cycle starts in the path
        let cycle_start = path.iter().position(|n| n == name).unwrap();
        let cycle_path: Vec<&str> = path[cycle_start..].iter().map(|s| s.as_str()).collect();

        // Report the cycle - get the span of the model that completes the cycle
        if let Some((def, _)) = resolve_definition(name, symbol_table, ancestors) {
            let cycle_str = format_cycle(&cycle_path, name);
            diagnostics.push(Diagnostic {
                message: format!("Circular inheritance detected: {}", cycle_str),
                severity: Severity::Error,
                span: def.span,
            });
        }
        return true;
    }

    // If fully visited, no cycle through this node
    if fully_visited.contains(name) {
        return false;
    }

    // Get the definition - check local first, then ancestors
    let Some((def, _)) = resolve_definition(name, symbol_table, ancestors) else {
        return false;
    };

    let DefinitionKind::Model { extends } = &def.kind else {
        return false;
    };

    let extends = extends.clone(); // Clone to avoid borrow issues

    // Add to current path
    path.push(name.to_string());
    path_set.insert(name.to_string());

    let mut found_cycle = false;

    // Check each parent
    for parent in &extends {
        if check_inheritance_cycle(
            parent,
            symbol_table,
            ancestors,
            path,
            path_set,
            fully_visited,
            diagnostics,
        ) {
            found_cycle = true;
            // Don't break - continue to find all cycles
        }
    }

    // Remove from current path
    path.pop();
    path_set.remove(name);

    // Mark as fully visited
    fully_visited.insert(name.to_string());

    found_cycle
}

/// Format a cycle path for display.
/// Example: "A -> B -> C -> A"
fn format_cycle(cycle_path: &[&str], back_to: &str) -> String {
    let mut result = cycle_path.join(" -> ");
    result.push_str(" -> ");
    result.push_str(back_to);
    result
}

// =============================================================================
// Circular Type Alias Detection
// =============================================================================

/// Detect circular references in type alias definitions.
///
/// Examples of cycles:
/// - `A: B` and `B: A` (direct cycle)
/// - `A: B`, `B: C`, `C: A` (indirect cycle)
/// - `A: A` (self-reference)
/// - `A: B | C` where `C: A` (cycle through union)
fn detect_type_alias_cycles(symbol_table: &SymbolTable, diagnostics: &mut Vec<Diagnostic>) {
    let mut fully_visited: HashSet<&str> = HashSet::new();

    for (name, def) in &symbol_table.definitions {
        if let DefinitionKind::TypeAlias { .. } = &def.kind {
            if !fully_visited.contains(name.as_str()) {
                let mut path: Vec<&str> = Vec::new();
                let mut path_set: HashSet<&str> = HashSet::new();

                check_type_alias_cycle(
                    name,
                    symbol_table,
                    &mut path,
                    &mut path_set,
                    &mut fully_visited,
                    diagnostics,
                );
            }
        }
    }
}

/// Recursive DFS to detect cycles in type alias references.
fn check_type_alias_cycle<'a>(
    name: &'a str,
    symbol_table: &'a SymbolTable,
    path: &mut Vec<&'a str>,
    path_set: &mut HashSet<&'a str>,
    fully_visited: &mut HashSet<&'a str>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    // If already in current path, we found a cycle
    if path_set.contains(name) {
        let cycle_start = path.iter().position(|&n| n == name).unwrap();
        let cycle_path: Vec<&str> = path[cycle_start..].to_vec();

        if let Some(def) = symbol_table.get(name) {
            let cycle_str = format_cycle(&cycle_path, name);
            diagnostics.push(Diagnostic {
                message: format!("Circular type reference detected: {}", cycle_str),
                severity: Severity::Error,
                span: def.span,
            });
        }
        return true;
    }

    // If fully visited, no cycle through this node
    if fully_visited.contains(name) {
        return false;
    }

    // Get the definition - only follow type aliases
    let Some(def) = symbol_table.get(name) else {
        return false;
    };

    let DefinitionKind::TypeAlias { references } = &def.kind else {
        // Hit a model or built-in type - not a cycle in the alias chain
        return false;
    };

    // Add to current path
    path.push(name);
    path_set.insert(name);

    let mut found_cycle = false;

    // Check each referenced type
    for reference in references {
        if check_type_alias_cycle(
            reference,
            symbol_table,
            path,
            path_set,
            fully_visited,
            diagnostics,
        ) {
            found_cycle = true;
        }
    }

    // Remove from current path
    path.pop();
    path_set.remove(name);

    // Mark as fully visited
    fully_visited.insert(name);

    found_cycle
}

// =============================================================================
// Reference Validation
// =============================================================================

fn validate_references(
    root: tree_sitter::Node,
    source: &str,
    symbol_table: &SymbolTable,
    model_fields: &HashMap<String, Vec<FieldInfo>>,
    ancestors: &[Ancestor],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut cursor = root.walk();

    for node in root.children(&mut cursor) {
        match node.kind() {
            "type_alias" => {
                validate_type_alias(node, source, symbol_table, ancestors, diagnostics);
            }
            "model_definition" => {
                validate_model(node, source, symbol_table, model_fields, ancestors, diagnostics);
            }
            _ => {}
        }
    }
}

fn validate_type_alias(
    node: tree_sitter::Node,
    source: &str,
    symbol_table: &SymbolTable,
    ancestors: &[Ancestor],
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Check the type expression on the right side of the alias
    let Some(type_node) = node.child_by_field_name("type") else {
        return;
    };

    validate_type_expression(type_node, source, symbol_table, ancestors, diagnostics);
}

fn validate_model(
    node: tree_sitter::Node,
    source: &str,
    symbol_table: &SymbolTable,
    model_fields: &HashMap<String, Vec<FieldInfo>>,
    ancestors: &[Ancestor],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let model_name = node
        .child_by_field_name("name")
        .map(|n| get_node_text(n, source))
        .unwrap_or("");

    // Validate extends clause
    if let Some(extends_node) = node.child_by_field_name("extends") {
        validate_extends(extends_node, source, symbol_table, ancestors, diagnostics);
    }

    // Check field types in the model body
    let Some(body_node) = node.child_by_field_name("body") else {
        return;
    };

    // Validate duplicate field names and invalid field overrides
    validate_model_fields(
        model_name,
        body_node,
        source,
        symbol_table,
        model_fields,
        ancestors,
        diagnostics,
    );

    let mut cursor = body_node.walk();

    for child in body_node.children(&mut cursor) {
        if child.kind() == "field_definition" {
            validate_field(child, source, symbol_table, ancestors, diagnostics);
        }
    }
}

/// Validate field definitions within a model body.
///
/// Checks for:
/// 1. Duplicate field definitions (same field name defined twice)
/// 2. Invalid field overrides (field_override targeting a field defined in the same model)
/// 3. Invalid field removals (removing a field that doesn't exist in parents)
/// 4. Invalid field overrides (overriding a field that doesn't exist in parents)
fn validate_model_fields(
    model_name: &str,
    body_node: tree_sitter::Node,
    source: &str,
    symbol_table: &SymbolTable,
    model_fields: &HashMap<String, Vec<FieldInfo>>,
    ancestors: &[Ancestor],
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Track field definitions: name -> span of first definition
    let mut defined_fields: HashMap<String, Span> = HashMap::new();
    // Track field overrides to check after we've collected all definitions
    let mut override_fields: Vec<(String, Span)> = Vec::new();
    // Track field removals to validate
    let mut removal_fields: Vec<(String, Span)> = Vec::new();

    let mut cursor = body_node.walk();

    for child in body_node.children(&mut cursor) {
        if let Some(name_node) = child.child_by_field_name("name") {
            let field_name = get_node_text(name_node, source).to_string();
            let span = node_span(name_node);

            match child.kind() {
                "field_definition" => {
                    if let Some(first_span) = defined_fields.get(&field_name) {
                        diagnostics.push(Diagnostic {
                            message: format!(
                                "Duplicate field '{}' (first defined at line {})",
                                field_name,
                                first_span.start.line + 1
                            ),
                            severity: Severity::Error,
                            span,
                        });
                    } else {
                        defined_fields.insert(field_name, span);
                    }
                }
                "field_override" => {
                    override_fields.push((field_name, span));
                }
                "field_removal" => {
                    removal_fields.push((field_name, span));
                }
                _ => {}
            }
        }
    }

    // Check that field_override doesn't target locally-defined fields
    for (override_name, override_span) in &override_fields {
        if let Some(def_span) = defined_fields.get(override_name) {
            diagnostics.push(Diagnostic {
                message: format!(
                    "Cannot override field '{}' defined in the same model (line {}). \
                     Use inline plugin syntax instead: `{}: Type {{ @plugin {{...}} }}`",
                    override_name,
                    def_span.start.line + 1,
                    override_name
                ),
                severity: Severity::Error,
                span: *override_span,
            });
        } else {
            // Check that the field exists in a parent model
            if !field_exists_in_parents(model_name, override_name, model_fields, symbol_table, ancestors) {
                diagnostics.push(Diagnostic {
                    message: format!(
                        "Cannot override field '{}': not found in any parent model",
                        override_name
                    ),
                    severity: Severity::Error,
                    span: *override_span,
                });
            }
        }
    }

    // Check that field removals reference fields that exist in parents
    for (removal_name, removal_span) in &removal_fields {
        if !field_exists_in_parents(model_name, removal_name, model_fields, symbol_table, ancestors) {
            diagnostics.push(Diagnostic {
                message: format!(
                    "Cannot remove field '{}': not found in any parent model",
                    removal_name
                ),
                severity: Severity::Error,
                span: *removal_span,
            });
        }
    }
}

fn validate_extends(
    extends_node: tree_sitter::Node,
    source: &str,
    symbol_table: &SymbolTable,
    ancestors: &[Ancestor],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut cursor = extends_node.walk();

    for parent_node in extends_node.children_by_field_name("parent", &mut cursor) {
        let parent_name = get_node_text(parent_node, source);

        match resolve_definition(parent_name, symbol_table, ancestors) {
            None => {
                diagnostics.push(Diagnostic {
                    message: format!("Undefined type '{}' in extends clause", parent_name),
                    severity: Severity::Error,
                    span: node_span(parent_node),
                });
            }
            Some((def, _)) => {
                if matches!(def.kind, DefinitionKind::TypeAlias { .. }) {
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
    ancestors: &[Ancestor],
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Field might not have a type (untyped fields default to string)
    let Some(type_node) = node.child_by_field_name("type") else {
        return;
    };

    validate_type_expression(type_node, source, symbol_table, ancestors, diagnostics);
}

fn validate_type_expression(
    node: tree_sitter::Node,
    source: &str,
    symbol_table: &SymbolTable,
    ancestors: &[Ancestor],
    diagnostics: &mut Vec<Diagnostic>,
) {
    match node.kind() {
        "type_identifier" => {
            let type_name = get_node_text(node, source);
            if !is_type_defined(type_name, symbol_table, ancestors) {
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
                    validate_type_expression(child, source, symbol_table, ancestors, diagnostics);
                }
            }
        }
        "union_type" => {
            // Union can have type_identifiers, string_literals, and array_types
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "type_identifier" | "array_type" => {
                        validate_type_expression(child, source, symbol_table, ancestors, diagnostics);
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