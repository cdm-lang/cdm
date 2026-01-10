use super::*;
use cdm_utils::{Position, Span};

fn test_span() -> Span {
    Span {
        start: Position { line: 5, column: 10 },
        end: Position { line: 5, column: 20 },
    }
}

#[test]
fn test_severity_equality() {
    assert_eq!(Severity::Error, Severity::Error);
    assert_eq!(Severity::Warning, Severity::Warning);
    assert_ne!(Severity::Error, Severity::Warning);
}

#[test]
fn test_diagnostic_equality() {
    let diag1 = Diagnostic {
        message: "Test error".to_string(),
        severity: Severity::Error,
        span: test_span(),
    };

    let diag2 = Diagnostic {
        message: "Test error".to_string(),
        severity: Severity::Error,
        span: test_span(),
    };

    assert_eq!(diag1, diag2);
}

#[test]
fn test_diagnostic_inequality_message() {
    let diag1 = Diagnostic {
        message: "Error 1".to_string(),
        severity: Severity::Error,
        span: test_span(),
    };

    let diag2 = Diagnostic {
        message: "Error 2".to_string(),
        severity: Severity::Error,
        span: test_span(),
    };

    assert_ne!(diag1, diag2);
}

#[test]
fn test_diagnostic_inequality_severity() {
    let diag1 = Diagnostic {
        message: "Test".to_string(),
        severity: Severity::Error,
        span: test_span(),
    };

    let diag2 = Diagnostic {
        message: "Test".to_string(),
        severity: Severity::Warning,
        span: test_span(),
    };

    assert_ne!(diag1, diag2);
}

#[test]
fn test_diagnostic_display_error() {
    let diag = Diagnostic {
        message: "Undefined type 'Foo'".to_string(),
        severity: Severity::Error,
        span: Span {
            start: Position { line: 0, column: 5 },
            end: Position { line: 0, column: 10 },
        },
    };

    let output = format!("{}", diag);
    assert_eq!(output, "error[1:6]: Undefined type 'Foo'");
}

#[test]
fn test_diagnostic_display_warning() {
    let diag = Diagnostic {
        message: "Shadowing built-in type".to_string(),
        severity: Severity::Warning,
        span: Span {
            start: Position { line: 10, column: 0 },
            end: Position { line: 10, column: 15 },
        },
    };

    let output = format!("{}", diag);
    assert_eq!(output, "warning[11:1]: Shadowing built-in type");
}

#[test]
fn test_diagnostic_display_multiline_span() {
    let diag = Diagnostic {
        message: "Syntax error".to_string(),
        severity: Severity::Error,
        span: Span {
            start: Position { line: 5, column: 10 },
            end: Position { line: 7, column: 5 },
        },
    };

    let output = format!("{}", diag);
    // Display uses start position
    assert_eq!(output, "error[6:11]: Syntax error");
}

#[test]
fn test_diagnostic_clone() {
    let diag1 = Diagnostic {
        message: "Test error".to_string(),
        severity: Severity::Error,
        span: test_span(),
    };

    let diag2 = diag1.clone();
    assert_eq!(diag1, diag2);
}

#[test]
fn test_severity_debug() {
    assert_eq!(format!("{:?}", Severity::Error), "Error");
    assert_eq!(format!("{:?}", Severity::Warning), "Warning");
}

#[test]
fn test_diagnostic_debug() {
    let diag = Diagnostic {
        message: "Test".to_string(),
        severity: Severity::Error,
        span: test_span(),
    };

    let debug_output = format!("{:?}", diag);
    assert!(debug_output.contains("Test"));
    assert!(debug_output.contains("Error"));
}

#[test]
fn test_diagnostic_with_long_message() {
    let long_message = "This is a very long error message that contains a lot of information about what went wrong in the validation process. It should still be displayed correctly regardless of its length.";

    let diag = Diagnostic {
        message: long_message.to_string(),
        severity: Severity::Error,
        span: test_span(),
    };

    let output = format!("{}", diag);
    assert!(output.contains(long_message));
    assert!(output.starts_with("error[6:11]: "));
}

