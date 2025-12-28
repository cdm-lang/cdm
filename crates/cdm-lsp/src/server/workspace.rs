//! Workspace management with dependency tracking
//!
//! This module tracks file dependencies via @extends directives and manages
//! multi-file validation with caching for performance.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use tower_lsp::lsp_types::Url;
use tree_sitter::{Parser, Node};

/// Workspace-wide dependency tracker
#[derive(Clone)]
pub struct Workspace {
    state: Arc<RwLock<WorkspaceState>>,
}

struct WorkspaceState {
    /// Dependency graph: file -> files that extend it
    dependents: HashMap<Url, HashSet<Url>>,

    /// Reverse dependency graph: file -> files it extends
    dependencies: HashMap<Url, HashSet<Url>>,

    /// Cached parse trees for performance
    parse_cache: HashMap<Url, CachedParse>,

    /// Root directory for resolving relative paths
    root_uri: Option<Url>,
}

struct CachedParse {
    /// The document text
    #[allow(dead_code)] // Used in tests
    text: String,
}

impl Workspace {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(WorkspaceState {
                dependents: HashMap::new(),
                dependencies: HashMap::new(),
                parse_cache: HashMap::new(),
                root_uri: None,
            })),
        }
    }

    /// Set the workspace root directory
    pub fn set_root(&self, root_uri: Url) {
        let mut state = self.state.write().unwrap();
        state.root_uri = Some(root_uri);
    }

    /// Update a document and rebuild its dependencies
    pub fn update_document(&self, uri: Url, text: String) {
        let mut state = self.state.write().unwrap();

        // Parse the document and extract @extends directives
        let extends_paths = extract_extends_directives(&text);

        // Resolve @extends paths to URIs
        let dependencies = extends_paths
            .iter()
            .filter_map(|path| {
                resolve_path_to_uri(&uri, path, state.root_uri.as_ref())
            })
            .collect::<HashSet<_>>();

        // Remove old dependency relationships
        let old_deps = state.dependencies.get(&uri).cloned();
        if let Some(old_deps) = old_deps {
            for old_dep in old_deps {
                if let Some(dependents) = state.dependents.get_mut(&old_dep) {
                    dependents.remove(&uri);
                }
            }
        }

        // Add new dependency relationships
        for dep in &dependencies {
            state.dependents
                .entry(dep.clone())
                .or_insert_with(HashSet::new)
                .insert(uri.clone());
        }

        state.dependencies.insert(uri.clone(), dependencies);

        // Cache the parse tree (simplified - just store text for now)
        state.parse_cache.insert(uri.clone(), CachedParse {
            text: text.clone(),
        });
    }

    /// Remove a document from the workspace
    pub fn remove_document(&self, uri: &Url) {
        let mut state = self.state.write().unwrap();

        // Remove from caches
        state.parse_cache.remove(uri);

        // Remove dependency relationships
        if let Some(deps) = state.dependencies.remove(uri) {
            for dep in deps {
                if let Some(dependents) = state.dependents.get_mut(&dep) {
                    dependents.remove(uri);
                }
            }
        }

        // Remove as a dependent
        state.dependents.remove(uri);
    }

    /// Get all files that depend on the given file (directly or indirectly)
    pub fn get_all_dependents(&self, uri: &Url) -> Vec<Url> {
        let state = self.state.read().unwrap();
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = vec![uri.clone()];

        while let Some(current) = queue.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }

            if let Some(dependents) = state.dependents.get(&current) {
                for dependent in dependents {
                    if !visited.contains(dependent) {
                        result.push(dependent.clone());
                        queue.push(dependent.clone());
                    }
                }
            }
        }

        result
    }

    /// Get the dependency chain for a file (files it extends, in order)
    #[allow(dead_code)] // Used in tests
    pub fn get_dependency_chain(&self, uri: &Url) -> Vec<Url> {
        let state = self.state.read().unwrap();
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut current = uri.clone();

        loop {
            if !visited.insert(current.clone()) {
                // Circular dependency, stop
                break;
            }

            if let Some(deps) = state.dependencies.get(&current) {
                // For simplicity, take the first dependency
                // In a real implementation, we'd need to handle multiple @extends
                if let Some(dep) = deps.iter().next() {
                    result.push(dep.clone());
                    current = dep.clone();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        result
    }

    /// Get cached text for a document
    #[allow(dead_code)] // Used in tests
    pub fn get_cached_text(&self, uri: &Url) -> Option<String> {
        let state = self.state.read().unwrap();
        state.parse_cache.get(uri).map(|cached| cached.text.clone())
    }

}

/// Extract @extends directives from a CDM file
fn extract_extends_directives(text: &str) -> Vec<String> {
    let mut extends_paths = Vec::new();

    // Parse the document
    let mut parser = Parser::new();
    if parser.set_language(&grammar::LANGUAGE.into()).is_err() {
        return extends_paths;
    }

    let tree = match parser.parse(text, None) {
        Some(t) => t,
        None => return extends_paths,
    };

    // Walk the tree to find @extends directives
    let root = tree.root_node();
    walk_extends_directives(root, text, &mut extends_paths);

    extends_paths
}

/// Recursively walk the tree to find @extends directives
fn walk_extends_directives(node: Node, text: &str, extends_paths: &mut Vec<String>) {
    if node.kind() == "extends_directive" {
        // Extract the file path from the directive
        if let Some(path_node) = node.child_by_field_name("path") {
            if let Ok(path) = path_node.utf8_text(text.as_bytes()) {
                // Remove quotes if present
                let path = path.trim_matches('"').trim_matches('\'');
                extends_paths.push(path.to_string());
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_extends_directives(child, text, extends_paths);
    }
}

/// Resolve a relative path to a URI
fn resolve_path_to_uri(base_uri: &Url, path: &str, _root_uri: Option<&Url>) -> Option<Url> {
    // Convert base URI to path
    let base_path = base_uri.to_file_path().ok()?;
    let base_dir = base_path.parent()?;

    // Resolve the relative path
    let resolved_path = base_dir.join(path);

    // Convert back to URI
    Url::from_file_path(resolved_path).ok()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_update_and_dependencies() {
        let workspace = Workspace::new();

        let uri1 = Url::parse("file:///base.cdm").unwrap();
        let uri2 = Url::parse("file:///derived.cdm").unwrap();

        // Update base file (no dependencies)
        workspace.update_document(uri1.clone(), "User { name: string #1 } #10".to_string());

        // Update derived file (extends base.cdm)
        workspace.update_document(
            uri2.clone(),
            "@extends \"base.cdm\"\n\nAdminUser extends User { role: string #1 } #20".to_string(),
        );

        // Check dependencies
        let deps = workspace.get_dependency_chain(&uri2);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], uri1);

        // Check dependents
        let dependents = workspace.get_all_dependents(&uri1);
        assert_eq!(dependents.len(), 1);
        assert!(dependents.contains(&uri2));
    }

    #[test]
    fn test_workspace_remove_document() {
        let workspace = Workspace::new();

        let uri1 = Url::parse("file:///base.cdm").unwrap();
        let uri2 = Url::parse("file:///derived.cdm").unwrap();

        workspace.update_document(uri1.clone(), "User { } #10".to_string());
        workspace.update_document(uri2.clone(), "@extends \"base.cdm\"".to_string());

        // Remove the derived file
        workspace.remove_document(&uri2);

        // Should no longer have dependents
        let dependents = workspace.get_all_dependents(&uri1);
        assert_eq!(dependents.len(), 0);
    }

    #[test]
    fn test_extract_extends_directives() {
        let text = r#"
@extends "base.cdm"
@extends "mixins/timestamps.cdm"

User extends BaseUser {
  name: string #1
} #10
"#;

        let extends = extract_extends_directives(text);
        assert_eq!(extends.len(), 2);
        assert!(extends.contains(&"base.cdm".to_string()));
        assert!(extends.contains(&"mixins/timestamps.cdm".to_string()));
    }

    #[test]
    fn test_cached_text() {
        let workspace = Workspace::new();
        let uri = Url::parse("file:///test.cdm").unwrap();

        let text = "User { name: string #1 } #10";
        workspace.update_document(uri.clone(), text.to_string());

        let cached = workspace.get_cached_text(&uri);
        assert_eq!(cached, Some(text.to_string()));
    }
}
