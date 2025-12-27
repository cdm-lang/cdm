// Format command implementation
// Handles auto-assignment of entity IDs and (future) code formatting

use crate::{Diagnostic, FileResolver, Severity, Span};
use cdm_utils::Position;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tree_sitter::Node;

// =============================================================================
// Public API
// =============================================================================

/// Options for the format command
#[derive(Debug, Clone)]
pub struct FormatOptions {
    /// Auto-assign entity IDs to entities without them
    pub assign_ids: bool,

    /// Check formatting without writing changes (dry-run)
    pub check: bool,

    /// Write changes to files (default: true, set to false for --check)
    pub write: bool,

    /// Number of spaces for indentation (default: 2)
    pub indent_size: usize,

    /// Format whitespace (indentation, spacing, etc.)
    pub format_whitespace: bool,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            assign_ids: false,
            check: false,
            write: true,
            indent_size: 2,
            format_whitespace: true,
        }
    }
}

/// Result of formatting a file
#[derive(Debug)]
pub struct FormatResult {
    /// Path to the file that was formatted
    pub path: PathBuf,

    /// Whether the file was modified
    pub modified: bool,

    /// Entity ID assignments made (if assign_ids was true)
    pub assignments: Vec<IdAssignment>,

    /// Diagnostics generated during formatting
    pub diagnostics: Vec<Diagnostic>,
}

/// Represents an entity ID assignment
#[derive(Debug, Clone)]
pub struct IdAssignment {
    /// Type of entity
    pub entity_type: EntityType,

    /// Name of the entity
    pub entity_name: String,

    /// Model name (for fields only)
    pub model_name: Option<String>,

    /// Assigned ID
    pub assigned_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityType {
    TypeAlias,
    Model,
    Field,
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityType::TypeAlias => write!(f, "Type alias"),
            EntityType::Model => write!(f, "Model"),
            EntityType::Field => write!(f, "Field"),
        }
    }
}

/// Format one or more CDM files
pub fn format_files(
    paths: &[PathBuf],
    options: &FormatOptions,
) -> Result<Vec<FormatResult>, Vec<Diagnostic>> {
    let mut results = Vec::new();
    let mut all_diagnostics = Vec::new();

    for path in paths {
        match format_file(path, options) {
            Ok(result) => {
                // Collect diagnostics even on success
                all_diagnostics.extend(result.diagnostics.clone());
                results.push(result);
            }
            Err(diagnostics) => {
                all_diagnostics.extend(diagnostics);
            }
        }
    }

    if all_diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return Err(all_diagnostics);
    }

    Ok(results)
}

/// Format a single CDM file
pub fn format_file(
    path: &Path,
    options: &FormatOptions,
) -> Result<FormatResult, Vec<Diagnostic>> {
    // Load the file and its ancestors (for context-aware ID validation)
    let tree = FileResolver::load(path).map_err(|diagnostics| diagnostics)?;

    // Parse the file
    let parser = crate::GrammarParser::new(&tree.main);
    let parse_tree = parser.parse().map_err(|e| {
        vec![Diagnostic {
            severity: Severity::Error,
            message: format!("Failed to parse {}: {}", path.display(), e),
            span: Span {
                start: Position { line: 0, column: 0 },
                end: Position { line: 0, column: 0 },
            },
        }]
    })?;
    let root = parse_tree.root_node();
    let source = tree.main.source().map_err(|e| {
        vec![Diagnostic {
            severity: Severity::Error,
            message: format!("Failed to read {}: {}", path.display(), e),
            span: Span {
                start: Position { line: 0, column: 0 },
                end: Position { line: 0, column: 0 },
            },
        }]
    })?;

    // Check for parse errors
    if root.has_error() {
        return Err(vec![Diagnostic {
            severity: Severity::Error,
            message: format!("File {} has parse errors, cannot format", path.display()),
            span: Span {
                start: Position { line: 0, column: 0 },
                end: Position { line: 0, column: 0 },
            },
        }]);
    }

    let mut diagnostics = Vec::new();
    let mut assignments = Vec::new();
    let mut modified = false;

    // If assign_ids is enabled, collect and assign IDs
    if options.assign_ids {
        // Collect all existing IDs from this file and ancestors
        let mut tracker = EntityIdTracker::new();

        // First, collect IDs from ancestors to avoid conflicts
        for ancestor in &tree.ancestors {
            let ancestor_parser = crate::GrammarParser::new(ancestor);
            if let Ok(ancestor_tree) = ancestor_parser.parse() {
                if let Ok(ancestor_source) = ancestor.source() {
                    collect_entity_ids(
                        ancestor_tree.root_node(),
                        &ancestor_source,
                        &mut tracker,
                        &mut diagnostics,
                    );
                }
            }
        }

        // Then collect IDs from the current file
        collect_entity_ids(root, &source, &mut tracker, &mut diagnostics);

        // Find entities without IDs and assign them
        assignments = assign_missing_ids(root, &source, &mut tracker);
        modified = !assignments.is_empty();
    }

    // Generate formatted source
    let new_source = if options.format_whitespace {
        // Format whitespace and optionally add IDs
        let formatted = format_source(root, &source, &assignments, options.indent_size);
        // Check if formatting changed anything
        if formatted != source {
            modified = true;
        }
        Some(formatted)
    } else if modified {
        // Only add IDs without reformatting whitespace
        Some(reconstruct_source(root, &source, &assignments, options.indent_size))
    } else {
        None
    };

    // If there are changes and write is enabled, write the file
    let new_source = if (modified || new_source.is_some()) && options.write {
        new_source
    } else {
        None
    };

    // Write the file if needed
    if let Some(new_source) = new_source {
        if !options.check {
            // Atomic write: write to temp file, then rename
            write_file_atomic(path, &new_source).map_err(|e| {
                vec![Diagnostic {
                    severity: Severity::Error,
                    message: format!("Failed to write file {}: {}", path.display(), e),
                    span: Span {
                        start: Position { line: 0, column: 0 },
                        end: Position { line: 0, column: 0 },
                    },
                }]
            })?;
        }
    }

    Ok(FormatResult {
        path: path.to_path_buf(),
        modified,
        assignments,
        diagnostics,
    })
}

