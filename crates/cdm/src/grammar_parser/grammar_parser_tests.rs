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

// =============================================================================
// STRUCT AND CONSTRUCTOR TESTS
// =============================================================================

#[test]
fn test_grammar_parser_debug_impl() {
    let file_path = fixtures_path().join("single_file/simple.cdm");
    let tree = FileResolver::load(&file_path).unwrap();
    let parser = GrammarParser::new(&tree.main);

    let debug_str = format!("{:?}", parser);
    assert!(debug_str.contains("GrammarParser"));
    assert!(debug_str.contains("loaded_file"));
    assert!(debug_str.contains("cached_tree"));
}

#[test]
fn test_grammar_parser_new_creates_empty_cache() {
    let file_path = fixtures_path().join("single_file/simple.cdm");
    let tree = FileResolver::load(&file_path).unwrap();
    let parser = GrammarParser::new(&tree.main);

    // Initially the cache should be None (before first parse)
    let debug_str = format!("{:?}", parser);
    assert!(debug_str.contains("None"), "Cache should be None before parsing");
}

// =============================================================================
// PARSING COMPLEX STRUCTURES TESTS
// =============================================================================

#[test]
fn test_parse_file_with_extends_directive() {
    let file_path = fixtures_path().join("single_extends/child.cdm");
    let tree = FileResolver::load(&file_path).unwrap();
    let parser = GrammarParser::new(&tree.main);

    let result = parser.parse();
    assert!(result.is_ok());

    let syntax_tree = result.unwrap();
    assert_eq!(syntax_tree.root_node().kind(), "source_file");
    assert!(!syntax_tree.root_node().has_error());
}

#[test]
fn test_parse_file_with_multiple_extends() {
    let file_path = fixtures_path().join("multiple_extends/child.cdm");
    let tree = FileResolver::load(&file_path).unwrap();
    let parser = GrammarParser::new(&tree.main);

    let result = parser.parse();
    assert!(result.is_ok());

    let syntax_tree = result.unwrap();
    assert!(!syntax_tree.root_node().has_error());
}

#[test]
fn test_parse_file_with_nested_chain() {
    let file_path = fixtures_path().join("nested_chain/client.cdm");
    let tree = FileResolver::load(&file_path).unwrap();
    let parser = GrammarParser::new(&tree.main);

    let result = parser.parse();
    assert!(result.is_ok());

    let syntax_tree = result.unwrap();
    assert!(!syntax_tree.root_node().has_error());
}

#[test]
fn test_parse_file_with_model_extends() {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_model_extends.cdm");

    std::fs::write(
        &temp_file,
        r#"
BaseModel {
  id: number #1
  created_at: string #2
} #10

User extends BaseModel {
  name: string #10
  email: string #11
} #20
"#,
    )
    .unwrap();

    let loaded_file = LoadedFile::new_for_test(temp_file.clone());
    let parser = GrammarParser::new(&loaded_file);
    let result = parser.parse();

    std::fs::remove_file(&temp_file).unwrap();

    assert!(result.is_ok());
    let tree = result.unwrap();
    assert!(!tree.root_node().has_error());
}

#[test]
fn test_parse_file_with_type_alias() {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_type_alias.cdm");

    // CDM uses colon syntax for type aliases: Name: type
    std::fs::write(
        &temp_file,
        r#"
UUID: string #1
Email: string #2
Status: "active" | "inactive" | "pending" #3

User {
  id: UUID #10
  email: Email #11
  status: Status #12
} #100
"#,
    )
    .unwrap();

    let loaded_file = LoadedFile::new_for_test(temp_file.clone());
    let parser = GrammarParser::new(&loaded_file);
    let result = parser.parse();

    std::fs::remove_file(&temp_file).unwrap();

    assert!(result.is_ok());
    let tree = result.unwrap();
    assert!(!tree.root_node().has_error());
}

#[test]
fn test_parse_file_with_comments() {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_comments.cdm");

    std::fs::write(
        &temp_file,
        r#"
// This is a single line comment
User {
  // Field comment
  id: number #1  // Inline comment
  name: string #2
} #10
"#,
    )
    .unwrap();

    let loaded_file = LoadedFile::new_for_test(temp_file.clone());
    let parser = GrammarParser::new(&loaded_file);
    let result = parser.parse();

    std::fs::remove_file(&temp_file).unwrap();

    assert!(result.is_ok());
    let tree = result.unwrap();
    assert!(!tree.root_node().has_error());
}

