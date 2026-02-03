use super::*;
use cdm_plugin_interface::TypeExpression;
use crate::{ParsedType, PrimitiveType, PluginImport, PluginSource, convert_type_expression};
use std::fs;
use std::path::PathBuf;

fn fixtures_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_fixtures")
        .join("build")
}

fn test_output_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_output")
}

fn test_span() -> cdm_utils::Span {
    cdm_utils::Span {
        start: cdm_utils::Position { line: 0, column: 0 },
        end: cdm_utils::Position { line: 0, column: 0 },
    }
}

#[test]
fn test_convert_type_expression_primitives() {
    let string_type = ParsedType::Primitive(PrimitiveType::String);
    let result = convert_type_expression(&string_type);
    assert!(matches!(result, TypeExpression::Identifier { name } if name == "string"));

    let number_type = ParsedType::Primitive(PrimitiveType::Number);
    let result = convert_type_expression(&number_type);
    assert!(matches!(result, TypeExpression::Identifier { name } if name == "number"));

    let boolean_type = ParsedType::Primitive(PrimitiveType::Boolean);
    let result = convert_type_expression(&boolean_type);
    assert!(matches!(result, TypeExpression::Identifier { name } if name == "boolean"));
}

#[test]
fn test_convert_type_expression_reference() {
    let ref_type = ParsedType::Reference("User".to_string());
    let result = convert_type_expression(&ref_type);
    assert!(matches!(result, TypeExpression::Identifier { name } if name == "User"));
}

#[test]
fn test_convert_type_expression_array() {
    let array_type = ParsedType::Array(Box::new(ParsedType::Primitive(PrimitiveType::String)));
    let result = convert_type_expression(&array_type);

    match result {
        TypeExpression::Array { element_type } => {
            assert!(matches!(*element_type, TypeExpression::Identifier { name } if name == "string"));
        }
        _ => panic!("Expected Array type expression"),
    }
}

#[test]
fn test_convert_type_expression_nested_array() {
    // string[][]
    let nested_array = ParsedType::Array(Box::new(
        ParsedType::Array(Box::new(ParsedType::Primitive(PrimitiveType::String)))
    ));
    let result = convert_type_expression(&nested_array);

    match result {
        TypeExpression::Array { element_type } => {
            match *element_type {
                TypeExpression::Array { element_type: inner } => {
                    assert!(matches!(*inner, TypeExpression::Identifier { name } if name == "string"));
                }
                _ => panic!("Expected nested Array"),
            }
        }
        _ => panic!("Expected Array type expression"),
    }
}

#[test]
fn test_convert_type_expression_union() {
    let union_type = ParsedType::Union(vec![
        ParsedType::Primitive(PrimitiveType::String),
        ParsedType::Primitive(PrimitiveType::Number),
    ]);
    let result = convert_type_expression(&union_type);

    match result {
        TypeExpression::Union { types } => {
            assert_eq!(types.len(), 2);
            assert!(matches!(&types[0], TypeExpression::Identifier { name } if name == "string"));
            assert!(matches!(&types[1], TypeExpression::Identifier { name } if name == "number"));
        }
        _ => panic!("Expected Union type expression"),
    }
}

#[test]
fn test_convert_type_expression_string_literal() {
    let literal_type = ParsedType::Literal("active".to_string());
    let result = convert_type_expression(&literal_type);
    assert!(matches!(result, TypeExpression::StringLiteral { value } if value == "active"));
}

#[test]
fn test_convert_type_expression_null() {
    let null_type = ParsedType::Null;
    let result = convert_type_expression(&null_type);
    assert!(matches!(result, TypeExpression::Identifier { name } if name == "null"));
}

#[test]
fn test_convert_type_expression_complex_union() {
    // "active" | "inactive" | null
    let union_type = ParsedType::Union(vec![
        ParsedType::Literal("active".to_string()),
        ParsedType::Literal("inactive".to_string()),
        ParsedType::Null,
    ]);
    let result = convert_type_expression(&union_type);

    match result {
        TypeExpression::Union { types } => {
            assert_eq!(types.len(), 3);
            assert!(matches!(&types[0], TypeExpression::StringLiteral { value } if value == "active"));
            assert!(matches!(&types[1], TypeExpression::StringLiteral { value } if value == "inactive"));
            assert!(matches!(&types[2], TypeExpression::Identifier { name } if name == "null"));
        }
        _ => panic!("Expected Union type expression"),
    }
}