// =============================================================================
// Entity ID Tracking
// =============================================================================

/// Tracks used entity IDs and computes next available IDs
struct EntityIdTracker {
    /// Global IDs used by type aliases and models
    global_ids: HashSet<u64>,

    /// Next available global ID
    next_global_id: u64,

    /// Per-model field IDs
    model_field_ids: HashMap<String, HashSet<u64>>,

    /// Next available field ID per model
    next_field_ids: HashMap<String, u64>,
}

impl EntityIdTracker {
    fn new() -> Self {
        Self {
            global_ids: HashSet::new(),
            next_global_id: 1,
            model_field_ids: HashMap::new(),
            next_field_ids: HashMap::new(),
        }
    }

    /// Register a type alias or model ID
    fn add_global_id(&mut self, id: u64) {
        self.global_ids.insert(id);
        if id >= self.next_global_id {
            self.next_global_id = id + 1;
        }
    }

    /// Register a field ID for a specific model
    fn add_field_id(&mut self, model_name: &str, id: u64) {
        self.model_field_ids
            .entry(model_name.to_string())
            .or_insert_with(HashSet::new)
            .insert(id);

        let next_id = self.next_field_ids
            .entry(model_name.to_string())
            .or_insert(1);

        if id >= *next_id {
            *next_id = id + 1;
        }
    }

    /// Get the next available global ID (for type aliases and models)
    fn next_global_id(&mut self) -> u64 {
        let id = self.next_global_id;
        self.next_global_id += 1;
        self.global_ids.insert(id);
        id
    }

    /// Get the next available field ID for a specific model
    fn next_field_id(&mut self, model_name: &str) -> u64 {
        let next_id = self.next_field_ids
            .entry(model_name.to_string())
            .or_insert(1);

        let id = *next_id;
        *next_id += 1;

        self.model_field_ids
            .entry(model_name.to_string())
            .or_insert_with(HashSet::new)
            .insert(id);

        id
    }
}

// =============================================================================
// ID Collection (Pass 1: Find existing IDs)
// =============================================================================

/// Collect all existing entity IDs from the AST
fn collect_entity_ids(
    root: Node,
    source: &str,
    tracker: &mut EntityIdTracker,
    _diagnostics: &mut Vec<Diagnostic>,
) {
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        match child.kind() {
            "type_alias" => {
                if let Some(id) = extract_entity_id(child, source) {
                    tracker.add_global_id(id);
                }
            }
            "model_definition" => {
                // Get model name
                let model_name = child
                    .child_by_field_name("name")
                    .map(|n| get_node_text(n, source))
                    .unwrap_or_default();

                // Collect model ID
                if let Some(id) = extract_entity_id(child, source) {
                    tracker.add_global_id(id);
                }

                // Collect field IDs
                if let Some(body) = child.child_by_field_name("body") {
                    collect_field_ids(body, source, &model_name, tracker);
                }
            }
            _ => {}
        }
    }
}

/// Collect field IDs from a model body
fn collect_field_ids(
    body: Node,
    source: &str,
    model_name: &str,
    tracker: &mut EntityIdTracker,
) {
    let mut cursor = body.walk();

    for member in body.children(&mut cursor) {
        if member.kind() == "field_definition" {
            if let Some(id) = extract_entity_id(member, source) {
                tracker.add_field_id(model_name, id);
            }
        }
    }
}

