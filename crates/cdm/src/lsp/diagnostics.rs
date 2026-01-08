use tower_lsp::lsp_types::*;
use crate::{validate, Diagnostic as CdmDiagnostic, Severity};

/// Compute diagnostics for a CDM document
pub fn compute_diagnostics(text: &str, _uri: &Url) -> Vec<Diagnostic> {
    // For now, validate without ancestors (single-file validation)
    // TODO: In the future, we should resolve extends and pass ancestors
    let ancestors = vec![];

    let validation_result = validate(text, &ancestors);

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