#[test]
fn test_convert_type_expression_array_of_references() {
    // User[]
    let array_type = ParsedType::Array(Box::new(ParsedType::Reference("User".to_string())));
    let result = convert_type_expression(&array_type);

    match result {
        TypeExpression::Array { element_type } => {
            assert!(matches!(*element_type, TypeExpression::Identifier { name } if name == "User"));
        }
        _ => panic!("Expected Array type expression"),
    }
}

#[test]
fn test_write_output_files_single_file() {
    let output_dir = test_output_path().join("single_file");
    let _ = fs::remove_dir_all(&output_dir); // Clean up from previous runs
    let file_path = output_dir.join("output.txt");

    let files = vec![OutputFile {
        path: file_path.to_string_lossy().to_string(),
        content: "test content".to_string(),
    }];

    // Use current dir as source_dir for absolute path test
    let result = write_output_files(&files, Path::new("."));
    assert!(result.is_ok());

    let content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "test content");
}

#[test]
fn test_write_output_files_creates_directories() {
    let output_dir = test_output_path().join("nested_dirs");
    let _ = fs::remove_dir_all(&output_dir); // Clean up from previous runs
    let file_path = output_dir.join("nested").join("dir").join("output.txt");

    let files = vec![OutputFile {
        path: file_path.to_string_lossy().to_string(),
        content: "nested content".to_string(),
    }];

    // Use current dir as source_dir for absolute path test
    let result = write_output_files(&files, Path::new("."));
    assert!(result.is_ok());

    assert!(file_path.exists());
    let content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "nested content");
}

#[test]
fn test_write_output_files_multiple_files() {
    let output_dir = test_output_path().join("multiple_files");
    let _ = fs::remove_dir_all(&output_dir); // Clean up from previous runs
    let file1 = output_dir.join("file1.txt");
    let file2 = output_dir.join("file2.txt");

    let files = vec![
        OutputFile {
            path: file1.to_string_lossy().to_string(),
            content: "content 1".to_string(),
        },
        OutputFile {
            path: file2.to_string_lossy().to_string(),
            content: "content 2".to_string(),
        },
    ];

    // Use current dir as source_dir for absolute path test
    let result = write_output_files(&files, Path::new("."));
    assert!(result.is_ok());

    assert_eq!(fs::read_to_string(&file1).unwrap(), "content 1");
    assert_eq!(fs::read_to_string(&file2).unwrap(), "content 2");
}

#[test]
fn test_write_output_files_empty_list() {
    let files: Vec<OutputFile> = vec![];
    let result = write_output_files(&files, Path::new("."));
    assert!(result.is_ok());
}

#[test]
fn test_write_output_files_relative_paths() {
    // Test that relative paths are resolved relative to source_dir
    let source_dir = test_output_path().join("relative_test");
    let _ = fs::create_dir_all(&source_dir);

    let files = vec![
        OutputFile {
            path: "output.txt".to_string(),  // Relative path
            content: "relative content".to_string(),
        },
        OutputFile {
            path: "build/types.ts".to_string(),  // Relative path with subdirectory
            content: "typescript content".to_string(),
        },
    ];

    let result = write_output_files(&files, &source_dir);
    assert!(result.is_ok());

    // Verify files were written relative to source_dir
    let output_file = source_dir.join("output.txt");
    let types_file = source_dir.join("build/types.ts");

    assert!(output_file.exists(), "output.txt should exist at {}", output_file.display());
    assert!(types_file.exists(), "build/types.ts should exist at {}", types_file.display());

    assert_eq!(fs::read_to_string(&output_file).unwrap(), "relative content");
    assert_eq!(fs::read_to_string(&types_file).unwrap(), "typescript content");

    // Clean up
    let _ = fs::remove_dir_all(&source_dir);
}

