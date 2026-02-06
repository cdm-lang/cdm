// Descendant ID Collector
// Collects entity IDs from all files that extend a given file

use crate::dependency_graph::DependencyGraph;
use crate::file_resolver::LoadedFile;
use crate::grammar_parser::GrammarParser;
use crate::{Diagnostic, Severity, Span};
use cdm_utils::Position;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tree_sitter::Node;

/// IDs collected from descendant files
#[derive(Debug, Default)]
pub struct DescendantIds {
    /// Global IDs (models and type aliases) used by descendants
    pub global_ids: HashSet<u64>,

    /// Per-model field IDs used by descendants
    /// Key: model_name, Value: Set of field IDs
    pub model_field_ids: HashMap<String, HashSet<u64>>,
}

impl DescendantIds {
    /// Create empty descendant IDs
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a global ID is used by any descendant
    pub fn has_global_id(&self, id: u64) -> bool {
        self.global_ids.contains(&id)
    }

    /// Check if a field ID is used by any descendant for a given model
    pub fn has_field_id(&self, model_name: &str, id: u64) -> bool {
        self.model_field_ids
            .get(model_name)
            .map(|ids| ids.contains(&id))
            .unwrap_or(false)
    }

    /// Get all global IDs used by descendants
    pub fn global_ids(&self) -> &HashSet<u64> {
        &self.global_ids
    }

    /// Get field IDs for a specific model used by descendants
    pub fn field_ids_for_model(&self, model_name: &str) -> Option<&HashSet<u64>> {
        self.model_field_ids.get(model_name)
    }
}

/// Collect entity IDs from all files that extend the given file
///
/// # Arguments
/// * `graph` - The dependency graph built from project files
/// * `file` - The file to find descendant IDs for
///
/// # Returns
/// * `Ok(DescendantIds)` - IDs from all descendant files
/// * `Err(Vec<Diagnostic>)` - Errors encountered while parsing descendants
pub fn collect_descendant_ids(
    graph: &DependencyGraph,
    file: &Path,
) -> Result<DescendantIds, Vec<Diagnostic>> {
    let mut result = DescendantIds::new();
    let mut diagnostics = Vec::new();

    // Get all files that extend this file (directly or transitively)
    let dependents = graph.get_all_dependents(file);

    for dependent_path in dependents {
        // Load and parse the dependent file
        let loaded = LoadedFile::new_for_build(dependent_path.clone());
        let parser = GrammarParser::new(&loaded);

        // Parse the file
        let tree = match parser.parse() {
            Ok(t) => t,
            Err(e) => {
                diagnostics.push(Diagnostic {
                    severity: Severity::Warning,
                    message: format!(
                        "Failed to parse descendant file {}: {}",
                        dependent_path.display(),
                        e
                    ),
                    span: Span {
                        start: Position { line: 0, column: 0 },
                        end: Position { line: 0, column: 0 },
                    },
                });
                continue;
            }
        };

        // Read source
        let source = match loaded.source() {
            Ok(s) => s,
            Err(e) => {
                diagnostics.push(Diagnostic {
                    severity: Severity::Warning,
                    message: format!(
                        "Failed to read descendant file {}: {}",
                        dependent_path.display(),
                        e
                    ),
                    span: Span {
                        start: Position { line: 0, column: 0 },
                        end: Position { line: 0, column: 0 },
                    },
                });
                continue;
            }
        };

        // Extract IDs from the parsed tree
        extract_ids_from_tree(tree.root_node(), &source, &mut result);
    }

    Ok(result)
}

/// Extract entity IDs from a parsed tree
fn extract_ids_from_tree(root: Node, source: &str, result: &mut DescendantIds) {
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        match child.kind() {
            "type_alias" => {
                if let Some(id) = extract_entity_id(child, source) {
                    result.global_ids.insert(id);
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
                    result.global_ids.insert(id);
                }

                // Collect field IDs
                if let Some(body) = child.child_by_field_name("body") {
                    extract_field_ids(body, source, &model_name, result);
                }
            }
            _ => {}
        }
    }
}

/// Extract field IDs from a model body
fn extract_field_ids(body: Node, source: &str, model_name: &str, result: &mut DescendantIds) {
    let mut cursor = body.walk();

    for member in body.children(&mut cursor) {
        if member.kind() == "field_definition" {
            if let Some(id) = extract_entity_id(member, source) {
                result
                    .model_field_ids
                    .entry(model_name.to_string())
                    .or_default()
                    .insert(id);
            }
        }
    }
}

/// Extract entity ID from a node
fn extract_entity_id(node: Node, source: &str) -> Option<u64> {
    node.child_by_field_name("id").and_then(|id_node| {
        let text = get_node_text(id_node, source);
        text.strip_prefix('#')
            .and_then(|num| num.parse::<u64>().ok())
    })
}

/// Get text content of a node
fn get_node_text(node: Node, source: &str) -> String {
    source[node.start_byte()..node.end_byte()].to_string()
}

#[cfg(test)]
#[path = "descendant_ids/descendant_ids_tests.rs"]
mod descendant_ids_tests;
