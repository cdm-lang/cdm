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
    if crate::is_builtin_type(&symbol) {
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
    if crate::is_builtin_type(&symbol) {
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
#[path = "rename/rename_tests.rs"]
mod rename_tests;