#[test]
fn test_diagnostic_with_special_characters() {
    let diag = Diagnostic {
        message: "Type 'User<T>' cannot extend 'Base<U>'".to_string(),
        severity: Severity::Error,
        span: test_span(),
    };

    let output = format!("{}", diag);
    assert!(output.contains("User<T>"));
    assert!(output.contains("Base<U>"));
}

#[test]
fn test_diagnostic_with_newlines_in_message() {
    let diag = Diagnostic {
        message: "Multiple errors:\n- Error 1\n- Error 2".to_string(),
        severity: Severity::Error,
        span: test_span(),
    };

    let output = format!("{}", diag);
    assert!(output.contains("Multiple errors"));
    assert!(output.contains("Error 1"));
    assert!(output.contains("Error 2"));
}

#[test]
fn test_diagnostic_span_zero_based() {
    // Verify that display adds 1 to line and column for human-readable output
    let diag = Diagnostic {
        message: "Error at start of file".to_string(),
        severity: Severity::Error,
        span: Span {
            start: Position { line: 0, column: 0 },
            end: Position { line: 0, column: 5 },
        },
    };

    let output = format!("{}", diag);
    // Should display as line 1, column 1 (not 0, 0)
    assert_eq!(output, "error[1:1]: Error at start of file");
}

#[test]
fn test_diagnostic_vector_operations() {
    let mut diagnostics = Vec::new();

    diagnostics.push(Diagnostic {
        message: "Error 1".to_string(),
        severity: Severity::Error,
        span: test_span(),
    });

    diagnostics.push(Diagnostic {
        message: "Warning 1".to_string(),
        severity: Severity::Warning,
        span: test_span(),
    });

    assert_eq!(diagnostics.len(), 2);

    let errors: Vec<_> = diagnostics.iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();

    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].message, "Error 1");
}

#[test]
fn test_severity_copy_trait() {
    let sev1 = Severity::Error;
    let sev2 = sev1; // Copy

    // Both should still be usable
    assert_eq!(sev1, Severity::Error);
    assert_eq!(sev2, Severity::Error);
}

#[test]
fn test_diagnostic_with_unicode() {
    let diag = Diagnostic {
        message: "Type '用户' is undefined 🚨".to_string(),
        severity: Severity::Error,
        span: test_span(),
    };

    let output = format!("{}", diag);
    assert!(output.contains("用户"));
    assert!(output.contains("🚨"));
}

#[test]
fn test_error_codes_are_unique() {
    let codes = vec![
        E401_PLUGIN_NOT_FOUND,
        E402_INVALID_PLUGIN_CONFIG,
        E403_MISSING_PLUGIN_EXPORT,
        E404_PLUGIN_EXECUTION_FAILED,
        E405_PLUGIN_OUTPUT_TOO_LARGE,
        E406_MISSING_OUTPUT_CONFIG,
        E501_DUPLICATE_ENTITY_ID,
        E502_DUPLICATE_FIELD_ID,
        E503_REUSED_ID,
        W005_MISSING_ENTITY_ID,
        W006_MISSING_FIELD_ID,
    ];

    // Check that all codes are unique
    for (i, code1) in codes.iter().enumerate() {
        for (j, code2) in codes.iter().enumerate() {
            if i != j {
                assert_ne!(code1, code2, "Error codes {} and {} are not unique", code1, code2);
            }
        }
    }
}

#[test]
fn test_error_codes_format() {
    // Verify error codes follow the expected format (E### or W###)
    assert!(E401_PLUGIN_NOT_FOUND.starts_with('E'));
    assert!(E402_INVALID_PLUGIN_CONFIG.starts_with('E'));
    assert!(W005_MISSING_ENTITY_ID.starts_with('W'));
    assert!(W006_MISSING_FIELD_ID.starts_with('W'));

    // Verify they have 4 characters (letter + 3 digits)
    assert_eq!(E401_PLUGIN_NOT_FOUND.len(), 4);
    assert_eq!(W005_MISSING_ENTITY_ID.len(), 4);
}
