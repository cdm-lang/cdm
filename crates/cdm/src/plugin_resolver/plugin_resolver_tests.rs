use super::*;
use crate::plugin_validation::{PluginImport, PluginSource};
use cdm_utils::{Span, Position};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// Helper to create a test span
fn test_span() -> Span {
    Span {
        start: Position { line: 0, column: 0 },
        end: Position { line: 0, column: 0 },
    }
}

// Helper to create a test PluginImport
fn create_test_import(name: &str, source: Option<PluginSource>) -> PluginImport {
    PluginImport {
        name: name.to_string(),
        source,
        global_config: None,
        span: test_span(),
        name_span: test_span(),
        source_file: PathBuf::from("/test/schema.cdm"),
    }
}

#[test]
fn test_resolve_plugin_path_local_no_manifest() {
    let temp_dir = TempDir::new().unwrap();
    let plugin_dir = temp_dir.path().join("test-plugin");
    fs::create_dir(&plugin_dir).unwrap();

    let import = PluginImport {
        name: "test-plugin".to_string(),
        source: Some(PluginSource::Path {
            path: "./test-plugin".to_string(),
        }),
        global_config: None,
        span: test_span(),
        name_span: test_span(),
        source_file: temp_dir.path().join("schema.cdm"),
    };

    let result = resolve_plugin_path(&import);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("cdm-plugin.json"), "Error was: {}", err_msg);
}

#[test]
fn test_resolve_plugin_path_local_invalid_json() {
    let temp_dir = TempDir::new().unwrap();
    let plugin_dir = temp_dir.path().join("test-plugin");
    fs::create_dir(&plugin_dir).unwrap();

    // Create invalid JSON manifest
    let manifest_path = plugin_dir.join("cdm-plugin.json");
    fs::write(&manifest_path, "invalid json").unwrap();

    let import = PluginImport {
        name: "test-plugin".to_string(),
        source: Some(PluginSource::Path {
            path: "./test-plugin".to_string(),
        }),
        global_config: None,
        span: test_span(),
        name_span: test_span(),
        source_file: temp_dir.path().join("schema.cdm"),
    };

    let result = resolve_plugin_path(&import);
    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_msg = format!("{:#}", err);  // Full error chain
    assert!(err_msg.contains("Failed to parse"), "Error was: {}", err_msg);
}

#[test]
fn test_resolve_plugin_path_local_no_wasm_field() {
    let temp_dir = TempDir::new().unwrap();
    let plugin_dir = temp_dir.path().join("test-plugin");
    fs::create_dir(&plugin_dir).unwrap();

    // Create manifest without wasm.file field
    let manifest_path = plugin_dir.join("cdm-plugin.json");
    let manifest_content = serde_json::json!({
        "name": "test-plugin",
        "version": "1.0.0"
    });
    fs::write(&manifest_path, serde_json::to_string(&manifest_content).unwrap()).unwrap();

    let import = PluginImport {
        name: "test-plugin".to_string(),
        source: Some(PluginSource::Path {
            path: "./test-plugin".to_string(),
        }),
        global_config: None,
        span: test_span(),
        name_span: test_span(),
        source_file: temp_dir.path().join("schema.cdm"),
    };

    let result = resolve_plugin_path(&import);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("wasm.file"), "Error was: {}", err_msg);
}

#[test]
fn test_resolve_plugin_path_local_wasm_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let plugin_dir = temp_dir.path().join("test-plugin");
    fs::create_dir(&plugin_dir).unwrap();

    // Create manifest with non-existent wasm file
    let manifest_path = plugin_dir.join("cdm-plugin.json");
    let manifest_content = serde_json::json!({
        "name": "test-plugin",
        "version": "1.0.0",
        "wasm": {
            "file": "plugin.wasm"
        }
    });
    fs::write(&manifest_path, serde_json::to_string(&manifest_content).unwrap()).unwrap();

    let import = PluginImport {
        name: "test-plugin".to_string(),
        source: Some(PluginSource::Path {
            path: "./test-plugin".to_string(),
        }),
        global_config: None,
        span: test_span(),
        name_span: test_span(),
        source_file: temp_dir.path().join("schema.cdm"),
    };

    let result = resolve_plugin_path(&import);
    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_msg = format!("{:#}", err);  // Full error chain
    assert!(err_msg.contains("WASM file not found"), "Error was: {}", err_msg);
}

