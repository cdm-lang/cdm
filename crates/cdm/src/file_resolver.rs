use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::diagnostics::{Diagnostic, Position, Severity, Span};
use crate::validate::{extract_extends_paths, validate, ValidationResult};

/// Resolves CDM files and their @extends dependencies, building a complete
/// ValidationResult with all ancestors loaded and merged.
pub struct FileResolver {
    /// Cache of already-loaded files to avoid redundant parsing
    /// and to detect circular dependencies
    loaded_files: HashSet<PathBuf>,
}

impl FileResolver {
    /// Create a new FileResolver
    pub fn new() -> Self {
        Self {
            loaded_files: HashSet::new(),
        }
    }

    /// Resolve a CDM file and all its @extends dependencies.
    ///
    /// This is the main entry point. It loads the specified file and recursively
    /// loads all files referenced by @extends directives, building a complete
    /// ValidationResult with the ancestor chain.
    ///
    /// # Arguments
    /// * `file_path` - Absolute or relative path to the CDM file to load
    ///
    /// # Returns
    /// * `Ok(ValidationResult)` - Successfully loaded and validated schema with ancestors
    /// * `Err(Vec<Diagnostic>)` - Validation errors or file I/O errors
    pub fn resolve_with_ancestors(
        file_path: impl AsRef<Path>,
    ) -> Result<ValidationResult, Vec<Diagnostic>> {
        let absolute_path = Self::to_absolute_path(file_path.as_ref())?;

        let mut resolver = Self::new();
        resolver.load_file_recursive(&absolute_path)
    }

    /// Recursively load a file and all its @extends dependencies
    fn load_file_recursive(
        &mut self,
        file_path: &Path,
    ) -> Result<ValidationResult, Vec<Diagnostic>> {
        // Check for circular dependencies
        if self.loaded_files.contains(file_path) {
            return Err(vec![Diagnostic {
                message: format!("Circular @extends detected: {} is already in the extends chain", file_path.display()),
                severity: Severity::Error,
                span: Span {
                    start: Position { line: 0, column: 0 },
                    end: Position { line: 0, column: 0 },
                },
            }]);
        }

        // Mark this file as being loaded (for circular detection)
        self.loaded_files.insert(file_path.to_path_buf());

        // Read the file contents
        let source = fs::read_to_string(file_path).map_err(|err| {
            vec![Diagnostic {
                message: format!("Failed to read file {}: {}", file_path.display(), err),
                severity: Severity::Error,
                span: Span {
                    start: Position { line: 0, column: 0 },
                    end: Position { line: 0, column: 0 },
                },
            }]
        })?;

        // Extract @extends paths from the source
        let extends_paths = extract_extends_paths(&source);

        // Recursively load all ancestor files
        let mut ancestors = Vec::new();
        for extends_path in extends_paths {
            // Resolve the extends path relative to the current file
            let resolved_path = self.resolve_path(file_path, &extends_path);

            // Recursively load the ancestor file
            let ancestor_result = self.load_file_recursive(&resolved_path)?;

            // Convert the ancestor's result into an Ancestor struct
            let ancestor = ancestor_result.into_ancestor(resolved_path.display().to_string());

            // Also inherit the ancestor's ancestors (flatten the chain)
            // Note: validate() function will handle the full ancestor chain
            ancestors.push(ancestor);
        }

        // Validate the current file WITH all ancestors loaded
        let result = validate(&source, &ancestors);

        // Check if validation had errors
        if result.has_errors() {
            return Err(result.diagnostics);
        }

        Ok(result)
    }

    /// Resolve a relative path from an @extends directive
    ///
    /// Handles paths like:
    /// - `./types.cdm` - same directory as current file
    /// - `../shared/base.cdm` - parent directory
    /// - `../../common/types.cdm` - up two levels
    fn resolve_path(&self, current_file: &Path, extends_path: &str) -> PathBuf {
        let current_dir = current_file
            .parent()
            .unwrap_or_else(|| Path::new("."));

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

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test_fixtures")
            .join("file_resolver")
    }

    #[test]
    fn test_resolve_single_file_no_extends() {
        let file_path = fixtures_path().join("single_file/simple.cdm");
        let result = FileResolver::resolve_with_ancestors(&file_path);

        assert!(result.is_ok(), "Failed to resolve: {:?}", result.err());

        let validation = result.unwrap();
        assert!(!validation.has_errors());

        // Check that we have a User model
        assert!(validation.symbol_table.definitions.contains_key("User"));
    }

    #[test]
    fn test_resolve_with_single_extends() {
        let file_path = fixtures_path().join("single_extends/child.cdm");
        let result = FileResolver::resolve_with_ancestors(&file_path);

        assert!(result.is_ok(), "Failed to resolve: {:?}", result.err());

        let validation = result.unwrap();
        assert!(!validation.has_errors());

        // Should have PublicUser in child file
        assert!(validation.symbol_table.definitions.contains_key("PublicUser"));

        // PublicUser should have its own fields (inheritance is resolved via ancestors)
        let public_user_fields = &validation.model_fields["PublicUser"];
        assert!(public_user_fields.iter().any(|f| f.name == "avatar_url")); // New field added in child
    }

    #[test]
    fn test_resolve_with_multiple_extends() {
        let file_path = fixtures_path().join("multiple_extends/child.cdm");
        let result = FileResolver::resolve_with_ancestors(&file_path);

        assert!(result.is_ok(), "Failed to resolve: {:?}", result.err());

        let validation = result.unwrap();
        assert!(!validation.has_errors());

        // User should have its own fields (extends Timestamped via ancestors)
        let user_fields = &validation.model_fields["User"];
        assert!(user_fields.iter().any(|f| f.name == "id"));
        assert!(user_fields.iter().any(|f| f.name == "email"));
    }

    #[test]
    fn test_resolve_nested_extends_chain() {
        let file_path = fixtures_path().join("nested_chain/mobile.cdm");
        let result = FileResolver::resolve_with_ancestors(&file_path);

        assert!(result.is_ok(), "Failed to resolve: {:?}", result.err());

        let validation = result.unwrap();
        assert!(!validation.has_errors());

        // MobileUser should have its own field (inherits from ClientUser via ancestors)
        let mobile_user_fields = &validation.model_fields["MobileUser"];
        assert!(mobile_user_fields.iter().any(|f| f.name == "device_token")); // Added in mobile
    }

    #[test]
    fn test_circular_extends_detected() {
        let file_path = fixtures_path().join("circular/a.cdm");
        let result = FileResolver::resolve_with_ancestors(&file_path);

        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("Circular @extends detected"));
    }

    #[test]
    fn test_file_not_found_error() {
        let file_path = fixtures_path().join("invalid/missing_extends.cdm");
        let result = FileResolver::resolve_with_ancestors(&file_path);

        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("Failed to read file"));
    }
}