#[test]
fn test_resolve_plugin_path_with_cdm_plugin_json() {
    use tempfile::TempDir;

    // Create a temporary plugin directory with cdm-plugin.json
    let temp_dir = TempDir::new().unwrap();
    let plugin_dir = temp_dir.path().join("test-plugin");
    fs::create_dir(&plugin_dir).unwrap();

    // Create cdm-plugin.json
    let manifest = serde_json::json!({
        "name": "test-plugin",
        "wasm": {
            "file": "test-plugin.wasm"
        }
    });
    fs::write(
        plugin_dir.join("cdm-plugin.json"),
        serde_json::to_string_pretty(&manifest).unwrap()
    ).unwrap();

    // Create dummy WASM file
    fs::write(plugin_dir.join("test-plugin.wasm"), b"wasm").unwrap();

    let source_file = temp_dir.path().join("schema.cdm");
    fs::write(&source_file, "").unwrap();

    let import = PluginImport {
        name: "test-plugin".to_string(),
        source: Some(PluginSource::Path {
            path: "test-plugin".to_string(),
        }),
        source_file: source_file.clone(),
        global_config: None,
        span: test_span(),
        name_span: test_span(),
    };

    let result = crate::plugin_resolver::resolve_plugin_path(&import);
    assert!(result.is_ok(), "Should resolve plugin path: {:?}", result.err());
    assert_eq!(result.unwrap(), plugin_dir.join("test-plugin.wasm"));
}

#[test]
fn test_resolve_plugin_path_missing_cdm_plugin_json() {
    use tempfile::TempDir;

    // Create a temporary plugin directory WITHOUT cdm-plugin.json
    let temp_dir = TempDir::new().unwrap();
    let plugin_dir = temp_dir.path().join("test-plugin");
    fs::create_dir(&plugin_dir).unwrap();

    let source_file = temp_dir.path().join("schema.cdm");
    fs::write(&source_file, "").unwrap();

    let import = PluginImport {
        name: "test-plugin".to_string(),
        source: Some(PluginSource::Path {
            path: "test-plugin".to_string(),
        }),
        source_file: source_file.clone(),
        global_config: None,
        span: test_span(),
        name_span: test_span(),
    };

    let result = crate::plugin_resolver::resolve_plugin_path(&import);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cdm-plugin.json"));
}

#[test]
fn test_resolve_plugin_path_file_not_found() {
    let fixtures = fixtures_path();
    let source_file = fixtures.join("schema.cdm");

    let import = PluginImport {
        name: "nonexistent".to_string(),
        source: Some(PluginSource::Path {
            path: "nonexistent".to_string(),
        }),
        source_file: source_file.clone(),
        global_config: None,
        span: test_span(),
        name_span: test_span(),
    };

    let result = crate::plugin_resolver::resolve_plugin_path(&import);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cdm-plugin.json"));
}

#[test]
#[serial_test::serial]
fn test_resolve_plugin_path_registry_plugin() {
    // This test verifies that a plugin can be resolved from the registry
    // It uses the real typescript plugin from the registry
    let source_file = PathBuf::from("test.cdm");

    let import = PluginImport {
        name: "typescript".to_string(),
        source: None, // No source = try local, then registry
        global_config: Some(serde_json::json!({
            "version": "0.1.0"
        })),
        source_file: source_file.clone(),
        span: test_span(),
        name_span: test_span(),
    };

    let result = crate::plugin_resolver::resolve_plugin_path(&import);

    // Should succeed - will download from registry if not cached
    assert!(
        result.is_ok(),
        "Registry plugin resolution should succeed: {:?}",
        result.err()
    );

    let wasm_path = result.unwrap();
    assert!(
        wasm_path.exists(),
        "Resolved WASM file should exist: {}",
        wasm_path.display()
    );

    // Verify it's in the cache directory (platform-specific location with "plugins/typescript")
    let path_str = wasm_path.to_string_lossy();
    assert!(
        path_str.contains("plugins/typescript"),
        "Plugin should be cached in plugins/typescript directory, got: {}",
        path_str
    );
}

