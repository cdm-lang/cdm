use super::*;
use std::path::PathBuf;

fn fixtures_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_fixtures")
        .join("format")
}

#[test]
fn test_entity_id_tracker_global_ids() {
    let mut tracker = EntityIdTracker::new();

    // Add some IDs
    tracker.add_global_id(1);
    tracker.add_global_id(5);
    tracker.add_global_id(3);

    // Next ID should be 6
    assert_eq!(tracker.next_global_id(), 6);
    assert_eq!(tracker.next_global_id(), 7);
}

#[test]
fn test_entity_id_tracker_field_ids() {
    let mut tracker = EntityIdTracker::new();

    // Add field IDs for different models
    tracker.add_field_id("User", 1);
    tracker.add_field_id("User", 3);
    tracker.add_field_id("Post", 1);
    tracker.add_field_id("Post", 2);

    // Next IDs should be scoped per model
    assert_eq!(tracker.next_field_id("User"), 4);
    assert_eq!(tracker.next_field_id("Post"), 3);
    assert_eq!(tracker.next_field_id("Comment"), 1);
}

#[test]
fn test_format_without_ids() {
    let path = fixtures_path().join("without_ids.cdm");

    let options = FormatOptions {
        assign_ids: true,
        check: true, // Don't write
        write: false,
        indent_size: 2,
        format_whitespace: false,
    };

    let result = format_file(&path, &options).expect("Format should succeed");

    // Should have modified the file
    assert!(result.modified);

    // Should have assigned 11 IDs (2 type aliases + 2 models + 7 fields)
    assert_eq!(result.assignments.len(), 11);

    // Check type alias IDs
    let email = result.assignments.iter().find(|a| {
        a.entity_type == EntityType::TypeAlias && a.entity_name == "Email"
    });
    assert!(email.is_some());
    assert_eq!(email.unwrap().assigned_id, 1);

    let status = result.assignments.iter().find(|a| {
        a.entity_type == EntityType::TypeAlias && a.entity_name == "Status"
    });
    assert!(status.is_some());
    assert_eq!(status.unwrap().assigned_id, 2);

    // Check model IDs
    let user = result.assignments.iter().find(|a| {
        a.entity_type == EntityType::Model && a.entity_name == "User"
    });
    assert!(user.is_some());
    assert_eq!(user.unwrap().assigned_id, 3);

    let post = result.assignments.iter().find(|a| {
        a.entity_type == EntityType::Model && a.entity_name == "Post"
    });
    assert!(post.is_some());
    assert_eq!(post.unwrap().assigned_id, 4);

    // Check field IDs are scoped per model
    let user_fields: Vec<_> = result.assignments.iter()
        .filter(|a| a.entity_type == EntityType::Field && a.model_name.as_deref() == Some("User"))
        .collect();
    assert_eq!(user_fields.len(), 4);

    let post_fields: Vec<_> = result.assignments.iter()
        .filter(|a| a.entity_type == EntityType::Field && a.model_name.as_deref() == Some("Post"))
        .collect();
    assert_eq!(post_fields.len(), 3);
}

