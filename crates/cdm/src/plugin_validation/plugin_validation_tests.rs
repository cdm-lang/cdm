use crate::{validate_tree, FileResolver, LoadedFileTree, Diagnostic};
use crate::plugin_validation::extract_plugin_imports;
use std::path::PathBuf;

// Helper to get the path to test fixtures
fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_fixtures")
        .join("plugin_validation")
        .join(name)
}

// Helper to load a file tree
fn load_fixture(name: &str) -> Result<LoadedFileTree, Vec<Diagnostic>> {
    let path = fixture_path(name);
    FileResolver::load(path)
}

// Helper to check if the docs plugin WASM exists
fn docs_plugin_exists() -> bool {
    // The test fixtures use "../../../../crates/cdm-plugin-docs" as the plugin path
    // relative to the test_fixtures/plugin_validation directory.
    // So from CARGO_MANIFEST_DIR (crates/cdm), we need to go to ../../crates/cdm-plugin-docs
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/cdm-plugin-docs/target/wasm32-wasip1/release/cdm_plugin_docs.wasm")
        .exists()
}

#[test]
fn test_valid_plugin_configuration() {
    if !docs_plugin_exists() {
        eprintln!("Skipping test - cdm_plugin_docs.wasm not found");
        return;
    }

    let tree = load_fixture("valid_plugin.cdm").expect("Failed to load fixture");
    let result = validate_tree(tree);

    assert!(
        result.is_ok(),
        "Expected valid plugin configuration to pass, got: {:?}",
        result.err()
    );

    let validation_result = result.unwrap();
    assert!(
        !validation_result.has_errors(),
        "Expected no errors, got: {:?}",
        validation_result.diagnostics
    );
}

#[test]
fn test_invalid_format_fails_level_2_validation() {
    if !docs_plugin_exists() {
        eprintln!("Skipping test - cdm_plugin_docs.wasm not found");
        return;
    }

    let tree = load_fixture("invalid_format.cdm").expect("Failed to load fixture");
    let result = validate_tree(tree);

    assert!(result.is_err(), "Expected validation to fail");

    let diagnostics = result.unwrap_err();
    assert!(
        diagnostics.len() > 0,
        "Expected at least one error diagnostic"
    );

    // Should contain error about invalid format
    let has_format_error = diagnostics
        .iter()
        .any(|d| d.message.to_lowercase().contains("format"));

    assert!(
        has_format_error,
        "Expected error about invalid format, got: {:?}",
        diagnostics
    );
}

#[test]
fn test_plugin_not_imported_error() {
    let tree = load_fixture("plugin_not_imported.cdm").expect("Failed to load fixture");
    let result = validate_tree(tree);

    assert!(result.is_err(), "Expected validation to fail");

    let diagnostics = result.unwrap_err();

    // Should contain E402 error about plugin not imported
    let has_e402_error = diagnostics.iter().any(|d| {
        d.message.contains("E402") && d.message.to_lowercase().contains("not imported")
    });

    assert!(
        has_e402_error,
        "Expected E402 error about plugin not imported, got: {:?}",
        diagnostics
    );
}

#[test]
fn test_plugin_not_found_error() {
    let tree = load_fixture("plugin_not_found.cdm").expect("Failed to load fixture");
    let result = validate_tree(tree);

    assert!(result.is_err(), "Expected validation to fail");

    let diagnostics = result.unwrap_err();

    // Should contain E401 error about plugin not found
    let has_e401_error = diagnostics
        .iter()
        .any(|d| d.message.contains("E401") && d.message.to_lowercase().contains("not found"));

    assert!(
        has_e401_error,
        "Expected E401 error about plugin not found, got: {:?}",
        diagnostics
    );
}

#[test]
fn test_missing_required_field_fails_level_1_validation() {
    if !docs_plugin_exists() {
        eprintln!("Skipping test - cdm_plugin_docs.wasm not found");
        return;
    }

    // Note: The docs plugin schema has 'format' field with a default value of "markdown",
    // so it's implicitly optional. The missing_required_field.cdm fixture is actually
    // valid now with our updated behavior where fields with defaults are optional.
    let tree = load_fixture("missing_required_field.cdm").expect("Failed to load fixture");
    let result = validate_tree(tree);

    // This should now succeed because 'format' has a default value
    assert!(result.is_ok(), "Expected validation to succeed with defaults applied, got: {:?}", result);
}