#[test]
fn test_parse_file_with_optional_fields() {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_optional.cdm");

    std::fs::write(
        &temp_file,
        r#"
User {
  id: number #1
  name: string #2
  email?: string #3
  bio?: string #4
} #10
"#,
    )
    .unwrap();

    let loaded_file = LoadedFile::new_for_test(temp_file.clone());
    let parser = GrammarParser::new(&loaded_file);
    let result = parser.parse();

    std::fs::remove_file(&temp_file).unwrap();

    assert!(result.is_ok());
    let tree = result.unwrap();
    assert!(!tree.root_node().has_error());
}

#[test]
fn test_parse_file_with_array_types() {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_arrays.cdm");

    std::fs::write(
        &temp_file,
        r#"
User {
  id: number #1
  tags: string[] #2
  scores: number[] #3
  friends: User[] #4
} #10
"#,
    )
    .unwrap();

    let loaded_file = LoadedFile::new_for_test(temp_file.clone());
    let parser = GrammarParser::new(&loaded_file);
    let result = parser.parse();

    std::fs::remove_file(&temp_file).unwrap();

    assert!(result.is_ok());
    let tree = result.unwrap();
    assert!(!tree.root_node().has_error());
}

#[test]
fn test_parse_file_with_union_types() {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_union.cdm");

    // CDM uses colon syntax for type aliases
    std::fs::write(
        &temp_file,
        r#"
StringOrNumber: string | number #1

User {
  id: string | number #10
  data: StringOrNumber #11
} #100
"#,
    )
    .unwrap();

    let loaded_file = LoadedFile::new_for_test(temp_file.clone());
    let parser = GrammarParser::new(&loaded_file);
    let result = parser.parse();

    std::fs::remove_file(&temp_file).unwrap();

    assert!(result.is_ok());
    let tree = result.unwrap();
    assert!(!tree.root_node().has_error());
}

#[test]
fn test_parse_file_with_plugin_import() {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_plugin.cdm");

    std::fs::write(
        &temp_file,
        r#"
@sql { dialect: "postgres" }
@typescript { output_dir: "./types" }

User {
  id: number #1
  name: string #2
} #10
"#,
    )
    .unwrap();

    let loaded_file = LoadedFile::new_for_test(temp_file.clone());
    let parser = GrammarParser::new(&loaded_file);
    let result = parser.parse();

    std::fs::remove_file(&temp_file).unwrap();

    assert!(result.is_ok());
    let tree = result.unwrap();
    assert!(!tree.root_node().has_error());
}

#[test]
fn test_parse_file_with_template_import() {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_template_import.cdm");

    std::fs::write(
        &temp_file,
        r#"
import sql from "sql/postgres-types"
import auth from "cdm/auth" { version: "^2.0.0" }

User {
  id: sql.UUID #1
  name: string #2
} #10
"#,
    )
    .unwrap();

    let loaded_file = LoadedFile::new_for_test(temp_file.clone());
    let parser = GrammarParser::new(&loaded_file);
    let result = parser.parse();

    std::fs::remove_file(&temp_file).unwrap();

    assert!(result.is_ok());
    let tree = result.unwrap();
    assert!(!tree.root_node().has_error());
}

#[test]
fn test_parse_file_with_extends_template() {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_extends_template.cdm");

    std::fs::write(
        &temp_file,
        r#"
extends "cdm/auth" { version: "^2.0.0" }

User extends AuthUser {
  profile_pic: string #10
} #100
"#,
    )
    .unwrap();

    let loaded_file = LoadedFile::new_for_test(temp_file.clone());
    let parser = GrammarParser::new(&loaded_file);
    let result = parser.parse();

    std::fs::remove_file(&temp_file).unwrap();

    assert!(result.is_ok());
    let tree = result.unwrap();
    assert!(!tree.root_node().has_error());
}

#[test]
fn test_parse_file_with_qualified_types() {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_qualified.cdm");

    std::fs::write(
        &temp_file,
        r#"
import sql from "./sql_types"

User {
  id: sql.UUID #1
  email: sql.Varchar #2
  created_at: sql.Timestamp #3
} #10
"#,
    )
    .unwrap();

    let loaded_file = LoadedFile::new_for_test(temp_file.clone());
    let parser = GrammarParser::new(&loaded_file);
    let result = parser.parse();

    std::fs::remove_file(&temp_file).unwrap();

    assert!(result.is_ok());
    let tree = result.unwrap();
    assert!(!tree.root_node().has_error());
}

// =============================================================================
// CACHING BEHAVIOR TESTS
// =============================================================================

