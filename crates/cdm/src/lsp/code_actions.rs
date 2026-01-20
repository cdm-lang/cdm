//! Code actions (quick fixes) for CDM documents
//!
//! This module provides LSP code actions including:
//! - Add missing entity ID (W005)
//! - Add missing field ID (W006)
//! - Create type alias for undefined types
//! - Download missing plugin (E401)
//! - Download missing template (E601)

use tower_lsp::lsp_types::*;
use tree_sitter::{Node, Parser};
use std::collections::{HashMap, HashSet};

use super::position::byte_span_to_lsp_range;

/// Compute code actions for the given range in the document
///
/// # Arguments
/// * `text` - The document text
/// * `range` - The range where code actions are requested (cursor position)
/// * `cursor_diagnostics` - Diagnostics at the cursor position (from client)
/// * `all_diagnostics` - All diagnostics in the document (for checking other missing plugins)
/// * `uri` - The document URI
pub fn compute_code_actions(
    text: &str,
    range: Range,
    cursor_diagnostics: &[Diagnostic],
    all_diagnostics: &[Diagnostic],
    uri: &Url,
) -> Option<Vec<CodeActionOrCommand>> {
    let mut actions = Vec::new();
    let mut missing_plugins_at_cursor: HashSet<String> = HashSet::new();
    let mut missing_templates_at_cursor: HashSet<String> = HashSet::new();

    // Parse the document
    let mut parser = Parser::new();
    parser.set_language(&grammar::LANGUAGE.into()).ok()?;
    let tree = parser.parse(text, None)?;
    let root = tree.root_node();

    // Check for diagnostics in the range that we can fix
    for diagnostic in cursor_diagnostics {
        // Check if diagnostic overlaps with the requested range
        if !ranges_overlap(&diagnostic.range, &range) {
            continue;
        }

        // Handle W005: Missing entity ID on model
        if diagnostic.message.contains("W005") || diagnostic.message.contains("missing entity ID") {
            if let Some(action) = create_add_entity_id_action(text, &diagnostic.range, &root, uri) {
                actions.push(CodeActionOrCommand::CodeAction(action));
            }
        }

        // Handle W006: Missing field ID
        if diagnostic.message.contains("W006") || diagnostic.message.contains("missing field ID") {
            if let Some(action) = create_add_field_id_action(text, &diagnostic.range, &root, uri) {
                actions.push(CodeActionOrCommand::CodeAction(action));
            }
        }

        // Handle undefined type errors
        if diagnostic.message.contains("Undefined type") {
            if let Some(action) = create_type_alias_action(text, &diagnostic.range, uri) {
                actions.push(CodeActionOrCommand::CodeAction(action));
            }
        }

        // Handle E401: Plugin not found (missing plugin)
        if diagnostic.message.contains("E401") || diagnostic.message.contains("Plugin not found") {
            if let Some(plugin_name) = extract_plugin_name(&diagnostic.message) {
                // Track unique plugin names to avoid duplicate actions
                if !missing_plugins_at_cursor.contains(&plugin_name) {
                    missing_plugins_at_cursor.insert(plugin_name.clone());

                    // Create action to download this specific plugin
                    let action = create_download_plugin_action(&plugin_name);
                    actions.push(CodeActionOrCommand::CodeAction(action));
                }
            }
        }

        // Handle E601: Template not found (missing template)
        if diagnostic.message.contains("E601") || diagnostic.message.contains("Template not found") {
            if let Some(template_name) = extract_template_name(&diagnostic.message) {
                // Track unique template names to avoid duplicate actions
                if !missing_templates_at_cursor.contains(&template_name) {
                    missing_templates_at_cursor.insert(template_name.clone());

                    // Create action to download this specific template
                    let action = create_download_template_action(&template_name);
                    actions.push(CodeActionOrCommand::CodeAction(action));
                }
            }
        }
    }

    // Count all missing plugins in the document (not just at cursor)
    let all_missing_plugins: HashSet<String> = all_diagnostics
        .iter()
        .filter(|d| d.message.contains("E401") || d.message.contains("Plugin not found"))
        .filter_map(|d| extract_plugin_name(&d.message))
        .collect();

    // Show "download all plugins" if there are missing plugins at cursor AND at least one other
    // missing plugin elsewhere in the document (i.e., total > plugins at cursor)
    if !missing_plugins_at_cursor.is_empty() && all_missing_plugins.len() > missing_plugins_at_cursor.len() {
        let download_all_action = create_download_all_plugins_action();
        actions.push(CodeActionOrCommand::CodeAction(download_all_action));
    }

    // Count all missing templates in the document (not just at cursor)
    let all_missing_templates: HashSet<String> = all_diagnostics
        .iter()
        .filter(|d| d.message.contains("E601") || d.message.contains("Template not found"))
        .filter_map(|d| extract_template_name(&d.message))
        .collect();

    // Show "download all templates" if there are missing templates at cursor AND at least one other
    // missing template elsewhere in the document (i.e., total > templates at cursor)
    if !missing_templates_at_cursor.is_empty() && all_missing_templates.len() > missing_templates_at_cursor.len() {
        let download_all_action = create_download_all_templates_action();
        actions.push(CodeActionOrCommand::CodeAction(download_all_action));
    }

    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}

