//! Document formatting features
//!
//! This module provides document formatting using the existing cdm::format_file() function.

use tower_lsp::lsp_types::*;

/// Format a CDM document and return text edits
pub fn format_document(text: &str, uri: &Url, assign_ids: bool) -> Option<Vec<TextEdit>> {
    // Convert URI to path
    let path = uri.to_file_path().ok()?;

    // Create a temporary file with the document content
    let temp_file = create_temp_file(&path, text).ok()?;

    // Format the file using crate::format_file
    let options = crate::FormatOptions {
        assign_ids,
        check: false,
        write: true,
        indent_size: 2,
        format_whitespace: true,
    };

    let _result = crate::format_file(temp_file.path(), &options).ok()?;

    // Read the formatted content
    let formatted_text = std::fs::read_to_string(temp_file.path()).ok()?;

    // If the content didn't change, return None
    if formatted_text == text {
        return None;
    }

    // Create a text edit that replaces the entire document
    let line_count = text.lines().count() as u32;
    let last_line = text.lines().last().unwrap_or("");
    let last_char = last_line.len() as u32;

    Some(vec![TextEdit {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: line_count.saturating_sub(1),
                character: last_char,
            },
        },
        new_text: formatted_text,
    }])
}

/// Create a temporary file with the document content
fn create_temp_file(original_path: &std::path::Path, content: &str) -> std::io::Result<tempfile::NamedTempFile> {
    use std::io::Write;

    let mut temp_file = if let Some(parent) = original_path.parent() {
        // Create temp file in the same directory as the original
        tempfile::Builder::new()
            .suffix(".cdm")
            .tempfile_in(parent)?
    } else {
        // Fallback to system temp directory
        tempfile::Builder::new()
            .suffix(".cdm")
            .tempfile()?
    };

    temp_file.write_all(content.as_bytes())?;
    temp_file.flush()?;

    Ok(temp_file)
}


#[cfg(test)]
#[path = "formatting/formatting_tests.rs"]
mod formatting_tests;