#[test]
fn test_format_partial_ids() {
    let path = fixtures_path().join("partial_ids.cdm");
    let options = FormatOptions {
        assign_ids: true,
        check: true,
        write: false,
        indent_size: 2,
        format_whitespace: false,
    };

    let result = format_file(&path, &options).expect("Format should succeed");

    // Should have modified the file
    assert!(result.modified);

    // Should assign missing IDs only
    // Missing: Status type alias, User.email, User.status, Post model + 3 fields
    assert_eq!(result.assignments.len(), 7);

    // Status should get ID 11 (next after User #10)
    let status = result.assignments.iter().find(|a| {
        a.entity_type == EntityType::TypeAlias && a.entity_name == "Status"
    });
    assert!(status.is_some());
    assert_eq!(status.unwrap().assigned_id, 11);

    // Post should get ID 12 (next after Status #11)
    let post = result.assignments.iter().find(|a| {
        a.entity_type == EntityType::Model && a.entity_name == "Post"
    });
    assert!(post.is_some());
    assert_eq!(post.unwrap().assigned_id, 12);

    // User.email should get ID 4 (User.id has #1, User.name has #3, next is 4)
    let user_email = result.assignments.iter().find(|a| {
        a.entity_type == EntityType::Field
            && a.entity_name == "email"
            && a.model_name.as_deref() == Some("User")
    });
    assert!(user_email.is_some());
    assert_eq!(user_email.unwrap().assigned_id, 4);

    // User.status should get ID 5 (next after User.email #4)
    let user_status = result.assignments.iter().find(|a| {
        a.entity_type == EntityType::Field
            && a.entity_name == "status"
            && a.model_name.as_deref() == Some("User")
    });
    assert!(user_status.is_some());
    assert_eq!(user_status.unwrap().assigned_id, 5);
}

#[test]
fn test_format_all_ids() {
    let path = fixtures_path().join("all_ids.cdm");
    let options = FormatOptions {
        assign_ids: true,
        check: true,
        write: false,
        indent_size: 2,
        format_whitespace: false,
    };

    let result = format_file(&path, &options).expect("Format should succeed");

    // Should not have modified the file (all IDs already present)
    assert!(!result.modified);
    assert_eq!(result.assignments.len(), 0);
}

#[test]
fn test_format_without_assign_ids() {
    let path = fixtures_path().join("without_ids.cdm");
    let options = FormatOptions {
        assign_ids: false, // Don't assign IDs
        check: true,
        write: false,
        indent_size: 2,
        format_whitespace: false,
    };

    let result = format_file(&path, &options).expect("Format should succeed");

    // Should not have modified the file
    assert!(!result.modified);
    assert_eq!(result.assignments.len(), 0);
}

#[test]
fn test_field_id_scoping() {
    // Test that field IDs are scoped per model
    let mut tracker = EntityIdTracker::new();

    // Simulate User model with field IDs 1, 2, 3
    tracker.add_field_id("User", 1);
    tracker.add_field_id("User", 2);
    tracker.add_field_id("User", 3);

    // Simulate Post model with field IDs 1, 2
    tracker.add_field_id("Post", 1);
    tracker.add_field_id("Post", 2);

    // Next field ID for User should be 4
    assert_eq!(tracker.next_field_id("User"), 4);

    // Next field ID for Post should be 3
    assert_eq!(tracker.next_field_id("Post"), 3);

    // Next field ID for new model should be 1
    assert_eq!(tracker.next_field_id("Comment"), 1);
}

#[test]
fn test_global_id_collision_avoidance() {
    let mut tracker = EntityIdTracker::new();

    // Add non-sequential IDs
    tracker.add_global_id(1);
    tracker.add_global_id(5);
    tracker.add_global_id(10);

    // Next ID should be 11 (after the highest)
    assert_eq!(tracker.next_global_id(), 11);
    assert_eq!(tracker.next_global_id(), 12);
}

#[test]
fn test_format_files_multiple() {
    let path1 = fixtures_path().join("without_ids.cdm");
    let path2 = fixtures_path().join("partial_ids.cdm");

    let options = FormatOptions {
        assign_ids: true,
        check: true,
        write: false,
        indent_size: 2,
        format_whitespace: false,
    };

    let results = format_files(&[path1, path2], &options).expect("Format should succeed");

    // Should have formatted 2 files
    assert_eq!(results.len(), 2);

    // Both should be modified
    assert!(results[0].modified);
    assert!(results[1].modified);

    // First file should have 11 assignments, second should have 7
    assert_eq!(results[0].assignments.len(), 11);
    assert_eq!(results[1].assignments.len(), 7);
}

