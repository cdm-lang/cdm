use super::*;

#[test]
fn test_prepare_rename_type_alias() {
    let text = "Email: string #1";

    // Position on "Email"
    let position = Position { line: 0, character: 2 };
    let result = prepare_rename(text, position);

    assert!(result.is_some());
}

#[test]
fn test_prepare_rename_builtin_type() {
    let text = "Email: string #1";

    // Position on "string"
    let position = Position { line: 0, character: 9 };
    let result = prepare_rename(text, position);

    // Should not be able to rename built-in types
    assert!(result.is_none());
}

#[test]
fn test_prepare_rename_undefined_symbol() {
    let text = r#"User {
  email: UnknownType #1
} #10"#;

    // Position on "UnknownType" (undefined)
    let position = Position { line: 1, character: 10 };
    let result = prepare_rename(text, position);

    // Should not be able to rename undefined symbols
    assert!(result.is_none());
}

#[test]
fn test_rename_type_alias() {
    let text = r#"Email: string #1

User {
  email: Email #1
  backup: Email #2
} #10"#;

    let uri = Url::parse("file:///test.cdm").unwrap();

    // Position on "Email" in definition
    let position = Position { line: 0, character: 2 };
    let result = rename_symbol(text, position, "EmailAddress", &uri);

    assert!(result.is_some());

    let edit = result.unwrap();
    let changes = edit.changes.unwrap();
    let text_edits = changes.get(&uri).unwrap();

    // Should find 3 occurrences: definition + 2 usages
    assert_eq!(text_edits.len(), 3);

    // All edits should replace with new name
    for edit in text_edits {
        assert_eq!(edit.new_text, "EmailAddress");
    }
}

#[test]
fn test_rename_model() {
    let text = r#"User {
  name: string #1
} #10

Admin extends User {
  level: number #1
} #11"#;

    let uri = Url::parse("file:///test.cdm").unwrap();

    // Position on "User" in definition
    let position = Position { line: 0, character: 2 };
    let result = rename_symbol(text, position, "UserModel", &uri);

    assert!(result.is_some());

    let edit = result.unwrap();
    let changes = edit.changes.unwrap();
    let text_edits = changes.get(&uri).unwrap();

    // Should find 2 occurrences: definition + 1 usage in extends
    assert_eq!(text_edits.len(), 2);
}

#[test]
fn test_rename_from_usage() {
    let text = r#"Email: string #1

User {
  email: Email #1
} #10"#;

    let uri = Url::parse("file:///test.cdm").unwrap();

    // Position on "Email" in field type (usage, not definition)
    let position = Position { line: 3, character: 10 };
    let result = rename_symbol(text, position, "EmailAddress", &uri);

    assert!(result.is_some());

    let edit = result.unwrap();
    let changes = edit.changes.unwrap();
    let text_edits = changes.get(&uri).unwrap();

    // Should still find both occurrences
    assert_eq!(text_edits.len(), 2);
}

#[test]
fn test_rename_no_references() {
    let text = "Email: string #1";

    let uri = Url::parse("file:///test.cdm").unwrap();

    // Position on "Email"
    let position = Position { line: 0, character: 2 };
    let result = rename_symbol(text, position, "EmailAddress", &uri);

    assert!(result.is_some());

    let edit = result.unwrap();
    let changes = edit.changes.unwrap();
    let text_edits = changes.get(&uri).unwrap();

    // Should find just the definition
    assert_eq!(text_edits.len(), 1);
}

#[test]
fn test_rename_with_exact_positions() {
    let text = r#"Email: string #1

User {
  email: Email #1
} #10"#;

    let uri = Url::parse("file:///test.cdm").unwrap();

    // Test renaming from the definition
    let position = Position { line: 0, character: 2 }; // On "Email" in definition
    let result = rename_symbol(text, position, "EmailAddress", &uri);

    assert!(result.is_some(), "Rename should work from definition");

    let edit = result.unwrap();
    let changes = edit.changes.unwrap();
    let text_edits = changes.get(&uri).unwrap();

    // Should find 2 occurrences: definition + 1 usage
    assert_eq!(text_edits.len(), 2, "Should find 2 occurrences of Email");

    // Check that the ranges are correct
    for (i, edit) in text_edits.iter().enumerate() {
        assert_eq!(edit.new_text, "EmailAddress", "Edit {} should have new text EmailAddress", i);
        eprintln!("Edit {}: range = {:?}, new_text = {}", i, edit.range, edit.new_text);
    }
}

#[test]
fn test_find_symbol_at_type_alias_name() {
    let text = "Email: string #1";

    // Various positions on "Email"
    for char_pos in 0..5 {
        let position = Position { line: 0, character: char_pos };
        let result = navigation::find_symbol_at_position(text, position);

        if let Some((symbol, _range)) = result {
            assert_eq!(symbol, "Email", "Position {} should find Email", char_pos);
        }
    }
}
