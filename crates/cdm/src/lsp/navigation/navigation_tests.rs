use super::*;

#[test]
fn test_extract_type_alias() {
    let text = r#"Email: string #1"#;
    let definitions = extract_definitions(text);

    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions[0].0, "Email");
    assert_eq!(definitions[0].1.kind, DefinitionKind::TypeAlias);
    assert!(definitions[0].1.hover_text.contains("Email: string"));
}

#[test]
fn test_extract_multiple_type_aliases() {
    let text = r#"
Email: string #1
Status: "active" | "inactive" #2
"#;
    let definitions = extract_definitions(text);

    assert_eq!(definitions.len(), 2);
    assert_eq!(definitions[0].0, "Email");
    assert_eq!(definitions[1].0, "Status");
}

#[test]
fn test_extract_model() {
    let text = r#"
User {
  name: string #1
  email: string #2
} #10
"#;
    let definitions = extract_definitions(text);

    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions[0].0, "User");
    assert_eq!(definitions[0].1.kind, DefinitionKind::Model);
    assert!(definitions[0].1.hover_text.contains("User"));
}

#[test]
fn test_extract_model_with_extends() {
    let text = r#"
BaseUser {
  id: string #1
} #10

AdminUser extends BaseUser {
  role: string #1
} #20
"#;
    let definitions = extract_definitions(text);

    assert_eq!(definitions.len(), 2);
    assert_eq!(definitions[0].0, "BaseUser");
    assert_eq!(definitions[1].0, "AdminUser");
    assert!(definitions[1].1.hover_text.contains("extends BaseUser"));
}

#[test]
fn test_find_symbol_at_position_type_alias() {
    let text = r#"Email: string #1

User {
  email: Email #1
} #10
"#;
    // Position on "Email" in the field declaration (line 3, character 9)
    let position = Position { line: 3, character: 9 };

    let result = find_symbol_at_position(text, position);
    assert!(result.is_some());

    let (symbol, _range) = result.unwrap();
    assert_eq!(symbol, "Email");
}

#[test]
fn test_find_all_references() {
    let text = r#"Email: string #1

User {
  email: Email #1
  backup_email: Email #2
} #10
"#;
    let references = find_all_references(text, "Email");

    // Should find: definition + 2 uses = 3 total
    assert_eq!(references.len(), 3);
}

#[test]
fn test_find_references_to_model() {
    let text = r#"User {
  name: string #1
} #10

Post {
  author: User #1
  reviewer: User #2
} #20
"#;
    let references = find_all_references(text, "User");

    // Should find: definition + 2 uses = 3 total
    assert_eq!(references.len(), 3);
}

#[test]
fn test_mixed_types_and_models() {
    let text = r#"Email: string #1

User {
  name: string #1
  email: Email #2
} #10

AdminUser extends User {
  role: string #1
} #20
"#;
    let definitions = extract_definitions(text);

    assert_eq!(definitions.len(), 3);

    // Check type alias
    assert_eq!(definitions[0].0, "Email");
    assert_eq!(definitions[0].1.kind, DefinitionKind::TypeAlias);

    // Check models
    assert_eq!(definitions[1].0, "User");
    assert_eq!(definitions[1].1.kind, DefinitionKind::Model);

    assert_eq!(definitions[2].0, "AdminUser");
    assert_eq!(definitions[2].1.kind, DefinitionKind::Model);
}