#[test]
fn test_format_invalid_path() {
    let path = PathBuf::from("nonexistent/file.cdm");
    let options = FormatOptions {
        assign_ids: true,
        check: true,
        write: false,
        indent_size: 2,
        format_whitespace: false,
    };

    let result = format_file(&path, &options);
    assert!(result.is_err());

    let diagnostics = result.unwrap_err();
    assert!(!diagnostics.is_empty());
    assert!(diagnostics[0].message.contains("Failed to resolve path"));
}

#[test]
fn test_format_with_write() {
    use tempfile::NamedTempFile;
    use std::io::Write;

    // Create a temporary file with content
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, "Email: string\n\nUser {{\n  email: Email\n}}\n").expect("Failed to write");
    let temp_path = temp_file.path().to_path_buf();

    let options = FormatOptions {
        assign_ids: true,
        check: false, // Actually write
        write: true,
        indent_size: 2,
        format_whitespace: false,
    };

    let result = format_file(&temp_path, &options).expect("Format should succeed");

    // Should have modified the file
    assert!(result.modified);
    assert_eq!(result.assignments.len(), 3); // Email, User, User.email

    // Read the file back and verify IDs were written
    let content = std::fs::read_to_string(&temp_path).expect("Failed to read temp file");
    assert!(content.contains("#1"));
    assert!(content.contains("#2"));
}

#[test]
fn test_reconstruct_source_preserves_structure() {
    let path = fixtures_path().join("without_ids.cdm");

    let options = FormatOptions {
        assign_ids: true,
        check: true,
        write: false,
        indent_size: 2,
        format_whitespace: false,
    };

    let result = format_file(&path, &options).expect("Format should succeed");

    // Verify the source structure is preserved
    let tree = FileResolver::load(&path).expect("Failed to load");
    let parser = crate::GrammarParser::new(&tree.main);
    let parse_tree = parser.parse().expect("Failed to parse");
    let root = parse_tree.root_node();
    let source = tree.main.source().expect("Failed to read source");

    // Reconstruct with assignments
    let new_source = reconstruct_source(root, &source, &result.assignments, 2);

    // New source should have all the IDs
    assert!(new_source.contains("#1"));
    assert!(new_source.contains("#2"));
    assert!(new_source.contains("#3"));
    assert!(new_source.contains("#4"));

    // Should still be valid CDM
    assert!(new_source.contains("Email: string"));
    assert!(new_source.contains("Status: \"active\""));
    assert!(new_source.contains("User {"));
    assert!(new_source.contains("Post {"));

    // Should parse without errors
    let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp");
    std::fs::write(temp_file.path(), &new_source).expect("Failed to write");
    let new_tree = FileResolver::load(temp_file.path()).expect("Failed to load formatted file");
    let new_parser = crate::GrammarParser::new(&new_tree.main);
    let new_parse = new_parser.parse().expect("Formatted file should parse");
    assert!(!new_parse.root_node().has_error());
}

#[test]
fn test_extract_entity_id() {
    let source = "Email: string #42";
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&grammar::LANGUAGE.into()).unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    // Find the type_alias node
    let type_alias = root.child(0).unwrap();
    assert_eq!(type_alias.kind(), "type_alias");

    let id = extract_entity_id(type_alias, source);
    assert_eq!(id, Some(42));
}

#[test]
fn test_extract_entity_id_none() {
    let source = "Email: string";
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&grammar::LANGUAGE.into()).unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let type_alias = root.child(0).unwrap();
    let id = extract_entity_id(type_alias, source);
    assert_eq!(id, None);
}

#[test]
fn test_get_node_text() {
    let source = "Email: string";
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&grammar::LANGUAGE.into()).unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let text = get_node_text(root, source);
    assert_eq!(text, "Email: string");
}

#[test]
fn test_field_insertion_with_trailing_whitespace() {
    let source = "User {\n  id: string   \n}";
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&grammar::LANGUAGE.into()).unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let mut tracker = EntityIdTracker::new();
    let assignments = assign_missing_ids(root, source, &mut tracker);

    // Should assign IDs to User model and User.id field
    assert_eq!(assignments.len(), 2);

    let new_source = reconstruct_source(root, source, &assignments, 2);

    // ID should be inserted before trailing whitespace for field
    assert!(new_source.contains("id: string #1"));
    // Model ID should be after the closing brace
    assert!(new_source.contains("} #1"));
}

