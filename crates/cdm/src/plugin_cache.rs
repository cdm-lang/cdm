use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::registry;

#[cfg(test)]
use tempfile;

/// Cache metadata for a downloaded plugin
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CacheMetadata {
    pub plugin_name: String,
    pub version: String,
    pub downloaded_at: String,
    pub source: CacheSource,
    pub checksum: String,
}

/// Source of a cached plugin
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum CacheSource {
    Registry { registry_url: String },
    Git { url: String, commit: String },
    Local { path: String },
}

/// Download and cache a plugin WASM file
pub fn cache_plugin(
    name: &str,
    version: &str,
    wasm_url: &str,
    checksum: &str,
) -> Result<PathBuf> {
    let plugin_dir = get_plugin_cache_dir(name, version)?;
    let wasm_path = plugin_dir.join("plugin.wasm");

    // Download WASM file
    println!("Downloading plugin '{}'@{} from {}...", name, version, wasm_url);

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .context("Failed to create HTTP client")?;

    let response = client
        .get(wasm_url)
        .send()
        .context("Failed to download plugin")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "HTTP error {} while downloading plugin from {}",
            response.status(),
            wasm_url
        );
    }

    let bytes = response.bytes().context("Failed to read response bytes")?;

    // Verify checksum
    verify_checksum(&bytes, checksum)?;

    // Write to disk
    fs::write(&wasm_path, &bytes).context("Failed to write WASM file")?;

    // Write metadata
    let metadata = CacheMetadata {
        plugin_name: name.to_string(),
        version: version.to_string(),
        downloaded_at: current_timestamp_string(),
        source: CacheSource::Registry {
            registry_url: registry::get_registry_url(),
        },
        checksum: checksum.to_string(),
    };

    fs::write(
        plugin_dir.join("metadata.json"),
        serde_json::to_string_pretty(&metadata)?,
    )
    .context("Failed to write metadata")?;

    println!("Cached plugin to {}", wasm_path.display());

    Ok(wasm_path)
}

/// Get path to a cached plugin, if it exists and is valid
pub fn get_cached_plugin(name: &str, version: &str) -> Result<Option<PathBuf>> {
    let plugin_dir = registry::get_cache_path()?.join("plugins").join(format!("{}@{}", name, version));

    if !plugin_dir.exists() {
        return Ok(None);
    }

    let wasm_path = plugin_dir.join("plugin.wasm");
    let meta_path = plugin_dir.join("metadata.json");

    if !wasm_path.exists() || !meta_path.exists() {
        return Ok(None);
    }

    // Verify checksum if metadata exists
    if let Ok(meta_content) = fs::read_to_string(&meta_path) {
        if let Ok(meta) = serde_json::from_str::<CacheMetadata>(&meta_content) {
            if let Ok(wasm_bytes) = fs::read(&wasm_path) {
                if verify_checksum(&wasm_bytes, &meta.checksum).is_ok() {
                    return Ok(Some(wasm_path));
                }
            }
        }
    }

    Ok(None)
}

/// List all cached plugins
pub fn list_cached_plugins() -> Result<Vec<(String, String, CacheMetadata)>> {
    let plugins_dir = registry::get_cache_path()?.join("plugins");

    if !plugins_dir.exists() {
        return Ok(Vec::new());
    }

    let mut result = Vec::new();

    for entry in fs::read_dir(&plugins_dir)? {
        let entry = entry?;
        let dir_name = entry.file_name();
        let dir_name_str = dir_name.to_string_lossy();

        // Parse "{name}@{version}" format
        if let Some((name, version)) = dir_name_str.split_once('@') {
            let meta_path = entry.path().join("metadata.json");
            if meta_path.exists() {
                if let Ok(meta_content) = fs::read_to_string(&meta_path) {
                    if let Ok(meta) = serde_json::from_str::<CacheMetadata>(&meta_content) {
                        result.push((name.to_string(), version.to_string(), meta));
                    }
                }
            }
        }
    }

    Ok(result)
}

/// Clear cache for a specific plugin (all versions)
pub fn clear_plugin_cache(name: &str) -> Result<()> {
    let plugins_dir = registry::get_cache_path()?.join("plugins");

    if !plugins_dir.exists() {
        return Ok(());
    }

    // Remove all directories matching "{name}@*"
    for entry in fs::read_dir(&plugins_dir)? {
        let entry = entry?;
        let dir_name = entry.file_name();
        let dir_name_str = dir_name.to_string_lossy();

        if let Some((plugin_name, _)) = dir_name_str.split_once('@') {
            if plugin_name == name {
                fs::remove_dir_all(entry.path())
                    .context(format!("Failed to remove cache for plugin '{}'", name))?;
            }
        }
    }

    Ok(())
}

