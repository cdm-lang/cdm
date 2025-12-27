// diagnostics.rs
use std::fmt;
use cdm_utils::Span;

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

// Plugin error codes
#[allow(dead_code)]
pub const E401_PLUGIN_NOT_FOUND: &str = "E401";
#[allow(dead_code)]
pub const E402_INVALID_PLUGIN_CONFIG: &str = "E402";
#[allow(dead_code)]
pub const E403_MISSING_PLUGIN_EXPORT: &str = "E403";
#[allow(dead_code)]
pub const E404_PLUGIN_EXECUTION_FAILED: &str = "E404";
#[allow(dead_code)]
pub const E405_PLUGIN_OUTPUT_TOO_LARGE: &str = "E405";
#[allow(dead_code)]
pub const E406_MISSING_OUTPUT_CONFIG: &str = "E406";

// Entity ID error codes
#[allow(dead_code)]
pub const E501_DUPLICATE_ENTITY_ID: &str = "E501";
#[allow(dead_code)]
pub const E502_DUPLICATE_FIELD_ID: &str = "E502";
#[allow(dead_code)]
pub const E503_REUSED_ID: &str = "E503";

// Entity ID warning codes
#[allow(dead_code)]
pub const W005_MISSING_ENTITY_ID: &str = "W005";
#[allow(dead_code)]
pub const W006_MISSING_FIELD_ID: &str = "W006";