#[test]
#[serial_test::serial]
fn test_resolve_plugin_path_registry_plugin_cached() {
    // This test verifies that cached plugins are reused
    // First resolution will download (if needed), second should use cache
    let source_file = PathBuf::from("test.cdm");

    let import = PluginImport {
        name: "typescript".to_string(),
        source: None,
        global_config: Some(serde_json::json!({
            "version": "0.1.0"
        })),
        source_file: source_file.clone(),
        span: test_span(),
        name_span: test_span(),
    };

    // First resolution
    let result1 = crate::plugin_resolver::resolve_plugin_path(&import);
    assert!(result1.is_ok(), "First resolution should succeed");
    let path1 = result1.unwrap();

    // Second resolution should return the same cached path
    let result2 = crate::plugin_resolver::resolve_plugin_path(&import);
    assert!(result2.is_ok(), "Second resolution should succeed");
    let path2 = result2.unwrap();

    assert_eq!(path1, path2, "Cached plugin should return same path");
    assert!(path1.exists(), "Cached plugin file should exist");
}

#[test]
fn test_resolve_plugin_path_default_not_found() {
    let fixtures = fixtures_path();
    let source_file = fixtures.join("schema.cdm");

    let import = PluginImport {
        name: "nonexistent-plugin-12345".to_string(),
        source: None,
        global_config: None,
        source_file: source_file.clone(),
        span: test_span(),
        name_span: test_span(),
    };

    let result = crate::plugin_resolver::resolve_plugin_path(&import);
    assert!(result.is_err());
    // Should fail because plugin doesn't exist locally or in registry
}

#[test]
fn test_load_plugin_nonexistent_file() {
    let fixtures = fixtures_path();
    let source_file = fixtures.join("schema.cdm");

    let import = PluginImport {
        name: "test".to_string(),
        source: Some(PluginSource::Path {
            path: "nonexistent.wasm".to_string(),
        }),
        source_file: source_file.clone(),
        global_config: None,
        span: test_span(),
        name_span: test_span(),
    };

    let result = load_plugin(&import);
    assert!(result.is_err());
}

#[test]
fn test_plugin_configs_flow_through_system() {
    // Test that plugin configs are properly extracted and stored
    let source = r#"
        @sql { dialect: "postgres" }
        @validation

        User {
            id: number
            email: string {
                @sql { type: "VARCHAR(320)" }
                @validation { format: "email" }
            }

            @sql { table: "users" }
            @validation { unique: ["email"] }
        }

        Status: "active" | "pending" {
            @sql { type: "ENUM" }
        }
    "#;

    // Validate and extract configs
    let result = crate::validate(source, &[]);
    assert!(!result.has_errors(), "Validation should succeed");

    // Build resolved schema
    let resolved = crate::build_resolved_schema(
        &result.symbol_table,
        &result.model_fields,
        &[],
        &result.removal_names,
        &result.field_removals,
    );

    // Check model configs
    let user_model = resolved.models.get("User").expect("User model should exist");
    assert_eq!(user_model.plugin_configs.len(), 2, "User should have 2 plugin configs");
    assert!(user_model.plugin_configs.contains_key("sql"));
    assert!(user_model.plugin_configs.contains_key("validation"));

    // Check field configs
    let email_field = user_model.fields.iter()
        .find(|f| f.name == "email")
        .expect("email field should exist");
    assert_eq!(email_field.plugin_configs.len(), 2, "email field should have 2 plugin configs");
    assert!(email_field.plugin_configs.contains_key("sql"));
    assert!(email_field.plugin_configs.contains_key("validation"));

    // Check type alias configs
    let status_alias = resolved.type_aliases.get("Status").expect("Status type alias should exist");
    assert_eq!(status_alias.plugin_configs.len(), 1, "Status should have 1 plugin config");
    assert!(status_alias.plugin_configs.contains_key("sql"));
}