#[test]
fn test_parse_caching_returns_same_tree() {
    let file_path = fixtures_path().join("single_file/simple.cdm");
    let tree = FileResolver::load(&file_path).unwrap();
    let parser = GrammarParser::new(&tree.main);

    // Parse multiple times
    let tree1 = parser.parse().unwrap();
    let tree1_root_kind = tree1.root_node().kind().to_string();
    drop(tree1);

    let tree2 = parser.parse().unwrap();
    let tree2_root_kind = tree2.root_node().kind().to_string();
    drop(tree2);

    let tree3 = parser.parse().unwrap();
    let tree3_root_kind = tree3.root_node().kind().to_string();

    // All should have the same root node kind
    assert_eq!(tree1_root_kind, tree2_root_kind);
    assert_eq!(tree2_root_kind, tree3_root_kind);
}

#[test]
fn test_cache_populated_after_parse() {
    let file_path = fixtures_path().join("single_file/simple.cdm");
    let tree = FileResolver::load(&file_path).unwrap();
    let parser = GrammarParser::new(&tree.main);

    // Before parsing
    let debug_before = format!("{:?}", parser);
    assert!(debug_before.contains("None"), "Cache should be None before parsing");

    // Parse
    let _ = parser.parse().unwrap();

    // After parsing
    let debug_after = format!("{:?}", parser);
    assert!(debug_after.contains("Some"), "Cache should be Some after parsing");
}

#[test]
fn test_extract_extends_uses_cached_tree() {
    let file_path = fixtures_path().join("single_extends/child.cdm");
    let tree = FileResolver::load(&file_path).unwrap();
    let parser = GrammarParser::new(&tree.main);

    // Call extract_extends_paths first (which will parse)
    let extends1 = parser.extract_extends_paths();
    assert_eq!(extends1.len(), 1);

    // Call again - should use cached tree
    let extends2 = parser.extract_extends_paths();
    assert_eq!(extends2.len(), 1);

    // Results should be the same
    assert_eq!(extends1[0], extends2[0]);
}

// =============================================================================
// EXTRACT EXTENDS TESTS
// =============================================================================

#[test]
fn test_extract_extends_nested_chain() {
    // Client extends Mobile extends Base
    let file_path = fixtures_path().join("nested_chain/client.cdm");
    let tree = FileResolver::load(&file_path).unwrap();
    let parser = GrammarParser::new(&tree.main);

    let extends = parser.extract_extends_paths();
    // Client only directly extends one file
    assert_eq!(extends.len(), 1);
    assert!(extends[0].contains("base.cdm") || extends[0].contains("mobile.cdm"));
}

#[test]
fn test_extract_extends_preserves_order() {
    let file_path = fixtures_path().join("multiple_extends/child.cdm");
    let tree = FileResolver::load(&file_path).unwrap();
    let parser = GrammarParser::new(&tree.main);

    let extends = parser.extract_extends_paths();
    assert_eq!(extends.len(), 2);

    // Paths should be in the order they appear in the file
    // Both should be relative paths
    for path in &extends {
        assert!(path.starts_with("./") || path.starts_with("../"));
    }
}

#[test]
fn test_extract_extends_with_mixed_directives() {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_mixed_directives.cdm");

    std::fs::write(
        &temp_file,
        r#"
extends "./base1.cdm"
@sql { dialect: "postgres" }
extends "./base2.cdm"
@typescript { output_dir: "./types" }

User {
  id: number #1
} #10
"#,
    )
    .unwrap();

    let loaded_file = LoadedFile::new_for_test(temp_file.clone());
    let parser = GrammarParser::new(&loaded_file);
    let extends = parser.extract_extends_paths();

    std::fs::remove_file(&temp_file).unwrap();

    // Should extract both extends directives
    assert_eq!(extends.len(), 2);
    assert!(extends[0].contains("base1.cdm"));
    assert!(extends[1].contains("base2.cdm"));
}

#[test]
fn test_extract_extends_ignores_extends_keyword() {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_extends_keyword.cdm");

    std::fs::write(
        &temp_file,
        r#"
extends "./base.cdm"

// extends keyword for model inheritance is different from extends
User extends BaseModel {
  name: string #1
} #10
"#,
    )
    .unwrap();

    let loaded_file = LoadedFile::new_for_test(temp_file.clone());
    let parser = GrammarParser::new(&loaded_file);
    let extends = parser.extract_extends_paths();

    std::fs::remove_file(&temp_file).unwrap();

    // Should only extract extends directive, not model extends
    assert_eq!(extends.len(), 1);
    assert!(extends[0].contains("base.cdm"));
}

