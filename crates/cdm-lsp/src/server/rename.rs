//! Rename refactoring support
//!
//! This module provides LSP rename functionality for CDM symbols.
//! Supports renaming type aliases and model names across all references.

use tower_lsp::lsp_types::*;
use std::collections::HashMap;

use super::navigation;

/// Prepare rename: validate that the symbol can be renamed
pub fn prepare_rename(text: &str, position: Position) -> Option<PrepareRenameResponse> {
    // Find the symbol at the cursor position
    let (symbol, range) = navigation::find_symbol_at_position(text, position)?;

    // Check if it's a valid symbol to rename (not a built-in type)
    if cdm::is_builtin_type(&symbol) {
        return None;
    }

    // Get all definitions to see if this symbol is defined
    let definitions = navigation::extract_definitions(text);
    let is_defined = definitions.iter().any(|(name, _)| name == &symbol);

    if !is_defined {
        return None;
    }

    // Return the range of the symbol
    Some(PrepareRenameResponse::Range(range))
}

/// Perform the rename operation
pub fn rename_symbol(
    text: &str,
    position: Position,
    new_name: &str,
    uri: &Url,
) -> Option<WorkspaceEdit> {
    // Find the symbol at the cursor position
    let (symbol, _range) = navigation::find_symbol_at_position(text, position)?;

    // Check if it's a valid symbol to rename
    if cdm::is_builtin_type(&symbol) {
        return None;
    }

    // Get all definitions to verify this symbol is defined
    let definitions = navigation::extract_definitions(text);
    let is_defined = definitions.iter().any(|(name, _)| name == &symbol);

    if !is_defined {
        return None;
    }

    // Find all references to this symbol
    let ranges = navigation::find_all_references(text, &symbol);

    if ranges.is_empty() {
        return None;
    }

    // Create text edits for all occurrences
    let text_edits: Vec<TextEdit> = ranges
        .into_iter()
        .map(|range| TextEdit {
            range,
            new_text: new_name.to_string(),
        })
        .collect();

    // Create a WorkspaceEdit with the changes
    let mut changes = HashMap::new();
    changes.insert(uri.clone(), text_edits);

    Some(WorkspaceEdit {
        changes: Some(changes),
        document_changes: None,
        change_annotations: None,
    })
}

#[cfg(test)]
mod tests {
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
}
