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
