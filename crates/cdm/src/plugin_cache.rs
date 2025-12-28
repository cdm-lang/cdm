use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::registry;

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
#[path = "plugin_cache/plugin_cache_tests.rs"]
mod plugin_cache_tests;