// =============================================================================
// ID Assignment (Pass 2: Assign missing IDs)
// =============================================================================

/// Find entities without IDs and assign them
fn assign_missing_ids(
    root: Node,
    source: &str,
    tracker: &mut EntityIdTracker,
) -> Vec<IdAssignment> {
    let mut assignments = Vec::new();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        match child.kind() {
            "type_alias" => {
                // Check if type alias has an ID
                if extract_entity_id(child, source).is_none() {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = get_node_text(name_node, source);
                        let id = tracker.next_global_id();

                        assignments.push(IdAssignment {
                            entity_type: EntityType::TypeAlias,
                            entity_name: name,
                            model_name: None,
                            assigned_id: id,
                        });
                    }
                }
            }
            "model_definition" => {
                // Get model name
                let model_name = child
                    .child_by_field_name("name")
                    .map(|n| get_node_text(n, source))
                    .unwrap_or_default();

                // Check if model has an ID
                if extract_entity_id(child, source).is_none() {
                    let id = tracker.next_global_id();

                    assignments.push(IdAssignment {
                        entity_type: EntityType::Model,
                        entity_name: model_name.clone(),
                        model_name: None,
                        assigned_id: id,
                    });
                }

                // Assign field IDs
                if let Some(body) = child.child_by_field_name("body") {
                    assign_field_ids(body, source, &model_name, tracker, &mut assignments);
                }
            }
            _ => {}
        }
    }

    assignments
}

/// Assign IDs to fields without them
fn assign_field_ids(
    body: Node,
    source: &str,
    model_name: &str,
    tracker: &mut EntityIdTracker,
    assignments: &mut Vec<IdAssignment>,
) {
    let mut cursor = body.walk();

    for member in body.children(&mut cursor) {
        if member.kind() == "field_definition" {
            // Check if field has an ID
            if extract_entity_id(member, source).is_none() {
                if let Some(name_node) = member.child_by_field_name("name") {
                    let field_name = get_node_text(name_node, source);
                    let id = tracker.next_field_id(model_name);

                    assignments.push(IdAssignment {
                        entity_type: EntityType::Field,
                        entity_name: field_name,
                        model_name: Some(model_name.to_string()),
                        assigned_id: id,
                    });
                }
            }
        }
    }
}

// =============================================================================
// Source Reconstruction (Pass 3: Rebuild source with IDs)
// =============================================================================

/// Reconstruct the source code with entity IDs inserted
fn reconstruct_source(
    root: Node,
    source: &str,
    assignments: &[IdAssignment],
    _indent_size: usize,
) -> String {
    // Build a map of (entity_type, entity_name, model_name) -> assigned_id
    let mut assignment_map: HashMap<(EntityType, String, Option<String>), u64> = HashMap::new();
    for assignment in assignments {
        assignment_map.insert(
            (
                assignment.entity_type,
                assignment.entity_name.clone(),
                assignment.model_name.clone(),
            ),
            assignment.assigned_id,
        );
    }

    // Collect all insertion points
    let mut insertions: Vec<(usize, String)> = Vec::new(); // (byte_position, text_to_insert)

    collect_insertions(root, source, &assignment_map, &mut insertions);

    // Sort insertions by position (descending) so we can apply them back-to-front
    // This way byte offsets remain valid
    insertions.sort_by(|a, b| b.0.cmp(&a.0));

    // Apply insertions
    let mut result = source.to_string();
    for (pos, text) in insertions {
        result.insert_str(pos, &text);
    }

    result
}

/// Collect insertion points for entity IDs
fn collect_insertions(
    root: Node,
    source: &str,
    assignment_map: &HashMap<(EntityType, String, Option<String>), u64>,
    insertions: &mut Vec<(usize, String)>,
) {
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        match child.kind() {
            "type_alias" => {
                collect_type_alias_insertion(child, source, assignment_map, insertions);
            }
            "model_definition" => {
                collect_model_insertions(child, source, assignment_map, insertions);
            }
            _ => {}
        }
    }
}

/// Collect insertion point for a type alias
fn collect_type_alias_insertion(
    node: Node,
    source: &str,
    assignment_map: &HashMap<(EntityType, String, Option<String>), u64>,
    insertions: &mut Vec<(usize, String)>,
) {
    // Get the name
    let name = node
        .child_by_field_name("name")
        .map(|n| get_node_text(n, source))
        .unwrap_or_default();

    // Check if we need to add an ID
    let key = (EntityType::TypeAlias, name.clone(), None);

    if let Some(&assigned_id) = assignment_map.get(&key) {
        // Find insertion point - at the end of the node, trimming trailing whitespace
        let end = node.end_byte();
        let text = &source[node.start_byte()..end];
        let trimmed_len = text.trim_end().len();
        let insert_pos = node.start_byte() + trimmed_len;

        insertions.push((insert_pos, format!(" #{}", assigned_id)));
    }
}

