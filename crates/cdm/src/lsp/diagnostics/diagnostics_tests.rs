use super::*;

#[test]
fn test_plugin_not_found_generates_e401() {
    // Use a fake plugin name that will never be cached
    let text = r#"
@nonexistent_plugin_xyz123

User {
  name: string #1
} #10
"#;

    let uri = Url::parse("file:///test.cdm").unwrap();
    let diagnostics = compute_diagnostics(text, &uri);

    // Should have E401 error for plugin not found
    assert!(diagnostics.len() > 0, "Expected at least one diagnostic for missing plugin");

    let first_diag = &diagnostics[0];
    assert_eq!(first_diag.severity, Some(DiagnosticSeverity::ERROR));
    assert!(
        first_diag.message.contains("E401") && first_diag.message.contains("not found"),
        "Expected E401 plugin not found error, got: {}",
        first_diag.message
    );
}

#[test]
fn test_valid_file_no_diagnostics() {
    let text = r#"
User {
  name: string #1
} #10
"#;

    let uri = Url::parse("file:///test.cdm").unwrap();
    let diagnostics = compute_diagnostics(text, &uri);

    // Should have no errors for valid CDM
    assert_eq!(diagnostics.len(), 0);
}

#[test]
fn test_unknown_type_error() {
    let text = r#"
User {
  email: UnknownType #1
} #10
"#;

    let uri = Url::parse("file:///test.cdm").unwrap();
    let diagnostics = compute_diagnostics(text, &uri);

    // Should have E103 error for undefined type
    assert!(diagnostics.len() > 0);

    let first_diag = &diagnostics[0];
    assert_eq!(first_diag.severity, Some(DiagnosticSeverity::ERROR));
    assert!(first_diag.message.contains("Undefined type"));
}