#[test]
fn test_plugin_configs_passed_to_specific_plugin() {
    // Regression test for Bug #1: Verify that model/field plugin configs
    // are correctly filtered and passed to each individual plugin.
    // This test mimics the exact scenario from BUGS.md
    let source = r#"
        @typescript { file_strategy: "per_model" }

        User {
            id: string #1
            name: string #2

            @typescript {
                file_name: "models/User.ts",
                readonly: true
            }
        } #10

        Post {
            title: string #1
            content: string {
                @typescript { type_override: "string | null" }
            } #2

            @typescript { file_name: "models/Post.ts" }
        } #11
    "#;

    // Validate and extract configs
    let result = crate::validate(source, &[]);
    assert!(!result.has_errors(), "Validation should succeed");

    // Build resolved schema to verify extraction worked
    let resolved = crate::build_resolved_schema(
        &result.symbol_table,
        &result.model_fields,
        &[],
        &result.removal_names,
        &result.field_removals,
    );

    // Verify User model has typescript config
    let user_model = resolved.models.get("User").expect("User model should exist");
    assert!(user_model.plugin_configs.contains_key("typescript"),
        "User model should have typescript config");
    let user_ts_config = &user_model.plugin_configs["typescript"];
    assert_eq!(user_ts_config.get("file_name").and_then(|v| v.as_str()), Some("models/User.ts"));
    assert_eq!(user_ts_config.get("readonly").and_then(|v| v.as_bool()), Some(true));

    // Verify Post model has typescript config
    let post_model = resolved.models.get("Post").expect("Post model should exist");
    assert!(post_model.plugin_configs.contains_key("typescript"),
        "Post model should have typescript config");
    let post_ts_config = &post_model.plugin_configs["typescript"];
    assert_eq!(post_ts_config.get("file_name").and_then(|v| v.as_str()), Some("models/Post.ts"));

    // Verify Post content field has typescript config
    let content_field = post_model.fields.iter()
        .find(|f| f.name == "content")
        .expect("content field should exist");
    assert!(content_field.plugin_configs.contains_key("typescript"),
        "content field should have typescript config");
    let content_ts_config = &content_field.plugin_configs["typescript"];
    assert_eq!(content_ts_config.get("type_override").and_then(|v| v.as_str()),
        Some("string | null"));

    // Now test that build_cdm_schema_for_plugin correctly filters configs per plugin
    use crate::plugin_validation::extract_plugin_imports;

    let parsed_tree = result.tree.as_ref().expect("Tree should exist");
    let _plugin_imports = extract_plugin_imports(parsed_tree.root_node(), source, Path::new("test.cdm"));

    // Build schema for typescript plugin specifically
    let plugin_schema = build_cdm_schema_for_plugin(&result, &[], "typescript")
        .expect("Should build schema for typescript plugin");

    // Verify User model config is passed
    let user_in_schema = plugin_schema.models.get("User").expect("User should be in schema");
    let user_config_obj = user_in_schema.config.as_object().expect("Config should be object");
    assert_eq!(user_config_obj.get("file_name").and_then(|v| v.as_str()), Some("models/User.ts"),
        "User model config should contain file_name");
    assert_eq!(user_config_obj.get("readonly").and_then(|v| v.as_bool()), Some(true),
        "User model config should contain readonly");

    // Verify Post model config is passed
    let post_in_schema = plugin_schema.models.get("Post").expect("Post should be in schema");
    let post_config_obj = post_in_schema.config.as_object().expect("Config should be object");
    assert_eq!(post_config_obj.get("file_name").and_then(|v| v.as_str()), Some("models/Post.ts"),
        "Post model config should contain file_name");

    // Verify Post content field config is passed
    let content_in_schema = post_in_schema.fields.iter()
        .find(|f| f.name == "content")
        .expect("content field should exist");
    let content_config_obj = content_in_schema.config.as_object().expect("Config should be object");
    assert_eq!(content_config_obj.get("type_override").and_then(|v| v.as_str()),
        Some("string | null"),
        "content field config should contain type_override");

    // Verify that a different plugin would get empty configs
    let other_plugin_schema = build_cdm_schema_for_plugin(&result, &[], "nonexistent")
        .expect("Should build schema for nonexistent plugin");

    let user_other = other_plugin_schema.models.get("User").expect("User should exist");
    assert_eq!(user_other.config.as_object().unwrap().len(), 0,
        "Configs for nonexistent plugin should be empty");

    // BUG CHECK: The test above passes, which means configs ARE in the resolved schema
    // and ARE being filtered correctly. So if the e2e test fails, the bug is elsewhere.
    // Log this for debugging
    eprintln!("DEBUG: Model configs in resolved schema exist and are being passed correctly");
}

