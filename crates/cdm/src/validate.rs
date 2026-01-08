// validate.rs
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::Path;

use crate::{
    Ancestor, Definition, DefinitionKind, Diagnostic, FieldInfo, Severity,
    SymbolTable, ImportedNamespace, field_exists_in_parents, is_builtin_type,
    is_type_defined, is_type_reference_defined, resolve_definition,
};
use crate::diagnostics::{
    E501_DUPLICATE_ENTITY_ID, E502_DUPLICATE_FIELD_ID,
    W005_MISSING_ENTITY_ID, W006_MISSING_FIELD_ID,
};
use crate::file_resolver::LoadedFileTree;
use crate::resolved_schema::{build_resolved_schema, find_references_in_resolved};
use crate::plugin_validation::validate_plugins;
use crate::template_resolver::{
    extract_template_imports, resolve_template, get_import_entity_id_source,
    TemplateImport,
};
use crate::template_validation::validate_template_imports;
use cdm_utils::{EntityId, Position, Span};

#[cfg(test)]
#[path = "validate/validate_tests.rs"]
mod validate_tests;

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

impl fmt::Display for ValidationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.diagnostics.is_empty() {
            writeln!(f, "✓ No errors or warnings")?;
        } else {
            let errors: Vec<_> = self.diagnostics.iter()
                .filter(|d| d.severity == Severity::Error)
                .collect();
            let warnings: Vec<_> = self.diagnostics.iter()
                .filter(|d| d.severity == Severity::Warning)
                .collect();

            if !errors.is_empty() {
                writeln!(f, "Errors ({}):", errors.len())?;
                for diagnostic in &errors {
                    writeln!(f, "  {}", diagnostic)?;
                }
            }

            if !warnings.is_empty() {
                if !errors.is_empty() {
                    writeln!(f)?;
                }
                writeln!(f, "Warnings ({}):", warnings.len())?;
                for diagnostic in &warnings {
                    writeln!(f, "  {}", diagnostic)?;
                }
            }
        }

        writeln!(f)?;
        write!(f, "{}", self.symbol_table)?;

        Ok(())
    }
}

/// Validate a CDM source file with full cross-file context.
///
/// # Parameters
///
/// - `source`: The CDM source code to validate
/// - `ancestors`: Resolved ancestor files from the `extends` chain, ordered from
///   immediate parent to most distant ancestor. Pass an empty slice if this file
///   has no `extends` directive.
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
/// The caller is responsible for resolving `extends` directives and building
/// the ancestor chain. A typical workflow:
///
/// 1. Parse the source to find `extends` directive (use `extract_extends_path()`)
/// 2. Resolve the file path
/// 3. Recursively validate ancestor files (with their own ancestors)
/// 4. Use `ValidationResult::into_ancestor()` to convert results
///
/// # Example
///
/// ```ignore
/// // For a file with no extends
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

/// Validate a CDM source file with template namespaces pre-loaded.
///
/// This is an extended version of `validate()` that accepts pre-loaded template
/// namespaces. Use this when you need to validate a file that imports templates.
pub fn validate_with_templates(
    source: &str,
    ancestors: &[Ancestor],
    namespaces: Vec<ImportedNamespace>,
) -> ValidationResult {
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

    // Semantic validation with templates
    let (symbol_table, model_fields) =
        collect_semantic_errors_with_templates(tree.root_node(), source, ancestors, namespaces, &mut diagnostics);

    ValidationResult {
        diagnostics,
        tree: Some(tree),
        symbol_table,
        model_fields,
    }
}