#[test]
fn test_unknown_field_fails_level_1_validation() {
    if !docs_plugin_exists() {
        eprintln!("Skipping test - cdm_plugin_docs.wasm not found");
        return;
    }

    let tree = load_fixture("unknown_field.cdm").expect("Failed to load fixture");
    let result = validate_tree(tree);

    assert!(result.is_err(), "Expected validation to fail");

    let diagnostics = result.unwrap_err();

    // Should contain E402 error about unknown field
    let has_unknown_field_error = diagnostics
        .iter()
        .any(|d| d.message.contains("E402") && d.message.to_lowercase().contains("unknown"));

    assert!(
        has_unknown_field_error,
        "Expected E402 error about unknown field, got: {:?}",
        diagnostics
    );
}

#[test]
fn test_fail_fast_level_1_before_level_2() {
    if !docs_plugin_exists() {
        eprintln!("Skipping test - cdm_plugin_docs.wasm not found");
        return;
    }

    // This test verifies that if Level 1 validation fails, Level 2 is not run.
    // We need to use a fixture that has a truly invalid config (unknown field)
    // instead of missing_required_field which is now valid with defaults.
    let tree = load_fixture("unknown_field.cdm").expect("Failed to load fixture");
    let result = validate_tree(tree);

    assert!(result.is_err(), "Expected validation to fail");

    let diagnostics = result.unwrap_err();

    // Should ONLY have Level 1 errors (E402 with schema validation message)
    // Should NOT have Level 2 errors (plugin-specific validation errors)
    let all_are_level_1_errors = diagnostics.iter().all(|d| {
        // Level 1 errors are prefixed with E402
        d.message.contains("E402")
    });

    assert!(
        all_are_level_1_errors,
        "Expected only Level 1 (schema) errors, got: {:?}",
        diagnostics
    );
}

#[test]
fn test_no_plugins_no_validation() {
    // Create a simple CDM file without any plugin imports
    let cdm_content = r#"
User {
  id: number
  name: string
}
"#;

    // Write to a temporary file
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("no_plugins_test.cdm");
    std::fs::write(&temp_file, cdm_content).unwrap();

    let tree = FileResolver::load(&temp_file).expect("Failed to load file");
    let result = validate_tree(tree);

    // Should succeed - no plugins means no plugin validation
    assert!(
        result.is_ok(),
        "Expected validation to succeed without plugins, got: {:?}",
        result.err()
    );

    // Cleanup
    std::fs::remove_file(&temp_file).ok();
}

#[test]
fn test_model_level_plugin_config() {
    if !docs_plugin_exists() {
        eprintln!("Skipping test - cdm_plugin_docs.wasm not found");
        return;
    }

    let tree = load_fixture("model_config_test.cdm").expect("Failed to load fixture");
    let result = validate_tree(tree);

    assert!(
        result.is_ok(),
        "Expected valid model-level config to pass, got: {:?}",
        result.err()
    );
}

#[test]
fn test_field_level_plugin_config() {
    if !docs_plugin_exists() {
        eprintln!("Skipping test - cdm_plugin_docs.wasm not found");
        return;
    }

    let tree = load_fixture("field_config_test.cdm").expect("Failed to load fixture");
    let result = validate_tree(tree);

    assert!(
        result.is_ok(),
        "Expected valid field-level config to pass, got: {:?}",
        result.err()
    );
}

#[test]
fn test_plugin_imported_in_ancestor() {
    if !docs_plugin_exists() {
        eprintln!("Skipping test - cdm_plugin_docs.wasm not found");
        return;
    }

    let tree = load_fixture("ancestor_imports/child.cdm").expect("Failed to load fixture");
    let result = validate_tree(tree);

    assert!(
        result.is_ok(),
        "Expected plugin imported in ancestor to be recognized, got: {:?}",
        result.err()
    );

    let validation_result = result.unwrap();
    assert!(
        !validation_result.has_errors(),
        "Expected no errors when plugin imported in ancestor, got: {:?}",
        validation_result.diagnostics
    );
}