/// Clear entire plugin cache
pub fn clear_all_cache() -> Result<()> {
    let plugins_dir = registry::get_cache_path()?.join("plugins");

    if plugins_dir.exists() {
        fs::remove_dir_all(&plugins_dir)
            .context("Failed to remove plugins cache directory")?;
    }

    // Recreate the directory
    fs::create_dir_all(&plugins_dir)?;

    Ok(())
}

/// Verify checksum of downloaded data
fn verify_checksum(data: &[u8], expected_checksum: &str) -> Result<()> {
    // Parse expected checksum format: "sha256:hexstring"
    let parts: Vec<&str> = expected_checksum.split(':').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid checksum format: {}", expected_checksum);
    }

    let (algorithm, expected_hash) = (parts[0], parts[1]);

    match algorithm {
        "sha256" => {
            let mut hasher = Sha256::new();
            hasher.update(data);
            let actual_hash = format!("{:x}", hasher.finalize());

            if actual_hash != expected_hash {
                anyhow::bail!(
                    "Checksum mismatch!\n  Expected: sha256:{}\n  Actual:   sha256:{}",
                    expected_hash,
                    actual_hash
                );
            }
        }
        _ => anyhow::bail!("Unsupported checksum algorithm: {}", algorithm),
    }

    Ok(())
}

/// Get plugin-specific cache directory
fn get_plugin_cache_dir(name: &str, version: &str) -> Result<PathBuf> {
    let plugin_dir = registry::get_cache_path()?
        .join("plugins")
        .join(format!("{}@{}", name, version));

    fs::create_dir_all(&plugin_dir)
        .context(format!("Failed to create plugin cache directory for {}@{}", name, version))?;

    Ok(plugin_dir)
}

