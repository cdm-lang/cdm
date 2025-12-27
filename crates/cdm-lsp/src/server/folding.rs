//! Code folding support for CDM documents
//!
//! This module provides LSP folding ranges for CDM syntax elements:
//! - Model bodies
//! - Plugin blocks
//! - Object literals (for plugin configs)

use tower_lsp::lsp_types::*;
use tree_sitter::{Node, Parser};

use super::position::byte_offset_to_lsp_position;

/// Compute folding ranges for the given CDM document
pub fn compute_folding_ranges(text: &str) -> Option<Vec<FoldingRange>> {
    let mut parser = Parser::new();
    parser.set_language(&grammar::LANGUAGE.into()).ok()?;
    let tree = parser.parse(text, None)?;

    let mut ranges = Vec::new();
    let root = tree.root_node();

    collect_folding_ranges(&root, text, &mut ranges);

    if ranges.is_empty() {
        None
    } else {
        Some(ranges)
    }
}

/// Recursively collect folding ranges from the syntax tree
fn collect_folding_ranges(node: &Node, text: &str, ranges: &mut Vec<FoldingRange>) {
    match node.kind() {
        "model_body" => {
            if let Some(range) = create_folding_range_for_braces(node, text) {
                ranges.push(range);
            }
        }
        "plugin_block" => {
            if let Some(range) = create_folding_range_for_braces(node, text) {
                ranges.push(range);
            }
        }
        "object_literal" => {
            if let Some(range) = create_folding_range_for_braces(node, text) {
                ranges.push(range);
            }
        }
        _ => {}
    }

    // Recursively process children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_folding_ranges(&child, text, ranges);
    }
}

/// Create a folding range for a node that has braces
fn create_folding_range_for_braces(node: &Node, text: &str) -> Option<FoldingRange> {
    // Get the start and end positions
    let start_pos = byte_offset_to_lsp_position(text, node.start_byte());
    let end_pos = byte_offset_to_lsp_position(text, node.end_byte());

    // Only create a folding range if it spans multiple lines
    if end_pos.line <= start_pos.line {
        return None;
    }

    // Create the folding range
    // Start at the line with the opening brace, end at the line with the closing brace
    Some(FoldingRange {
        start_line: start_pos.line,
        start_character: None, // Use None to fold from end of line
        end_line: end_pos.line,
        end_character: None, // Use None to fold to end of line
        kind: Some(FoldingRangeKind::Region),
        collapsed_text: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_folding() {
        let text = r#"User {
  name: string #1
  email: string #2
  age: number #3
} #10"#;

        let ranges = compute_folding_ranges(text).unwrap();

        // Should have one folding range for the model body
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].start_line, 0); // Line with opening brace
        assert_eq!(ranges[0].end_line, 4);   // Line with closing brace
        assert_eq!(ranges[0].kind, Some(FoldingRangeKind::Region));
    }

    #[test]
    fn test_nested_folding() {
        let text = r#"User {
  name: string {
    @sql { type: "VARCHAR(255)" }
  } #1
} #10"#;

        let ranges = compute_folding_ranges(text).unwrap();

        // Should have folding ranges for:
        // 1. Model body
        // 2. Plugin block
        // 3. Object literal
        assert!(ranges.len() >= 2);
    }

    #[test]
    fn test_multiple_models_folding() {
        let text = r#"User {
  name: string #1
} #10

Admin extends User {
  level: number #1
} #11

Post {
  title: string #1
  content: string #2
} #12"#;

        let ranges = compute_folding_ranges(text).unwrap();

        // Should have three folding ranges (one for each model body)
        assert_eq!(ranges.len(), 3);
    }

    #[test]
    fn test_single_line_no_folding() {
        let text = "Email: string #1";

        let ranges = compute_folding_ranges(text);

        // Should have no folding ranges for single-line content
        assert!(ranges.is_none() || ranges.unwrap().is_empty());
    }

    #[test]
    fn test_plugin_config_folding() {
        let text = r#"@sql {
  dialect: "postgres",
  schema: "public"
}

User {
  name: string #1
} #10"#;

        let ranges = compute_folding_ranges(text).unwrap();

        // Should have folding ranges for:
        // 1. Plugin config object
        // 2. Model body
        assert!(ranges.len() >= 2);
    }
}
