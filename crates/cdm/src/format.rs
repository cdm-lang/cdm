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
        // Skip whitespace-only nodes (handled by formatting)
        let kind = child.kind();
        if kind.starts_with('\n') || kind.starts_with('\r') {
            continue;
        }

        let formatted = match kind {
            "comment" => format_comment(child, source),
            "type_alias" => format_type_alias(child, source, &assignment_map),
            "model_definition" => format_model(child, source, &assignment_map, &indent),
            // Preserve all other node types (extends_directive, plugin_import,
            // model_removal, etc.) by outputting their original source text.
            // This ensures we don't lose any language elements during formatting.
            _ => get_node_text(child, source),
        };

        if !first_item {
            output.push('\n');
        }
        output.push_str(&formatted);
        output.push('\n');
        first_item = false;
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
            // array_type is: type_identifier "[" "]"
            // The first child is the type_identifier
            let mut cursor = node.walk();
            let element_type = node.children(&mut cursor)
                .find(|child| child.kind() == "type_identifier")
                .map(|n| get_node_text(n, source))
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

    // Check for extends clause
    let extends_str = if let Some(extends_clause) = node.child_by_field_name("extends") {
        let mut parent_names = Vec::new();
        let mut cursor = extends_clause.walk();
        for child in extends_clause.children(&mut cursor) {
            if child.kind() == "identifier" {
                parent_names.push(get_node_text(child, source));
            }
        }
        if !parent_names.is_empty() {
            format!(" extends {}", parent_names.join(", "))
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let mut output = format!("{}{} {{\n", model_name, extends_str);

    // Format model members
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for member in body.children(&mut cursor) {
            // Skip structural tokens and whitespace
            let kind = member.kind();
            if kind == "{" || kind == "}" || kind.starts_with('\n') || kind.starts_with('\r') {
                continue;
            }

            let member_str = match kind {
                "field_definition" => format_field(member, source, &model_name, assignment_map, indent),
                // Preserve all other member types (plugin_config, field_removal,
                // field_override, etc.) by outputting their original source text
                // with proper indentation.
                _ => format!("{}{}", indent, get_node_text(member, source)),
            };
            output.push_str(&member_str);
            output.push('\n');
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

    // Check for optional marker (?)
    let optional_marker = if node.child_by_field_name("optional").is_some() {
        "?"
    } else {
        ""
    };

    // Check for type - only add ": type" if there is a type
    let type_node = node.child_by_field_name("type");
    let type_part = if let Some(type_node) = type_node {
        let type_str = format_type(type_node, source);

        // Check for default value
        let default_part = if let Some(default_node) = node.child_by_field_name("default") {
            format!(" = {}", get_node_text(default_node, source))
        } else {
            String::new()
        };

        // Check for inline plugin block
        let plugins_part = if let Some(plugins_node) = node.child_by_field_name("plugins") {
            format!(" {}", get_node_text(plugins_node, source))
        } else {
            String::new()
        };

        format!(": {}{}{}", type_str, default_part, plugins_part)
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

    format!("{}{}{}{}{}", indent, field_name, optional_marker, type_part, id)
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

#[cfg(test)]
#[path = "format/format_tests.rs"]
mod format_tests;