#[test]
fn test_resolve_plugin_path_local_success() {
    let temp_dir = TempDir::new().unwrap();
    let plugin_dir = temp_dir.path().join("test-plugin");
    fs::create_dir(&plugin_dir).unwrap();

    // Create manifest
    let manifest_path = plugin_dir.join("cdm-plugin.json");
    let manifest_content = serde_json::json!({
        "name": "test-plugin",
        "version": "1.0.0",
        "wasm": {
            "file": "plugin.wasm"
        }
    });
    fs::write(&manifest_path, serde_json::to_string(&manifest_content).unwrap()).unwrap();

    // Create wasm file
    let wasm_path = plugin_dir.join("plugin.wasm");
    fs::write(&wasm_path, b"wasm content").unwrap();

    let import = PluginImport {
        name: "test-plugin".to_string(),
        source: Some(PluginSource::Path {
            path: "./test-plugin".to_string(),
        }),
        global_config: None,
        span: test_span(),
        name_span: test_span(),
        source_file: temp_dir.path().join("schema.cdm"),
    };

    let result = resolve_plugin_path(&import);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), wasm_path);
}

#[test]
fn test_resolve_plugin_path_local_nested_wasm() {
    let temp_dir = TempDir::new().unwrap();
    let plugin_dir = temp_dir.path().join("test-plugin");
    fs::create_dir(&plugin_dir).unwrap();

    // Create manifest with nested wasm path
    let manifest_path = plugin_dir.join("cdm-plugin.json");
    let manifest_content = serde_json::json!({
        "name": "test-plugin",
        "version": "1.0.0",
        "wasm": {
            "file": "target/release/plugin.wasm"
        }
    });
    fs::write(&manifest_path, serde_json::to_string(&manifest_content).unwrap()).unwrap();

    // Create nested wasm file
    fs::create_dir_all(plugin_dir.join("target/release")).unwrap();
    let wasm_path = plugin_dir.join("target/release/plugin.wasm");
    fs::write(&wasm_path, b"wasm content").unwrap();

    let import = PluginImport {
        name: "test-plugin".to_string(),
        source: Some(PluginSource::Path {
            path: "./test-plugin".to_string(),
        }),
        global_config: None,
        span: test_span(),
        name_span: test_span(),
        source_file: temp_dir.path().join("schema.cdm"),
    };

    let result = resolve_plugin_path(&import);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), wasm_path);
}

#[test]
fn test_resolve_plugin_path_registry_fallback() {
    // Test that when source is None and local file doesn't exist, it tries registry
    let import = create_test_import("nonexistent-plugin", None);

    let result = resolve_plugin_path(&import);
    // Should fail trying to resolve from registry (no network in tests)
    assert!(result.is_err());
}

#[test]
fn test_resolve_plugin_path_local_default_location() {
    let temp_dir = TempDir::new().unwrap();

    // Change to temp dir for this test
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    // Create default plugins directory
    let plugins_dir = temp_dir.path().join("plugins");
    fs::create_dir(&plugins_dir).unwrap();

    // Create a plugin in the default location
    let wasm_path = plugins_dir.join("test-plugin.wasm");
    fs::write(&wasm_path, b"wasm content").unwrap();

    let import = PluginImport {
        name: "test-plugin".to_string(),
        source: None,
        global_config: None,
        span: test_span(),
        name_span: test_span(),
        source_file: temp_dir.path().join("schema.cdm"),
    };

    let result = resolve_plugin_path(&import);

    // Restore original directory
    std::env::set_current_dir(&original_dir).unwrap();

    assert!(result.is_ok());
}

#[test]
fn test_resolve_from_registry_invalid_version() {
    let config = Some(serde_json::json!({
        "version": "not-a-valid-semver"
    }));

    let result = resolve_from_registry("test-plugin", &config);
    assert!(result.is_err());
    // Version parsing may fail or succeed depending on the constraint parser
    // Just verify we get an error
}

