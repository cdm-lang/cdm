use std::cell::RefCell;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::diagnostics::{Diagnostic, Severity};
use crate::grammar_parser::GrammarParser;
use cdm_utils::{Position, Span};

/// A loaded CDM file with lazy source reading and caching
#[derive(Debug)]
pub struct LoadedFile {
    /// Absolute path to the file
    pub path: PathBuf,
    /// Cached source content (loaded on first access)
    cached_source: RefCell<Option<String>>,
}

impl LoadedFile {
    /// Create a new LoadedFile without reading the file yet
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            cached_source: RefCell::new(None),
        }
    }

    /// Create a new LoadedFile for testing purposes
    #[cfg(test)]
    pub(crate) fn new_for_test(path: PathBuf) -> Self {
        Self::new(path)
    }

    /// Get the source content, reading and caching it on first access
    pub fn source(&self) -> Result<String, std::io::Error> {
        // Check if already cached
        if let Some(cached) = self.cached_source.borrow().as_ref() {
            return Ok(cached.clone());
        }

        // Read from file
        let source = fs::read_to_string(&self.path)?;

        // Cache it
        *self.cached_source.borrow_mut() = Some(source.clone());

        Ok(source)
    }
}

/// The complete file tree with a main file and its ancestors
#[derive(Debug)]
pub struct LoadedFileTree {
    /// The main file that was requested
    pub main: LoadedFile,
    /// All ancestor files in dependency order (immediate ancestors first)
    pub ancestors: Vec<LoadedFile>,
}

/// Resolves CDM files and their @extends dependencies.
///
/// FileResolver is responsible for:
/// - Loading CDM files from the filesystem (lazily)
/// - Resolving relative @extends paths
/// - Building the complete dependency tree
/// - Detecting circular dependencies
///
/// It does NOT perform validation or parsing - that's the responsibility of the validate module.
/// Files are not read until you call `source()` on a LoadedFile.
pub struct FileResolver {
    /// Cache of already-loaded files to detect circular dependencies
    loaded_files: HashSet<PathBuf>,
}

impl FileResolver {
    /// Create a new FileResolver
    pub fn new() -> Self {
        Self {
            loaded_files: HashSet::new(),
        }
    }

    /// Load a CDM file and all its @extends dependencies.
    ///
    /// Files are not read immediately - they're loaded lazily when you call
    /// `source()` on a LoadedFile. This minimizes memory usage.
    ///
    /// # Arguments
    /// * `file_path` - Absolute or relative path to the CDM file to load
    ///
    /// # Returns
    /// * `Ok(LoadedFileTree)` - The main file and all its ancestors
    /// * `Err(Vec<Diagnostic>)` - File I/O errors or circular dependency errors
    ///
    /// # Example
    /// ```no_run
    /// use cdm::FileResolver;
    ///
    /// let tree = FileResolver::load("schema.cdm").unwrap();
    /// println!("Main file: {}", tree.main.path.display());
    /// println!("Ancestors: {}", tree.ancestors.len());
    ///
    /// // Read source when needed
    /// let source = tree.main.source().unwrap();
    /// ```
    pub fn load(file_path: impl AsRef<Path>) -> Result<LoadedFileTree, Vec<Diagnostic>> {
        let absolute_path = Self::to_absolute_path(file_path.as_ref())?;
        let mut resolver = Self::new();
        resolver.load_file_tree(&absolute_path)
    }

    /// Internal method: Load file tree
    fn load_file_tree(&mut self, file_path: &Path) -> Result<LoadedFileTree, Vec<Diagnostic>> {
        let main = self.load_single_file(file_path)?;

        // Extract @extends paths from the main file using GrammarParser
        let parser = GrammarParser::new(&main);
        let extends_paths = parser.extract_extends_paths();

        // Load all ancestors recursively
        let mut ancestors = Vec::new();
        for extends_path in extends_paths {
            let resolved_path = self.resolve_path(file_path, &extends_path);
            let ancestor_tree = self.load_file_tree(&resolved_path)?;

            // Add this ancestor's ancestors first (depth-first order)
            ancestors.extend(ancestor_tree.ancestors);

            // Then add this ancestor itself
            ancestors.push(ancestor_tree.main);
        }

        Ok(LoadedFileTree { main, ancestors })
    }

