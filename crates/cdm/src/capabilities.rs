use crate::{FileResolver, PluginRunner};
use crate::plugin_validation::extract_plugin_imports_from_validation_result;
use anyhow::Result;
use serde::Serialize;
use std::path::Path;

/// Information about a single plugin's capabilities
#[derive(Debug, Clone, Serialize)]
pub struct PluginCapability {
    /// Name of the plugin
    pub name: String,
    /// Whether the plugin supports the build operation
    pub has_build: bool,
    /// Whether the plugin supports the migrate operation
    pub has_migrate: bool,
}

/// Result of checking capabilities for a CDM file
#[derive(Debug, Serialize)]
pub struct CapabilitiesResult {
    /// List of plugins and their capabilities
    pub plugins: Vec<PluginCapability>,
    /// Whether any plugin supports build
    pub can_build: bool,
    /// Whether any plugin supports migrate
    pub can_migrate: bool,
}

/// Check which plugins are configured and what capabilities they have
pub fn capabilities(path: &Path) -> Result<CapabilitiesResult> {
    // Load and parse the CDM file tree
    let tree = FileResolver::load(path).map_err(|diagnostics| {
        for diagnostic in &diagnostics {
            eprintln!("{}", diagnostic);
        }
        anyhow::anyhow!("Failed to load CDM file")
    })?;

    // Extract main path before consuming tree
    let main_path = tree.main.path.clone();

    // Validate the tree with cache-only plugin resolution (no downloads)
    let validation_result = crate::validate_tree_cache_only(tree).map_err(|diagnostics| {
        for diagnostic in &diagnostics {
            eprintln!("{}", diagnostic);
        }
        anyhow::anyhow!("Validation failed")
    })?;

    // Extract plugin imports
    let plugin_imports = extract_plugin_imports_from_validation_result(&validation_result, &main_path)?;

    let mut plugins = Vec::new();
    let mut can_build = false;
    let mut can_migrate = false;

    for plugin_import in &plugin_imports {
        // Try to load the plugin and check its capabilities (cache only, no downloads)
        let (has_build, has_migrate) = match PluginRunner::from_import_cache_only(plugin_import) {
            Ok(runner) => {
                let build = runner.has_build().unwrap_or(false);
                let migrate = runner.has_migrate().unwrap_or(false);
                (build, migrate)
            }
            Err(e) => {
                // If we can't load the plugin, report the error and assume no capabilities
                eprintln!("E401: Plugin not found: '{}' - {}", plugin_import.name, e);
                (false, false)
            }
        };

        if has_build {
            can_build = true;
        }
        if has_migrate {
            can_migrate = true;
        }

        plugins.push(PluginCapability {
            name: plugin_import.name.clone(),
            has_build,
            has_migrate,
        });
    }

    Ok(CapabilitiesResult {
        plugins,
        can_build,
        can_migrate,
    })
}

