use std::cell::RefCell;

use crate::file_resolver::LoadedFile;

/// A grammar parser that wraps a LoadedFile and provides cached parsing.
///
/// GrammarParser is responsible for:
/// - Parsing CDM source files using tree-sitter
/// - Caching the parsed tree for reuse
/// - Extracting @extends paths from parsed source
///
/// It does NOT perform semantic validation - that's the responsibility of the validate module.
#[derive(Debug)]
pub struct GrammarParser<'a> {
    /// Reference to the loaded file
    loaded_file: &'a LoadedFile,
    /// Cached parsed tree (parsed on first access)
    cached_tree: RefCell<Option<tree_sitter::Tree>>,
}

impl<'a> GrammarParser<'a> {
    /// Create a new GrammarParser for a LoadedFile
    pub fn new(loaded_file: &'a LoadedFile) -> Self {
        Self {
            loaded_file,
            cached_tree: RefCell::new(None),
        }
    }

    /// Parse the loaded file, caching the result for subsequent calls.
    ///
    /// Returns a reference to the cached tree. The tree is parsed once and cached.
    ///
    /// # Returns
    /// * `Ok(&tree_sitter::Tree)` - Reference to the parsed syntax tree
    /// * `Err(String)` - Parse error or file reading error
    ///
    /// # Example
    /// ```no_run
    /// use cdm::{FileResolver, GrammarParser};
    ///
    /// let tree = FileResolver::load("schema.cdm").unwrap();
    /// let parser = GrammarParser::new(&tree.main);
    /// let syntax_tree = parser.parse().unwrap();
    /// ```
    pub fn parse(&self) -> Result<std::cell::Ref<'_, tree_sitter::Tree>, String> {
        // Check if already cached
        if self.cached_tree.borrow().is_none() {
            // Read source
            let source = self
                .loaded_file
                .source()
                .map_err(|e| format!("Failed to read file: {}", e))?;

            // Parse with tree-sitter
            let mut parser = tree_sitter::Parser::new();
            parser
                .set_language(&grammar::LANGUAGE.into())
                .map_err(|e| format!("Failed to set language: {}", e))?;

            let tree = parser
                .parse(&source, None)
                .ok_or_else(|| "Failed to parse source".to_string())?;

            // Cache the tree
            *self.cached_tree.borrow_mut() = Some(tree);
        }