#[test]
fn test_missing_build_output_allowed() {
    // This test verifies that plugins can be imported without build_output/migrations_output.
    // When these are missing, the build/migrate phases will simply skip that plugin,
    // but validation should still pass so plugins can be used for configuration inheritance.
    if !docs_plugin_exists() {
        eprintln!("Skipping test - cdm_plugin_docs.wasm not found");
        return;
    }

    let tree = load_fixture("missing_build_output.cdm").expect("Failed to load fixture");
    let result = validate_tree(tree);

    assert!(
        result.is_ok(),
        "Expected validation to succeed without build_output (plugin will be skipped during build), got: {:?}",
        result.err()
    );

    let validation_result = result.unwrap();
    assert!(
        !validation_result.has_errors(),
        "Expected no errors when build_output is missing (plugin will be skipped during build), got: {:?}",
        validation_result.diagnostics
    );
}

#[test]
fn test_extract_plugin_imports_name_span() {
    // Test that name_span correctly points to just the plugin name, not the whole block
    let source = "@typescript { build_output: \"out.ts\" }\n";
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&grammar::LANGUAGE.into()).unwrap();
    let tree = parser.parse(source, None).unwrap();

    let imports = extract_plugin_imports(
        tree.root_node(),
        source,
        &PathBuf::from("/test/schema.cdm"),
    );

    assert_eq!(imports.len(), 1, "Expected 1 import");
    let import = &imports[0];

    // The full span should cover the entire plugin import
    // @typescript { build_output: "out.ts" }
    // ^                                    ^
    // 0                                   38
    assert_eq!(import.span.start.column, 0, "Full span should start at column 0");
    assert!(import.span.end.column > 10, "Full span should extend past the plugin name");

    // The name_span should only cover "typescript"
    // @typescript
    //  ^        ^
    //  1       11
    assert_eq!(import.name_span.start.column, 1, "Name span should start after @");
    assert_eq!(import.name_span.end.column, 11, "Name span should end after 'typescript'");
    assert_eq!(import.name, "typescript");
}

#[test]
fn test_extract_plugin_imports_name_span_simple() {
    // Test with a simple plugin import without config
    let source = "@sql\n";
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&grammar::LANGUAGE.into()).unwrap();
    let tree = parser.parse(source, None).unwrap();

    let imports = extract_plugin_imports(
        tree.root_node(),
        source,
        &PathBuf::from("/test/schema.cdm"),
    );

    assert_eq!(imports.len(), 1, "Expected 1 import");
    let import = &imports[0];

    // For simple imports, both spans should cover @sql
    // @sql
    //  ^  ^
    //  1  4
    assert_eq!(import.name_span.start.column, 1, "Name span should start after @");
    assert_eq!(import.name_span.end.column, 4, "Name span should end after 'sql'");
    assert_eq!(import.name, "sql");
}

#[test]
fn test_plugin_not_found_error_span_only_covers_name() {
    // This test verifies that E401 error span only covers the plugin name, not the config block
    let tree = load_fixture("plugin_not_found_with_config.cdm").expect("Failed to load fixture");
    let result = validate_tree(tree);

    assert!(result.is_err(), "Expected validation to fail");

    let diagnostics = result.unwrap_err();

    // Find the E401 error
    let e401_error = diagnostics.iter().find(|d| d.message.contains("E401"));
    assert!(e401_error.is_some(), "Expected E401 error, got: {:?}", diagnostics);

    let error = e401_error.unwrap();

    // The fixture file has:
    // @nonexistent { some_config: "value", another: 123 }
    //  ^         ^
    //  1        12
    // The error span should only cover "nonexistent", not the whole config block
    assert_eq!(error.span.start.line, 0, "Error should be on line 0");
    assert_eq!(error.span.start.column, 1, "Error span should start after @");
    assert_eq!(error.span.end.column, 12, "Error span should end after 'nonexistent', not extend to config block");
}
