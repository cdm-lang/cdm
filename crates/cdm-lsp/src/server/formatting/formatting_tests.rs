use super::*;

#[test]
fn test_format_document_with_bad_whitespace() {
    use std::io::Write;

    let text = r#"Email:string#1

User{
id:string#1
email:Email#2
}#10
"#;

    // Create a real temporary file
    let mut temp_file = tempfile::Builder::new()
        .suffix(".cdm")
        .tempfile()
        .unwrap();
    write!(temp_file, "{}", text).unwrap();
    temp_file.flush().unwrap();

    let uri = Url::from_file_path(temp_file.path()).unwrap();

    let edits = format_document(text, &uri, false);
    assert!(edits.is_some());

    let edits = edits.unwrap();
    assert_eq!(edits.len(), 1);

    let formatted = &edits[0].new_text;
    // Should have proper spacing
    assert!(formatted.contains("Email: string #1"));
    assert!(formatted.contains("User {"));
    assert!(formatted.contains("  id: string #1"));
    assert!(formatted.contains("} #10"));
}

#[test]
fn test_format_document_no_changes() {
    let text = r#"Email: string #1

User {
  id: string #1
} #10
"#;

    let uri = Url::parse("file:///test.cdm").unwrap();

    let edits = format_document(text, &uri, false);
    // Should return None if no changes
    assert!(edits.is_none());
}

#[test]
fn test_format_document_invalid_syntax() {
    let text = "This is not valid CDM syntax { { {";

    let uri = Url::parse("file:///test.cdm").unwrap();

    let edits = format_document(text, &uri, false);
    // Should return None if formatting fails
    assert!(edits.is_none());
}
