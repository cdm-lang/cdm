use std::cell::RefCell;

use crate::file_resolver::LoadedFile;

/// A grammar parser that wraps a LoadedFile and provides cached parsing.
///
/// GrammarParser is responsible for:
/// - Parsing CDM source files using tree-sitter
/// - Caching the parsed tree for reuse
/// - Extracting extends paths from parsed source
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

    /// Extract all extends paths from the loaded file.
    ///
    /// This will parse the file if not already parsed, then extract the extends directives.
    ///
    /// # Returns
    /// * `Vec<String>` - List of extends paths in order they appear
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

        // Extract extends paths
        let root = tree_ref.root_node();
        let mut cursor = root.walk();
        let mut paths = Vec::new();

        for node in root.children(&mut cursor) {
            if node.kind() == "extends_template" {
                if let Some(source_node) = node.child_by_field_name("source") {
                    // source_node is template_source, which contains local_path, git_reference, or registry_name
                    // Only extract local paths (./path or ../path), not registry names or git refs
                    let mut inner_cursor = source_node.walk();
                    for child in source_node.children(&mut inner_cursor) {
                        if child.kind() == "local_path" {
                            let path_text = child
                                .utf8_text(source.as_bytes())
                                .unwrap_or("")
                                .to_string();
                            paths.push(path_text);
                        }
                    }
                }
            }
        }

        paths
    }
}

#[cfg(test)]
#[path = "grammar_parser/grammar_parser_tests.rs"]
mod grammar_parser_tests;
