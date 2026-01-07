//! Template validation logic for CDM templates
//!
//! This module provides validation functions for template imports:
//! - Validating namespace uniqueness
//! - Checking for unknown namespace references
//! - Validating qualified type references

use std::collections::HashSet;
use std::path::Path;

use crate::diagnostics::{Diagnostic, Severity};
use crate::symbol_table::{QualifiedName, SymbolTable};
use crate::template_resolver::{extract_template_extends, extract_template_imports, TemplateExtends, TemplateImport};
use cdm_utils::Span;

/// Validate template imports for namespace conflicts
///
/// Returns diagnostics for any issues found:
/// - E605: Duplicate namespace
pub fn validate_template_imports(
    imports: &[TemplateImport],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen_namespaces: HashSet<String> = HashSet::new();

    for import in imports {
        if seen_namespaces.contains(&import.namespace) {
            diagnostics.push(Diagnostic {
                message: format!(
                    "E605: Duplicate namespace '{}'. Each template import must use a unique namespace.",
                    import.namespace
                ),
                severity: Severity::Error,
                span: import.span.clone(),
            });
        } else {
            seen_namespaces.insert(import.namespace.clone());
        }
    }

    diagnostics
}

/// Validate that a qualified type reference exists
///
/// Returns diagnostics for:
/// - E606: Unknown namespace
/// - E001: Unknown type in namespace
pub fn validate_qualified_type_reference(
    type_ref: &str,
    span: &Span,
    symbol_table: &SymbolTable,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if let Some(qualified) = QualifiedName::parse(type_ref) {
        let root_ns = qualified.root_namespace();

        // Check if namespace exists
        if !symbol_table.has_namespace(root_ns) {
            diagnostics.push(Diagnostic {
                message: format!(
                    "E606: Unknown namespace '{}'. Did you forget to import a template?",
                    root_ns
                ),
                severity: Severity::Error,
                span: span.clone(),
            });
            return diagnostics;
        }

        // Check if the type exists in the namespace
        let mut current_ns = symbol_table.get_namespace(root_ns);

        // Navigate through nested namespaces
        for (i, ns_part) in qualified.namespace_parts[1..].iter().enumerate() {
            if let Some(ns) = current_ns {
                current_ns = ns.symbol_table.get_namespace(ns_part);
                if current_ns.is_none() {
                    let path: Vec<&str> = qualified.namespace_parts[..=i+1].iter().map(|s| s.as_str()).collect();
                    diagnostics.push(Diagnostic {
                        message: format!(
                            "E606: Unknown namespace '{}' in '{}'",
                            ns_part,
                            path.join(".")
                        ),
                        severity: Severity::Error,
                        span: span.clone(),
                    });
                    return diagnostics;
                }
            }
        }

        // Check if type exists in final namespace
        if let Some(ns) = current_ns {
            if !ns.symbol_table.is_defined(&qualified.name) {
                diagnostics.push(Diagnostic {
                    message: format!(
                        "E001: Unknown type '{}' in namespace '{}'",
                        qualified.name,
                        qualified.namespace_parts.join(".")
                    ),
                    severity: Severity::Error,
                    span: span.clone(),
                });
            }
        }
    }

    diagnostics
}

/// Extract template imports from parsed source
pub fn extract_templates_from_source(
    tree: &tree_sitter::Tree,
    source: &str,
    source_file: &Path,
) -> (Vec<TemplateImport>, Vec<TemplateExtends>) {
    let root = tree.root_node();
    let imports = extract_template_imports(root, source, source_file);
    let extends = extract_template_extends(root, source, source_file);
    (imports, extends)
}

/// Check if all imported namespaces are used
///
/// Returns warnings for:
/// - W101: Template imported but never used
pub fn check_unused_namespaces(
    imports: &[TemplateImport],
    used_namespaces: &HashSet<String>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for import in imports {
        if !used_namespaces.contains(&import.namespace) {
            diagnostics.push(Diagnostic {
                message: format!(
                    "W101: Template '{}' imported as '{}' but never used",
                    get_template_source_name(&import.source),
                    import.namespace
                ),
                severity: Severity::Warning,
                span: import.span.clone(),
            });
        }
    }

    diagnostics
}

/// Get a display name for a template source
fn get_template_source_name(source: &crate::template_resolver::TemplateSource) -> String {
    match source {
        crate::template_resolver::TemplateSource::Registry { name } => name.clone(),
        crate::template_resolver::TemplateSource::Git { url } => url.clone(),
        crate::template_resolver::TemplateSource::Local { path } => path.clone(),
    }
}

/// Extract used namespaces from type references in the source
///
/// Scans through all type identifiers and collects namespaces used in qualified names
pub fn collect_used_namespaces(
    root: tree_sitter::Node,
    source: &str,
) -> HashSet<String> {
    let mut used = HashSet::new();
    collect_namespaces_recursive(root, source, &mut used);
    used
}

fn collect_namespaces_recursive(
    node: tree_sitter::Node,
    source: &str,
    used: &mut HashSet<String>,
) {
    // Check if this is a qualified_identifier
    if node.kind() == "qualified_identifier" {
        if let Some(ns_node) = node.child_by_field_name("namespace") {
            if let Ok(ns) = ns_node.utf8_text(source.as_bytes()) {
                used.insert(ns.to_string());
            }
        }
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_namespaces_recursive(child, source, used);
    }
}

#[cfg(test)]
#[path = "template_validation/template_validation_tests.rs"]
mod template_validation_tests;
