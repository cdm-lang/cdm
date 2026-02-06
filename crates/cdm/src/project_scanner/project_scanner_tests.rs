use super::*;
use std::fs::{self, File};
use tempfile::TempDir;

fn create_test_project() -> TempDir {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create directory structure
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("src/models")).unwrap();
    fs::create_dir_all(root.join("node_modules/some-package")).unwrap();
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::create_dir_all(root.join("target")).unwrap();

    // Create .cdm files
    File::create(root.join("schema.cdm")).unwrap();
    File::create(root.join("src/base.cdm")).unwrap();
    File::create(root.join("src/models/user.cdm")).unwrap();
    File::create(root.join("src/models/post.cdm")).unwrap();

    // Create .cdm files in ignored directories (should not be found)
    File::create(root.join("node_modules/some-package/schema.cdm")).unwrap();
    File::create(root.join("target/generated.cdm")).unwrap();

    // Create non-.cdm files (should not be found)
    File::create(root.join("src/readme.md")).unwrap();
    File::create(root.join("src/config.json")).unwrap();

    temp
}

#[test]
fn test_scan_finds_cdm_files() {
    let temp = create_test_project();
    let scanner = ProjectScanner::new(temp.path());

    let files = scanner.scan().unwrap();

    // Should find exactly 4 .cdm files (excluding ignored directories)
    assert_eq!(files.len(), 4);

    // Verify all expected files are found
    let file_names: Vec<String> = files
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();

    assert!(file_names.contains(&"schema.cdm".to_string()));
    assert!(file_names.contains(&"base.cdm".to_string()));
    assert!(file_names.contains(&"user.cdm".to_string()));
    assert!(file_names.contains(&"post.cdm".to_string()));
}

#[test]
fn test_scan_ignores_node_modules() {
    let temp = create_test_project();
    let scanner = ProjectScanner::new(temp.path());

    let files = scanner.scan().unwrap();

    // No file should be from node_modules
    for file in &files {
        let path_str = file.to_string_lossy();
        assert!(
            !path_str.contains("node_modules"),
            "Found file in node_modules: {}",
            path_str
        );
    }
}

#[test]
fn test_scan_ignores_target_directory() {
    let temp = create_test_project();
    let scanner = ProjectScanner::new(temp.path());

    let files = scanner.scan().unwrap();

    // No file should be from target
    for file in &files {
        let path_str = file.to_string_lossy();
        assert!(
            !path_str.contains("/target/"),
            "Found file in target: {}",
            path_str
        );
    }
}

#[test]
fn test_scan_ignores_hidden_directories() {
    let temp = create_test_project();

    // Create a hidden directory with a .cdm file
    fs::create_dir_all(temp.path().join(".hidden")).unwrap();
    File::create(temp.path().join(".hidden/secret.cdm")).unwrap();

    let scanner = ProjectScanner::new(temp.path());
    let files = scanner.scan().unwrap();

    // No file should be from .hidden
    for file in &files {
        let path_str = file.to_string_lossy();
        assert!(
            !path_str.contains(".hidden"),
            "Found file in .hidden: {}",
            path_str
        );
    }
}

#[test]
fn test_scan_returns_sorted_paths() {
    let temp = create_test_project();
    let scanner = ProjectScanner::new(temp.path());

    let files = scanner.scan().unwrap();

    // Verify files are sorted
    let sorted: Vec<PathBuf> = {
        let mut v = files.clone();
        v.sort();
        v
    };

    assert_eq!(files, sorted);
}

#[test]
fn test_scan_empty_directory() {
    let temp = TempDir::new().unwrap();
    let scanner = ProjectScanner::new(temp.path());

    let files = scanner.scan().unwrap();

    assert!(files.is_empty());
}

#[test]
fn test_scan_nonexistent_directory() {
    let scanner = ProjectScanner::new("/nonexistent/path/that/does/not/exist");

    let files = scanner.scan().unwrap();

    // Should return empty list, not error
    assert!(files.is_empty());
}

#[test]
fn test_find_project_root_with_git() {
    let temp = create_test_project();
    let file_path = temp.path().join("src/models/user.cdm");

    let root = ProjectScanner::find_project_root(&file_path);

    assert!(root.is_some());
    assert_eq!(root.unwrap(), temp.path());
}

#[test]
fn test_find_project_root_without_git() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create structure without .git
    fs::create_dir_all(root.join("src")).unwrap();
    File::create(root.join("src/schema.cdm")).unwrap();

    let file_path = root.join("src/schema.cdm");
    let found_root = ProjectScanner::find_project_root(&file_path);

    // Should fall back to the file's parent directory
    assert!(found_root.is_some());
    assert_eq!(found_root.unwrap(), root.join("src"));
}

#[test]
fn test_find_project_root_nested_git() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Create nested git structure
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::create_dir_all(root.join("packages/sub/.git")).unwrap();
    fs::create_dir_all(root.join("packages/sub/src")).unwrap();
    File::create(root.join("packages/sub/src/schema.cdm")).unwrap();

    let file_path = root.join("packages/sub/src/schema.cdm");
    let found_root = ProjectScanner::find_project_root(&file_path);

    // Should find the closest .git (inner one)
    assert!(found_root.is_some());
    assert_eq!(found_root.unwrap(), root.join("packages/sub"));
}