/// Load template imports and convert them to ImportedNamespaces.
///
/// This function:
/// 1. Resolves each template import to its location
/// 2. Parses the template CDM file
/// 3. Validates the template (recursively handling its own imports)
/// 4. Builds a symbol table for the template
/// 5. Returns ImportedNamespace structs ready to be added to the main symbol table
pub fn load_template_namespaces(
    imports: &[TemplateImport],
    project_root: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<ImportedNamespace> {
    let mut namespaces = Vec::new();

    for import in imports {
        match load_single_template(import, project_root) {
            Ok(namespace) => {
                namespaces.push(namespace);
            }
            Err(err) => {
                diagnostics.push(Diagnostic {
                    message: format!(
                        "Failed to load template '{}': {}",
                        import.namespace, err
                    ),
                    severity: Severity::Error,
                    span: import.span.clone(),
                });
            }
        }
    }

    namespaces
}

/// Load a single template and convert it to an ImportedNamespace.
fn load_single_template(
    import: &TemplateImport,
    project_root: &Path,
) -> Result<ImportedNamespace, String> {
    // Resolve the template location
    let loaded = resolve_template(import)
        .map_err(|e| e.to_string())?;

    // Read the template entry file
    let template_source = std::fs::read_to_string(&loaded.entry_path)
        .map_err(|e| format!("Failed to read template file {}: {}", loaded.entry_path.display(), e))?;

    // Parse the template
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&grammar::LANGUAGE.into())
        .expect("Failed to load grammar");

    let tree = parser.parse(&template_source, None)
        .ok_or_else(|| "Failed to parse template file".to_string())?;

    // Check for syntax errors
    if tree.root_node().has_error() {
        return Err("Template file contains syntax errors".to_string());
    }

    // Build symbol table for the template
    // Note: Templates can have their own imports, but we don't recursively load them yet
    // This is a simplification - full implementation would handle nested templates
    let mut diagnostics = Vec::new();
    let (symbol_table, model_fields) = collect_definitions(
        tree.root_node(),
        &template_source,
        &[], // Templates don't inherit from the main file's ancestors
        &mut diagnostics,
    );

    // Check for validation errors in the template
    if diagnostics.iter().any(|d| d.severity == Severity::Error) {
        let errors: Vec<String> = diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .map(|d| d.message.clone())
            .collect();
        return Err(format!("Template validation errors: {}", errors.join("; ")));
    }

    // Get the entity ID source for this template
    let template_source_id = get_import_entity_id_source(import, project_root);

    Ok(ImportedNamespace {
        name: import.namespace.clone(),
        template_path: loaded.entry_path.clone(),
        symbol_table,
        model_fields,
        template_source: template_source_id,
    })
}

/// Validate a LoadedFileTree with all its ancestors.
///
/// This is the high-level API for validating CDM files that have been loaded
/// via FileResolver. It validates ancestors first (in streaming fashion) and
/// then validates the main file.
///
/// # Arguments
/// * `tree` - The loaded file tree from FileResolver
///
/// # Returns
/// * `Ok(ValidationResult)` - Successfully validated schema
/// * `Err(Vec<Diagnostic>)` - Validation errors or file reading errors
///
/// # Memory efficiency
/// This function validates in streaming fashion - each ancestor is validated
/// and converted to an Ancestor struct before the next is processed, minimizing
/// peak memory usage.
pub fn validate_tree(tree: LoadedFileTree) -> Result<ValidationResult, Vec<Diagnostic>> {
    validate_tree_with_options(tree, false)
}

