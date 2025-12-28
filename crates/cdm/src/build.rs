use crate::{FileResolver, PluginRunner, ValidationResult, build_cdm_schema_for_plugin};
use crate::plugin_validation::{extract_plugin_imports, PluginImport};
use anyhow::{Result, Context};
use cdm_plugin_interface::OutputFile;
use std::path::{Path, PathBuf};
use std::fs;

/// Build output files from a CDM schema using configured plugins
pub fn build(path: &Path) -> Result<()> {
    // Load and parse the CDM file tree
    let tree = FileResolver::load(path).map_err(|diagnostics| {
        for diagnostic in &diagnostics {
            eprintln!("{}", diagnostic);
        }
        anyhow::anyhow!("Failed to load CDM file")
    })?;

    // Extract data we need before consuming tree
    let main_path = tree.main.path.clone();
    let ancestors: Vec<_> = tree.ancestors.iter().map(|a| a.path.clone()).collect();

    // Validate the tree (consumes tree)
    let validation_result = crate::validate_tree(tree).map_err(|diagnostics| {
        for diagnostic in &diagnostics {
            eprintln!("{}", diagnostic);
        }
        anyhow::anyhow!("Validation failed")
    })?;

    // Check for validation errors
    let has_errors = validation_result
        .diagnostics
        .iter()
        .any(|d| d.severity == crate::Severity::Error);

    if has_errors {
        for diagnostic in &validation_result.diagnostics {
            if diagnostic.severity == crate::Severity::Error {
                eprintln!("{}", diagnostic);
            }
        }
        return Err(anyhow::anyhow!("Cannot build: validation errors found"));
    }

    // Step 1: Extract plugin imports
    let plugin_imports = extract_plugin_imports_from_tree(&validation_result, &main_path)?;

    if plugin_imports.is_empty() {
        println!("No plugins configured - nothing to build");
        return Ok(());
    }

    // Step 2: Process each plugin
    let mut all_output_files = Vec::new();

    // Get the source file directory for resolving relative output paths
    let source_dir = path.parent()
        .ok_or_else(|| anyhow::anyhow!("Source file has no parent directory"))?;

    for plugin_import in &plugin_imports {
        println!("Running plugin: {}", plugin_import.name);

        // Load the plugin
        let mut runner = load_plugin(plugin_import)?;

        // Check if plugin supports build operation
        match runner.has_build() {
            Ok(false) => {
                println!("  Skipped: Plugin '{}' does not support build", plugin_import.name);
                continue;
            }
            Err(e) => {
                eprintln!("  Warning: Failed to check build capability for plugin '{}': {}", plugin_import.name, e);
                continue;
            }
            Ok(true) => {
                // Plugin supports build, proceed
            }
        }

        // Get the plugin's global config (or empty JSON object)
        let global_config = plugin_import.global_config.clone()
            .unwrap_or(serde_json::json!({}));

        // Build schema with this plugin's configs extracted
        let plugin_schema = build_cdm_schema_for_plugin(
            &validation_result,
            &ancestors,
            &plugin_import.name
        )?;

        // Extract build_output from config (if specified)
        let build_output = global_config
            .get("build_output")
            .and_then(|v| v.as_str())
            .map(|s| PathBuf::from(s));

        // Call the plugin's build function
        match runner.build(plugin_schema, global_config) {
            Ok(mut output_files) => {
                println!("  Generated {} file(s)", output_files.len());

                // If build_output is specified, prepend it to all output file paths
                if let Some(ref build_dir) = build_output {
                    for file in &mut output_files {
                        let file_path = Path::new(&file.path);
                        // Only prepend if the path is relative
                        if file_path.is_relative() {
                            file.path = build_dir.join(file_path).to_string_lossy().to_string();
                        }
                    }
                }

                all_output_files.extend(output_files);
            }
            Err(e) => {
                eprintln!("  Warning: Plugin '{}' build failed: {}", plugin_import.name, e);
            }
        }
    }

    // Step 4: Write all output files (resolve relative paths from source directory)
    write_output_files(&all_output_files, source_dir)?;

    println!("\n✓ Build completed successfully");
    println!("  {} plugin(s) executed", plugin_imports.len());
    println!("  {} file(s) generated", all_output_files.len());

    Ok(())
}

/// Extract plugin imports from the validated tree using the shared function
fn extract_plugin_imports_from_tree(
    validation_result: &ValidationResult,
    main_path: &Path,
) -> Result<Vec<PluginImport>> {
    let parsed_tree = validation_result.tree.as_ref()
        .context("No parsed tree available")?;

    // We need to re-read the source file since tree was consumed
    let main_source = fs::read_to_string(main_path)
        .with_context(|| format!("Failed to read source file: {}", main_path.display()))?;

    let root = parsed_tree.root_node();
    Ok(extract_plugin_imports(root, &main_source, main_path))
}


/// Load a plugin from its import specification
fn load_plugin(import: &PluginImport) -> Result<PluginRunner> {
    let wasm_path = crate::plugin_resolver::resolve_plugin_path(import)?;
    PluginRunner::new(&wasm_path)
        .with_context(|| format!("Failed to load plugin '{}'", import.name))
}

/// Write output files to disk, resolving paths relative to source_dir
fn write_output_files(files: &[OutputFile], source_dir: &Path) -> Result<()> {
    for file in files {
        let file_path = Path::new(&file.path);

        // Resolve the path relative to the source directory
        let resolved_path = if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            source_dir.join(file_path)
        };

        // Create parent directories if needed
        if let Some(parent) = resolved_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        // Write the file
        fs::write(&resolved_path, &file.content)
            .with_context(|| format!("Failed to write file: {}", resolved_path.display()))?;

        println!("  Wrote: {}", resolved_path.display());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
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
    fn test_resolve_plugin_path_with_explicit_path() {
        let fixtures = fixtures_path();
        let plugin_file = fixtures.join("test-plugin.wasm");
        let source_file = fixtures.join("schema.cdm");

        let import = PluginImport {
            name: "test-plugin".to_string(),
            source: Some(PluginSource::Path {
                path: "test-plugin.wasm".to_string(),
            }),
            source_file: source_file.clone(),
            global_config: None,
            span: test_span(),
        };

        let result = crate::plugin_resolver::resolve_plugin_path(&import);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), plugin_file);
    }

    #[test]
    fn test_resolve_plugin_path_adds_wasm_extension() {
        let fixtures = fixtures_path();
        let plugin_file = fixtures.join("test-plugin.wasm");
        let source_file = fixtures.join("schema.cdm");

        let import = PluginImport {
            name: "test-plugin".to_string(),
            source: Some(PluginSource::Path {
                path: "test-plugin".to_string(), // No .wasm extension
            }),
            source_file: source_file.clone(),
            global_config: None,
            span: test_span(),
        };

        let result = crate::plugin_resolver::resolve_plugin_path(&import);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), plugin_file);
    }

    #[test]
    fn test_resolve_plugin_path_file_not_found() {
        let fixtures = fixtures_path();
        let source_file = fixtures.join("schema.cdm");

        let import = PluginImport {
            name: "nonexistent".to_string(),
            source: Some(PluginSource::Path {
                path: "nonexistent.wasm".to_string(),
            }),
            source_file: source_file.clone(),
            global_config: None,
            span: test_span(),
        };

        let result = crate::plugin_resolver::resolve_plugin_path(&import);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
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
            &[],
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
            &[],
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
}