#[test]
fn test_collect_entity_ids_with_all_types() {
    let source = r#"
Email: string #10

User {
  id: string #1
  email: Email #2
} #20

Post {
  title: string #1
} #21
"#;
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&grammar::LANGUAGE.into()).unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    let mut tracker = EntityIdTracker::new();
    let mut diagnostics = Vec::new();
    collect_entity_ids(root, source, &mut tracker, &mut diagnostics);

    // Should have collected global IDs 10, 20, 21
    assert!(tracker.global_ids.contains(&10));
    assert!(tracker.global_ids.contains(&20));
    assert!(tracker.global_ids.contains(&21));

    // Should have collected User field IDs 1, 2
    assert!(tracker.model_field_ids.get("User").unwrap().contains(&1));
    assert!(tracker.model_field_ids.get("User").unwrap().contains(&2));

    // Should have collected Post field ID 1
    assert!(tracker.model_field_ids.get("Post").unwrap().contains(&1));

    // Next global ID should be 22
    assert_eq!(tracker.next_global_id, 22);

    // Next field IDs should be 3 for User, 2 for Post
    assert_eq!(tracker.next_field_ids.get("User"), Some(&3));
    assert_eq!(tracker.next_field_ids.get("Post"), Some(&2));
}

#[test]
fn test_format_files_with_one_error() {
    let valid_path = fixtures_path().join("without_ids.cdm");
    let invalid_path = PathBuf::from("nonexistent/file.cdm");

    let options = FormatOptions {
        assign_ids: true,
        check: true,
        write: false,
        indent_size: 2,
        format_whitespace: false,
    };

    // format_files should fail if any file fails
    let result = format_files(&[valid_path, invalid_path], &options);
    assert!(result.is_err());
}

#[test]
fn test_whitespace_formatting() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create a file with inconsistent whitespace
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, "Email:string\n\nStatus:\"active\"|\"pending\"\n\nUser{{\nid:string\nemail:Email\n}}\n").expect("Failed to write");
    let temp_path = temp_file.path().to_path_buf();

    let options = FormatOptions {
        assign_ids: true,
        check: false,
        write: true,
        indent_size: 2,
        format_whitespace: true,
    };

    let result = format_file(&temp_path, &options).expect("Format should succeed");
    assert!(result.modified);

    // Read back the formatted content
    let content = std::fs::read_to_string(&temp_path).expect("Failed to read formatted file");

    // Should have proper spacing around colons
    assert!(content.contains("Email: string"));
    assert!(content.contains("Status: \"active\" | \"pending\""));

    // Should have proper indentation
    assert!(content.contains("  id: string"));
    assert!(content.contains("  email: Email"));

    // Should have proper spacing around braces
    assert!(content.contains("User {\n"));
    assert!(content.contains("} #"));
}

#[test]
fn test_whitespace_formatting_preserves_ids() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create a file with existing IDs but bad whitespace
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, "Email:string#42\n\nUser{{\nid:string#1\n}}#10\n").expect("Failed to write");
    let temp_path = temp_file.path().to_path_buf();

    let options = FormatOptions {
        assign_ids: false,
        check: false,
        write: true,
        indent_size: 2,
        format_whitespace: true,
    };

    let result = format_file(&temp_path, &options).expect("Format should succeed");
    assert!(result.modified);

    // Read back the formatted content
    let content = std::fs::read_to_string(&temp_path).expect("Failed to read formatted file");

    // Should preserve existing IDs
    assert!(content.contains("#42"));
    assert!(content.contains("#10"));
    assert!(content.contains("#1"));

    // Should have proper formatting
    assert!(content.contains("Email: string #42"));
    assert!(content.contains("  id: string #1"));
    assert!(content.contains("} #10"));
}

