// Dependency Graph
// Builds a bidirectional dependency graph from CDM files for descendant lookups

use crate::file_resolver::LoadedFile;
use crate::grammar_parser::GrammarParser;
use crate::{Diagnostic, Severity, Span};
use cdm_utils::Position;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

/// A bidirectional dependency graph for CDM files
///
/// Tracks both:
/// - Which files a given file extends (dependencies/parents)
/// - Which files extend a given file (dependents/children)
#[derive(Debug, Default)]
pub struct DependencyGraph {
    /// Map of file path -> files it directly extends (parents)
    dependencies: HashMap<PathBuf, HashSet<PathBuf>>,

    /// Map of file path -> files that directly extend it (children)
    dependents: HashMap<PathBuf, HashSet<PathBuf>>,
}

impl DependencyGraph {
    /// Create an empty dependency graph
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a dependency graph from a list of CDM files
    ///
    /// Parses each file to extract extends directives and builds
    /// the bidirectional graph.
    ///
    /// # Arguments
    /// * `files` - List of absolute paths to CDM files
    ///
    /// # Returns
    /// * `Ok(DependencyGraph)` - The built graph
    /// * `Err(Vec<Diagnostic>)` - Warnings about files that couldn't be parsed
    pub fn build(files: &[PathBuf]) -> Result<Self, Vec<Diagnostic>> {
        let mut graph = Self::new();
        let mut diagnostics = Vec::new();

        // Create a set of known files for quick lookup
        let known_files: HashSet<PathBuf> = files.iter().cloned().collect();

        for file_path in files {
            // Create LoadedFile
            let loaded = LoadedFile::new_for_build(file_path.clone());

            // Parse to extract extends paths
            let parser = GrammarParser::new(&loaded);
            let extends_paths = parser.extract_extends_paths();

            // Resolve each extends path and add to graph
            for extends_path in extends_paths {
                let resolved = resolve_extends_path(file_path, &extends_path);

                // Canonicalize to get consistent paths
                let canonical = match resolved.canonicalize() {
                    Ok(p) => p,
                    Err(_) => {
                        // File doesn't exist - emit warning and skip
                        diagnostics.push(Diagnostic {
                            severity: Severity::Warning,
                            message: format!(
                                "Extended file not found: {} (referenced from {})",
                                extends_path,
                                file_path.display()
                            ),
                            span: Span {
                                start: Position { line: 0, column: 0 },
                                end: Position { line: 0, column: 0 },
                            },
                        });
                        continue;
                    }
                };

                // Only track dependencies to known files in our project
                if known_files.contains(&canonical) {
                    graph.add_dependency(file_path.clone(), canonical);
                }
            }
        }

        // Return graph even if there were warnings
        if diagnostics.iter().any(|d| d.severity == Severity::Error) {
            Err(diagnostics)
        } else {
            Ok(graph)
        }
    }

    /// Add a dependency relationship: child extends parent
    fn add_dependency(&mut self, child: PathBuf, parent: PathBuf) {
        // child -> parent (dependencies)
        self.dependencies
            .entry(child.clone())
            .or_default()
            .insert(parent.clone());

        // parent -> child (dependents)
        self.dependents.entry(parent).or_default().insert(child);
    }

    /// Get all files that directly or transitively extend the given file
    ///
    /// Uses breadth-first search to find all descendants.
    ///
    /// # Arguments
    /// * `file` - The file to find dependents of
    ///
    /// # Returns
    /// All files that extend this file, directly or through other files
    pub fn get_all_dependents(&self, file: &Path) -> Vec<PathBuf> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        // Start with direct dependents
        if let Some(direct) = self.dependents.get(file) {
            for dep in direct {
                queue.push_back(dep.clone());
            }
        }

        // BFS to find all transitive dependents
        while let Some(current) = queue.pop_front() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());
            result.push(current.clone());

            // Add this file's dependents to the queue
            if let Some(deps) = self.dependents.get(&current) {
                for dep in deps {
                    if !visited.contains(dep) {
                        queue.push_back(dep.clone());
                    }
                }
            }
        }

        // Sort for deterministic ordering
        result.sort();
        result
    }

    /// Get all files that the given file directly or transitively extends
    ///
    /// Uses breadth-first search to find all ancestors.
    ///
    /// # Arguments
    /// * `file` - The file to find dependencies of
    ///
    /// # Returns
    /// All files that this file extends, directly or through other files
    pub fn get_all_dependencies(&self, file: &Path) -> Vec<PathBuf> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        // Start with direct dependencies
        if let Some(direct) = self.dependencies.get(file) {
            for dep in direct {
                queue.push_back(dep.clone());
            }
        }

        // BFS to find all transitive dependencies
        while let Some(current) = queue.pop_front() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());
            result.push(current.clone());

            // Add this file's dependencies to the queue
            if let Some(deps) = self.dependencies.get(&current) {
                for dep in deps {
                    if !visited.contains(dep) {
                        queue.push_back(dep.clone());
                    }
                }
            }
        }

        // Sort for deterministic ordering
        result.sort();
        result
    }

    /// Check if file `child` depends on (extends) file `parent`
    pub fn depends_on(&self, child: &Path, parent: &Path) -> bool {
        self.get_all_dependencies(child).contains(&parent.to_path_buf())
    }

    /// Get the direct dependents (children) of a file
    pub fn direct_dependents(&self, file: &Path) -> Vec<PathBuf> {
        self.dependents
            .get(file)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get the direct dependencies (parents) of a file
    pub fn direct_dependencies(&self, file: &Path) -> Vec<PathBuf> {
        self.dependencies
            .get(file)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }
}

/// Resolve an extends path relative to a file
fn resolve_extends_path(current_file: &Path, extends_path: &str) -> PathBuf {
    let current_dir = current_file.parent().unwrap_or_else(|| Path::new("."));
    current_dir.join(extends_path)
}

#[cfg(test)]
#[path = "dependency_graph/dependency_graph_tests.rs"]
mod dependency_graph_tests;
