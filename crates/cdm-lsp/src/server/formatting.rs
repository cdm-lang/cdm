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

    // Format the file using cdm::format_file
    let options = cdm::FormatOptions {
        assign_ids,
        check: false,
        write: true,
        indent_size: 2,
        format_whitespace: true,
    };

    let _result = cdm::format_file(temp_file.path(), &options).ok()?;

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
mod tests {
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
}