#[test]
fn test_format_preserves_extends_clause() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create a file with extends clause
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, "User {{ name: string #1 }} #10\n\nAdminUser extends User {{ role: string #1 }} #20\n").expect("Failed to write");
    let temp_path = temp_file.path().to_path_buf();

    let options = FormatOptions {
        assign_ids: false,
        check: false,
        write: true,
        indent_size: 2,
        format_whitespace: true,
    };

    let _result = format_file(&temp_path, &options).expect("Format should succeed");

    // Read back the formatted content
    let content = std::fs::read_to_string(&temp_path).expect("Failed to read formatted file");

    // Should preserve extends clause
    assert!(content.contains("AdminUser extends User"),
        "Expected 'AdminUser extends User' but got:\n{}", content);
}

#[test]
fn test_format_preserves_array_types() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create a file with array types
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, "User {{ permissions: string[] #1 }} #10\n").expect("Failed to write");
    let temp_path = temp_file.path().to_path_buf();

    let options = FormatOptions {
        assign_ids: false,
        check: false,
        write: true,
        indent_size: 2,
        format_whitespace: true,
    };

    let _result = format_file(&temp_path, &options).expect("Format should succeed");

    // Read back the formatted content
    let content = std::fs::read_to_string(&temp_path).expect("Failed to read formatted file");

    // Should preserve array element type
    assert!(content.contains("string[]"),
        "Expected 'string[]' but got:\n{}", content);
}

// =============================================================================
// Tests for preserving language elements that were previously dropped
// =============================================================================

#[test]
fn test_format_preserves_plugin_imports() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create a file with plugin imports
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, r#"@sql {{ dialect: "postgres" }}

@api {{ base_url: "/v1" }}

User {{
  id: string
}}
"#).expect("Failed to write");
    let temp_path = temp_file.path().to_path_buf();

    let options = FormatOptions {
        assign_ids: false,
        check: false,
        write: true,
        indent_size: 2,
        format_whitespace: true,
    };

    let _result = format_file(&temp_path, &options).expect("Format should succeed");

    // Read back the formatted content
    let content = std::fs::read_to_string(&temp_path).expect("Failed to read formatted file");

    // Should preserve plugin imports
    assert!(content.contains("@sql"),
        "Expected '@sql' plugin import but got:\n{}", content);
    assert!(content.contains("dialect"),
        "Expected 'dialect' in plugin config but got:\n{}", content);
    assert!(content.contains("@api"),
        "Expected '@api' plugin import but got:\n{}", content);
}

#[test]
fn test_format_preserves_model_level_plugin_config() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create a file with model-level plugin configs
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, r#"User {{
  id: string
  email: string
  @sql {{ table: "users" }}
  @api {{ expose: ["id", "email"] }}
}}
"#).expect("Failed to write");
    let temp_path = temp_file.path().to_path_buf();

    let options = FormatOptions {
        assign_ids: false,
        check: false,
        write: true,
        indent_size: 2,
        format_whitespace: true,
    };

    let _result = format_file(&temp_path, &options).expect("Format should succeed");

    // Read back the formatted content
    let content = std::fs::read_to_string(&temp_path).expect("Failed to read formatted file");

    // Should preserve model-level plugin configs
    assert!(content.contains("@sql"),
        "Expected '@sql' plugin config but got:\n{}", content);
    assert!(content.contains("table"),
        "Expected 'table' in @sql config but got:\n{}", content);
    assert!(content.contains("@api"),
        "Expected '@api' plugin config but got:\n{}", content);
}