/// Collect insertion points for a model and its fields
fn collect_model_insertions(
    node: Node,
    source: &str,
    assignment_map: &HashMap<(EntityType, String, Option<String>), u64>,
    insertions: &mut Vec<(usize, String)>,
) {
    // Get the model name
    let model_name = node
        .child_by_field_name("name")
        .map(|n| get_node_text(n, source))
        .unwrap_or_default();

    // Check if model needs an ID
    let model_key = (EntityType::Model, model_name.clone(), None);
    if let Some(&assigned_id) = assignment_map.get(&model_key) {
        // Find the closing brace of the model body
        if let Some(body) = node.child_by_field_name("body") {
            let body_end = body.end_byte();
            // Insert ID after the closing brace
            insertions.push((body_end, format!(" #{}", assigned_id)));
        }
    }

    // Collect field IDs
    if let Some(body) = node.child_by_field_name("body") {
        collect_field_insertions(body, source, &model_name, assignment_map, insertions);
    }
}

/// Collect insertion points for fields in a model body
fn collect_field_insertions(
    body: Node,
    source: &str,
    model_name: &str,
    assignment_map: &HashMap<(EntityType, String, Option<String>), u64>,
    insertions: &mut Vec<(usize, String)>,
) {
    let mut cursor = body.walk();

    for member in body.children(&mut cursor) {
        if member.kind() == "field_definition" {
            // Get field name
            let field_name = member
                .child_by_field_name("name")
                .map(|n| get_node_text(n, source))
                .unwrap_or_default();

            // Check if we need to add an ID
            let key = (EntityType::Field, field_name.clone(), Some(model_name.to_string()));

            if let Some(&assigned_id) = assignment_map.get(&key) {
                // Find insertion point - at the end of the field, trimming trailing whitespace
                let end = member.end_byte();
                let text = &source[member.start_byte()..end];
                let trimmed_len = text.trim_end().len();
                let insert_pos = member.start_byte() + trimmed_len;

                insertions.push((insert_pos, format!(" #{}", assigned_id)));
            }
        }
    }
}

// =============================================================================
// Whitespace Formatting
// =============================================================================

/// Format source code with proper whitespace and optionally add IDs
fn format_source(
    root: Node,
    source: &str,
    assignments: &[IdAssignment],
    indent_size: usize,
) -> String {
    // Build ID assignment map for quick lookup
    let mut assignment_map = HashMap::new();
    for assignment in assignments {
        assignment_map.insert(
            (
                assignment.entity_type,
                assignment.entity_name.clone(),
                assignment.model_name.clone(),
            ),
            assignment.assigned_id,
        );
    }

    let mut output = String::new();
    let indent = " ".repeat(indent_size);
    let mut cursor = root.walk();
    let mut first_item = true;

    for child in root.children(&mut cursor) {
        match child.kind() {
            "comment" => {
                if !first_item {
                    output.push('\n');
                }
                output.push_str(&format_comment(child, source));
                output.push('\n');
                first_item = false;
            }
            "type_alias" => {
                if !first_item {
                    output.push('\n');
                }
                output.push_str(&format_type_alias(child, source, &assignment_map));
                output.push('\n');
                first_item = false;
            }
            "model_definition" => {
                if !first_item {
                    output.push('\n');
                }
                output.push_str(&format_model(child, source, &assignment_map, &indent));
                output.push('\n');
                first_item = false;
            }
            _ => {
                // Skip whitespace and other nodes
            }
        }
    }

    output
}

/// Format a comment
fn format_comment(node: Node, source: &str) -> String {
    get_node_text(node, source)
}

/// Format a type alias
fn format_type_alias(
    node: Node,
    source: &str,
    assignment_map: &HashMap<(EntityType, String, Option<String>), u64>,
) -> String {
    let name = node
        .child_by_field_name("name")
        .map(|n| get_node_text(n, source))
        .unwrap_or_default();

    let type_node = node.child_by_field_name("type");
    let type_str = if let Some(type_node) = type_node {
        format_type(type_node, source)
    } else {
        String::new()
    };

    // Check for ID (existing or assigned)
    let id = if let Some(id_node) = node.child_by_field_name("id") {
        format!(" {}", get_node_text(id_node, source))
    } else if let Some(&assigned_id) = assignment_map.get(&(EntityType::TypeAlias, name.clone(), None)) {
        format!(" #{}", assigned_id)
    } else {
        String::new()
    };

    format!("{}: {}{}", name, type_str, id)
}

