use super::*;

#[test]
fn test_type_alias_symbol() {
    let text = "Email: string #1";
    let symbols = compute_document_symbols(text).unwrap();

    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].name, "Email");
    assert_eq!(symbols[0].kind, SymbolKind::TYPE_PARAMETER);
    assert!(symbols[0].detail.is_some());
}

#[test]
fn test_model_symbol_with_fields() {
    let text = r#"User {
  name: string #1
  email: Email #2
  age?: number #3
} #10"#;

    let symbols = compute_document_symbols(text).unwrap();

    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].name, "User");
    assert_eq!(symbols[0].kind, SymbolKind::CLASS);

    let children = symbols[0].children.as_ref().unwrap();
    assert_eq!(children.len(), 3);
    assert_eq!(children[0].name, "name");
    assert_eq!(children[1].name, "email");
    assert_eq!(children[2].name, "age");

    // Check optional field detail
    assert!(children[2].detail.as_ref().unwrap().starts_with('?'));
}

#[test]
fn test_model_with_extends() {
    let text = r#"AdminUser extends User, Timestamped {
  level: number #1
} #20"#;

    let symbols = compute_document_symbols(text).unwrap();

    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].name, "AdminUser");
    assert!(symbols[0].detail.as_ref().unwrap().contains("User"));
    assert!(symbols[0].detail.as_ref().unwrap().contains("Timestamped"));
}

#[test]
fn test_field_removal() {
    let text = r#"AdminUser extends User {
  -password
  admin_level: number #1
} #20"#;

    let symbols = compute_document_symbols(text).unwrap();

    let children = symbols[0].children.as_ref().unwrap();
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].name, "-password");
    assert_eq!(children[0].tags, Some(vec![SymbolTag::DEPRECATED]));
}

#[test]
fn test_multiple_definitions() {
    let text = r#"Email: string #1

User {
  name: string #1
  email: Email #2
} #10

Admin extends User {
  level: number #1
} #11"#;

    let symbols = compute_document_symbols(text).unwrap();

    assert_eq!(symbols.len(), 3);
    assert_eq!(symbols[0].name, "Email");
    assert_eq!(symbols[0].kind, SymbolKind::TYPE_PARAMETER);
    assert_eq!(symbols[1].name, "User");
    assert_eq!(symbols[1].kind, SymbolKind::CLASS);
    assert_eq!(symbols[2].name, "Admin");
    assert_eq!(symbols[2].kind, SymbolKind::CLASS);
}
