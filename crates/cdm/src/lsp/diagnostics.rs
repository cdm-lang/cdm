use std::path::Path;
use tower_lsp::lsp_types::*;
use crate::{
    validate::validate_with_templates,
    template_resolver::extract_template_imports,
    template_validation::validate_template_imports,
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

    let namespaces = if let Some(tree) = parser.parse(text, None) {
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

        ns
    } else {
        vec![]
    };

    // Validate with template namespaces
    let ancestors = vec![];
    let validation_result = validate_with_templates(text, &ancestors, namespaces);

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