/// Get current timestamp as ISO 8601 string (simplified)
fn current_timestamp_string() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Simple timestamp format (Unix timestamp for now)
    timestamp.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn test_verify_checksum_valid() {
        let data = b"test data";
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = format!("{:x}", hasher.finalize());
        let checksum = format!("sha256:{}", hash);

        assert!(verify_checksum(data, &checksum).is_ok());
    }

    #[test]
    fn test_verify_checksum_invalid() {
        let data = b"test data";
        let checksum = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

        assert!(verify_checksum(data, checksum).is_err());
    }

    #[test]
    fn test_verify_checksum_bad_format() {
        let data = b"test data";
        let checksum = "invalid";

        assert!(verify_checksum(data, checksum).is_err());
    }

    #[test]
    fn test_verify_checksum_unsupported_algorithm() {
        let data = b"test data";
        let checksum = "md5:abc123";

        let result = verify_checksum(data, checksum);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }

    #[test]
    #[serial]
    fn test_get_plugin_cache_dir() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("CDM_CACHE_DIR", temp_dir.path().to_str().unwrap());
        }

        let result = get_plugin_cache_dir("test-plugin", "1.0.0");
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.exists());
        assert!(path.to_string_lossy().contains("test-plugin@1.0.0"));

        unsafe {
            std::env::remove_var("CDM_CACHE_DIR");
        }
    }

    #[test]
    fn test_current_timestamp_string() {
        let timestamp = current_timestamp_string();
        assert!(!timestamp.is_empty());
        // Should be a valid number (Unix timestamp)
        assert!(timestamp.parse::<u64>().is_ok());
    }

    #[test]
    #[serial]
    fn test_get_cached_plugin_not_exists() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("CDM_CACHE_DIR", temp_dir.path().to_str().unwrap());
        }

        let result = get_cached_plugin("nonexistent-plugin", "1.0.0");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);

        unsafe {
            std::env::remove_var("CDM_CACHE_DIR");
        }
    }

    #[test]
    #[serial]
    fn test_get_cached_plugin_missing_wasm() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("CDM_CACHE_DIR", temp_dir.path().to_str().unwrap());
        }

        // Create plugin directory with metadata but no WASM file
        let plugin_dir = temp_dir.path().join("plugins").join("test-plugin@1.0.0");
        fs::create_dir_all(&plugin_dir).unwrap();

        let metadata = CacheMetadata {
            plugin_name: "test-plugin".to_string(),
            version: "1.0.0".to_string(),
            downloaded_at: current_timestamp_string(),
            source: CacheSource::Registry {
                registry_url: "https://example.com".to_string(),
            },
            checksum: "sha256:abc123".to_string(),
        };
        fs::write(
            plugin_dir.join("metadata.json"),
            serde_json::to_string_pretty(&metadata).unwrap(),
        )
        .unwrap();

        let result = get_cached_plugin("test-plugin", "1.0.0");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);

        unsafe {
            std::env::remove_var("CDM_CACHE_DIR");
        }
    }

    #[test]
    #[serial]
    fn test_get_cached_plugin_missing_metadata() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("CDM_CACHE_DIR", temp_dir.path().to_str().unwrap());
        }

        // Create plugin directory with WASM but no metadata
        let plugin_dir = temp_dir.path().join("plugins").join("test-plugin@1.0.0");
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(plugin_dir.join("plugin.wasm"), b"fake wasm").unwrap();

        let result = get_cached_plugin("test-plugin", "1.0.0");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);

        unsafe {
            std::env::remove_var("CDM_CACHE_DIR");
        }
    }

    #[test]
    #[serial]
    fn test_get_cached_plugin_checksum_mismatch() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("CDM_CACHE_DIR", temp_dir.path().to_str().unwrap());
        }

        // Create plugin with valid metadata but wrong checksum
        let plugin_dir = temp_dir.path().join("plugins").join("test-plugin@1.0.0");
        fs::create_dir_all(&plugin_dir).unwrap();

        let wasm_data = b"test wasm data";
        fs::write(plugin_dir.join("plugin.wasm"), wasm_data).unwrap();

        // Wrong checksum
        let metadata = CacheMetadata {
            plugin_name: "test-plugin".to_string(),
            version: "1.0.0".to_string(),
            downloaded_at: current_timestamp_string(),
            source: CacheSource::Registry {
                registry_url: "https://example.com".to_string(),
            },
            checksum: "sha256:0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        };
        fs::write(
            plugin_dir.join("metadata.json"),
            serde_json::to_string_pretty(&metadata).unwrap(),
        )
        .unwrap();

        let result = get_cached_plugin("test-plugin", "1.0.0");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);

        unsafe {
            std::env::remove_var("CDM_CACHE_DIR");
        }
    }

    #[test]
    #[serial]
    fn test_get_cached_plugin_valid() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("CDM_CACHE_DIR", temp_dir.path().to_str().unwrap());
        }

        // Create valid cached plugin
        let plugin_dir = temp_dir.path().join("plugins").join("test-plugin@1.0.0");
        fs::create_dir_all(&plugin_dir).unwrap();

        let wasm_data = b"test wasm data";
        fs::write(plugin_dir.join("plugin.wasm"), wasm_data).unwrap();

        // Calculate correct checksum
        let mut hasher = Sha256::new();
        hasher.update(wasm_data);
        let hash = format!("{:x}", hasher.finalize());
        let checksum = format!("sha256:{}", hash);

        let metadata = CacheMetadata {
            plugin_name: "test-plugin".to_string(),
            version: "1.0.0".to_string(),
            downloaded_at: current_timestamp_string(),
            source: CacheSource::Registry {
                registry_url: "https://example.com".to_string(),
            },
            checksum,
        };
        fs::write(
            plugin_dir.join("metadata.json"),
            serde_json::to_string_pretty(&metadata).unwrap(),
        )
        .unwrap();

        let result = get_cached_plugin("test-plugin", "1.0.0");
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());

        unsafe {
            std::env::remove_var("CDM_CACHE_DIR");
        }
    }

    #[test]
    #[serial]
    fn test_list_cached_plugins_empty() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("CDM_CACHE_DIR", temp_dir.path().to_str().unwrap());
        }

        let result = list_cached_plugins();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);

        unsafe {
            std::env::remove_var("CDM_CACHE_DIR");
        }
    }

    #[test]
    #[serial]
    fn test_list_cached_plugins_multiple() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("CDM_CACHE_DIR", temp_dir.path().to_str().unwrap());
        }

        // Create multiple cached plugins
        let plugins_dir = temp_dir.path().join("plugins");
        fs::create_dir_all(&plugins_dir).unwrap();

        for (name, version) in [("plugin1", "1.0.0"), ("plugin2", "2.0.0")] {
            let plugin_dir = plugins_dir.join(format!("{}@{}", name, version));
            fs::create_dir_all(&plugin_dir).unwrap();

            let metadata = CacheMetadata {
                plugin_name: name.to_string(),
                version: version.to_string(),
                downloaded_at: current_timestamp_string(),
                source: CacheSource::Git {
                    url: "https://example.com".to_string(),
                    commit: "abc123".to_string(),
                },
                checksum: "sha256:test".to_string(),
            };
            fs::write(
                plugin_dir.join("metadata.json"),
                serde_json::to_string_pretty(&metadata).unwrap(),
            )
            .unwrap();
        }

        let result = list_cached_plugins();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);

        unsafe {
            std::env::remove_var("CDM_CACHE_DIR");
        }
    }

    #[test]
    #[serial]
    fn test_list_cached_plugins_invalid_format() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("CDM_CACHE_DIR", temp_dir.path().to_str().unwrap());
        }

        // Create directory with invalid name format
        let plugins_dir = temp_dir.path().join("plugins");
        fs::create_dir_all(&plugins_dir).unwrap();
        fs::create_dir_all(plugins_dir.join("invalid-name-no-version")).unwrap();

        let result = list_cached_plugins();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);

        unsafe {
            std::env::remove_var("CDM_CACHE_DIR");
        }
    }

    #[test]
    #[serial]
    fn test_clear_plugin_cache() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("CDM_CACHE_DIR", temp_dir.path().to_str().unwrap());
        }

        // Create multiple versions of same plugin
        let plugins_dir = temp_dir.path().join("plugins");
        fs::create_dir_all(&plugins_dir).unwrap();

        for version in ["1.0.0", "2.0.0"] {
            let plugin_dir = plugins_dir.join(format!("test-plugin@{}", version));
            fs::create_dir_all(&plugin_dir).unwrap();
        }

        // Also create a different plugin
        fs::create_dir_all(plugins_dir.join("other-plugin@1.0.0")).unwrap();

        // Clear test-plugin cache
        let result = clear_plugin_cache("test-plugin");
        assert!(result.is_ok());

        // test-plugin versions should be gone
        assert!(!plugins_dir.join("test-plugin@1.0.0").exists());
        assert!(!plugins_dir.join("test-plugin@2.0.0").exists());

        // other-plugin should still exist
        assert!(plugins_dir.join("other-plugin@1.0.0").exists());

        unsafe {
            std::env::remove_var("CDM_CACHE_DIR");
        }
    }

    #[test]
    #[serial]
    fn test_clear_plugin_cache_nonexistent() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("CDM_CACHE_DIR", temp_dir.path().to_str().unwrap());
        }

        // Should not error even if plugin doesn't exist
        let result = clear_plugin_cache("nonexistent-plugin");
        assert!(result.is_ok());

        unsafe {
            std::env::remove_var("CDM_CACHE_DIR");
        }
    }

    #[test]
    #[serial]
    fn test_clear_all_cache() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("CDM_CACHE_DIR", temp_dir.path().to_str().unwrap());
        }

        // Create several plugins
        let plugins_dir = temp_dir.path().join("plugins");
        fs::create_dir_all(&plugins_dir).unwrap();
        fs::create_dir_all(plugins_dir.join("plugin1@1.0.0")).unwrap();
        fs::create_dir_all(plugins_dir.join("plugin2@1.0.0")).unwrap();

        let result = clear_all_cache();
        assert!(result.is_ok());

        // plugins directory should exist but be empty
        assert!(plugins_dir.exists());
        assert!(fs::read_dir(&plugins_dir).unwrap().next().is_none());

        unsafe {
            std::env::remove_var("CDM_CACHE_DIR");
        }
    }

    #[test]
    fn test_cache_metadata_serialization() {
        let metadata = CacheMetadata {
            plugin_name: "test".to_string(),
            version: "1.0.0".to_string(),
            downloaded_at: "2024-01-01T00:00:00Z".to_string(),
            source: CacheSource::Local {
                path: "/path/to/plugin".to_string(),
            },
            checksum: "sha256:abc123".to_string(),
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: CacheMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.plugin_name, "test");
        assert_eq!(deserialized.version, "1.0.0");
    }

    #[test]
    fn test_cache_source_variants() {
        let registry = CacheSource::Registry {
            registry_url: "https://registry.com".to_string(),
        };
        let git = CacheSource::Git {
            url: "https://github.com/test/repo".to_string(),
            commit: "abc123".to_string(),
        };
        let local = CacheSource::Local {
            path: "/local/path".to_string(),
        };

        // Test serialization round-trip
        for source in [registry, git, local] {
            let json = serde_json::to_string(&source).unwrap();
            let _deserialized: CacheSource = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_verify_checksum_empty_data() {
        let data = b"";
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = format!("{:x}", hasher.finalize());
        let checksum = format!("sha256:{}", hash);

        assert!(verify_checksum(data, &checksum).is_ok());
    }

    #[test]
    fn test_verify_checksum_large_data() {
        let data = vec![0u8; 1024 * 1024]; // 1 MB of zeros
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let hash = format!("{:x}", hasher.finalize());
        let checksum = format!("sha256:{}", hash);

        assert!(verify_checksum(&data, &checksum).is_ok());
    }
}