/// Format a type expression
fn format_type(node: Node, source: &str) -> String {
    match node.kind() {
        "union_type" => {
            let mut parts = Vec::new();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() != "|" {
                    parts.push(format_type(child, source));
                }
            }
            parts.join(" | ")
        }
        "array_type" => {
            let element_type = node
                .child_by_field_name("element")
                .map(|n| format_type(n, source))
                .unwrap_or_default();
            format!("{}[]", element_type)
        }
        "optional_type" => {
            let base_type = node
                .child_by_field_name("type")
                .map(|n| format_type(n, source))
                .unwrap_or_default();
            format!("{}?", base_type)
        }
        _ => get_node_text(node, source),
    }
}

/// Format a model definition
fn format_model(
    node: Node,
    source: &str,
    assignment_map: &HashMap<(EntityType, String, Option<String>), u64>,
    indent: &str,
) -> String {
    let model_name = node
        .child_by_field_name("name")
        .map(|n| get_node_text(n, source))
        .unwrap_or_default();

    let mut output = format!("{} {{\n", model_name);

    // Format fields
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for member in body.children(&mut cursor) {
            if member.kind() == "field_definition" {
                let field_str = format_field(member, source, &model_name, assignment_map, indent);
                output.push_str(&field_str);
                output.push('\n');
            }
        }
    }

    // Close model and add ID
    output.push('}');

    // Check for ID (existing or assigned)
    if let Some(id_node) = node.child_by_field_name("id") {
        output.push_str(&format!(" {}", get_node_text(id_node, source)));
    } else if let Some(&assigned_id) = assignment_map.get(&(EntityType::Model, model_name.clone(), None)) {
        output.push_str(&format!(" #{}", assigned_id));
    }

    output
}

/// Format a field definition
fn format_field(
    node: Node,
    source: &str,
    model_name: &str,
    assignment_map: &HashMap<(EntityType, String, Option<String>), u64>,
    indent: &str,
) -> String {
    let field_name = node
        .child_by_field_name("name")
        .map(|n| get_node_text(n, source))
        .unwrap_or_default();

    let type_node = node.child_by_field_name("type");
    let type_str = if let Some(type_node) = type_node {
        format_type(type_node, source)
    } else {
        String::new()
    };

    // Check for ID (existing or assigned)
    let id = if let Some(id_node) = node.child_by_field_name("id") {
        format!(" {}", get_node_text(id_node, source))
    } else if let Some(&assigned_id) = assignment_map.get(&(EntityType::Field, field_name.clone(), Some(model_name.to_string()))) {
        format!(" #{}", assigned_id)
    } else {
        String::new()
    };

    format!("{}{}: {}{}", indent, field_name, type_str, id)
}

// =============================================================================
// Utility Functions
// =============================================================================

/// Extract entity ID from a node (reuse from validate.rs logic)
fn extract_entity_id(node: Node, source: &str) -> Option<u64> {
    node.child_by_field_name("id")
        .and_then(|id_node| {
            let text = get_node_text(id_node, source);
            text.strip_prefix('#')
                .and_then(|num| num.parse::<u64>().ok())
        })
}

/// Get text content of a node
fn get_node_text(node: Node, source: &str) -> String {
    source[node.start_byte()..node.end_byte()].to_string()
}