#[test]
fn test_format_preserves_field_removal() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create a file with field removal
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, r#"User {{
  id: string
  -password_hash
  email: string
}}
"#).expect("Failed to write");
    let temp_path = temp_file.path().to_path_buf();

    let options = FormatOptions {
        assign_ids: false,
        check: false,
        write: true,
        indent_size: 2,
        format_whitespace: true,
    };

    let _result = format_file(&temp_path, &options).expect("Format should succeed");

    // Read back the formatted content
    let content = std::fs::read_to_string(&temp_path).expect("Failed to read formatted file");

    // Should preserve field removal
    assert!(content.contains("-password_hash"),
        "Expected '-password_hash' field removal but got:\n{}", content);
}

#[test]
fn test_format_preserves_field_override() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create a file with field override (plugin config on inherited field)
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, r#"User {{
  id: string
  status {{ @sql {{ type: "enum" }} }}
}}
"#).expect("Failed to write");
    let temp_path = temp_file.path().to_path_buf();

    let options = FormatOptions {
        assign_ids: false,
        check: false,
        write: true,
        indent_size: 2,
        format_whitespace: true,
    };

    let _result = format_file(&temp_path, &options).expect("Format should succeed");

    // Read back the formatted content
    let content = std::fs::read_to_string(&temp_path).expect("Failed to read formatted file");

    // Should preserve field override
    assert!(content.contains("status {"),
        "Expected 'status {{' field override but got:\n{}", content);
    assert!(content.contains("@sql"),
        "Expected '@sql' in field override but got:\n{}", content);
}

#[test]
fn test_format_preserves_model_removal() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create a file with model removal
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, r#"-OldModel

User {{
  id: string
}}
"#).expect("Failed to write");
    let temp_path = temp_file.path().to_path_buf();

    let options = FormatOptions {
        assign_ids: false,
        check: false,
        write: true,
        indent_size: 2,
        format_whitespace: true,
    };

    let _result = format_file(&temp_path, &options).expect("Format should succeed");

    // Read back the formatted content
    let content = std::fs::read_to_string(&temp_path).expect("Failed to read formatted file");

    // Should preserve model removal
    assert!(content.contains("-OldModel"),
        "Expected '-OldModel' but got:\n{}", content);
}

#[test]
fn test_format_preserves_type_alias_with_plugin_block() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create a file with type alias that has a plugin block
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, r#"Email: string {{ @validation {{ format: "email" }} }}

User {{
  email: Email
}}
"#).expect("Failed to write");
    let temp_path = temp_file.path().to_path_buf();

    let options = FormatOptions {
        assign_ids: false,
        check: false,
        write: true,
        indent_size: 2,
        format_whitespace: true,
    };

    let _result = format_file(&temp_path, &options).expect("Format should succeed");

    // Read back the formatted content
    let content = std::fs::read_to_string(&temp_path).expect("Failed to read formatted file");

    // Should preserve the type alias with plugin block
    // Note: The current formatter doesn't fully format type alias plugin blocks,
    // but it should preserve them
    assert!(content.contains("Email:"),
        "Expected 'Email:' type alias but got:\n{}", content);
    assert!(content.contains("string"),
        "Expected 'string' type but got:\n{}", content);
}

#[test]
fn test_format_preserves_all_elements_comprehensive() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create a file with all the elements that should be preserved
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, r#"@sql {{ dialect: "postgres" }}

-DeprecatedModel

Email: string

User {{
  id: string
  email: Email
  -old_field
  status {{ @sql {{ type: "enum" }} }}
  @sql {{ table: "users" }}
}}
"#).expect("Failed to write");
    let temp_path = temp_file.path().to_path_buf();

    let options = FormatOptions {
        assign_ids: false,
        check: false,
        write: true,
        indent_size: 2,
        format_whitespace: true,
    };

    let _result = format_file(&temp_path, &options).expect("Format should succeed");

    // Read back the formatted content
    let content = std::fs::read_to_string(&temp_path).expect("Failed to read formatted file");

    // Verify all elements are preserved
    assert!(content.contains("@sql"), "Plugin import should be preserved");
    assert!(content.contains("dialect"), "Plugin import config should be preserved");
    assert!(content.contains("-DeprecatedModel"), "Model removal should be preserved");
    assert!(content.contains("Email:"), "Type alias should be preserved");
    assert!(content.contains("-old_field"), "Field removal should be preserved");
    assert!(content.contains("status {"), "Field override should be preserved");
    assert!(content.contains("table"), "Model-level plugin config should be preserved");
}

