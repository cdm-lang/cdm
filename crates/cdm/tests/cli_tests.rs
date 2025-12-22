// tests/cli_tests.rs
use std::process::Command;
use std::fs;

fn cdm_binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cdm"))
}

#[test]
fn test_validate_valid_file() {
    // Create a temp file
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("test_valid.cdm");
    fs::write(&test_file, "Email: string").unwrap();

    let output = cdm_binary()
        .args(["validate", test_file.to_str().unwrap()])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    
    // Cleanup
    fs::remove_file(test_file).ok();
}

#[test]
fn test_validate_file_with_errors() {
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("test_invalid.cdm");
    fs::write(&test_file, "User { email: Emaill }").unwrap();

    let output = cdm_binary()
        .args(["validate", test_file.to_str().unwrap()])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Undefined type"));

    fs::remove_file(test_file).ok();
}

#[test]
fn test_validate_missing_file() {
    let output = cdm_binary()
        .args(["validate", "nonexistent.cdm"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to resolve path") || stderr.contains("No such file"));
}

#[test]
fn test_version_flag() {
    let output = cdm_binary()
        .arg("--version")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("cdm"));
}

#[test]
fn test_help_flag() {
    let output = cdm_binary()
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("CLI for contextual data modeling"));
}