/// Extract plugin name from error messages like:
/// - "E401: Plugin not found: 'typescript' - Plugin 'typescript' not found in cache..."
/// - "Plugin 'typescript' not found in cache..."
fn extract_plugin_name(message: &str) -> Option<String> {
    // Try to match "Plugin 'name' not found"
    if let Some(start) = message.find("Plugin '") {
        let rest = &message[start + 8..];
        if let Some(end) = rest.find('\'') {
            return Some(rest[..end].to_string());
        }
    }

    // Try to match "Plugin not found: 'name'"
    if let Some(start) = message.find("Plugin not found: '") {
        let rest = &message[start + 19..];
        if let Some(end) = rest.find('\'') {
            return Some(rest[..end].to_string());
        }
    }

    None
}

/// Create a code action to download a missing plugin
fn create_download_plugin_action(plugin_name: &str) -> CodeAction {
    CodeAction {
        title: format!("Download plugin '{}'", plugin_name),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: None,
        edit: None,
        command: Some(Command {
            title: format!("Download plugin '{}'", plugin_name),
            command: "cdm.downloadPlugin".to_string(),
            arguments: Some(vec![serde_json::Value::String(plugin_name.to_string())]),
        }),
        is_preferred: Some(true),
        disabled: None,
        data: None,
    }
}

/// Create a code action to download all missing plugins
fn create_download_all_plugins_action() -> CodeAction {
    CodeAction {
        title: "Download all missing plugins".to_string(),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: None,
        edit: None,
        command: Some(Command {
            title: "Download all missing plugins".to_string(),
            command: "cdm.downloadAllPlugins".to_string(),
            arguments: None,
        }),
        is_preferred: Some(false),
        disabled: None,
        data: None,
    }
}

/// Extract template name from error messages like:
/// - "E601: Template not found: 'sql-types' - Template 'sql-types' not found in registry..."
/// - "Template 'sql-types' not found in registry..."
/// - "Failed to load template 'sql': ..."
fn extract_template_name(message: &str) -> Option<String> {
    // Try to match "Template 'name' not found"
    if let Some(start) = message.find("Template '") {
        let rest = &message[start + 10..];
        if let Some(end) = rest.find('\'') {
            return Some(rest[..end].to_string());
        }
    }

    // Try to match "Template not found: 'name'"
    if let Some(start) = message.find("Template not found: '") {
        let rest = &message[start + 21..];
        if let Some(end) = rest.find('\'') {
            return Some(rest[..end].to_string());
        }
    }

    // Try to match "Failed to load template 'name'"
    if let Some(start) = message.find("Failed to load template '") {
        let rest = &message[start + 25..];
        if let Some(end) = rest.find('\'') {
            return Some(rest[..end].to_string());
        }
    }

    None
}

/// Create a code action to download a missing template
fn create_download_template_action(template_name: &str) -> CodeAction {
    CodeAction {
        title: format!("Download template '{}'", template_name),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: None,
        edit: None,
        command: Some(Command {
            title: format!("Download template '{}'", template_name),
            command: "cdm.downloadTemplate".to_string(),
            arguments: Some(vec![serde_json::Value::String(template_name.to_string())]),
        }),
        is_preferred: Some(true),
        disabled: None,
        data: None,
    }
}

/// Create a code action to download all missing templates
fn create_download_all_templates_action() -> CodeAction {
    CodeAction {
        title: "Download all missing templates".to_string(),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: None,
        edit: None,
        command: Some(Command {
            title: "Download all missing templates".to_string(),
            command: "cdm.downloadAllTemplates".to_string(),
            arguments: None,
        }),
        is_preferred: Some(false),
        disabled: None,
        data: None,
    }
}