#[test]
fn test_extract_extends_ignores_extends_template() {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_template_extends.cdm");

    std::fs::write(
        &temp_file,
        r#"
extends "./local_file.cdm"
extends "cdm/auth" { version: "^2.0.0" }

User {
  id: number #1
} #10
"#,
    )
    .unwrap();

    let loaded_file = LoadedFile::new_for_test(temp_file.clone());
    let parser = GrammarParser::new(&loaded_file);
    let extends = parser.extract_extends_paths();

    std::fs::remove_file(&temp_file).unwrap();

    // Should only extract local file paths, not registry template extends
    assert_eq!(extends.len(), 1);
    assert!(extends[0].contains("local_file.cdm"));
}

// =============================================================================
// TREE STRUCTURE TESTS
// =============================================================================

#[test]
fn test_tree_has_correct_children_count() {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_children.cdm");

    std::fs::write(
        &temp_file,
        r#"
User {
  id: number #1
} #10

Post {
  id: number #1
} #11
"#,
    )
    .unwrap();

    let loaded_file = LoadedFile::new_for_test(temp_file.clone());
    let parser = GrammarParser::new(&loaded_file);
    let tree = parser.parse().unwrap();

    std::fs::remove_file(&temp_file).unwrap();

    let root = tree.root_node();

    // Count model definitions (node kind is "model_definition")
    let mut model_count = 0;
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "model_definition" {
            model_count += 1;
        }
    }

    assert_eq!(model_count, 2);
}

#[test]
fn test_tree_node_positions() {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_positions.cdm");

    std::fs::write(&temp_file, "User {\n  id: number #1\n} #10").unwrap();

    let loaded_file = LoadedFile::new_for_test(temp_file.clone());
    let parser = GrammarParser::new(&loaded_file);
    let tree = parser.parse().unwrap();

    std::fs::remove_file(&temp_file).unwrap();

    let root = tree.root_node();

    // Find the model node (kind is "model_definition")
    let mut cursor = root.walk();
    let model_node = root.children(&mut cursor).find(|n| n.kind() == "model_definition");
    assert!(model_node.is_some());

    let model = model_node.unwrap();
    let start = model.start_position();
    let end = model.end_position();

    // Model should start at line 0 (first line)
    assert_eq!(start.row, 0);
    // Model should span multiple lines
    assert!(end.row > start.row || end.column > start.column);
}

// =============================================================================
// EDGE CASES AND UNICODE TESTS
// =============================================================================

#[test]
fn test_parse_file_with_unicode_content() {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_unicode.cdm");

    std::fs::write(
        &temp_file,
        r#"
// 用户模型 - Chinese comment
// Пользователь - Russian comment
// 🎉 Emoji in comment
User {
  id: number #1
  name: string #2  // 名前
} #10
"#,
    )
    .unwrap();

    let loaded_file = LoadedFile::new_for_test(temp_file.clone());
    let parser = GrammarParser::new(&loaded_file);
    let result = parser.parse();

    std::fs::remove_file(&temp_file).unwrap();

    assert!(result.is_ok());
    let tree = result.unwrap();
    assert!(!tree.root_node().has_error());
}

#[test]
fn test_parse_file_with_whitespace_only() {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_whitespace.cdm");

    std::fs::write(&temp_file, "   \n\n\t\t\n   \n").unwrap();

    let loaded_file = LoadedFile::new_for_test(temp_file.clone());
    let parser = GrammarParser::new(&loaded_file);
    let result = parser.parse();

    std::fs::remove_file(&temp_file).unwrap();

    assert!(result.is_ok());
    let tree = result.unwrap();
    assert_eq!(tree.root_node().kind(), "source_file");
}

#[test]
fn test_parse_file_with_only_comments() {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_only_comments.cdm");

    std::fs::write(
        &temp_file,
        r#"
// Comment line 1
// Comment line 2
// Comment line 3
"#,
    )
    .unwrap();

    let loaded_file = LoadedFile::new_for_test(temp_file.clone());
    let parser = GrammarParser::new(&loaded_file);
    let result = parser.parse();

    std::fs::remove_file(&temp_file).unwrap();

    assert!(result.is_ok());
    let tree = result.unwrap();
    assert!(!tree.root_node().has_error());
}

