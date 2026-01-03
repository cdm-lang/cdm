use super::*;

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