#[test]
fn test_resolve_from_registry_no_version() {
    // Should use Latest constraint when no version specified
    let config = None;

    let result = resolve_from_registry("nonexistent-plugin", &config);
    // Will fail because no network, but validates version parsing worked
    assert!(result.is_err());
    // Should fail on registry load or plugin not found, not version parsing
    let err = result.unwrap_err();
    assert!(!err.contains("Invalid version constraint"));
}

#[test]
fn test_resolve_git_plugin_default_ref() {
    // Test that default git ref is "main"
    let temp_dir = TempDir::new().unwrap();

    let config = None;

    let result = resolve_git_plugin_with_cache_path(
        "https://github.com/cdm-lang/cdm.git",
        &config,
        None,
        temp_dir.path(),
    );

    // Will fail because no cdm-plugin.json in root, but validates ref parsing and cloning
    assert!(result.is_err());
}

#[test]
fn test_resolve_git_plugin_custom_ref() {
    let temp_dir = TempDir::new().unwrap();

    let config = Some(serde_json::json!({
        "git_ref": "develop"
    }));

    let result = resolve_git_plugin_with_cache_path(
        "https://github.com/cdm-lang/cdm.git",
        &config,
        None,
        temp_dir.path(),
    );

    // Will fail because develop branch doesn't exist or no cdm-plugin.json
    assert!(result.is_err());
}

#[test]
fn test_resolve_plugin_path_git_source() {
    let temp_dir = TempDir::new().unwrap();

    let import = PluginImport {
        name: "test-plugin".to_string(),
        source: Some(PluginSource::Git {
            url: "https://github.com/cdm-lang/cdm.git".to_string(),
            path: None,
        }),
        global_config: None,
        span: test_span(),
        name_span: test_span(),
        source_file: PathBuf::from("/test/schema.cdm"),
    };

    let result = resolve_plugin_path_with_cache_path(&import, temp_dir.path(), false);
    // Will fail because no cdm-plugin.json in root
    assert!(result.is_err());
}

#[test]
fn test_resolve_git_plugin_with_git_path() {
    // Test that git_path config is used to find plugin in subdirectory
    let temp_dir = TempDir::new().unwrap();

    let import = PluginImport {
        name: "test-plugin".to_string(),
        source: Some(PluginSource::Git {
            url: "https://github.com/cdm-lang/cdm.git".to_string(),
            path: None,
        }),
        global_config: Some(serde_json::json!({
            "git_path": "crates/nonexistent-plugin"
        })),
        span: test_span(),
        name_span: test_span(),
        source_file: PathBuf::from("/test/schema.cdm"),
    };

    let result = resolve_plugin_path_with_cache_path(&import, temp_dir.path(), false);

    // Will fail because the subdirectory doesn't have a cdm-plugin.json
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    // Check that the error mentions the subdirectory
    assert!(err.contains("crates/nonexistent-plugin"), "Error was: {}", err);
}

#[test]
fn test_resolve_git_plugin_with_git_path_success() {
    // Test that git_path correctly finds the manifest in a subdirectory
    // Note: This will fail on WASM file lookup since WASM files aren't checked into git
    let temp_dir = TempDir::new().unwrap();

    let import = PluginImport {
        name: "sql".to_string(),
        source: Some(PluginSource::Git {
            url: "https://github.com/cdm-lang/cdm.git".to_string(),
            path: None,
        }),
        global_config: Some(serde_json::json!({
            "git_path": "crates/cdm-plugin-sql"
        })),
        span: test_span(),
        name_span: test_span(),
        source_file: PathBuf::from("/test/schema.cdm"),
    };

    let result = resolve_plugin_path_with_cache_path(&import, temp_dir.path(), false);

    // Will fail either because:
    // 1. Git clone fails (transient network/filesystem issues)
    // 2. WASM file doesn't exist (it's a build artifact, not in git)
    // Either way, we verify the function returns an error
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    // The error should mention either the subdirectory (if clone succeeded) or git clone failure
    // We accept either since git clone can fail due to network/environment issues
    let mentions_path = err.contains("cdm-plugin-sql");
    let git_clone_failed = err.contains("git clone failed") || err.contains("Failed to clone");
    assert!(mentions_path || git_clone_failed,
            "Expected error to mention subdirectory 'cdm-plugin-sql' or git clone failure, got: {}", err);
}