pub fn validate_tree_with_options(tree: LoadedFileTree, check_ids: bool) -> Result<ValidationResult, Vec<Diagnostic>> {
    // Validate all ancestors in streaming fashion
    let mut ancestors = Vec::new();
    let mut ancestor_sources = Vec::new();

    for loaded_ancestor in tree.ancestors {
        let source = loaded_ancestor.source().map_err(|err| {
            vec![Diagnostic {
                message: format!("Failed to read file {}: {}", loaded_ancestor.path.display(), err),
                severity: Severity::Error,
                span: Span {
                    start: Position { line: 0, column: 0 },
                    end: Position { line: 0, column: 0 },
                },
            }]
        })?;

        // Get project root for this ancestor
        let ancestor_project_root = loaded_ancestor.path.parent()
            .unwrap_or_else(|| Path::new("."));

        // Parse to extract template imports for ancestor
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&grammar::LANGUAGE.into())
            .expect("Failed to load grammar");

        let ancestor_namespaces = if let Some(parse_tree) = parser.parse(&source, None) {
            let template_imports = extract_template_imports(parse_tree.root_node(), &source, &loaded_ancestor.path);

            // Validate template imports
            let mut template_diagnostics = validate_template_imports(&template_imports);
            if template_diagnostics.iter().any(|d| d.severity == Severity::Error) {
                return Err(template_diagnostics);
            }

            // Load template namespaces for ancestor
            let namespaces = load_template_namespaces(&template_imports, ancestor_project_root, &mut template_diagnostics);
            if template_diagnostics.iter().any(|d| d.severity == Severity::Error) {
                return Err(template_diagnostics);
            }
            namespaces
        } else {
            Vec::new()
        };

        // Validate ancestor with its templates
        let ancestor_result = validate_with_templates(&source, &ancestors, ancestor_namespaces);

        // Check for validation errors
        if ancestor_result.has_errors() {
            return Err(ancestor_result.diagnostics);
        }

        // Convert to Ancestor and add to list
        // This frees the ValidationResult memory (tree, diagnostics, etc.)
        let ancestor_path = loaded_ancestor.path.clone();
        let ancestor = ancestor_result.into_ancestor(loaded_ancestor.path.display().to_string());
        ancestors.push(ancestor);

        // Keep source and path for plugin validation
        ancestor_sources.push((source, ancestor_path));
    }

    // Validate main file with all ancestors
    let main_path = tree.main.path.clone();
    let main_source = tree.main.source().map_err(|err| {
        vec![Diagnostic {
            message: format!("Failed to read file {}: {}", main_path.display(), err),
            severity: Severity::Error,
            span: Span {
                start: Position { line: 0, column: 0 },
                end: Position { line: 0, column: 0 },
            },
        }]
    })?;

    // Get the project root directory for resolving relative template paths
    let project_root = main_path.parent()
        .unwrap_or_else(|| Path::new("."));

    // First, parse the source to extract template imports
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&grammar::LANGUAGE.into())
        .expect("Failed to load grammar");

    let parse_tree = parser.parse(&main_source, None).ok_or_else(|| {
        vec![Diagnostic {
            message: "Failed to parse file".to_string(),
            severity: Severity::Error,
            span: Span {
                start: Position { line: 0, column: 0 },
                end: Position { line: 0, column: 0 },
            },
        }]
    })?;

    // Extract template imports
    let template_imports = extract_template_imports(parse_tree.root_node(), &main_source, &main_path);

    // Validate template imports for namespace conflicts
    let mut template_diagnostics = validate_template_imports(&template_imports);

    // If there are namespace conflicts, return early
    if template_diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return Err(template_diagnostics);
    }

    // Load template namespaces
    let namespaces = load_template_namespaces(&template_imports, project_root, &mut template_diagnostics);

    // If template loading failed, return early
    if template_diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return Err(template_diagnostics);
    }

    // Validate main file with templates loaded
    let mut result = validate_with_templates(&main_source, &ancestors, namespaces);

    // Add any template warnings to the result
    result.diagnostics.extend(template_diagnostics);

    // Check for semantic errors before plugin validation
    if result.has_errors() {
        return Err(result.diagnostics);
    }

    // Plugin validation (only if semantic validation passed)
    if let Some(ref parse_tree) = result.tree {
        validate_plugins(parse_tree, &main_source, &main_path, &ancestor_sources, &mut result.diagnostics);
    }

    // Check for plugin validation errors
    if result.has_errors() {
        return Err(result.diagnostics);
    }

    // Check for missing entity IDs if --check-ids flag is set
    if check_ids {
        warn_missing_ids(&result.symbol_table, &result.model_fields, &mut result.diagnostics);
    }

    Ok(result)
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

/// Extract entity ID from a node if it has an "id" field.
/// Returns None if no ID is present or if parsing fails.
fn extract_entity_id(node: tree_sitter::Node, source: &str) -> Option<u64> {
    node.child_by_field_name("id")
        .and_then(|id_node| {
            let text = get_node_text(id_node, source);
            // Remove the '#' prefix and parse
            text.strip_prefix('#')
                .and_then(|num| num.parse::<u64>().ok())
        })
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
    let mut removals: Vec<(String, Span, &str)> = Vec::new(); // (name, span, kind)

    // Extract all plugin configs and default values once upfront
    let plugin_data = crate::extract_structured_plugin_configs(root, source);

    let mut cursor = root.walk();

    // First pass: collect definitions and removals
    for node in root.children(&mut cursor) {
        match node.kind() {
            "type_alias" => {
                collect_type_alias(node, source, ancestors, &mut symbol_table, &plugin_data, diagnostics);
            }
            "model_definition" => {
                collect_model(
                    node,
                    source,
                    ancestors,
                    &mut symbol_table,
                    &mut model_fields,
                    &plugin_data,
                    diagnostics,
                );
            }
            "model_removal" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = get_node_text(name_node, source);
                    let span = node_span(name_node);
                    // Store as "removal" - we'll determine if it's a model or type alias during validation
                    removals.push((name.to_string(), span, "removal"));
                }
            }
            _ => {}
        }
    }

    // Second pass: validate removals
    validate_removals(&removals, &symbol_table, &model_fields, ancestors, diagnostics);

    (symbol_table, model_fields)
}

