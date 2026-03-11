use super::*;
use serde_json::json;

#[test]
fn test_extract_output_paths_single_string() {
    let config = json!({ "build_output": "./out" });
    let paths = extract_output_paths(&config, "build_output");
    assert_eq!(paths, vec![PathBuf::from("./out")]);
}

#[test]
fn test_extract_output_paths_array_of_strings() {
    let config = json!({ "build_output": ["./out1", "./out2", "./out3"] });
    let paths = extract_output_paths(&config, "build_output");
    assert_eq!(paths, vec![
        PathBuf::from("./out1"),
        PathBuf::from("./out2"),
        PathBuf::from("./out3"),
    ]);
}

#[test]
fn test_extract_output_paths_empty_string() {
    let config = json!({ "build_output": "" });
    let paths = extract_output_paths(&config, "build_output");
    assert!(paths.is_empty());
}

#[test]
fn test_extract_output_paths_empty_array() {
    let config = json!({ "build_output": [] });
    let paths = extract_output_paths(&config, "build_output");
    assert!(paths.is_empty());
}

#[test]
fn test_extract_output_paths_missing_key() {
    let config = json!({ "other_key": "value" });
    let paths = extract_output_paths(&config, "build_output");
    assert!(paths.is_empty());
}

#[test]
fn test_extract_output_paths_array_filters_non_strings() {
    let config = json!({ "build_output": ["./valid", 123, null, "./also_valid", ""] });
    let paths = extract_output_paths(&config, "build_output");
    assert_eq!(paths, vec![
        PathBuf::from("./valid"),
        PathBuf::from("./also_valid"),
    ]);
}

#[test]
fn test_extract_output_paths_null_value() {
    let config = json!({ "build_output": null });
    let paths = extract_output_paths(&config, "build_output");
    assert!(paths.is_empty());
}

#[test]
fn test_extract_output_paths_works_with_migrations_output() {
    let config = json!({ "migrations_output": ["./migrations/sql", "./shared/migrations"] });
    let paths = extract_output_paths(&config, "migrations_output");
    assert_eq!(paths, vec![
        PathBuf::from("./migrations/sql"),
        PathBuf::from("./shared/migrations"),
    ]);
}