#[test]
fn test_model_config_inherited_from_parent() {
    // BUG TEST: When a model extends another model, the child should inherit
    // the parent's model-level plugin config (like @sql { indexes: {...} }).
    //
    // Per spec Section 6.5:
    //   "Model-level config: Child's config merges with parent's config"
    //
    // Per spec Section 7.4 merge rules:
    //   - Objects: Deep merge (recursive)
    //   - Arrays: Replace entirely
    //   - Primitives: Replace entirely
    //
    // Using keyed object format for indexes enables proper inheritance via object merge.
    let source = r#"
        @sql

        Entity {
            id: string #1
            @sql {
                indexes: {
                    primary: { fields: ["id"], primary: true }
                }
            }
        } #1

        User extends Entity {
            name: string #2
        } #2
    "#;

    let result = crate::validate(source, &[]);
    assert!(!result.has_errors(), "Validation should succeed: {:?}", result.diagnostics);

    // Build schema for sql plugin
    let plugin_schema = build_cdm_schema_for_plugin(&result, &[], "sql")
        .expect("Should build schema for sql plugin");

    // Entity should have its own indexes
    let entity_in_schema = plugin_schema.models.get("Entity").expect("Entity should be in schema");
    let entity_config = entity_in_schema.config.as_object().expect("Config should be object");
    let entity_indexes = entity_config.get("indexes").expect("Entity should have indexes");
    assert!(entity_indexes.is_object(), "indexes should be object (keyed format)");
    assert_eq!(entity_indexes.as_object().unwrap().len(), 1, "Entity should have 1 index");

    // User extends Entity - it should inherit Entity's indexes
    let user_in_schema = plugin_schema.models.get("User").expect("User should be in schema");
    let user_config = user_in_schema.config.as_object().expect("Config should be object");

    let user_indexes = user_config.get("indexes")
        .expect("User should inherit indexes from Entity");
    assert!(user_indexes.is_object(), "indexes should be object (keyed format)");
    assert_eq!(user_indexes.as_object().unwrap().len(), 1,
        "User should inherit 1 index from Entity");

    // Check the inherited index has the correct content
    let index = user_indexes.as_object().unwrap().get("primary")
        .expect("User should have 'primary' index");
    assert_eq!(
        index.get("primary").and_then(|v| v.as_bool()),
        Some(true),
        "Inherited index should have primary: true"
    );
}

#[test]
fn test_model_config_child_indexes_merge_with_parent() {
    // When child defines its own indexes (keyed object format), they should MERGE with parent's
    // indexes via standard object deep merge.
    //
    // With keyed object format, child keys are merged with parent keys naturally.
    // This preserves inherited primary keys when child adds its own indexes.
    let source = r#"
        @sql

        Entity {
            id: string #1
            @sql {
                indexes: {
                    primary: { fields: ["id"], primary: true }
                }
            }
        } #1

        User extends Entity {
            email: string #2
            @sql {
                indexes: {
                    email_unique: { fields: ["email"], unique: true }
                }
            }
        } #2
    "#;

    let result = crate::validate(source, &[]);
    assert!(!result.has_errors(), "Validation should succeed");

    let plugin_schema = build_cdm_schema_for_plugin(&result, &[], "sql")
        .expect("Should build schema for sql plugin");

    let user_in_schema = plugin_schema.models.get("User").expect("User should be in schema");
    let user_config = user_in_schema.config.as_object().expect("Config should be object");
    let user_indexes = user_config.get("indexes").expect("User should have indexes");

    // Indexes should be a keyed object with both parent and child indexes
    let indexes_obj = user_indexes.as_object().expect("indexes should be an object");
    assert_eq!(indexes_obj.len(), 2,
        "User should have 2 indexes (parent's 'primary' + own 'email_unique')");

    // Should have the parent's "primary" index
    assert!(indexes_obj.contains_key("primary"),
        "User should have inherited 'primary' index from Entity");
    let primary_idx = indexes_obj.get("primary").unwrap();
    assert_eq!(primary_idx.get("primary").and_then(|v| v.as_bool()), Some(true),
        "'primary' index should have primary: true");

    // Should have child's "email_unique" index
    assert!(indexes_obj.contains_key("email_unique"),
        "User should have its own 'email_unique' index");
    let email_idx = indexes_obj.get("email_unique").unwrap();
    assert_eq!(email_idx.get("unique").and_then(|v| v.as_bool()), Some(true),
        "'email_unique' index should have unique: true");
}