/// Validate model and type alias removals.
///
/// Checks for:
/// 1. E302: Removing type alias that is still referenced by fields
/// 2. E303: Removing model that is still referenced by fields
///
/// Removals (-TypeName, -ModelName) are used in context files to exclude
/// definitions from ancestor files. They're invalid if the removed definition
/// is still being used in the final resolved schema (current file + inherited definitions).
fn validate_removals(
    removals: &[(String, Span, &str)],
    symbol_table: &SymbolTable,
    model_fields: &HashMap<String, Vec<FieldInfo>>,
    ancestors: &[Ancestor],
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Build the resolved schema (what would exist after applying removals)
    let resolved = build_resolved_schema(symbol_table, model_fields, ancestors, removals);

    for (removal_name, removal_span, _kind) in removals {
        // Determine if this is a model or type alias by checking ancestors
        let (is_model, is_type_alias) = ancestors.iter().find_map(|ancestor| {
            ancestor.symbol_table.definitions.get(removal_name).map(|def| {
                match &def.kind {
                    DefinitionKind::Model { .. } => (true, false),
                    DefinitionKind::TypeAlias { .. } => (false, true),
                }
            })
        }).unwrap_or((false, false));

        if !is_model && !is_type_alias {
            // Not found in any ancestor
            diagnostics.push(Diagnostic {
                message: format!(
                    "Cannot remove '{}': not found in any ancestor file",
                    removal_name
                ),
                severity: Severity::Error,
                span: *removal_span,
            });
            continue;
        }

        // Check if the removed item is still referenced in the resolved schema
        let references = find_references_in_resolved(&resolved, removal_name);

        if !references.is_empty() {
            let kind_name = if is_model { "model" } else { "type alias" };
            diagnostics.push(Diagnostic {
                message: format!(
                    "Cannot remove {} '{}': still referenced by {}",
                    kind_name,
                    removal_name,
                    references.join(", ")
                ),
                severity: Severity::Error,
                span: *removal_span,
            });
        }
    }
}

