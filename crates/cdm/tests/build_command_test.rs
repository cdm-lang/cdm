use std::path::PathBuf;

#[test]
fn test_build_with_valid_schema() {
    let path = PathBuf::from("test_fixtures/file_resolver/single_file/simple.cdm");
    let result = cdm::build(&path);

    // Should succeed with a valid schema
    assert!(result.is_ok(), "Build should succeed with valid schema");
}

#[test]
fn test_build_with_invalid_schema() {
    // Create a temporary invalid CDM file
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_invalid_build.cdm");

    std::fs::write(&temp_file, "User { id: InvalidType }").unwrap();

    let result = cdm::build(&temp_file);

    // Clean up
    let _ = std::fs::remove_file(&temp_file);

    // Should fail with an invalid schema
    assert!(result.is_err(), "Build should fail with invalid schema");
    assert!(result.unwrap_err().to_string().contains("Validation failed"));
}

#[test]
fn test_build_with_missing_file() {
    let path = PathBuf::from("nonexistent.cdm");
    let result = cdm::build(&path);

    // Should fail with file not found
    assert!(result.is_err(), "Build should fail with missing file");
    assert!(result.unwrap_err().to_string().contains("Failed to load CDM file"));
}