#[test]
fn test_model_config_deep_merge_objects() {
    // Objects should be deep merged (child adds to parent's object properties)
    let source = r#"
        @sql

        Entity {
            id: string #1
            @sql {
                naming: { table: "entity_table" },
                some_flag: true
            }
        } #1

        User extends Entity {
            name: string #2
            @sql {
                naming: { columns: "snake_case" }
            }
        } #2
    "#;

    let result = crate::validate(source, &[]);
    assert!(!result.has_errors(), "Validation should succeed");

    let plugin_schema = build_cdm_schema_for_plugin(&result, &[], "sql")
        .expect("Should build schema for sql plugin");

    let user_in_schema = plugin_schema.models.get("User").expect("User should be in schema");
    let user_config = user_in_schema.config.as_object().expect("Config should be object");

    // some_flag should be inherited from Entity
    assert_eq!(
        user_config.get("some_flag").and_then(|v| v.as_bool()),
        Some(true),
        "User should inherit some_flag from Entity"
    );

    // naming should be deep merged
    let naming = user_config.get("naming").expect("User should have naming config");
    let naming_obj = naming.as_object().expect("naming should be object");

    // Child's naming.columns should exist
    assert_eq!(
        naming_obj.get("columns").and_then(|v| v.as_str()),
        Some("snake_case"),
        "User should have its own naming.columns"
    );

    // Parent's naming.table should be inherited
    assert_eq!(
        naming_obj.get("table").and_then(|v| v.as_str()),
        Some("entity_table"),
        "User should inherit naming.table from Entity"
    );
}

#[test]
fn test_model_config_multi_level_inheritance() {
    // Test inheritance through multiple levels: GrandChild extends Child extends Parent
    let source = r#"
        @sql

        Entity {
            id: string #1
            @sql {
                indexes: {
                    primary: { fields: ["id"], primary: true }
                }
            }
        } #1

        Timestamped {
            created_at: string #1
        } #2

        TimestampedEntity extends Entity, Timestamped {
        } #3

        User extends TimestampedEntity {
            name: string #2
        } #4
    "#;

    let result = crate::validate(source, &[]);
    assert!(!result.has_errors(), "Validation should succeed");

    let plugin_schema = build_cdm_schema_for_plugin(&result, &[], "sql")
        .expect("Should build schema for sql plugin");

    // TimestampedEntity should inherit Entity's indexes
    let tse = plugin_schema.models.get("TimestampedEntity").expect("TimestampedEntity should exist");
    let tse_config = tse.config.as_object().expect("Config should be object");
    assert!(tse_config.get("indexes").is_some(),
        "TimestampedEntity should inherit indexes from Entity");

    // User should also inherit through the chain
    let user = plugin_schema.models.get("User").expect("User should exist");
    let user_config = user.config.as_object().expect("Config should be object");
    assert!(user_config.get("indexes").is_some(),
        "User should inherit indexes from Entity through TimestampedEntity");
}