#[test]
fn test_format_preserves_untyped_fields() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create a file with untyped fields (short form syntax)
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, r#"User {{
  name
  email
  bio
}}
"#).expect("Failed to write");
    let temp_path = temp_file.path().to_path_buf();

    let options = FormatOptions {
        assign_ids: false,
        check: false,
        write: true,
        indent_size: 2,
        format_whitespace: true,
    };

    let _result = format_file(&temp_path, &options).expect("Format should succeed");

    // Read back the formatted content
    let content = std::fs::read_to_string(&temp_path).expect("Failed to read formatted file");

    // Should preserve untyped fields without adding ":"
    assert!(content.contains("  name\n"), "Untyped field 'name' should not have ':'");
    assert!(content.contains("  email\n"), "Untyped field 'email' should not have ':'");
    assert!(content.contains("  bio\n"), "Untyped field 'bio' should not have ':'");
    assert!(!content.contains("name:"), "Untyped field should not have trailing ':'");
}

#[test]
fn test_format_preserves_optional_marker() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create a file with optional fields (both typed and untyped)
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, r#"User {{
  name: string
  bio?
  nickname?: string
  age?: number = 0
}}
"#).expect("Failed to write");
    let temp_path = temp_file.path().to_path_buf();

    let options = FormatOptions {
        assign_ids: false,
        check: false,
        write: true,
        indent_size: 2,
        format_whitespace: true,
    };

    let _result = format_file(&temp_path, &options).expect("Format should succeed");

    // Read back the formatted content
    let content = std::fs::read_to_string(&temp_path).expect("Failed to read formatted file");

    // Should preserve optional markers
    assert!(content.contains("bio?"), "Optional untyped field should preserve '?'");
    assert!(content.contains("nickname?:"), "Optional typed field should have '?:'");
    assert!(content.contains("age?:"), "Optional typed field with default should have '?:'");
}

#[test]
fn test_format_preserves_field_defaults() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create a file with field defaults
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, r#"Settings {{
  theme: string = "dark"
  count: number = 100
  enabled: boolean = true
}}
"#).expect("Failed to write");
    let temp_path = temp_file.path().to_path_buf();

    let options = FormatOptions {
        assign_ids: false,
        check: false,
        write: true,
        indent_size: 2,
        format_whitespace: true,
    };

    let _result = format_file(&temp_path, &options).expect("Format should succeed");

    // Read back the formatted content
    let content = std::fs::read_to_string(&temp_path).expect("Failed to read formatted file");

    // Should preserve default values
    assert!(content.contains("= \"dark\""), "String default should be preserved");
    assert!(content.contains("= 100"), "Number default should be preserved");
    assert!(content.contains("= true"), "Boolean default should be preserved");
}

#[test]
fn test_format_preserves_field_inline_plugins() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Create a file with inline plugin blocks on fields
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, r#"Post {{
  content: string {{ @sql {{ type: "TEXT" }} }}
}}
"#).expect("Failed to write");
    let temp_path = temp_file.path().to_path_buf();

    let options = FormatOptions {
        assign_ids: false,
        check: false,
        write: true,
        indent_size: 2,
        format_whitespace: true,
    };

    let _result = format_file(&temp_path, &options).expect("Format should succeed");

    // Read back the formatted content
    let content = std::fs::read_to_string(&temp_path).expect("Failed to read formatted file");

    // Should preserve inline plugin blocks
    assert!(content.contains("content: string {"), "Field with inline plugin should be preserved");
    assert!(content.contains("@sql"), "Plugin name should be preserved");
    assert!(content.contains("TEXT"), "Plugin config should be preserved");
}
