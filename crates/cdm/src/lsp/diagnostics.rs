use std::path::{Path, PathBuf};
use tower_lsp::lsp_types::*;
use crate::{
    validate::{validate_with_templates, load_template_namespaces},
    template_resolver::extract_template_imports,
    template_validation::validate_template_imports,
    plugin_validation::validate_plugins,
    file_resolver::FileResolver,
    symbol_table::Ancestor,
    Diagnostic as CdmDiagnostic, Severity,
};

/// Compute diagnostics for a CDM document
pub fn compute_diagnostics(text: &str, uri: &Url) -> Vec<Diagnostic> {
    // Get the file path for resolving relative imports
    let source_path = uri.to_file_path().unwrap_or_default();

    // Parse to extract template imports
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&grammar::LANGUAGE.into())
        .expect("Failed to load grammar");

    // Track ancestor sources for plugin validation (source, path)
    let mut ancestor_sources: Vec<(String, PathBuf)> = Vec::new();

    let (namespaces, ancestors) = if let Some(tree) = parser.parse(text, None) {
        // Extract and load template imports
        let template_imports = extract_template_imports(
            tree.root_node(),
            text,
            &source_path,
        );

        // Validate template imports
        let template_diagnostics = validate_template_imports(&template_imports);
        if template_diagnostics.iter().any(|d| d.severity == Severity::Error) {
            // Return early with template import errors
            return template_diagnostics
                .iter()
                .map(|diag| cdm_diagnostic_to_lsp(diag))
                .collect();
        }

        // Load template namespaces
        let project_root = source_path.parent().unwrap_or_else(|| Path::new("."));
        let mut load_diagnostics = Vec::new();
        let ns = crate::validate::load_template_namespaces(
            &template_imports,
            project_root,
            &mut load_diagnostics,
        );

        // If there were loading errors, include them in diagnostics
        if !load_diagnostics.is_empty() {
            return load_diagnostics
                .iter()
                .map(|diag| cdm_diagnostic_to_lsp(diag))
                .collect();
        }

        // Load ancestors from extends directives
        let ancestors: Vec<Ancestor> = if source_path.exists() {
            match FileResolver::load(&source_path) {
                Ok(loaded_tree) => {
                    // Convert loaded files to ancestors (excluding the current file)
                    let mut converted_ancestors: Vec<Ancestor> = Vec::new();
                    let mut prior_ancestors: Vec<Ancestor> = Vec::new();

                    for loaded_file in loaded_tree.ancestors.iter() {
                        // Read source
                        let source = match loaded_file.source() {
                            Ok(s) => s,
                            Err(_) => continue,
                        };

                        // Parse to extract template imports for ancestor
                        let mut ancestor_parser = tree_sitter::Parser::new();
                        ancestor_parser
                            .set_language(&grammar::LANGUAGE.into())
                            .expect("Failed to load grammar");

                        let ancestor_namespaces = if let Some(parse_tree) = ancestor_parser.parse(&source, None) {
                            let ancestor_imports = extract_template_imports(
                                parse_tree.root_node(),
                                &source,
                                &loaded_file.path,
                            );

                            // Load template namespaces for ancestor
                            let ancestor_project_root = loaded_file.path.parent().unwrap_or(Path::new("."));
                            let mut template_diags = Vec::new();
                            load_template_namespaces(&ancestor_imports, ancestor_project_root, &mut template_diags)
                        } else {
                            Vec::new()
                        };

                        // Validate ancestor with its templates
                        let ancestor_result = validate_with_templates(&source, &prior_ancestors, ancestor_namespaces);

                        // Skip if validation errors (don't block current file validation)
                        if ancestor_result.has_errors() {
                            continue;
                        }

                        // Track source and path for plugin validation
                        ancestor_sources.push((source.clone(), loaded_file.path.clone()));

                        // Convert to Ancestor
                        let ancestor = ancestor_result.into_ancestor(loaded_file.path.display().to_string());
                        prior_ancestors.push(ancestor.clone());
                        converted_ancestors.push(ancestor);
                    }

                    converted_ancestors
                }
                Err(_) => vec![]
            }
        } else {
            vec![]
        };

        (ns, ancestors)
    } else {
        (vec![], vec![])
    };

    // Validate with template namespaces and ancestors
    let mut validation_result = validate_with_templates(text, &ancestors, namespaces);

    // Plugin validation (only if semantic validation passed and we have a parse tree)
    // Use cache_only=true to avoid blocking on network requests in the LSP
    if !validation_result.has_errors() {
        if let Some(ref parse_tree) = validation_result.tree {
            validate_plugins(
                parse_tree,
                text,
                &source_path,
                &ancestor_sources,
                &mut validation_result.diagnostics,
                true, // cache_only - don't download plugins, show error if not cached
            );
        }
    }

    // Convert CDM diagnostics to LSP diagnostics
    validation_result
        .diagnostics
        .iter()
        .map(|diag| cdm_diagnostic_to_lsp(diag))
        .collect()
}

/// Convert a CDM diagnostic to an LSP diagnostic
fn cdm_diagnostic_to_lsp(diag: &CdmDiagnostic) -> Diagnostic {
    // Convert CDM span (line/column 0-indexed) to LSP position
    let range = Range {
        start: Position {
            line: diag.span.start.line as u32,
            character: diag.span.start.column as u32,
        },
        end: Position {
            line: diag.span.end.line as u32,
            character: diag.span.end.column as u32,
        },
    };

    let severity = match diag.severity {
        Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
    };

    Diagnostic {
        range,
        severity: Some(severity),
        code: None, // TODO: Extract error codes from message
        source: Some("cdm".to_string()),
        message: diag.message.clone(),
        related_information: None,
        tags: None,
        code_description: None,
        data: None,
    }
}


#[cfg(test)]
#[path = "diagnostics/diagnostics_tests.rs"]
mod diagnostics_tests;