/// Check if two ranges overlap
fn ranges_overlap(a: &Range, b: &Range) -> bool {
    // Ranges overlap if one starts before the other ends
    !(a.end.line < b.start.line ||
      (a.end.line == b.start.line && a.end.character < b.start.character) ||
      b.end.line < a.start.line ||
      (b.end.line == a.start.line && b.end.character < a.start.character))
}

/// Create a code action to add a missing entity ID to a model
fn create_add_entity_id_action(
    text: &str,
    diagnostic_range: &Range,
    root: &Node,
    uri: &Url,
) -> Option<CodeAction> {
    // Find the model definition at this location
    let model_node = find_model_at_range(root, text, diagnostic_range)?;

    // Calculate the next available entity ID
    let next_id = calculate_next_entity_id(root, text);

    // Find the position to insert the ID (after the closing brace)
    let insert_position = find_model_end_position(&model_node, text)?;

    let edit = TextEdit {
        range: Range::new(insert_position, insert_position),
        new_text: format!(" #{}", next_id),
    };

    let mut changes = HashMap::new();
    changes.insert(uri.clone(), vec![edit]);

    Some(CodeAction {
        title: format!("Add entity ID #{}", next_id),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: None,
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }),
        command: None,
        is_preferred: Some(true),
        disabled: None,
        data: None,
    })
}

/// Create a code action to add a missing field ID
fn create_add_field_id_action(
    text: &str,
    diagnostic_range: &Range,
    root: &Node,
    uri: &Url,
) -> Option<CodeAction> {
    // Find the field definition at this location
    let field_node = find_field_at_range(root, text, diagnostic_range)?;

    // Calculate the next available field ID within the containing model
    let model_node = find_parent_model(&field_node)?;
    let next_id = calculate_next_field_id(&model_node, text);

    // Find the position to insert the ID (at the end of the field definition)
    let insert_position = find_field_end_position(&field_node, text)?;

    let edit = TextEdit {
        range: Range::new(insert_position, insert_position),
        new_text: format!(" #{}", next_id),
    };

    let mut changes = HashMap::new();
    changes.insert(uri.clone(), vec![edit]);

    Some(CodeAction {
        title: format!("Add field ID #{}", next_id),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: None,
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }),
        command: None,
        is_preferred: Some(true),
        disabled: None,
        data: None,
    })
}

/// Create a code action to create a type alias for an undefined type
fn create_type_alias_action(
    text: &str,
    diagnostic_range: &Range,
    uri: &Url,
) -> Option<CodeAction> {
    // Extract the undefined type name from the diagnostic range
    let type_name = extract_text_at_range(text, diagnostic_range)?;

    // Calculate next entity ID for the new type alias
    let mut parser = Parser::new();
    parser.set_language(&grammar::LANGUAGE.into()).ok()?;
    let tree = parser.parse(text, None)?;
    let root = tree.root_node();
    let next_id = calculate_next_entity_id(&root, text);

    // Insert at the beginning of the file (after any extends or @plugin directives)
    let insert_position = find_type_alias_insert_position(&root, text)?;

    let edit = TextEdit {
        range: Range::new(insert_position, insert_position),
        new_text: format!("{}: string #{}\n\n", type_name, next_id),
    };

    let mut changes = HashMap::new();
    changes.insert(uri.clone(), vec![edit]);

    Some(CodeAction {
        title: format!("Create type alias for '{}'", type_name),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: None,
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }),
        command: None,
        is_preferred: Some(false),
        disabled: None,
        data: None,
    })
}

/// Find a model definition node at the given range
fn find_model_at_range<'a>(node: &Node<'a>, text: &str, range: &Range) -> Option<Node<'a>> {
    if node.kind() == "model_definition" {
        let node_range = byte_span_to_lsp_range(text, node.start_byte(), node.end_byte());
        if ranges_overlap(&node_range, range) {
            return Some(*node);
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = find_model_at_range(&child, text, range) {
            return Some(found);
        }
    }

    None
}

/// Find a field definition node at the given range
fn find_field_at_range<'a>(node: &Node<'a>, text: &str, range: &Range) -> Option<Node<'a>> {
    if node.kind() == "field_definition" {
        let node_range = byte_span_to_lsp_range(text, node.start_byte(), node.end_byte());
        if ranges_overlap(&node_range, range) {
            return Some(*node);
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = find_field_at_range(&child, text, range) {
            return Some(found);
        }
    }

    None
}

