use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Registry containing all available plugins
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Registry {
    pub version: u32,
    pub updated_at: String,
    pub plugins: HashMap<String, RegistryPlugin>,
}

/// Plugin metadata from registry
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegistryPlugin {
    pub description: String,
    pub repository: String,
    pub official: bool,
    pub versions: HashMap<String, RegistryVersion>,
    pub latest: String,
}

/// Version-specific plugin metadata
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegistryVersion {
    pub wasm_url: String,
    pub checksum: String,
}

/// Registry cache metadata
#[derive(Debug, Clone, Deserialize, Serialize)]
struct RegistryMeta {
    fetched_at: u64,    // Unix timestamp
    expires_at: u64,    // Unix timestamp
}

/// Load registry from cache or fetch fresh copy
pub fn load_registry() -> Result<Registry> {
    let cache_path = get_cache_path()?;
    let registry_file = cache_path.join("registry.json");
    let meta_file = cache_path.join("registry.meta.json");

    // Check if cached registry exists and is fresh
    if registry_file.exists() && meta_file.exists() {
        if let Ok(meta_content) = fs::read_to_string(&meta_file) {
            if let Ok(meta) = serde_json::from_str::<RegistryMeta>(&meta_content) {
                let now = current_timestamp();

                // Check if cache is still valid
                if now < meta.expires_at {
                    // Use cached registry
                    let content = fs::read_to_string(&registry_file)
                        .context("Failed to read cached registry")?;
                    return serde_json::from_str(&content)
                        .context("Failed to parse cached registry");
                }
            }
        }
    }

    // Fetch fresh registry
    fetch_and_cache_registry()
}

/// Force refresh the registry from remote
#[allow(dead_code)]
pub fn refresh_registry() -> Result<Registry> {
    fetch_and_cache_registry()
}

/// Fetch registry from remote URL and cache it
fn fetch_and_cache_registry() -> Result<Registry> {
    let registry_url = get_registry_url();

    // Use reqwest to fetch
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("Failed to create HTTP client")?;

    let response = client
        .get(&registry_url)
        .send()
        .context("Failed to fetch registry")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to fetch registry: HTTP {} from {}",
            response.status(),
            registry_url
        );
    }

    let content = response.text().context("Failed to read registry response")?;
    let registry: Registry = serde_json::from_str(&content)
        .context("Failed to parse registry JSON")?;

    // Cache registry
    let cache_path = get_cache_path()?;
    fs::write(cache_path.join("registry.json"), &content)
        .context("Failed to write registry cache")?;

    // Write metadata with expiration
    let now = current_timestamp();
    let ttl = get_cache_ttl();
    let meta = RegistryMeta {
        fetched_at: now,
        expires_at: now + ttl,
    };

    fs::write(
        cache_path.join("registry.meta.json"),
        serde_json::to_string_pretty(&meta)?,
    )
    .context("Failed to write registry metadata")?;

    Ok(registry)
}

/// Get registry URL from environment or default
pub fn get_registry_url() -> String {
    std::env::var("CDM_REGISTRY_URL").unwrap_or_else(|_| {
        "https://raw.githubusercontent.com/cdm-lang/cdm/refs/heads/main/registry.json".to_string()
    })
}

/// Get cache directory path, creating it if necessary
pub fn get_cache_path() -> Result<PathBuf> {
    // Allow override via environment variable
    if let Ok(cache_dir) = std::env::var("CDM_CACHE_DIR") {
        let path = PathBuf::from(cache_dir);
        fs::create_dir_all(&path)
            .context(format!("Failed to create cache directory: {}", path.display()))?;
        return Ok(path);
    }

    // Use platform-specific cache directory
    let cache_dir = if cfg!(target_os = "macos") {
        dirs::home_dir()
            .context("Could not determine home directory")?
            .join("Library")
            .join("Caches")
            .join("cdm")
    } else if cfg!(target_os = "windows") {
        dirs::cache_dir()
            .context("Could not determine cache directory")?
            .join("cdm")
    } else {
        // Linux and other Unix-like systems (follows XDG Base Directory spec)
        dirs::cache_dir()
            .context("Could not determine cache directory")?
            .join("cdm")
    };

    fs::create_dir_all(&cache_dir)
        .context(format!("Failed to create cache directory: {}", cache_dir.display()))?;

    Ok(cache_dir)
}

/// Get cache TTL in seconds from environment or default (24 hours)
fn get_cache_ttl() -> u64 {
    std::env::var("CDM_CACHE_TTL")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(86400) // 24 hours default
}