/// Atomic file write: write to temp, then rename
fn write_file_atomic(path: &Path, content: &str) -> std::io::Result<()> {
    use std::fs;
    use std::io::Write;

    let temp_path = path.with_extension("cdm.tmp");

    // Write to temp file
    let mut file = fs::File::create(&temp_path)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()?;
    drop(file);

    // Rename temp to final
    fs::rename(&temp_path, path)?;

    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_entity_id_tracker_global_ids() {
        let mut tracker = EntityIdTracker::new();

        // Add some IDs
        tracker.add_global_id(1);
        tracker.add_global_id(5);
        tracker.add_global_id(3);

        // Next ID should be 6
        assert_eq!(tracker.next_global_id(), 6);
        assert_eq!(tracker.next_global_id(), 7);
    }

    #[test]
    fn test_entity_id_tracker_field_ids() {
        let mut tracker = EntityIdTracker::new();

        // Add field IDs for different models
        tracker.add_field_id("User", 1);
        tracker.add_field_id("User", 3);
        tracker.add_field_id("Post", 1);
        tracker.add_field_id("Post", 2);

        // Next IDs should be scoped per model
        assert_eq!(tracker.next_field_id("User"), 4);
        assert_eq!(tracker.next_field_id("Post"), 3);
        assert_eq!(tracker.next_field_id("Comment"), 1);
    }

    #[test]
    fn test_format_without_ids() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("test_fixtures/format/without_ids.cdm");

        let options = FormatOptions {
            assign_ids: true,
            check: true, // Don't write
            write: false,
            indent_size: 2,
            format_whitespace: false,
        };

        let result = format_file(&path, &options).expect("Format should succeed");

        // Should have modified the file
        assert!(result.modified);

        // Should have assigned 11 IDs (2 type aliases + 2 models + 7 fields)
        assert_eq!(result.assignments.len(), 11);

        // Check type alias IDs
        let email = result.assignments.iter().find(|a| {
            a.entity_type == EntityType::TypeAlias && a.entity_name == "Email"
        });
        assert!(email.is_some());
        assert_eq!(email.unwrap().assigned_id, 1);

        let status = result.assignments.iter().find(|a| {
            a.entity_type == EntityType::TypeAlias && a.entity_name == "Status"
        });
        assert!(status.is_some());
        assert_eq!(status.unwrap().assigned_id, 2);

        // Check model IDs
        let user = result.assignments.iter().find(|a| {
            a.entity_type == EntityType::Model && a.entity_name == "User"
        });
        assert!(user.is_some());
        assert_eq!(user.unwrap().assigned_id, 3);

        let post = result.assignments.iter().find(|a| {
            a.entity_type == EntityType::Model && a.entity_name == "Post"
        });
        assert!(post.is_some());
        assert_eq!(post.unwrap().assigned_id, 4);

        // Check field IDs are scoped per model
        let user_fields: Vec<_> = result.assignments.iter()
            .filter(|a| a.entity_type == EntityType::Field && a.model_name.as_deref() == Some("User"))
            .collect();
        assert_eq!(user_fields.len(), 4);

        let post_fields: Vec<_> = result.assignments.iter()
            .filter(|a| a.entity_type == EntityType::Field && a.model_name.as_deref() == Some("Post"))
            .collect();
        assert_eq!(post_fields.len(), 3);
    }

    #[test]
    fn test_format_partial_ids() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("test_fixtures/format/partial_ids.cdm");
        let options = FormatOptions {
            assign_ids: true,
            check: true,
            write: false,
            indent_size: 2,
            format_whitespace: false,
        };

        let result = format_file(&path, &options).expect("Format should succeed");

        // Should have modified the file
        assert!(result.modified);

        // Should assign missing IDs only
        // Missing: Status type alias, User.email, User.status, Post model + 3 fields
        assert_eq!(result.assignments.len(), 7);

        // Status should get ID 11 (next after User #10)
        let status = result.assignments.iter().find(|a| {
            a.entity_type == EntityType::TypeAlias && a.entity_name == "Status"
        });
        assert!(status.is_some());
        assert_eq!(status.unwrap().assigned_id, 11);

        // Post should get ID 12 (next after Status #11)
        let post = result.assignments.iter().find(|a| {
            a.entity_type == EntityType::Model && a.entity_name == "Post"
        });
        assert!(post.is_some());
        assert_eq!(post.unwrap().assigned_id, 12);

        // User.email should get ID 4 (User.id has #1, User.name has #3, next is 4)
        let user_email = result.assignments.iter().find(|a| {
            a.entity_type == EntityType::Field
                && a.entity_name == "email"
                && a.model_name.as_deref() == Some("User")
        });
        assert!(user_email.is_some());
        assert_eq!(user_email.unwrap().assigned_id, 4);

        // User.status should get ID 5 (next after User.email #4)
        let user_status = result.assignments.iter().find(|a| {
            a.entity_type == EntityType::Field
                && a.entity_name == "status"
                && a.model_name.as_deref() == Some("User")
        });
        assert!(user_status.is_some());
        assert_eq!(user_status.unwrap().assigned_id, 5);
    }

    #[test]
    fn test_format_all_ids() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("test_fixtures/format/all_ids.cdm");
        let options = FormatOptions {
            assign_ids: true,
            check: true,
            write: false,
            indent_size: 2,
            format_whitespace: false,
        };

        let result = format_file(&path, &options).expect("Format should succeed");

        // Should not have modified the file (all IDs already present)
        assert!(!result.modified);
        assert_eq!(result.assignments.len(), 0);
    }

    #[test]
    fn test_format_without_assign_ids() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("test_fixtures/format/without_ids.cdm");
        let options = FormatOptions {
            assign_ids: false, // Don't assign IDs
            check: true,
            write: false,
            indent_size: 2,
            format_whitespace: false,
        };

        let result = format_file(&path, &options).expect("Format should succeed");

        // Should not have modified the file
        assert!(!result.modified);
        assert_eq!(result.assignments.len(), 0);
    }

    #[test]
    fn test_field_id_scoping() {
        // Test that field IDs are scoped per model
        let mut tracker = EntityIdTracker::new();

        // Simulate User model with field IDs 1, 2, 3
        tracker.add_field_id("User", 1);
        tracker.add_field_id("User", 2);
        tracker.add_field_id("User", 3);

        // Simulate Post model with field IDs 1, 2
        tracker.add_field_id("Post", 1);
        tracker.add_field_id("Post", 2);

        // Next field ID for User should be 4
        assert_eq!(tracker.next_field_id("User"), 4);

        // Next field ID for Post should be 3
        assert_eq!(tracker.next_field_id("Post"), 3);

        // Next field ID for new model should be 1
        assert_eq!(tracker.next_field_id("Comment"), 1);
    }

    #[test]
    fn test_global_id_collision_avoidance() {
        let mut tracker = EntityIdTracker::new();

        // Add non-sequential IDs
        tracker.add_global_id(1);
        tracker.add_global_id(5);
        tracker.add_global_id(10);

        // Next ID should be 11 (after the highest)
        assert_eq!(tracker.next_global_id(), 11);
        assert_eq!(tracker.next_global_id(), 12);
    }

    #[test]
    fn test_format_files_multiple() {
        let mut path1 = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path1.push("test_fixtures/format/without_ids.cdm");
        let mut path2 = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path2.push("test_fixtures/format/partial_ids.cdm");

        let options = FormatOptions {
            assign_ids: true,
            check: true,
            write: false,
            indent_size: 2,
            format_whitespace: false,
        };

        let results = format_files(&[path1, path2], &options).expect("Format should succeed");

        // Should have formatted 2 files
        assert_eq!(results.len(), 2);

        // Both should be modified
        assert!(results[0].modified);
        assert!(results[1].modified);

        // First file should have 11 assignments, second should have 7
        assert_eq!(results[0].assignments.len(), 11);
        assert_eq!(results[1].assignments.len(), 7);
    }

    #[test]
    fn test_format_invalid_path() {
        let path = PathBuf::from("nonexistent/file.cdm");
        let options = FormatOptions {
            assign_ids: true,
            check: true,
            write: false,
            indent_size: 2,
            format_whitespace: false,
        };

        let result = format_file(&path, &options);
        assert!(result.is_err());

        let diagnostics = result.unwrap_err();
        assert!(!diagnostics.is_empty());
        assert!(diagnostics[0].message.contains("Failed to resolve path"));
    }

    #[test]
    fn test_format_with_write() {
        use tempfile::NamedTempFile;
        use std::io::Write;

        // Create a temporary file with content
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        write!(temp_file, "Email: string\n\nUser {{\n  email: Email\n}}\n").expect("Failed to write");
        let temp_path = temp_file.path().to_path_buf();

        let options = FormatOptions {
            assign_ids: true,
            check: false, // Actually write
            write: true,
            indent_size: 2,
            format_whitespace: false,
        };

        let result = format_file(&temp_path, &options).expect("Format should succeed");

        // Should have modified the file
        assert!(result.modified);
        assert_eq!(result.assignments.len(), 3); // Email, User, User.email

        // Read the file back and verify IDs were written
        let content = std::fs::read_to_string(&temp_path).expect("Failed to read temp file");
        assert!(content.contains("#1"));
        assert!(content.contains("#2"));
    }

    #[test]
    fn test_reconstruct_source_preserves_structure() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("test_fixtures/format/without_ids.cdm");

        let options = FormatOptions {
            assign_ids: true,
            check: true,
            write: false,
            indent_size: 2,
            format_whitespace: false,
        };

        let result = format_file(&path, &options).expect("Format should succeed");

        // Verify the source structure is preserved
        let tree = FileResolver::load(&path).expect("Failed to load");
        let parser = crate::GrammarParser::new(&tree.main);
        let parse_tree = parser.parse().expect("Failed to parse");
        let root = parse_tree.root_node();
        let source = tree.main.source().expect("Failed to read source");

        // Reconstruct with assignments
        let new_source = reconstruct_source(root, &source, &result.assignments, 2);

        // New source should have all the IDs
        assert!(new_source.contains("#1"));
        assert!(new_source.contains("#2"));
        assert!(new_source.contains("#3"));
        assert!(new_source.contains("#4"));

        // Should still be valid CDM
        assert!(new_source.contains("Email: string"));
        assert!(new_source.contains("Status: \"active\""));
        assert!(new_source.contains("User {"));
        assert!(new_source.contains("Post {"));

        // Should parse without errors
        let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp");
        std::fs::write(temp_file.path(), &new_source).expect("Failed to write");
        let new_tree = FileResolver::load(temp_file.path()).expect("Failed to load formatted file");
        let new_parser = crate::GrammarParser::new(&new_tree.main);
        let new_parse = new_parser.parse().expect("Formatted file should parse");
        assert!(!new_parse.root_node().has_error());
    }

    #[test]
    fn test_extract_entity_id() {
        let source = "Email: string #42";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&grammar::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        // Find the type_alias node
        let type_alias = root.child(0).unwrap();
        assert_eq!(type_alias.kind(), "type_alias");

        let id = extract_entity_id(type_alias, source);
        assert_eq!(id, Some(42));
    }

    #[test]
    fn test_extract_entity_id_none() {
        let source = "Email: string";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&grammar::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        let type_alias = root.child(0).unwrap();
        let id = extract_entity_id(type_alias, source);
        assert_eq!(id, None);
    }

    #[test]
    fn test_get_node_text() {
        let source = "Email: string";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&grammar::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        let text = get_node_text(root, source);
        assert_eq!(text, "Email: string");
    }

    #[test]
    fn test_field_insertion_with_trailing_whitespace() {
        let source = "User {\n  id: string   \n}";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&grammar::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        let mut tracker = EntityIdTracker::new();
        let assignments = assign_missing_ids(root, source, &mut tracker);

        // Should assign IDs to User model and User.id field
        assert_eq!(assignments.len(), 2);

        let new_source = reconstruct_source(root, source, &assignments, 2);

        // ID should be inserted before trailing whitespace for field
        assert!(new_source.contains("id: string #1"));
        // Model ID should be after the closing brace
        assert!(new_source.contains("} #1"));
    }

    #[test]
    fn test_collect_entity_ids_with_all_types() {
        let source = r#"
Email: string #10

User {
  id: string #1
  email: Email #2
} #20

Post {
  title: string #1
} #21
"#;
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&grammar::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        let mut tracker = EntityIdTracker::new();
        let mut diagnostics = Vec::new();
        collect_entity_ids(root, source, &mut tracker, &mut diagnostics);

        // Should have collected global IDs 10, 20, 21
        assert!(tracker.global_ids.contains(&10));
        assert!(tracker.global_ids.contains(&20));
        assert!(tracker.global_ids.contains(&21));

        // Should have collected User field IDs 1, 2
        assert!(tracker.model_field_ids.get("User").unwrap().contains(&1));
        assert!(tracker.model_field_ids.get("User").unwrap().contains(&2));

        // Should have collected Post field ID 1
        assert!(tracker.model_field_ids.get("Post").unwrap().contains(&1));

        // Next global ID should be 22
        assert_eq!(tracker.next_global_id, 22);

        // Next field IDs should be 3 for User, 2 for Post
        assert_eq!(tracker.next_field_ids.get("User"), Some(&3));
        assert_eq!(tracker.next_field_ids.get("Post"), Some(&2));
    }

    #[test]
    fn test_format_files_with_one_error() {
        let mut valid_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        valid_path.push("test_fixtures/format/without_ids.cdm");
        let invalid_path = PathBuf::from("nonexistent/file.cdm");

        let options = FormatOptions {
            assign_ids: true,
            check: true,
            write: false,
            indent_size: 2,
            format_whitespace: false,
        };

        // format_files should fail if any file fails
        let result = format_files(&[valid_path, invalid_path], &options);
        assert!(result.is_err());
    }

    #[test]
    fn test_whitespace_formatting() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create a file with inconsistent whitespace
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        write!(temp_file, "Email:string\n\nStatus:\"active\"|\"pending\"\n\nUser{{\nid:string\nemail:Email\n}}\n").expect("Failed to write");
        let temp_path = temp_file.path().to_path_buf();

        let options = FormatOptions {
            assign_ids: true,
            check: false,
            write: true,
            indent_size: 2,
            format_whitespace: true,
        };

        let result = format_file(&temp_path, &options).expect("Format should succeed");
        assert!(result.modified);

        // Read back the formatted content
        let content = std::fs::read_to_string(&temp_path).expect("Failed to read formatted file");

        // Should have proper spacing around colons
        assert!(content.contains("Email: string"));
        assert!(content.contains("Status: \"active\" | \"pending\""));

        // Should have proper indentation
        assert!(content.contains("  id: string"));
        assert!(content.contains("  email: Email"));

        // Should have proper spacing around braces
        assert!(content.contains("User {\n"));
        assert!(content.contains("} #"));
    }

    #[test]
    fn test_whitespace_formatting_preserves_ids() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create a file with existing IDs but bad whitespace
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        write!(temp_file, "Email:string#42\n\nUser{{\nid:string#1\n}}#10\n").expect("Failed to write");
        let temp_path = temp_file.path().to_path_buf();

        let options = FormatOptions {
            assign_ids: false,
            check: false,
            write: true,
            indent_size: 2,
            format_whitespace: true,
        };

        let result = format_file(&temp_path, &options).expect("Format should succeed");
        assert!(result.modified);

        // Read back the formatted content
        let content = std::fs::read_to_string(&temp_path).expect("Failed to read formatted file");

        // Should preserve existing IDs
        assert!(content.contains("#42"));
        assert!(content.contains("#10"));
        assert!(content.contains("#1"));

        // Should have proper formatting
        assert!(content.contains("Email: string #42"));
        assert!(content.contains("  id: string #1"));
        assert!(content.contains("} #10"));
    }
}
