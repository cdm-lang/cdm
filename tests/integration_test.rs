//! CDM Integration Tests
//!
//! Tests the CLI commands against example files in the examples directory.
//! These tests verify end-to-end functionality of validate and build commands.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

/// Cache the built CLI path
static CLI_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Get the path to the project root (parent of tests directory)
fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tests dir should have parent")
        .to_path_buf()
}

/// Get the path to the examples directory
fn examples_dir() -> PathBuf {
    project_root().join("examples")
}

/// Build the CDM CLI and return the path to the binary (cached)
fn build_cli() -> PathBuf {
    CLI_PATH.get_or_init(|| {
        let root = project_root();

        let status = Command::new("cargo")
            .args(["build", "--release", "-p", "cdm"])
            .current_dir(&root)
            .status()
            .expect("Failed to execute cargo build");

        assert!(status.success(), "Failed to build CDM CLI");

        // The binary location depends on the target directory configuration
        // Try common locations
        let possible_paths = [
            root.join("target/release/cdm"),
            PathBuf::from("/var/tmp/rust-build/release/cdm"),
        ];

        for path in &possible_paths {
            if path.exists() {
                return path.clone();
            }
        }

        panic!("Could not find CDM binary after build. Tried: {:?}", possible_paths);
    }).clone()
}

/// Run a CDM command and return success status
fn run_cdm(binary: &PathBuf, args: &[&str], working_dir: &PathBuf) -> Result<String, String> {
    let output = Command::new(binary)
        .args(args)
        .current_dir(working_dir)
        .output()
        .expect("Failed to execute CDM command");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(format!("{}{}", stdout, stderr))
    } else {
        Err(format!("Command failed:\nstdout: {}\nstderr: {}", stdout, stderr))
    }
}

/// Clean up generated files before/after tests
fn cleanup_generated_files() {
    let examples = examples_dir();
    let _ = fs::remove_dir_all(examples.join("build"));
    let _ = fs::remove_dir_all(examples.join("migrate"));
    let _ = fs::remove_dir_all(examples.join(".cdm"));
}

// =============================================================================
// VALIDATION TESTS
// =============================================================================

#[test]
fn test_validate_base_cdm() {
    let binary = build_cli();
    let examples = examples_dir();

    let result = run_cdm(&binary, &["validate", "./base.cdm"], &examples);

    assert!(
        result.is_ok(),
        "Validation of base.cdm should succeed: {:?}",
        result.err()
    );
}

#[test]
fn test_validate_client_cdm() {
    let binary = build_cli();
    let examples = examples_dir();

    let result = run_cdm(&binary, &["validate", "./client.cdm"], &examples);

    // client.cdm may have warnings (shadowing) but should not have errors
    assert!(
        result.is_ok(),
        "Validation of client.cdm should succeed: {:?}",
        result.err()
    );
}

// =============================================================================
// BUILD TESTS
// =============================================================================

#[test]
#[serial_test::serial]
fn test_build_base_cdm() {
    let binary = build_cli();
    let examples = examples_dir();

    // Clean up before test
    cleanup_generated_files();

    let result = run_cdm(&binary, &["build", "./base.cdm"], &examples);

    assert!(
        result.is_ok(),
        "Build of base.cdm should succeed: {:?}",
        result.err()
    );

    // Verify TypeScript output was generated
    let ts_output = examples.join("build/types.ts");
    assert!(
        ts_output.exists(),
        "TypeScript types should be generated at {}",
        ts_output.display()
    );

    // Verify SQL output was generated
    let sql_output = examples.join("build/schema.postgres.sql");
    assert!(
        sql_output.exists(),
        "SQL schema should be generated at {}",
        sql_output.display()
    );

    // Clean up after test
    cleanup_generated_files();
}

#[test]
#[serial_test::serial]
fn test_build_client_cdm() {
    let binary = build_cli();
    let examples = examples_dir();

    // Clean up before test
    cleanup_generated_files();

    // First build base.cdm (client extends it)
    let _ = run_cdm(&binary, &["build", "./base.cdm"], &examples);

    let result = run_cdm(&binary, &["build", "./client.cdm"], &examples);

    assert!(
        result.is_ok(),
        "Build of client.cdm should succeed: {:?}",
        result.err()
    );

    // Verify client TypeScript output was generated
    let client_ts = examples.join("build/client/types.ts");
    assert!(
        client_ts.exists(),
        "Client TypeScript types should be generated at {}",
        client_ts.display()
    );

    // Clean up after test
    cleanup_generated_files();
}

// =============================================================================
// CLI TESTS
// =============================================================================

#[test]
fn test_cli_help() {
    let binary = build_cli();

    let result = run_cdm(&binary, &["--help"], &project_root());

    assert!(result.is_ok(), "CLI --help should succeed");

    let output = result.unwrap();
    assert!(
        output.contains("Usage:") || output.contains("USAGE:"),
        "Help output should contain usage information"
    );
}

#[test]
fn test_cli_validate_help() {
    let binary = build_cli();

    let result = run_cdm(&binary, &["validate", "--help"], &project_root());

    assert!(result.is_ok(), "CLI validate --help should succeed");
}

#[test]
fn test_cli_build_help() {
    let binary = build_cli();

    let result = run_cdm(&binary, &["build", "--help"], &project_root());

    assert!(result.is_ok(), "CLI build --help should succeed");
}

// =============================================================================
// OUTPUT VERIFICATION TESTS
// =============================================================================

#[test]
#[serial_test::serial]
fn test_typescript_output_contains_models() {
    let binary = build_cli();
    let examples = examples_dir();

    cleanup_generated_files();

    let result = run_cdm(&binary, &["build", "./base.cdm"], &examples);
    assert!(result.is_ok(), "Build should succeed");

    let ts_output = examples.join("build/types.ts");
    let content = fs::read_to_string(&ts_output).expect("Should read TypeScript output");

    // Verify key types are present
    // Note: UUID is now imported from pg.UUID template, not defined locally
    assert!(content.contains("export type Email"), "Should contain Email type");
    assert!(content.contains("export type Status"), "Should contain Status type");
    assert!(content.contains("export interface User"), "Should contain User interface");
    assert!(content.contains("export interface Post"), "Should contain Post interface");
    assert!(content.contains("export interface Comment"), "Should contain Comment interface");

    cleanup_generated_files();
}

#[test]
#[serial_test::serial]
fn test_sql_output_contains_tables() {
    let binary = build_cli();
    let examples = examples_dir();

    cleanup_generated_files();

    let result = run_cdm(&binary, &["build", "./base.cdm"], &examples);
    assert!(result.is_ok(), "Build should succeed");

    let sql_output = examples.join("build/schema.postgres.sql");
    let content = fs::read_to_string(&sql_output).expect("Should read SQL output");

    // Verify key tables are present
    assert!(content.contains("CREATE TABLE"), "Should contain CREATE TABLE statements");
    assert!(content.contains("users"), "Should contain users table");
    assert!(content.contains("posts"), "Should contain posts table");
    assert!(content.contains("comments"), "Should contain comments table");

    cleanup_generated_files();
}
