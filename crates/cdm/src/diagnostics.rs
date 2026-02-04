// diagnostics.rs
use std::fmt;
use cdm_utils::{Position, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub message: String,
    pub severity: Severity,
    pub span: Span,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let severity = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(
            f,
            "{}[{}:{}]: {}",
            severity,
            self.span.start.line + 1,
            self.span.start.column + 1,
            self.message
        )
    }
}

// Semantic error codes
pub const E104_RESERVED_TYPE_NAME: &str = "E104";

// Plugin error codes
pub const E401_PLUGIN_NOT_FOUND: &str = "E401";
pub const E402_INVALID_PLUGIN_CONFIG: &str = "E402";
pub const E403_MISSING_PLUGIN_EXPORT: &str = "E403";
pub const E404_PLUGIN_EXECUTION_FAILED: &str = "E404";
#[allow(dead_code)]
pub const E405_PLUGIN_OUTPUT_TOO_LARGE: &str = "E405";
#[allow(dead_code)]
pub const E406_MISSING_OUTPUT_CONFIG: &str = "E406";

// Entity ID error codes
pub const E501_DUPLICATE_ENTITY_ID: &str = "E501";
pub const E502_DUPLICATE_FIELD_ID: &str = "E502";
#[allow(dead_code)]
pub const E503_REUSED_ID: &str = "E503";

// Template error codes
pub const E601_TEMPLATE_NOT_FOUND: &str = "E601";
#[allow(dead_code)]
pub const E602_TEMPLATE_RESOLUTION_FAILED: &str = "E602";

// Entity ID warning codes
pub const W005_MISSING_ENTITY_ID: &str = "W005";
pub const W006_MISSING_FIELD_ID: &str = "W006";

/// Convert a tree-sitter Node to a Span for diagnostic reporting.
pub fn node_span(node: tree_sitter::Node) -> Span {
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

#[cfg(test)]
#[path = "diagnostics/diagnostics_tests.rs"]
mod diagnostics_tests;