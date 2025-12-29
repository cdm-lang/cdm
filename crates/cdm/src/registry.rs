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
#[path = "registry/registry_tests.rs"]
mod registry_tests;