        // Return reference to cached tree
        Ok(std::cell::Ref::map(
            self.cached_tree.borrow(),
            |opt| opt.as_ref().unwrap()
        ))
    }

    /// Extract all @extends paths from the loaded file.
    ///
    /// This will parse the file if not already parsed, then extract the @extends directives.
    ///
    /// # Returns
    /// * `Vec<String>` - List of @extends paths in order they appear
    ///
    /// # Example
    /// ```no_run
    /// use cdm::{FileResolver, GrammarParser};
    ///
    /// let tree = FileResolver::load("schema.cdm").unwrap();
    /// let parser = GrammarParser::new(&tree.main);
    /// let extends = parser.extract_extends_paths();
    /// println!("Extends: {:?}", extends);
    /// ```
    pub fn extract_extends_paths(&self) -> Vec<String> {
        // Parse the file (uses cached tree if available)
        let tree_ref = match self.parse() {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };

        // Read source (uses cached source from LoadedFile)
        let source = match self.loaded_file.source() {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        // Extract @extends paths
        let root = tree_ref.root_node();
        let mut cursor = root.walk();
        let mut paths = Vec::new();

        for node in root.children(&mut cursor) {
            if node.kind() == "extends_directive" {
                if let Some(path_node) = node.child_by_field_name("path") {
                    let path_text = path_node
                        .utf8_text(source.as_bytes())
                        .unwrap_or("")
                        .to_string();
                    paths.push(path_text);
                }
            }
        }

        paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_resolver::FileResolver;
    use std::path::PathBuf;

    fn fixtures_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test_fixtures")
            .join("file_resolver")
    }

    #[test]
    fn test_parse_single_file() {
        let file_path = fixtures_path().join("single_file/simple.cdm");
        let tree = FileResolver::load(&file_path).unwrap();
        let parser = GrammarParser::new(&tree.main);

        let syntax_tree = parser.parse();
        assert!(syntax_tree.is_ok());

        let tree = syntax_tree.unwrap();
        assert_eq!(tree.root_node().kind(), "source_file");
    }

    #[test]
    fn test_extract_extends_no_extends() {
        let file_path = fixtures_path().join("single_file/simple.cdm");
        let tree = FileResolver::load(&file_path).unwrap();
        let parser = GrammarParser::new(&tree.main);

        let extends = parser.extract_extends_paths();
        assert_eq!(extends.len(), 0);
    }

    #[test]
    fn test_extract_extends_single() {
        let file_path = fixtures_path().join("single_extends/child.cdm");
        let tree = FileResolver::load(&file_path).unwrap();
        let parser = GrammarParser::new(&tree.main);

        let extends = parser.extract_extends_paths();
        assert_eq!(extends.len(), 1);
        assert!(extends[0].contains("base.cdm"));
    }

    #[test]
    fn test_extract_extends_multiple() {
        let file_path = fixtures_path().join("multiple_extends/child.cdm");
        let tree = FileResolver::load(&file_path).unwrap();
        let parser = GrammarParser::new(&tree.main);

        let extends = parser.extract_extends_paths();
        assert_eq!(extends.len(), 2);
    }

    #[test]
    fn test_parse_caching() {
        let file_path = fixtures_path().join("single_file/simple.cdm");
        let tree = FileResolver::load(&file_path).unwrap();
        let parser = GrammarParser::new(&tree.main);

        // Parse twice - should work both times
        let tree1 = parser.parse();
        assert!(tree1.is_ok());

        let tree2 = parser.parse();
        assert!(tree2.is_ok());
    }

    #[test]
    fn test_parse_error_file_not_found() {
        // Create a LoadedFile pointing to a non-existent file
        let non_existent_path = fixtures_path().join("does_not_exist.cdm");
        let loaded_file = LoadedFile::new_for_test(non_existent_path);
        let parser = GrammarParser::new(&loaded_file);

        let result = parser.parse();
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(
            error.contains("Failed to read file"),
            "Expected 'Failed to read file' error, got: {}",
            error
        );
    }

    #[test]
    fn test_parse_error_file_deleted_after_creation() {
        // Create a temporary file that we'll delete
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_deleted_file.cdm");

        // Write some content first
        std::fs::write(&temp_file, "User { id: number }").unwrap();

        // Create LoadedFile
        let loaded_file = LoadedFile::new_for_test(temp_file.clone());

        // Delete the file before parsing
        std::fs::remove_file(&temp_file).unwrap();

        let parser = GrammarParser::new(&loaded_file);
        let result = parser.parse();

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(
            error.contains("Failed to read file"),
            "Expected 'Failed to read file' error, got: {}",
            error
        );
    }

    #[test]
    fn test_parse_error_permission_denied() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            // Create a temporary file with no read permissions
            let temp_dir = std::env::temp_dir();
            let temp_file = temp_dir.join("test_no_permission.cdm");

            // Write some content first
            std::fs::write(&temp_file, "User { id: number }").unwrap();

            // Remove read permissions
            let mut perms = std::fs::metadata(&temp_file).unwrap().permissions();
            perms.set_mode(0o000); // No permissions
            std::fs::set_permissions(&temp_file, perms).unwrap();

            // Create LoadedFile
            let loaded_file = LoadedFile::new_for_test(temp_file.clone());

            let parser = GrammarParser::new(&loaded_file);
            let result = parser.parse();

            // Cleanup - restore permissions before asserting
            let mut perms = std::fs::metadata(&temp_file).unwrap().permissions();
            perms.set_mode(0o644);
            std::fs::set_permissions(&temp_file, perms.clone()).unwrap();
            std::fs::remove_file(&temp_file).unwrap();

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert!(
                error.contains("Failed to read file"),
                "Expected 'Failed to read file' error, got: {}",
                error
            );
        }
    }

    #[test]
    fn test_parse_invalid_syntax() {
        // Create a temporary file with invalid CDM syntax
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_invalid_syntax.cdm");

        // Write content that tree-sitter can still parse (it's very permissive)
        // Tree-sitter will parse almost anything, but we can test that the tree contains ERROR nodes
        std::fs::write(&temp_file, "{{{{ invalid syntax ))))").unwrap();

        let loaded_file = LoadedFile::new_for_test(temp_file.clone());

        let parser = GrammarParser::new(&loaded_file);
        let result = parser.parse();

        // Cleanup
        std::fs::remove_file(&temp_file).unwrap();

        // Tree-sitter will still parse this, but the tree will contain ERROR nodes
        // The parse() function itself won't fail, but the tree will have errors
        assert!(result.is_ok());
        let tree = result.unwrap();

        // Check if the tree has ERROR nodes (indicating parse issues)
        let root = tree.root_node();
        assert!(root.has_error(), "Expected tree to have ERROR nodes for invalid syntax");
    }

    #[test]
    fn test_extract_extends_error_handling() {
        // Create a LoadedFile pointing to a non-existent file
        let non_existent_path = fixtures_path().join("does_not_exist.cdm");
        let loaded_file = LoadedFile::new_for_test(non_existent_path);
        let parser = GrammarParser::new(&loaded_file);

        // extract_extends_paths should return empty vec on error, not panic
        let extends = parser.extract_extends_paths();
        assert_eq!(extends.len(), 0);
    }

    #[test]
    fn test_parse_empty_file() {
        // Create a temporary empty file
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_empty.cdm");

        std::fs::write(&temp_file, "").unwrap();

        let loaded_file = LoadedFile::new_for_test(temp_file.clone());

        let parser = GrammarParser::new(&loaded_file);
        let result = parser.parse();

        // Cleanup
        std::fs::remove_file(&temp_file).unwrap();

        // Empty file should parse successfully
        assert!(result.is_ok());
        let tree = result.unwrap();
        assert_eq!(tree.root_node().kind(), "source_file");
    }
}
