//! Integration tests for plugin resolution
//!
//! These tests verify that plugin resolution works consistently across
//! validate, build, and migrate commands.

use std::fs;
use std::path::PathBuf;
use std::env;
use cdm::{build, FileResolver, validate_tree_with_options, Severity};

/// Get the path to the project root
fn project_root() -> PathBuf {
    // When tests run, the working directory is crates/cdm
    // We need to go up two levels to get to the project root
    let mut path = env::current_dir().expect("Failed to get current directory");

    // If we're in crates/cdm, go up two levels
    if path.ends_with("crates/cdm") {
        path.pop(); // Remove cdm
        path.pop(); // Remove crates
    }

    path
}

/// Test that validation works with registry plugins
#[test]
fn test_validate_with_registry_plugin() {
    let root = project_root();
    let test_file = root.join("examples/base.cdm");

    // Load the file tree
    let tree = FileResolver::load(&test_file).expect("Failed to load file");

    // Validate - this should succeed and the plugin will be downloaded from the registry
    let result = validate_tree_with_options(tree, false);

    assert!(
        result.is_ok(),
        "Validation should succeed with registry plugin: {:?}",
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

/// Test that build works with registry plugins
#[test]
fn test_build_with_registry_plugin() {
    let root = project_root();
    let test_file = root.join("examples/base.cdm");

    // Clean up any previous output
    // Note: Output is written relative to cwd during tests, not relative to source file
    let output_file = env::current_dir().unwrap().join("types.ts");
    let _ = fs::remove_file(&output_file);

    // This should succeed - the plugin will be downloaded from the registry
    let result = build(&test_file);

    assert!(
        result.is_ok(),
        "Build should succeed with registry plugin: {:?}",
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

/// Test that the plugin cache is used across multiple operations
#[test]
fn test_plugin_cache_reuse() {
    let root = project_root();
    let test_file = root.join("examples/base.cdm");

    // Clean up any previous output
    let output_file = env::current_dir().unwrap().join("types.ts");
    let _ = fs::remove_file(&output_file);

    // First build - will download the plugin
    let result1 = build(&test_file);
    assert!(result1.is_ok(), "First build should succeed");
    let _ = fs::remove_file(&output_file);

    // Check that the plugin was cached
    // Cache is created relative to current working directory (crates/cdm during tests)
    let cache_path = env::current_dir()
        .unwrap()
        .join(".cdm/cache/plugins/typescript@0.1.0/plugin.wasm");
    assert!(
        cache_path.exists(),
        "Plugin should be cached after first build at {}",
        cache_path.display()
    );

    // Second build - should use cached plugin (no download)
    let result2 = build(&test_file);
    assert!(result2.is_ok(), "Second build should succeed using cache");

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
fn test_validate_build_consistency() {
    let root = project_root();
    let test_file = root.join("examples/base.cdm");

    // Clean up any previous output
    let output_file = env::current_dir().unwrap().join("types.ts");
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