/// Get current Unix timestamp
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn test_parse_registry_json() {
        let json = r#"{
            "version": 1,
            "updated_at": "2024-01-15T10:30:00Z",
            "plugins": {
                "sql": {
                    "description": "Generate SQL schemas",
                    "repository": "git:https://github.com/cdm-lang/cdm-plugin-sql.git",
                    "official": true,
                    "versions": {
                        "1.0.0": {
                            "wasm_url": "https://example.com/plugin.wasm",
                            "checksum": "sha256:abc123"
                        }
                    },
                    "latest": "1.0.0"
                }
            }
        }"#;

        let registry: Registry = serde_json::from_str(json).unwrap();
        assert_eq!(registry.version, 1);
        assert_eq!(registry.plugins.len(), 1);

        let sql_plugin = registry.plugins.get("sql").unwrap();
        assert_eq!(sql_plugin.description, "Generate SQL schemas");
        assert_eq!(sql_plugin.official, true);
        assert_eq!(sql_plugin.latest, "1.0.0");
    }

    #[test]
    #[serial]
    fn test_get_registry_url_default() {
        // Save current value if it exists
        let saved = std::env::var("CDM_REGISTRY_URL").ok();

        unsafe {
            std::env::remove_var("CDM_REGISTRY_URL");
        }

        let url = get_registry_url();
        assert!(url.contains("github.com") || url.contains("cdm"));
        assert!(url.contains("registry.json"));

        // Restore previous value
        if let Some(val) = saved {
            unsafe {
                std::env::set_var("CDM_REGISTRY_URL", val);
            }
        }
    }

    #[test]
    #[serial]
    fn test_get_cache_ttl_default() {
        unsafe {
            std::env::remove_var("CDM_CACHE_TTL");
        }
        let ttl = get_cache_ttl();
        assert_eq!(ttl, 86400); // 24 hours
    }

    #[test]
    #[serial]
    fn test_get_cache_ttl_custom() {
        unsafe {
            std::env::set_var("CDM_CACHE_TTL", "3600");
        }
        let ttl = get_cache_ttl();
        assert_eq!(ttl, 3600);
        unsafe {
            std::env::remove_var("CDM_CACHE_TTL");
        }
    }

    #[test]
    #[serial]
    fn test_get_cache_ttl_invalid() {
        unsafe {
            std::env::set_var("CDM_CACHE_TTL", "invalid");
        }
        let ttl = get_cache_ttl();
        assert_eq!(ttl, 86400); // Should fallback to default
        unsafe {
            std::env::remove_var("CDM_CACHE_TTL");
        }
    }

    #[test]
    fn test_current_timestamp() {
        let ts = current_timestamp();
        assert!(ts > 0);
        // Check it's a reasonable timestamp (after 2020)
        assert!(ts > 1577836800); // Jan 1, 2020
    }

    #[test]
    #[serial]
    fn test_get_registry_url_custom() {
        unsafe {
            std::env::set_var("CDM_REGISTRY_URL", "https://custom-registry.com/registry.json");
        }
        let url = get_registry_url();
        assert_eq!(url, "https://custom-registry.com/registry.json");
        unsafe {
            std::env::remove_var("CDM_REGISTRY_URL");
        }
    }

    #[test]
    #[serial]
    fn test_get_cache_path() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("CDM_CACHE_DIR", temp_dir.path().to_str().unwrap());
        }

        let result = get_cache_path();
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.exists());
        assert!(path.is_dir());

        unsafe {
            std::env::remove_var("CDM_CACHE_DIR");
        }
    }

    #[test]
    #[serial]
    fn test_get_cache_path_default() {
        unsafe {
            std::env::remove_var("CDM_CACHE_DIR");
        }

        let result = get_cache_path();
        assert!(result.is_ok());

        let path = result.unwrap();
        // Default should be platform-specific cache directory containing "cdm"
        assert!(path.to_string_lossy().contains("cdm"));

        // Verify it's using the right platform-specific location
        if cfg!(target_os = "macos") {
            assert!(path.to_string_lossy().contains("Library/Caches"));
        } else if cfg!(target_os = "windows") {
            // Windows cache path typically contains "AppData" or "Local"
            let path_str = path.to_string_lossy();
            assert!(path_str.contains("Cache") || path_str.contains("cache"));
        } else {
            // Linux/Unix - should follow XDG spec
            assert!(path.to_string_lossy().contains("cache"));
        }

        unsafe {
            std::env::remove_var("CDM_CACHE_DIR");
        }
    }

    #[test]
    fn test_registry_serialization_round_trip() {
        let mut plugins = HashMap::new();
        plugins.insert(
            "test-plugin".to_string(),
            RegistryPlugin {
                description: "A test plugin".to_string(),
                repository: "https://github.com/test/plugin".to_string(),
                official: true,
                versions: {
                    let mut v = HashMap::new();
                    v.insert(
                        "1.0.0".to_string(),
                        RegistryVersion {
                            wasm_url: "https://example.com/plugin.wasm".to_string(),
                            checksum: "sha256:abc123".to_string(),
                        },
                    );
                    v
                },
                latest: "1.0.0".to_string(),
            },
        );

        let registry = Registry {
            version: 1,
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            plugins,
        };

        let json = serde_json::to_string(&registry).unwrap();
        let deserialized: Registry = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.version, 1);
        assert_eq!(deserialized.plugins.len(), 1);
        assert!(deserialized.plugins.contains_key("test-plugin"));
    }

    #[test]
    fn test_registry_plugin_with_multiple_versions() {
        let json = r#"{
            "description": "Multi-version plugin",
            "repository": "https://github.com/test/repo",
            "official": false,
            "versions": {
                "1.0.0": {
                    "wasm_url": "https://example.com/v1.wasm",
                    "checksum": "sha256:v1"
                },
                "2.0.0": {
                    "wasm_url": "https://example.com/v2.wasm",
                    "checksum": "sha256:v2"
                },
                "2.1.0": {
                    "wasm_url": "https://example.com/v2.1.wasm",
                    "checksum": "sha256:v2.1"
                }
            },
            "latest": "2.1.0"
        }"#;

        let plugin: RegistryPlugin = serde_json::from_str(json).unwrap();
        assert_eq!(plugin.versions.len(), 3);
        assert_eq!(plugin.latest, "2.1.0");
        assert!(!plugin.official);
    }

    #[test]
    fn test_registry_version_fields() {
        let version = RegistryVersion {
            wasm_url: "https://cdn.example.com/plugin.wasm".to_string(),
            checksum: "sha256:1234567890abcdef".to_string(),
        };

        let json = serde_json::to_string(&version).unwrap();
        let deserialized: RegistryVersion = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.wasm_url, "https://cdn.example.com/plugin.wasm");
        assert_eq!(deserialized.checksum, "sha256:1234567890abcdef");
    }

    #[test]
    #[serial]
    fn test_load_registry_cache_miss() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("CDM_CACHE_DIR", temp_dir.path().to_str().unwrap());
        }

        // No cached registry exists, should try to fetch
        // This will fail without network, but that's expected
        let result = load_registry();

        // Should either succeed (if network available) or fail with a fetch error
        if let Err(e) = result {
            let error_msg = e.to_string();
            // Error should be about fetching, not about cache
            assert!(
                error_msg.contains("fetch") || error_msg.contains("HTTP") || error_msg.contains("registry"),
                "Unexpected error: {}",
                error_msg
            );
        }

        unsafe {
            std::env::remove_var("CDM_CACHE_DIR");
        }
    }

    #[test]
    #[serial]
    fn test_load_registry_with_expired_cache() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        unsafe {
            std::env::set_var("CDM_CACHE_DIR", cache_dir.to_str().unwrap());
        }

        // Create expired cache
        let registry_file = cache_dir.join("registry.json");
        let meta_file = cache_dir.join("registry.meta.json");

        fs::write(
            &registry_file,
            r#"{"version": 1, "updated_at": "2020-01-01", "plugins": {}}"#,
        )
        .unwrap();

        let expired_meta = RegistryMeta {
            fetched_at: 1000000,
            expires_at: 1000001, // Already expired
        };
        fs::write(&meta_file, serde_json::to_string(&expired_meta).unwrap()).unwrap();

        // Should try to fetch fresh registry (will fail without network)
        let result = load_registry();

        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("fetch") || error_msg.contains("HTTP") || error_msg.contains("registry"),
                "Unexpected error: {}",
                error_msg
            );
        }

        unsafe {
            std::env::remove_var("CDM_CACHE_DIR");
        }
    }

    #[test]
    #[serial]
    fn test_load_registry_with_valid_cache() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        unsafe {
            std::env::set_var("CDM_CACHE_DIR", cache_dir.to_str().unwrap());
        }

        // Create valid cache that won't expire soon
        let registry_file = cache_dir.join("registry.json");
        let meta_file = cache_dir.join("registry.meta.json");

        let test_registry = Registry {
            version: 1,
            updated_at: "2024-01-01".to_string(),
            plugins: HashMap::new(),
        };

        fs::write(&registry_file, serde_json::to_string(&test_registry).unwrap()).unwrap();

        let now = current_timestamp();
        let valid_meta = RegistryMeta {
            fetched_at: now,
            expires_at: now + 86400, // Valid for 24 hours
        };
        fs::write(&meta_file, serde_json::to_string(&valid_meta).unwrap()).unwrap();

        // Should use cached registry
        let result = load_registry();
        assert!(result.is_ok());

        let registry = result.unwrap();
        assert_eq!(registry.version, 1);
        assert_eq!(registry.updated_at, "2024-01-01");

        unsafe {
            std::env::remove_var("CDM_CACHE_DIR");
        }
    }

    #[test]
    #[serial]
    fn test_load_registry_with_corrupted_cache() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path();

        unsafe {
            std::env::set_var("CDM_CACHE_DIR", cache_dir.to_str().unwrap());
        }

        // Create corrupted cache files
        let registry_file = cache_dir.join("registry.json");
        let meta_file = cache_dir.join("registry.meta.json");

        fs::write(&registry_file, "invalid json").unwrap();
        fs::write(&meta_file, "invalid json").unwrap();

        // Should try to fetch fresh registry (will fail without network)
        let result = load_registry();

        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("fetch") || error_msg.contains("HTTP") || error_msg.contains("registry") || error_msg.contains("parse"),
                "Unexpected error: {}",
                error_msg
            );
        }

        unsafe {
            std::env::remove_var("CDM_CACHE_DIR");
        }
    }
}
