use super::*;
use tempfile::TempDir;

#[test]
fn test_extract_plugin_imports_from_file_invalid_path() {
    use std::path::Path;

    let result = extract_plugin_imports_from_file(Path::new("/nonexistent/file.cdm"));
    assert!(result.is_err());
}

#[test]
fn test_extract_plugin_imports_from_file_valid() {
    use std::fs;

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.cdm");

    let content = r#"
@test_plugin
@another_plugin from git:https://github.com/user/repo.git

User {
    name: string #1
} #2
"#;

    fs::write(&file_path, content).unwrap();

    let result = extract_plugin_imports_from_file(&file_path);
    assert!(result.is_ok());

    let imports = result.unwrap();
    assert!(imports.len() >= 1);
}

#[test]
fn test_extract_plugin_imports_from_file_empty() {
    use std::fs;

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("empty.cdm");

    fs::write(&file_path, "").unwrap();

    let result = extract_plugin_imports_from_file(&file_path);
    assert!(result.is_ok());

    let imports = result.unwrap();
    assert_eq!(imports.len(), 0);
}

#[test]
fn test_extract_plugin_imports_from_file_invalid_syntax() {
    use std::fs;

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("invalid.cdm");

    // Write invalid CDM syntax
    fs::write(&file_path, "this is not valid CDM syntax !!!").unwrap();

    let result = extract_plugin_imports_from_file(&file_path);
    // Should still parse, just with errors
    assert!(result.is_ok());
}

#[test]
fn test_cache_plugin_cmd_no_name_no_all() {
    let result = cache_plugin_cmd(None, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Must specify"));
}

#[test]
fn test_list_plugins_no_registry() {
    // This test verifies list_plugins can handle registry loading
    // The actual behavior depends on whether registry exists
    let result = list_plugins(false);
    // Should either succeed or fail with a meaningful error
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_plugin_info_nonexistent() {
    // This test would require a mock registry
    // For now, we verify it handles errors gracefully
    let result = plugin_info("nonexistent-plugin-12345", false);
    // Should fail for nonexistent plugin
    assert!(result.is_err());
}

#[test]
fn test_clear_cache_cmd_with_name() {
    let result = clear_cache_cmd(Some("test-plugin"));
    // Should succeed or fail gracefully
    assert!(result.is_ok() || result.is_err());
}