fn collect_type_alias(
    node: tree_sitter::Node,
    source: &str,
    _ancestors: &[Ancestor],
    symbol_table: &mut SymbolTable,
    plugin_data: &crate::ExtractedPluginConfigs,
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

    // Note: Redefining a type alias from an ancestor is allowed - this is how
    // child files can override/extend parent definitions via the extends directive.
    // We only warn about shadowing built-in types.

    // Check for shadowing built-in types (warning)
    if is_builtin_type(name) {
        diagnostics.push(Diagnostic {
            message: format!("'{}' shadows built-in type", name),
            severity: Severity::Warning,
            span,
        });
    }

    // Extract type expression text and references
    let (references, type_expr) = if let Some(type_node) = node.child_by_field_name("type") {
        (
            extract_type_references(type_node, source),
            get_node_text(type_node, source).to_string(),
        )
    } else {
        (Vec::new(), String::new())
    };

    // Get plugin configs for this type alias
    let plugin_configs = plugin_data.type_alias_configs
        .get(name)
        .cloned()
        .unwrap_or_default();

    // Extract entity ID (local source for current file)
    let entity_id = extract_entity_id(node, source).map(EntityId::local);

    symbol_table.definitions.insert(
        name.to_string(),
        Definition {
            kind: DefinitionKind::TypeAlias { references, type_expr },
            span,
            plugin_configs,
            entity_id,
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
            // type_identifier can contain either a simple identifier or a qualified_identifier
            // We capture the full text which works for both cases
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
    _ancestors: &[Ancestor],
    symbol_table: &mut SymbolTable,
    model_fields: &mut HashMap<String, Vec<FieldInfo>>,
    plugin_data: &crate::ExtractedPluginConfigs,
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

    // Note: Redefining a model from an ancestor is allowed - this is how
    // child files can extend/modify parent models (add fields, remove fields, etc.)
    // via the extends directive. We only warn about shadowing built-in types.

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

    // Collect field information with plugin configs and defaults
    let fields = collect_field_info(node, source, name, plugin_data);
    model_fields.insert(name.to_string(), fields);

    // Get plugin configs for this model
    let plugin_configs = plugin_data.model_configs
        .get(name)
        .cloned()
        .unwrap_or_default();

    // Extract entity ID (local source for current file)
    let entity_id = extract_entity_id(node, source).map(EntityId::local);

    symbol_table.definitions.insert(
        name.to_string(),
        Definition {
            kind: DefinitionKind::Model { extends },
            span,
            plugin_configs,
            entity_id,
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
fn collect_field_info(
    node: tree_sitter::Node,
    source: &str,
    model_name: &str,
    plugin_data: &crate::ExtractedPluginConfigs,
) -> Vec<FieldInfo> {
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

                // Get plugin configs for this field
                let plugin_configs = plugin_data.field_configs
                    .get(&(model_name.to_string(), name.clone()))
                    .cloned()
                    .unwrap_or_default();

                // Get default value for this field
                let default_value = plugin_data.field_defaults
                    .get(&(model_name.to_string(), name.clone()))
                    .cloned();

                // Extract entity ID (local source for current file)
                let entity_id = extract_entity_id(child, source).map(EntityId::local);

                fields.push(FieldInfo {
                    name,
                    type_expr,
                    optional,
                    span,
                    plugin_configs,
                    default_value,
                    entity_id,
                });
            }
        }
    }

    fields
}

// =============================================================================
// Entity ID Validation
// =============================================================================

/// Validate entity IDs for uniqueness within their scope.
///
/// Entity IDs are now composite (source + local_id), and IDs only collide if they
/// have the same source AND the same local_id. For local files, all IDs have
/// EntityIdSource::Local, so we validate within that scope.
fn validate_entity_ids(
    symbol_table: &SymbolTable,
    model_fields: &HashMap<String, Vec<FieldInfo>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Track global IDs for models and type aliases (they share a global namespace)
    // Key is (source, local_id) - for local files, source is always Local
    let mut global_ids: HashMap<u64, (String, Span)> = HashMap::new();

    // Sort definitions by source position to ensure consistent error messages
    let mut sorted_defs: Vec<_> = symbol_table.definitions.iter().collect();
    sorted_defs.sort_by_key(|(_, def)| (def.span.start.line, def.span.start.column));

    // Check model and type alias IDs (global scope within same source)
    for (name, def) in sorted_defs {
        if let Some(ref entity_id) = def.entity_id {
            let local_id = entity_id.local_id;
            if let Some((existing_name, existing_span)) = global_ids.get(&local_id) {
                diagnostics.push(Diagnostic {
                    message: format!(
                        "{}: Duplicate entity ID #{}: already used by '{}' at line {}",
                        E501_DUPLICATE_ENTITY_ID, local_id, existing_name, existing_span.start.line + 1
                    ),
                    severity: Severity::Error,
                    span: def.span,
                });
            } else {
                global_ids.insert(local_id, (name.clone(), def.span));
            }
        }
    }

    // Check field IDs (scoped per model within same source)
    for (model_name, fields) in model_fields {
        let mut field_ids: HashMap<u64, (String, Span)> = HashMap::new();

        for field in fields {
            if let Some(ref entity_id) = field.entity_id {
                let local_id = entity_id.local_id;
                if let Some((existing_name, existing_span)) = field_ids.get(&local_id) {
                    diagnostics.push(Diagnostic {
                        message: format!(
                            "{}: Duplicate field ID #{} in model '{}': already used by field '{}' at line {}",
                            E502_DUPLICATE_FIELD_ID, local_id, model_name, existing_name, existing_span.start.line + 1
                        ),
                        severity: Severity::Error,
                        span: field.span,
                    });
                } else {
                    field_ids.insert(local_id, (field.name.clone(), field.span));
                }
            }
        }
    }
}

/// Warn about entities without IDs (for --check-ids flag)
fn warn_missing_ids(
    symbol_table: &SymbolTable,
    model_fields: &HashMap<String, Vec<FieldInfo>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Check models and type aliases
    for (name, def) in &symbol_table.definitions {
        if def.entity_id.is_none() {
            diagnostics.push(Diagnostic {
                message: format!("{}: Entity '{}' has no ID for migration tracking", W005_MISSING_ENTITY_ID, name),
                severity: Severity::Warning,
                span: def.span,
            });
        }
    }

    // Check fields
    for (model_name, fields) in model_fields {
        for field in fields {
            if field.entity_id.is_none() {
                diagnostics.push(Diagnostic {
                    message: format!(
                        "{}: Field '{}.{}' has no ID for migration tracking",
                        W006_MISSING_FIELD_ID, model_name, field.name
                    ),
                    severity: Severity::Warning,
                    span: field.span,
                });
            }
        }
    }
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

    // Pass 2d: Validate entity IDs
    validate_entity_ids(&symbol_table, &model_fields, diagnostics);

    (symbol_table, model_fields)
}

/// Semantic validation with template namespaces.
///
/// This is an extended version of `collect_semantic_errors` that also
/// accepts pre-loaded template namespaces and adds them to the symbol table.
fn collect_semantic_errors_with_templates(
    root: tree_sitter::Node,
    source: &str,
    ancestors: &[Ancestor],
    namespaces: Vec<ImportedNamespace>,
    diagnostics: &mut Vec<Diagnostic>,
) -> (SymbolTable, HashMap<String, Vec<FieldInfo>>) {
    // Pass 1: Build symbol table (also collects type alias references and model fields)
    let (mut symbol_table, model_fields) = collect_definitions(root, source, ancestors, diagnostics);

    // Add template namespaces to the symbol table
    for namespace in namespaces {
        symbol_table.add_namespace(namespace);
    }

    // Pass 2a: Detect inheritance cycles
    detect_inheritance_cycles(&symbol_table, ancestors, diagnostics);

    // Pass 2b: Detect type alias cycles
    detect_type_alias_cycles(&symbol_table, diagnostics);

    // Pass 2c: Validate references and fields (now with template support)
    validate_references(root, source, &symbol_table, &model_fields, ancestors, diagnostics);

    // Pass 2d: Validate entity IDs
    validate_entity_ids(&symbol_table, &model_fields, diagnostics);

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

    let DefinitionKind::TypeAlias { references, .. } = &def.kind else {
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
    let type_node = node.child_by_field_name("type");
    
    if let Some(type_node) = type_node {
        validate_type_expression(type_node, source, symbol_table, ancestors, diagnostics);
    }

    // Check default value type compatibility
    if let Some(default_node) = node.child_by_field_name("default") {
        // Determine the expected type - use the type expression if present, otherwise "string"
        let expected_type = type_node
            .map(|t| get_node_text(t, source))
            .unwrap_or("string");
        
        validate_default_value(
            default_node,
            expected_type,
            source,
            symbol_table,
            ancestors,
            diagnostics,
        );
    }
}

// =============================================================================
// Default Value Type Checking
// =============================================================================

/// The resolved base type of a CDM type expression.
#[derive(Debug, Clone, PartialEq)]
enum ResolvedType {
    /// A primitive type: string, number, boolean
    Primitive(String),
    /// A string literal union: "a" | "b" | "c"
    StringUnion(Vec<String>),
    /// An array type with element type
    Array(Box<ResolvedType>),
    /// A model or composite type (cannot have primitive default values)
    Model(String),
    /// Unknown type (undefined or circular reference)
    Unknown,
}

/// Resolve a type expression to its base type.
/// 
/// For type aliases, follows the chain until a primitive, union, or model is found.
/// Handles arrays, unions, and direct type references.
fn resolve_type(
    type_expr: &str,
    symbol_table: &SymbolTable,
    ancestors: &[Ancestor],
    visited: &mut HashSet<String>,
) -> ResolvedType {
    // Check for array type: TypeName[]
    if type_expr.ends_with("[]") {
        let element_type = &type_expr[..type_expr.len() - 2];
        let resolved_element = resolve_type(element_type, symbol_table, ancestors, visited);
        return ResolvedType::Array(Box::new(resolved_element));
    }
    
    // Check for union type (contains |)
    if type_expr.contains(" | ") || type_expr.contains('|') {
        // Parse the union - extract string literals
        let parts: Vec<&str> = type_expr.split('|').map(|s| s.trim()).collect();
        let mut string_literals = Vec::new();
        let mut has_non_string = false;
        
        for part in parts {
            if part.starts_with('"') && part.ends_with('"') && part.len() >= 2 {
                // Extract the string content (remove quotes)
                string_literals.push(part[1..part.len()-1].to_string());
            } else {
                // Non-string member in union
                has_non_string = true;
            }
        }
        
        // If it's a pure string literal union, return that
        if !has_non_string && !string_literals.is_empty() {
            return ResolvedType::StringUnion(string_literals);
        }
        
        // Mixed union - for now, treat as unknown (we can't easily type-check)
        return ResolvedType::Unknown;
    }
    
    // Check for primitive types (only spec-defined built-ins)
    match type_expr {
        "string" => return ResolvedType::Primitive("string".to_string()),
        "number" => return ResolvedType::Primitive("number".to_string()),
        "boolean" => return ResolvedType::Primitive("boolean".to_string()),
        "JSON" => return ResolvedType::Unknown, // JSON accepts any value
        _ => {}
    }
    
    // Prevent infinite recursion in circular type aliases
    if visited.contains(type_expr) {
        return ResolvedType::Unknown;
    }
    visited.insert(type_expr.to_string());
    
    // Look up in symbol table
    if let Some((def, _)) = resolve_definition(type_expr, symbol_table, ancestors) {
        match &def.kind {
            DefinitionKind::TypeAlias { references, type_expr: alias_type_expr } => {
                // For type aliases with a single reference, resolve transitively
                if references.len() == 1 {
                    return resolve_type(&references[0], symbol_table, ancestors, visited);
                }
                // For aliases with no identifier references (like string unions),
                // try to parse the original type expression
                if references.is_empty() && !alias_type_expr.is_empty() {
                    return resolve_type(alias_type_expr, symbol_table, ancestors, visited);
                }
                // Multiple references means a mixed union
                ResolvedType::Unknown
            }
            DefinitionKind::Model { .. } => {
                ResolvedType::Model(type_expr.to_string())
            }
        }
    } else {
        // Unknown/undefined type - skip validation
        ResolvedType::Unknown
    }
}

/// Validate that a default value is compatible with its declared type.
fn validate_default_value(
    default_node: tree_sitter::Node,
    type_expr: &str,
    source: &str,
    symbol_table: &SymbolTable,
    ancestors: &[Ancestor],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut visited = HashSet::new();
    let resolved_type = resolve_type(type_expr, symbol_table, ancestors, &mut visited);
    
    let default_kind = default_node.kind();
    let default_span = node_span(default_node);
    
    match resolved_type {
        ResolvedType::Primitive(ref prim) => {
            let expected_literal = match prim.as_str() {
                "string" => "string_literal",
                "number" => "number_literal",
                "boolean" => "boolean_literal",
                _ => return, // Unknown primitive, skip validation
            };
            
            if default_kind != expected_literal {
                let actual_type = literal_kind_to_type_name(default_kind);
                diagnostics.push(Diagnostic {
                    message: format!(
                        "Type mismatch: expected {} value for type '{}', found {}",
                        prim, type_expr, actual_type
                    ),
                    severity: Severity::Error,
                    span: default_span,
                });
            }
        }
        ResolvedType::StringUnion(ref variants) => {
            // Default must be a string literal that's one of the variants
            if default_kind != "string_literal" {
                let actual_type = literal_kind_to_type_name(default_kind);
                diagnostics.push(Diagnostic {
                    message: format!(
                        "Type mismatch: expected one of {:?}, found {}",
                        variants, actual_type
                    ),
                    severity: Severity::Error,
                    span: default_span,
                });
                return;
            }
            
            // Extract the string value (without quotes)
            let value = extract_string_literal_value(default_node, source);
            if !variants.contains(&value) {
                diagnostics.push(Diagnostic {
                    message: format!(
                        "Invalid default value \"{}\": expected one of {:?}",
                        value, variants
                    ),
                    severity: Severity::Error,
                    span: default_span,
                });
            }
        }
        ResolvedType::Array(ref element_type) => {
            // Default must be an array literal
            if default_kind != "array_literal" {
                let actual_type = literal_kind_to_type_name(default_kind);
                diagnostics.push(Diagnostic {
                    message: format!(
                        "Type mismatch: expected array value for type '{}', found {}",
                        type_expr, actual_type
                    ),
                    severity: Severity::Error,
                    span: default_span,
                });
                return;
            }
            
            // Validate each element in the array
            validate_array_elements(
                default_node,
                element_type,
                type_expr,
                source,
                diagnostics,
            );
        }
        ResolvedType::Model(_) => {
            // Models expect object literals
            if default_kind != "object_literal" {
                let actual_type = literal_kind_to_type_name(default_kind);
                diagnostics.push(Diagnostic {
                    message: format!(
                        "Type mismatch: expected object value for type '{}', found {}",
                        type_expr, actual_type
                    ),
                    severity: Severity::Error,
                    span: default_span,
                });
            }
        }
        ResolvedType::Unknown => {
            // Unknown type - skip validation (might be special types like DateTime, JSON)
        }
    }
}

/// Validate elements of an array literal against the expected element type.
fn validate_array_elements(
    array_node: tree_sitter::Node,
    element_type: &ResolvedType,
    full_type_expr: &str,
    source: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let expected_literal = match element_type {
        ResolvedType::Primitive(prim) => match prim.as_str() {
            "string" => Some("string_literal"),
            "number" => Some("number_literal"),
            "boolean" => Some("boolean_literal"),
            _ => None,
        },
        ResolvedType::StringUnion(_) => Some("string_literal"),
        ResolvedType::Model(_) => Some("object_literal"),
        _ => None,
    };
    
    let Some(expected) = expected_literal else {
        return; // Can't validate unknown element types
    };
    
    let mut cursor = array_node.walk();
    for child in array_node.children(&mut cursor) {
        // Skip brackets and commas
        let kind = child.kind();
        if kind == "[" || kind == "]" || kind == "," {
            continue;
        }
        
        if kind != expected {
            let actual_type = literal_kind_to_type_name(kind);
            diagnostics.push(Diagnostic {
                message: format!(
                    "Type mismatch in array: expected {} element for type '{}', found {}",
                    element_type_name(element_type), full_type_expr, actual_type
                ),
                severity: Severity::Error,
                span: node_span(child),
            });
        } else if let ResolvedType::StringUnion(variants) = element_type {
            // For string union arrays, check that each string is a valid variant
            if kind == "string_literal" {
                let value = extract_string_literal_value(child, source);
                if !variants.contains(&value) {
                    diagnostics.push(Diagnostic {
                        message: format!(
                            "Invalid array element \"{}\": expected one of {:?}",
                            value, variants
                        ),
                        severity: Severity::Error,
                        span: node_span(child),
                    });
                }
            }
        }
    }
}

/// Convert a literal node kind to a human-readable type name.
fn literal_kind_to_type_name(kind: &str) -> &str {
    match kind {
        "string_literal" => "string",
        "number_literal" => "number",
        "boolean_literal" => "boolean",
        "array_literal" => "array",
        "object_literal" => "object",
        _ => kind,
    }
}

/// Get a human-readable name for a resolved element type.
fn element_type_name(resolved: &ResolvedType) -> &str {
    match resolved {
        ResolvedType::Primitive(p) => p.as_str(),
        ResolvedType::StringUnion(_) => "string",
        ResolvedType::Array(_) => "array",
        ResolvedType::Model(_) => "object",
        ResolvedType::Unknown => "unknown",
    }
}

/// Extract the string content from a string_literal node (without quotes).
fn extract_string_literal_value(node: tree_sitter::Node, source: &str) -> String {
    // The string_literal contains: "content" with possible escape sequences
    // For simplicity, we'll just extract the text between quotes
    let text = get_node_text(node, source);
    if text.starts_with('"') && text.ends_with('"') && text.len() >= 2 {
        // Handle escape sequences - for now, just strip the quotes
        // TODO: properly handle escape sequences like \n, \", etc.
        text[1..text.len()-1].to_string()
    } else {
        text.to_string()
    }
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
            // type_identifier can contain either a simple identifier or a qualified_identifier
            let mut cursor = node.walk();
            let first_child = node.children(&mut cursor).next();

            if let Some(child) = first_child {
                if child.kind() == "qualified_identifier" {
                    // Handle qualified type: namespace.Type (e.g., pg.UUID, auth.Role)
                    let qualified_name = get_node_text(child, source);
                    if !is_type_reference_defined(qualified_name, symbol_table, ancestors) {
                        diagnostics.push(Diagnostic {
                            message: format!("Undefined type '{}'", qualified_name),
                            severity: Severity::Error,
                            span: node_span(child),
                        });
                    }
                } else {
                    // Simple identifier
                    let type_name = get_node_text(node, source);
                    if !is_type_defined(type_name, symbol_table, ancestors) {
                        diagnostics.push(Diagnostic {
                            message: format!("Undefined type '{}'", type_name),
                            severity: Severity::Error,
                            span: node_span(node),
                        });
                    }
                }
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