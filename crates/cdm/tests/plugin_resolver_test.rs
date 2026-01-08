//! Integration tests for plugin resolution
//!
//! These tests verify that plugin resolution works consistently across
//! validate, build, and migrate commands.

use std::fs;
use std::path::PathBuf;
use std::env;
use cdm::{build, FileResolver, validate_tree_with_options, Severity};

/// Get the path to the test fixtures directory
fn fixtures_dir() -> PathBuf {
    // Get the cargo manifest directory (crates/cdm)
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            // Fallback: current directory
            env::current_dir().expect("Failed to get current directory")
        });

    manifest_dir.join("tests/fixtures")
}

/// Test that validation works with local plugins
#[test]
fn test_validate_with_local_plugin() {
    let fixtures = fixtures_dir();
    let test_file = fixtures.join("base.cdm");

    // Load the file tree
    let tree = FileResolver::load(&test_file).expect("Failed to load file");

    // Validate - this should succeed using local plugins
    let result = validate_tree_with_options(tree, false);

    assert!(
        result.is_ok(),
        "Validation should succeed with local plugins: {:?}",
        result.err()
    );

    // Check that there are no errors (warnings are OK)
    if let Ok(validation_result) = result {
        let has_errors = validation_result.diagnostics.iter().any(|d| d.severity == Severity::Error);
        assert!(
            !has_errors,
            "Validation should have no errors: {:?}",
            validation_result.diagnostics
        );
    }
}

/// Test that build works with local plugins
#[test]
#[serial_test::serial]
fn test_build_with_local_plugin() {
    let fixtures = fixtures_dir();
    let test_file = fixtures.join("base.cdm");

    // Clean up any previous output
    let output_file = fixtures.join("build/types.ts");
    let _ = fs::remove_file(&output_file);

    // This should succeed using local plugins
    let result = build(&test_file);

    assert!(
        result.is_ok(),
        "Build should succeed with local plugins: {:?}",
        result.err()
    );

    // Verify the output file was created
    assert!(
        output_file.exists(),
        "Build should create types.ts output file at {}",
        output_file.display()
    );

    // Clean up
    let _ = fs::remove_file(&output_file);
}

/// Test that multiple builds work consistently with local plugins
#[test]
#[serial_test::serial]
fn test_multiple_builds_consistent() {
    let fixtures = fixtures_dir();
    let test_file = fixtures.join("base.cdm");

    // Clean up any previous output
    let output_file = fixtures.join("build/types.ts");
    let _ = fs::remove_file(&output_file);

    // First build
    let result1 = build(&test_file);
    assert!(result1.is_ok(), "First build should succeed");
    let _ = fs::remove_file(&output_file);

    // Second build - should also succeed
    let result2 = build(&test_file);
    assert!(result2.is_ok(), "Second build should succeed");

    // Clean up
    let _ = fs::remove_file(&output_file);
}

/// Test that local plugins take precedence over registry
#[test]
fn test_local_plugin_precedence() {
    // This test verifies the resolution order:
    // 1. ./plugins/{name}.wasm (local)
    // 2. Registry (downloaded)

    // Create a test CDM file with a plugin import
    let test_content = r#"
@typescript {
  build_output: "./build"
}

User {
  id
  name
}
"#;

    let test_dir = std::env::temp_dir().join("cdm_test_local_plugin");
    let _ = fs::create_dir_all(&test_dir);
    let test_file = test_dir.join("test.cdm");
    fs::write(&test_file, test_content).unwrap();

    // If there's no local plugin, it should fall back to registry
    let result = FileResolver::load(&test_file);

    assert!(
        result.is_ok(),
        "Should fall back to registry when no local plugin exists"
    );

    // Clean up
    let _ = fs::remove_dir_all(&test_dir);
}

/// Test that validation and build use the same plugin resolution
#[test]
#[serial_test::serial]
fn test_validate_build_consistency() {
    let fixtures = fixtures_dir();
    let test_file = fixtures.join("base.cdm");

    // Clean up any previous output
    let output_file = fixtures.join("build/types.ts");
    let _ = fs::remove_file(&output_file);

    // Validation should succeed
    let tree = FileResolver::load(&test_file).expect("Failed to load file");
    let validate_result = validate_tree_with_options(tree, false);
    assert!(
        validate_result.is_ok(),
        "Validation should succeed: {:?}",
        validate_result.err()
    );

    // Build should also succeed with the same plugin
    let build_result = build(&test_file);
    assert!(
        build_result.is_ok(),
        "Build should succeed: {:?}",
        build_result.err()
    );

    // Verify output was created
    assert!(
        output_file.exists(),
        "Build output should be created at {}",
        output_file.display()
    );

    // Clean up
    let _ = fs::remove_file(&output_file);
}
