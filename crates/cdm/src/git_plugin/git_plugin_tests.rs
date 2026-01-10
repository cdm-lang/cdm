use super::*;

#[test]
fn test_sanitize_git_url_https() {
    let url = "https://github.com/user/repo.git";
    let sanitized = sanitize_git_url(url);
    assert_eq!(sanitized, "github_com_user_repo");
}

#[test]
fn test_sanitize_git_url_http() {
    let url = "http://example.com/path/to/repo.git";
    let sanitized = sanitize_git_url(url);
    assert_eq!(sanitized, "example_com_path_to_repo");
}

#[test]
fn test_sanitize_git_url_ssh() {
    let url = "git@github.com:user/repo.git";
    let sanitized = sanitize_git_url(url);
    assert_eq!(sanitized, "github_com_user_repo");
}

#[test]
fn test_sanitize_git_url_no_git_extension() {
    let url = "https://github.com/user/repo";
    let sanitized = sanitize_git_url(url);
    assert_eq!(sanitized, "github_com_user_repo");
}

#[test]
fn test_sanitize_git_url_git_protocol() {
    let url = "git://github.com/user/repo.git";
    let sanitized = sanitize_git_url(url);
    assert_eq!(sanitized, "github_com_user_repo");
}

#[test]
fn test_sanitize_git_url_complex() {
    let url = "https://gitlab.example.com:8080/group/subgroup/repo.git";
    let sanitized = sanitize_git_url(url);
    assert_eq!(sanitized, "gitlab_example_com_8080_group_subgroup_repo");
}

#[test]
fn test_extract_wasm_from_repo_no_manifest() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    let result = extract_wasm_from_repo(repo_path, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("No cdm-plugin.json found"));
}

#[test]
fn test_extract_wasm_from_repo_invalid_json() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    // Create invalid JSON manifest
    let manifest_path = repo_path.join("cdm-plugin.json");
    fs::write(&manifest_path, "invalid json").unwrap();

    let result = extract_wasm_from_repo(repo_path, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Failed to parse"));
}

#[test]
fn test_extract_wasm_from_repo_no_wasm_field() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    // Create manifest without wasm.file field
    let manifest_path = repo_path.join("cdm-plugin.json");
    let manifest_content = serde_json::json!({
        "name": "test-plugin",
        "version": "1.0.0"
    });
    fs::write(&manifest_path, serde_json::to_string(&manifest_content).unwrap()).unwrap();

    let result = extract_wasm_from_repo(repo_path, None);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("wasm.file"), "Error was: {}", err_msg);
}

#[test]
fn test_extract_wasm_from_repo_wasm_file_not_found() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    // Create manifest with non-existent wasm file
    let manifest_path = repo_path.join("cdm-plugin.json");
    let manifest_content = serde_json::json!({
        "name": "test-plugin",
        "version": "1.0.0",
        "wasm": {
            "file": "plugin.wasm"
        }
    });
    fs::write(&manifest_path, serde_json::to_string(&manifest_content).unwrap()).unwrap();

    let result = extract_wasm_from_repo(repo_path, None);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("WASM file not found"));
}

#[test]
fn test_extract_wasm_from_repo_success() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    // Create manifest
    let manifest_path = repo_path.join("cdm-plugin.json");
    let manifest_content = serde_json::json!({
        "name": "test-plugin",
        "version": "1.0.0",
        "wasm": {
            "file": "plugin.wasm"
        }
    });
    fs::write(&manifest_path, serde_json::to_string(&manifest_content).unwrap()).unwrap();

    // Create wasm file
    let wasm_path = repo_path.join("plugin.wasm");
    fs::write(&wasm_path, b"wasm content").unwrap();

    let result = extract_wasm_from_repo(repo_path, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), wasm_path);
}

#[test]
fn test_extract_wasm_from_repo_nested_path() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    // Create manifest with nested wasm path
    let manifest_path = repo_path.join("cdm-plugin.json");
    let manifest_content = serde_json::json!({
        "name": "test-plugin",
        "version": "1.0.0",
        "wasm": {
            "file": "target/release/plugin.wasm"
        }
    });
    fs::write(&manifest_path, serde_json::to_string(&manifest_content).unwrap()).unwrap();

    // Create nested wasm file
    fs::create_dir_all(repo_path.join("target/release")).unwrap();
    let wasm_path = repo_path.join("target/release/plugin.wasm");
    fs::write(&wasm_path, b"wasm content").unwrap();

    let result = extract_wasm_from_repo(repo_path, None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), wasm_path);
}

#[test]
fn test_extract_wasm_from_repo_with_subdir() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    // Create subdirectory structure
    let subdir = "crates/my-plugin";
    fs::create_dir_all(repo_path.join(subdir)).unwrap();

    // Create manifest in subdirectory
    let manifest_path = repo_path.join(subdir).join("cdm-plugin.json");
    let manifest_content = serde_json::json!({
        "name": "my-plugin",
        "version": "1.0.0",
        "wasm": {
            "file": "plugin.wasm"
        }
    });
    fs::write(&manifest_path, serde_json::to_string(&manifest_content).unwrap()).unwrap();

    // Create wasm file in subdirectory
    let wasm_path = repo_path.join(subdir).join("plugin.wasm");
    fs::write(&wasm_path, b"wasm content").unwrap();

    let result = extract_wasm_from_repo(repo_path, Some(subdir));
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), wasm_path);
}

#[test]
fn test_extract_wasm_from_repo_subdir_not_found() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    let result = extract_wasm_from_repo(repo_path, Some("nonexistent/path"));
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("cdm-plugin.json"), "Error was: {}", err_msg);
}

#[test]
fn test_get_cache_path_returns_valid_path() {
    // Test that get_cache_path returns a valid path that can be used for caching
    use crate::registry;

    let cache_path = registry::get_cache_path();
    assert!(cache_path.is_ok(), "get_cache_path should return Ok");

    let path = cache_path.unwrap();
    // The path should contain "cdm" somewhere (platform-specific cache location)
    assert!(
        path.to_string_lossy().contains("cdm"),
        "Cache path should contain 'cdm': {}",
        path.display()
    );
}