/// Find the parent model of a field node
fn find_parent_model<'a>(field_node: &Node<'a>) -> Option<Node<'a>> {
    let mut current = field_node.parent()?;

    while current.kind() != "model_definition" {
        current = current.parent()?;
    }

    Some(current)
}

/// Find the end position of a model (after the closing brace, before any entity_id)
fn find_model_end_position(model_node: &Node, text: &str) -> Option<Position> {
    let mut cursor = model_node.walk();

    // Look for the model_body closing brace
    for child in model_node.children(&mut cursor) {
        if child.kind() == "model_body" {
            // The end of the model body is where we want to insert
            let end_byte = child.end_byte();
            return Some(byte_offset_to_position(text, end_byte));
        }
    }

    None
}

/// Find the end position of a field definition
fn find_field_end_position(field_node: &Node, text: &str) -> Option<Position> {
    let end_byte = field_node.end_byte();
    Some(byte_offset_to_position(text, end_byte))
}

/// Find the position to insert a new type alias (after directives, before definitions)
fn find_type_alias_insert_position(root: &Node, text: &str) -> Option<Position> {
    let mut cursor = root.walk();
    let mut last_directive_end = 0;

    for child in root.children(&mut cursor) {
        match child.kind() {
            "extends_template" | "plugin_import" | "template_import" => {
                last_directive_end = child.end_byte();
            }
            "type_alias" | "model_definition" => {
                // Insert before the first definition
                return Some(byte_offset_to_position(text, child.start_byte()));
            }
            _ => {}
        }
    }

    // If no definitions found, insert after directives (or at start if no directives)
    Some(byte_offset_to_position(text, last_directive_end))
}

/// Calculate the next available entity ID in the document
fn calculate_next_entity_id(root: &Node, text: &str) -> u32 {
    let mut max_id = 0;
    collect_max_entity_id(root, text, &mut max_id);
    max_id + 1
}

/// Recursively collect the maximum entity ID
fn collect_max_entity_id(node: &Node, text: &str, max_id: &mut u32) {
    if node.kind() == "entity_id" {
        if let Ok(id_text) = node.utf8_text(text.as_bytes()) {
            // Remove the '#' prefix and parse
            if let Ok(id) = id_text.trim_start_matches('#').parse::<u32>() {
                *max_id = (*max_id).max(id);
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_max_entity_id(&child, text, max_id);
    }
}

/// Calculate the next available field ID within a model
fn calculate_next_field_id(model_node: &Node, text: &str) -> u32 {
    let mut max_id = 0;

    // Look for model_body
    let mut cursor = model_node.walk();
    for child in model_node.children(&mut cursor) {
        if child.kind() == "model_body" {
            collect_max_field_id(&child, text, &mut max_id);
            break;
        }
    }

    max_id + 1
}

/// Collect the maximum field ID within a model body
fn collect_max_field_id(node: &Node, text: &str, max_id: &mut u32) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "field_definition" {
            // Look for entity_id within this field
            let mut field_cursor = child.walk();
            for field_child in child.children(&mut field_cursor) {
                if field_child.kind() == "entity_id" {
                    if let Ok(id_text) = field_child.utf8_text(text.as_bytes()) {
                        if let Ok(id) = id_text.trim_start_matches('#').parse::<u32>() {
                            *max_id = (*max_id).max(id);
                        }
                    }
                }
            }
        }
    }
}

/// Extract text at the given LSP range
fn extract_text_at_range(text: &str, range: &Range) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();

    if range.start.line as usize >= lines.len() {
        return None;
    }

    let line = lines[range.start.line as usize];
    let start = range.start.character as usize;
    let end = range.end.character as usize;

    if start >= line.len() || end > line.len() {
        return None;
    }

    Some(line[start..end].to_string())
}

/// Convert byte offset to LSP position
fn byte_offset_to_position(text: &str, offset: usize) -> Position {
    let mut line = 0;
    let mut character = 0;

    for (i, c) in text.char_indices() {
        if i >= offset {
            break;
        }
        if c == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }
    }

    Position { line, character }
}

#[cfg(test)]
#[path = "code_actions/code_actions_tests.rs"]
mod code_actions_tests;