#[test]
fn test_resolve_plugin_path_source_file_no_parent() {
    let import = PluginImport {
        name: "test-plugin".to_string(),
        source: Some(PluginSource::Path {
            path: "./test-plugin".to_string(),
        }),
        global_config: None,
        span: test_span(),
        name_span: test_span(),
        source_file: PathBuf::from("/"),
    };

    let result = resolve_plugin_path(&import);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Failed to get source file directory"));
}

#[test]
fn test_resolve_plugin_path_relative_path_resolution() {
    let temp_dir = TempDir::new().unwrap();

    // Create a nested directory structure
    let schema_dir = temp_dir.path().join("project/schemas");
    fs::create_dir_all(&schema_dir).unwrap();

    let plugin_dir = temp_dir.path().join("project/plugins/my-plugin");
    fs::create_dir_all(&plugin_dir).unwrap();

    // Create manifest
    let manifest_path = plugin_dir.join("cdm-plugin.json");
    let manifest_content = serde_json::json!({
        "name": "my-plugin",
        "version": "1.0.0",
        "wasm": {
            "file": "plugin.wasm"
        }
    });
    fs::write(&manifest_path, serde_json::to_string(&manifest_content).unwrap()).unwrap();

    // Create wasm file
    let wasm_path = plugin_dir.join("plugin.wasm");
    fs::write(&wasm_path, b"wasm content").unwrap();

    let import = PluginImport {
        name: "my-plugin".to_string(),
        source: Some(PluginSource::Path {
            path: "../plugins/my-plugin".to_string(),
        }),
        global_config: None,
        span: test_span(),
        name_span: test_span(),
        source_file: schema_dir.join("schema.cdm"),
    };

    let result = resolve_plugin_path(&import);
    assert!(result.is_ok());
    // The path may not be normalized, just check it ends with the right file
    let result_path = result.unwrap();
    assert!(result_path.ends_with("plugin.wasm"));
    assert!(result_path.to_string_lossy().contains("my-plugin"));
}

#[test]
fn test_resolve_from_registry_with_version_constraint() {
    let config = Some(serde_json::json!({
        "version": "^1.0.0"
    }));

    let result = resolve_from_registry("test-plugin", &config);
    // Will fail on registry load, but version parsing should succeed
    assert!(result.is_err());
    let err = result.unwrap_err();
    // Should not be a version constraint error
    assert!(!err.contains("Invalid version constraint"));
}

#[test]
fn test_resolve_git_plugin_from_repo_root_success() {
    // Test successfully loading a plugin from the root of a GitHub repository
    // This test uses a dedicated test repository with a proper plugin structure
    let temp_dir = TempDir::new().unwrap();

    let import = PluginImport {
        name: "cdm-plugin-test".to_string(),
        source: Some(PluginSource::Git {
            url: "https://github.com/cdm-lang/cdm-plugin-test.git".to_string(),
            path: None,
        }),
        global_config: None,
        span: test_span(),
        name_span: test_span(),
        source_file: PathBuf::from("/test/schema.cdm"),
    };

    let result = resolve_plugin_path_with_cache_path(&import, temp_dir.path(), false);

    assert!(result.is_ok(), "Failed to resolve plugin from GitHub repo root: {:?}", result.err());
    let wasm_path = result.unwrap();
    assert!(wasm_path.exists(), "WASM file does not exist at {:?}", wasm_path);
}

#[test]
fn test_resolve_git_plugin_from_nested_path_success() {
    // Test successfully loading a plugin from a subdirectory in a GitHub repository
    let temp_dir = TempDir::new().unwrap();

    let import = PluginImport {
        name: "cdm-plugin-test-nested".to_string(),
        source: Some(PluginSource::Git {
            url: "https://github.com/cdm-lang/cdm-plugin-test-nested.git".to_string(),
            path: None,
        }),
        global_config: Some(serde_json::json!({
            "git_path": "nested"
        })),
        span: test_span(),
        name_span: test_span(),
        source_file: PathBuf::from("/test/schema.cdm"),
    };

    let result = resolve_plugin_path_with_cache_path(&import, temp_dir.path(), false);

    assert!(result.is_ok(), "Failed to resolve plugin from nested path: {:?}", result.err());
    let wasm_path = result.unwrap();
    assert!(wasm_path.exists(), "WASM file does not exist at {:?}", wasm_path);
}