#[test]
fn test_parse_large_file() {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_large.cdm");

    // Generate a file with many models
    let mut content = String::new();
    for i in 0..100 {
        content.push_str(&format!(
            "Model{} {{\n  id: number #{}\n  name: string #{}\n}} #{}\n\n",
            i,
            i * 10 + 1,
            i * 10 + 2,
            i * 100 + 10
        ));
    }

    std::fs::write(&temp_file, &content).unwrap();

    let loaded_file = LoadedFile::new_for_test(temp_file.clone());
    let parser = GrammarParser::new(&loaded_file);
    let result = parser.parse();

    std::fs::remove_file(&temp_file).unwrap();

    assert!(result.is_ok());
    let tree = result.unwrap();
    assert!(!tree.root_node().has_error());

    // Verify model count (kind is "model_definition")
    let root = tree.root_node();
    let mut model_count = 0;
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "model_definition" {
            model_count += 1;
        }
    }
    assert_eq!(model_count, 100);
}

#[test]
fn test_parse_file_with_nested_types() {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_nested_types.cdm");

    // Test union types with arrays
    std::fs::write(
        &temp_file,
        r#"
StringArray: string[] #1
MixedType: string | number | boolean #2

Container {
  data: string[] #1
  items: MixedType[] #2
  value: string | number #3
} #10
"#,
    )
    .unwrap();

    let loaded_file = LoadedFile::new_for_test(temp_file.clone());
    let parser = GrammarParser::new(&loaded_file);
    let result = parser.parse();

    std::fs::remove_file(&temp_file).unwrap();

    assert!(result.is_ok());
    let tree = result.unwrap();
    assert!(!tree.root_node().has_error());
}

#[test]
fn test_parse_file_with_string_literals() {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_string_literals.cdm");

    // CDM uses colon syntax for type aliases
    std::fs::write(
        &temp_file,
        r#"
Status: "pending" | "active" | "deleted" #1
Priority: "low" | "medium" | "high" | "critical" #2

Task {
  status: Status #1
  priority: Priority #2
} #10
"#,
    )
    .unwrap();

    let loaded_file = LoadedFile::new_for_test(temp_file.clone());
    let parser = GrammarParser::new(&loaded_file);
    let result = parser.parse();

    std::fs::remove_file(&temp_file).unwrap();

    assert!(result.is_ok());
    let tree = result.unwrap();
    assert!(!tree.root_node().has_error());
}

// =============================================================================
// FIELD REMOVAL TESTS
// =============================================================================

#[test]
fn test_parse_file_with_field_removals() {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_removals.cdm");

    std::fs::write(
        &temp_file,
        r#"
BaseModel {
  id: number #1
  internal_data: string #2
  password_hash: string #3
} #10

PublicModel extends BaseModel {
  -internal_data
  -password_hash
  display_name: string #10
} #20
"#,
    )
    .unwrap();

    let loaded_file = LoadedFile::new_for_test(temp_file.clone());
    let parser = GrammarParser::new(&loaded_file);
    let result = parser.parse();

    std::fs::remove_file(&temp_file).unwrap();

    assert!(result.is_ok());
    let tree = result.unwrap();
    assert!(!tree.root_node().has_error());
}

// =============================================================================
// COMPLETE CDM FILE TESTS
// =============================================================================

#[test]
fn test_parse_complete_cdm_file() {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_complete.cdm");

    // CDM uses colon syntax for type aliases and model_definition node type
    std::fs::write(
        &temp_file,
        r#"
// Complete CDM example file
@sql { dialect: "postgres" }
@typescript { output_dir: "./generated" }

// Type aliases (using colon syntax)
UUID: string #1
Email: string #2
Status: "active" | "inactive" | "pending" #3

// Base model
BaseModel {
  id: UUID #1
  created_at: string #2
  updated_at: string #3
} #100

// User model
User extends BaseModel {
  email: Email #10
  name: string #11
  status: Status #12
  roles: string[] #13
  profile?: UserProfile #14
} #200

// User profile
UserProfile {
  bio?: string #1
  avatar_url?: string #2
  settings: JSON #3
} #300

// JSON type alias
JSON: string #4
"#,
    )
    .unwrap();

    let loaded_file = LoadedFile::new_for_test(temp_file.clone());
    let parser = GrammarParser::new(&loaded_file);
    let result = parser.parse();

    std::fs::remove_file(&temp_file).unwrap();

    assert!(result.is_ok());
    let tree = result.unwrap();
    assert!(!tree.root_node().has_error());

    // Count different node types
    let root = tree.root_node();
    let mut model_count = 0;
    let mut type_alias_count = 0;
    let mut plugin_count = 0;

    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "model_definition" => model_count += 1,
            "type_alias" => type_alias_count += 1,
            "plugin_import" => plugin_count += 1,
            _ => {}
        }
    }

    assert_eq!(model_count, 3, "Expected 3 models");
    assert_eq!(type_alias_count, 4, "Expected 4 type aliases");
    assert_eq!(plugin_count, 2, "Expected 2 plugin imports");
}