#[test]
fn test_local_type_alias_config_inherited_by_field() {
    // Per spec Section 4.4:
    //   "When a type alias is used in a field, the field inherits the alias's plugin configuration"
    //   "Field-level plugin configuration merges with (and can override) alias-level configuration"
    //
    // This tests LOCAL type aliases (not template/qualified types like sqlType.UUID)
    let source = r#"
        @sql

        Email: string {
            @sql { type: "VARCHAR(320)" }
        } #1

        User {
            email: Email #1
        } #2
    "#;

    let result = crate::validate(source, &[]);
    assert!(!result.has_errors(), "Validation should succeed: {:?}", result.diagnostics);

    let plugin_schema = build_cdm_schema_for_plugin(&result, &[], "sql")
        .expect("Should build schema for sql plugin");

    let user = plugin_schema.models.get("User").expect("User should exist");
    let email_field = user.fields.iter().find(|f| f.name == "email")
        .expect("email field should exist");

    let email_config = email_field.config.as_object().expect("Config should be object");

    // BUG: Currently this fails because local type alias configs aren't inherited
    assert_eq!(
        email_config.get("type").and_then(|v| v.as_str()),
        Some("VARCHAR(320)"),
        "email field should inherit @sql {{ type }} from Email type alias"
    );
}

#[test]
fn test_local_type_alias_config_merged_with_field_config() {
    // Field config should override type alias config (merge with field winning)
    let source = r#"
        @sql

        Email: string {
            @sql { type: "VARCHAR(320)", nullable: false }
        } #1

        User {
            email: Email {
                @sql { nullable: true }
            } #1
        } #2
    "#;

    let result = crate::validate(source, &[]);
    assert!(!result.has_errors(), "Validation should succeed");

    let plugin_schema = build_cdm_schema_for_plugin(&result, &[], "sql")
        .expect("Should build schema for sql plugin");

    let user = plugin_schema.models.get("User").expect("User should exist");
    let email_field = user.fields.iter().find(|f| f.name == "email")
        .expect("email field should exist");

    let email_config = email_field.config.as_object().expect("Config should be object");

    // Should inherit type from alias
    assert_eq!(
        email_config.get("type").and_then(|v| v.as_str()),
        Some("VARCHAR(320)"),
        "email field should inherit @sql {{ type }} from Email type alias"
    );

    // Should override nullable with field's value
    assert_eq!(
        email_config.get("nullable").and_then(|v| v.as_bool()),
        Some(true),
        "email field's nullable should override type alias's nullable"
    );
}

#[test]
fn test_template_type_aliases_not_in_plugin_schema() {
    // BUG FIX TEST: When a file imports a template namespace, the qualified
    // type aliases (e.g., "sql.UUID") should NOT appear in the plugin schema.
    // They are internal to CDM's resolution and should be filtered out.
    //
    // This test verifies that:
    // 1. Template type aliases are resolved correctly for field types
    // 2. Qualified type aliases are NOT passed to plugins

    use std::path::PathBuf;

    // Use the existing template test fixture
    let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_fixtures")
        .join("templates")
        .join("namespace_resolution");
    let test_file = fixtures_dir.join("basic_import.cdm");

    // Load and validate the file
    let tree = crate::FileResolver::load(&test_file).unwrap();
    let result = crate::validate_tree(tree).unwrap();
    assert!(!result.has_errors(), "Validation should succeed: {:?}", result.diagnostics);

    // Build schema for typescript plugin
    let plugin_schema = build_cdm_schema_for_plugin(&result, &[], "typescript")
        .expect("Should build schema for typescript plugin");

    // Verify that no type alias has a qualified name (contains a dot)
    for (name, _) in &plugin_schema.type_aliases {
        assert!(
            !name.contains('.'),
            "Plugin schema should not contain qualified type alias '{}'. \
             Template types should be filtered out before passing to plugins.",
            name
        );
    }

    // Verify the User model exists and has the expected fields
    let user = plugin_schema.models.get("User").expect("User should exist in schema");
    assert_eq!(user.fields.len(), 4, "User should have 4 fields");

    // Verify field types are resolved to base types (string), not qualified names
    for field in &user.fields {
        match &field.field_type {
            TypeExpression::Identifier { name } => {
                assert!(
                    !name.contains('.'),
                    "Field '{}' should have resolved type, not qualified '{}'. \
                     Template types should be resolved to their base types.",
                    field.name, name
                );
                assert_eq!(
                    name, "string",
                    "Field '{}' should be resolved to 'string' (the base type of all SQL types)",
                    field.name
                );
            }
            _ => panic!("Expected Identifier type for field {}", field.name),
        }
    }
}