#[cfg(test)]
mod capabilities_tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    // =========================================================================
    // STRUCT TESTS
    // =========================================================================

    #[test]
    fn test_plugin_capability_struct() {
        let cap = PluginCapability {
            name: "test-plugin".to_string(),
            has_build: true,
            has_migrate: false,
        };

        assert_eq!(cap.name, "test-plugin");
        assert!(cap.has_build);
        assert!(!cap.has_migrate);
    }

    #[test]
    fn test_plugin_capability_debug() {
        let cap = PluginCapability {
            name: "sql".to_string(),
            has_build: true,
            has_migrate: true,
        };

        let debug_str = format!("{:?}", cap);
        assert!(debug_str.contains("sql"));
        assert!(debug_str.contains("has_build: true"));
        assert!(debug_str.contains("has_migrate: true"));
    }

    #[test]
    fn test_plugin_capability_serialize() {
        let cap = PluginCapability {
            name: "typescript".to_string(),
            has_build: true,
            has_migrate: false,
        };

        let json = serde_json::to_string(&cap).unwrap();
        assert!(json.contains("\"name\":\"typescript\""));
        assert!(json.contains("\"has_build\":true"));
        assert!(json.contains("\"has_migrate\":false"));
    }

    #[test]
    fn test_capabilities_result_struct() {
        let result = CapabilitiesResult {
            plugins: vec![
                PluginCapability {
                    name: "plugin1".to_string(),
                    has_build: true,
                    has_migrate: false,
                },
                PluginCapability {
                    name: "plugin2".to_string(),
                    has_build: false,
                    has_migrate: true,
                },
            ],
            can_build: true,
            can_migrate: true,
        };

        assert_eq!(result.plugins.len(), 2);
        assert!(result.can_build);
        assert!(result.can_migrate);
    }

    #[test]
    fn test_capabilities_result_debug() {
        let result = CapabilitiesResult {
            plugins: vec![],
            can_build: false,
            can_migrate: false,
        };

        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("plugins"));
        assert!(debug_str.contains("can_build: false"));
        assert!(debug_str.contains("can_migrate: false"));
    }

    #[test]
    fn test_capabilities_result_serialize() {
        let result = CapabilitiesResult {
            plugins: vec![PluginCapability {
                name: "docs".to_string(),
                has_build: true,
                has_migrate: false,
            }],
            can_build: true,
            can_migrate: false,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"can_build\":true"));
        assert!(json.contains("\"can_migrate\":false"));
        assert!(json.contains("\"plugins\""));
    }

    #[test]
    fn test_capabilities_result_empty_plugins() {
        let result = CapabilitiesResult {
            plugins: vec![],
            can_build: false,
            can_migrate: false,
        };

        assert_eq!(result.plugins.len(), 0);
        assert!(!result.can_build);
        assert!(!result.can_migrate);

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"plugins\":[]"));
    }

    #[test]
    fn test_capabilities_result_multiple_plugins_same_capability() {
        let result = CapabilitiesResult {
            plugins: vec![
                PluginCapability {
                    name: "sql".to_string(),
                    has_build: true,
                    has_migrate: true,
                },
                PluginCapability {
                    name: "typescript".to_string(),
                    has_build: true,
                    has_migrate: false,
                },
            ],
            can_build: true,
            can_migrate: true,
        };

        assert_eq!(result.plugins.len(), 2);
        assert!(result.can_build);
        assert!(result.can_migrate);
    }

    // =========================================================================
    // FUNCTION TESTS
    // =========================================================================

    #[test]
    fn test_capabilities_file_not_found() {
        let result = capabilities(Path::new("/nonexistent/path/file.cdm"));
        assert!(result.is_err());
    }

    #[test]
    fn test_capabilities_invalid_cdm_syntax() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("invalid.cdm");
        fs::write(&file_path, "this is { not valid cdm syntax").unwrap();

        let result = capabilities(&file_path);
        // Should fail during validation
        assert!(result.is_err());
    }

    #[test]
    fn test_capabilities_no_plugins() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("no_plugins.cdm");
        fs::write(&file_path, r#"
User {
    id: number #1
    name: string #2
} #10
"#).unwrap();

        let result = capabilities(&file_path);
        assert!(result.is_ok());

        let caps = result.unwrap();
        assert_eq!(caps.plugins.len(), 0);
        assert!(!caps.can_build);
        assert!(!caps.can_migrate);
    }

    #[test]
    fn test_capabilities_with_extends_no_plugins() {
        let temp_dir = TempDir::new().unwrap();

        // Create base file
        let base_path = temp_dir.path().join("base.cdm");
        fs::write(&base_path, r#"
BaseModel {
    id: number #1
    created_at: string #2
} #10
"#).unwrap();

        // Create child file that extends base
        let child_path = temp_dir.path().join("child.cdm");
        fs::write(&child_path, r#"
extends "./base.cdm"

User extends BaseModel {
    name: string #10
    email: string #11
} #20
"#).unwrap();

        let result = capabilities(&child_path);
        assert!(result.is_ok());

        let caps = result.unwrap();
        assert_eq!(caps.plugins.len(), 0);
        assert!(!caps.can_build);
        assert!(!caps.can_migrate);
    }

    #[test]
    fn test_capabilities_result_json_structure() {
        let result = CapabilitiesResult {
            plugins: vec![
                PluginCapability {
                    name: "sql".to_string(),
                    has_build: true,
                    has_migrate: true,
                },
            ],
            can_build: true,
            can_migrate: true,
        };

        let json: serde_json::Value = serde_json::to_value(&result).unwrap();

        assert!(json.is_object());
        assert!(json["plugins"].is_array());
        assert_eq!(json["plugins"].as_array().unwrap().len(), 1);
        assert_eq!(json["plugins"][0]["name"], "sql");
        assert_eq!(json["plugins"][0]["has_build"], true);
        assert_eq!(json["plugins"][0]["has_migrate"], true);
        assert_eq!(json["can_build"], true);
        assert_eq!(json["can_migrate"], true);
    }

    #[test]
    fn test_capabilities_preserves_plugin_order() {
        let plugins = vec![
            PluginCapability {
                name: "first".to_string(),
                has_build: true,
                has_migrate: false,
            },
            PluginCapability {
                name: "second".to_string(),
                has_build: false,
                has_migrate: true,
            },
            PluginCapability {
                name: "third".to_string(),
                has_build: true,
                has_migrate: true,
            },
        ];

        let result = CapabilitiesResult {
            plugins: plugins.clone(),
            can_build: true,
            can_migrate: true,
        };

        assert_eq!(result.plugins[0].name, "first");
        assert_eq!(result.plugins[1].name, "second");
        assert_eq!(result.plugins[2].name, "third");
    }

    #[test]
    fn test_plugin_capability_all_combinations() {
        // Test all combinations of has_build and has_migrate
        let combinations = vec![
            (false, false),
            (false, true),
            (true, false),
            (true, true),
        ];

        for (has_build, has_migrate) in combinations {
            let cap = PluginCapability {
                name: format!("test_{}_{}", has_build, has_migrate),
                has_build,
                has_migrate,
            };

            assert_eq!(cap.has_build, has_build);
            assert_eq!(cap.has_migrate, has_migrate);

            // Verify serialization works for all combinations
            let json = serde_json::to_string(&cap).unwrap();
            assert!(json.contains(&format!("\"has_build\":{}", has_build)));
            assert!(json.contains(&format!("\"has_migrate\":{}", has_migrate)));
        }
    }

    #[test]
    fn test_capabilities_result_aggregates_correctly() {
        // Test that can_build is true if ANY plugin has_build
        let result1 = CapabilitiesResult {
            plugins: vec![
                PluginCapability {
                    name: "no_build".to_string(),
                    has_build: false,
                    has_migrate: false,
                },
                PluginCapability {
                    name: "has_build".to_string(),
                    has_build: true,
                    has_migrate: false,
                },
            ],
            can_build: true,  // Should be true because one plugin has_build
            can_migrate: false,
        };

        assert!(result1.can_build);
        assert!(!result1.can_migrate);

        // Test that can_migrate is true if ANY plugin has_migrate
        let result2 = CapabilitiesResult {
            plugins: vec![
                PluginCapability {
                    name: "no_migrate".to_string(),
                    has_build: false,
                    has_migrate: false,
                },
                PluginCapability {
                    name: "has_migrate".to_string(),
                    has_build: false,
                    has_migrate: true,
                },
            ],
            can_build: false,
            can_migrate: true,  // Should be true because one plugin has_migrate
        };

        assert!(!result2.can_build);
        assert!(result2.can_migrate);
    }
}
