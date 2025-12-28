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