    /// Load a single file without processing its dependencies
    fn load_single_file(&mut self, file_path: &Path) -> Result<LoadedFile, Vec<Diagnostic>> {
        // Check for circular dependencies
        if self.loaded_files.contains(file_path) {
            return Err(vec![Diagnostic {
                message: format!(
                    "Circular @extends detected: {} is already in the extends chain",
                    file_path.display()
                ),
                severity: Severity::Error,
                span: Span {
                    start: Position { line: 0, column: 0 },
                    end: Position { line: 0, column: 0 },
                },
            }]);
        }

        // Check if file exists (but don't read it yet - lazy loading)
        if !file_path.exists() {
            return Err(vec![Diagnostic {
                message: format!("Failed to read file {}: No such file or directory", file_path.display()),
                severity: Severity::Error,
                span: Span {
                    start: Position { line: 0, column: 0 },
                    end: Position { line: 0, column: 0 },
                },
            }]);
        }

        // Mark as loaded
        self.loaded_files.insert(file_path.to_path_buf());

        // Create LoadedFile (file not read yet - lazy loading)
        Ok(LoadedFile::new(file_path.to_path_buf()))
    }

    /// Resolve a relative path from an @extends directive
    ///
    /// Handles paths like:
    /// - `./types.cdm` - same directory as current file
    /// - `../shared/base.cdm` - parent directory
    /// - `../../common/types.cdm` - up two levels
    fn resolve_path(&self, current_file: &Path, extends_path: &str) -> PathBuf {
        let current_dir = current_file.parent().unwrap_or_else(|| Path::new("."));
        current_dir.join(extends_path)
    }

    /// Convert a potentially relative path to an absolute path
    fn to_absolute_path(path: &Path) -> Result<PathBuf, Vec<Diagnostic>> {
        path.canonicalize().map_err(|err| {
            vec![Diagnostic {
                message: format!("Failed to resolve path {}: {}", path.display(), err),
                severity: Severity::Error,
                span: Span {
                    start: Position { line: 0, column: 0 },
                    end: Position { line: 0, column: 0 },
                },
            }]
        })
    }
}

impl Default for FileResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test_fixtures")
            .join("file_resolver")
    }

    #[test]
    fn test_load_single_file() {
        let file_path = fixtures_path().join("single_file/simple.cdm");
        let result = FileResolver::load(&file_path);

        assert!(result.is_ok(), "Failed to load: {:?}", result.err());

        let tree = result.unwrap();
        let source = tree.main.source().expect("Failed to read source");
        assert!(source.contains("User"));
        assert_eq!(tree.ancestors.len(), 0);
    }

    #[test]
    fn test_load_with_single_extends() {
        let file_path = fixtures_path().join("single_extends/child.cdm");
        let result = FileResolver::load(&file_path);

        assert!(result.is_ok(), "Failed to load: {:?}", result.err());

        let tree = result.unwrap();
        let source = tree.main.source().expect("Failed to read source");
        assert!(source.contains("PublicUser"));
        assert_eq!(tree.ancestors.len(), 1);

        let ancestor_source = tree.ancestors[0].source().expect("Failed to read ancestor source");
        assert!(ancestor_source.contains("User"));
    }

    #[test]
    fn test_load_with_multiple_extends() {
        let file_path = fixtures_path().join("multiple_extends/child.cdm");
        let result = FileResolver::load(&file_path);

        assert!(result.is_ok(), "Failed to load: {:?}", result.err());

        let tree = result.unwrap();
        let source = tree.main.source().expect("Failed to read source");
        assert!(source.contains("User"));
        assert_eq!(tree.ancestors.len(), 2); // types.cdm and mixins.cdm
    }

    #[test]
    fn test_load_nested_chain() {
        let file_path = fixtures_path().join("nested_chain/mobile.cdm");
        let result = FileResolver::load(&file_path);

        assert!(result.is_ok(), "Failed to load: {:?}", result.err());

        let tree = result.unwrap();
        let source = tree.main.source().expect("Failed to read source");
        assert!(source.contains("MobileUser"));
        // Should have client.cdm and base.cdm (client's ancestor)
        assert_eq!(tree.ancestors.len(), 2);
    }

    #[test]
    fn test_load_circular_detected() {
        let file_path = fixtures_path().join("circular/a.cdm");
        let result = FileResolver::load(&file_path);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("Circular @extends detected"));
    }

    #[test]
    fn test_load_file_not_found() {
        let file_path = fixtures_path().join("invalid/missing_extends.cdm");
        let result = FileResolver::load(&file_path);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("Failed to read file"));
    }
}
