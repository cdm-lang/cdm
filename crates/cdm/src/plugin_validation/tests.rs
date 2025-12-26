use crate::{validate_tree, FileResolver, LoadedFileTree, Diagnostic};
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
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/wasm32-wasip1/release/cdm_plugin_docs.wasm")
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

    let tree = load_fixture("missing_required_field.cdm").expect("Failed to load fixture");
    let result = validate_tree(tree);

    assert!(result.is_err(), "Expected validation to fail");

    let diagnostics = result.unwrap_err();

    // Should contain E402 error about missing required field
    let has_missing_field_error = diagnostics.iter().any(|d| {
        d.message.contains("E402")
            && (d.message.to_lowercase().contains("required")
                || d.message.to_lowercase().contains("missing"))
    });

    assert!(
        has_missing_field_error,
        "Expected E402 error about missing required field 'format', got: {:?}",
        diagnostics
    );
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
    // We use the missing_required_field case - if Level 2 ran, we'd get a different error.
    let tree = load_fixture("missing_required_field.cdm").expect("Failed to load fixture");
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

    let cdm_content = r#"
@docs from ../../../../target/wasm32-wasip1/release/cdm_plugin_docs.wasm {
  format: "markdown",
  build_output: "./docs"
}

User {
  @docs {
    description: "A user model",
    hidden: false
  }

  id: number
}
"#;

    let test_fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_fixtures/plugin_validation");
    let temp_file = test_fixtures_dir.join("model_config_test.cdm");
    std::fs::write(&temp_file, cdm_content).unwrap();

    let tree = FileResolver::load(&temp_file).expect("Failed to load file");
    let result = validate_tree(tree);

    assert!(
        result.is_ok(),
        "Expected valid model-level config to pass, got: {:?}",
        result.err()
    );

    // Cleanup
    std::fs::remove_file(&temp_file).ok();
}

#[test]
fn test_field_level_plugin_config() {
    if !docs_plugin_exists() {
        eprintln!("Skipping test - cdm_plugin_docs.wasm not found");
        return;
    }

    let cdm_content = r#"
@docs from ../../../../target/wasm32-wasip1/release/cdm_plugin_docs.wasm {
  format: "markdown",
  build_output: "./docs"
}

User {
  id: number {
    @docs {
      description: "User ID field",
      deprecated: true
    }
  }
}
"#;

    let test_fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_fixtures/plugin_validation");
    let temp_file = test_fixtures_dir.join("field_config_test.cdm");
    std::fs::write(&temp_file, cdm_content).unwrap();

    let tree = FileResolver::load(&temp_file).expect("Failed to load file");
    let result = validate_tree(tree);

    assert!(
        result.is_ok(),
        "Expected valid field-level config to pass, got: {:?}",
        result.err()
    );

    // Cleanup
    std::fs::remove_file(&temp_file).ok();
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
fn test_missing_build_output_error() {
    if !docs_plugin_exists() {
        eprintln!("Skipping test - cdm_plugin_docs.wasm not found");
        return;
    }

    let tree = load_fixture("missing_build_output.cdm").expect("Failed to load fixture");
    let result = validate_tree(tree);

    assert!(result.is_err(), "Expected validation to fail when build_output is missing");

    let diagnostics = result.unwrap_err();

    // Should contain E406 error about missing build_output
    let has_e406_error = diagnostics.iter().any(|d| {
        d.message.contains("E406") && d.message.contains("build_output")
    });

    assert!(
        has_e406_error,
        "Expected E406 error about missing build_output, got: {:?}",
        diagnostics
    );
}
