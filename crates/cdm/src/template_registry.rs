use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::registry::get_cache_path;

/// Registry containing all available templates
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TemplateRegistry {
    pub version: u32,
    pub updated_at: String,
    pub templates: HashMap<String, RegistryTemplate>,
}

/// Template metadata from registry
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegistryTemplate {
    pub description: String,
    pub repository: String,
    pub official: bool,
    pub versions: HashMap<String, RegistryTemplateVersion>,
    pub latest: String,
}

/// Version-specific template metadata
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegistryTemplateVersion {
    /// Git URL for the template
    pub git_url: String,
    /// Git ref (tag, branch, commit) for this version
    pub git_ref: String,
    /// Optional subdirectory within the repository
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_path: Option<String>,
}

/// Template registry cache metadata
#[derive(Debug, Clone, Deserialize, Serialize)]
struct TemplateRegistryMeta {
    fetched_at: u64,
    expires_at: u64,
}

/// Load template registry from cache or fetch fresh copy
pub fn load_template_registry() -> Result<TemplateRegistry> {
    load_template_registry_with_cache_path(&get_cache_path()?)
}

/// Load template registry from cache or fetch fresh copy with explicit cache path (for testing)
pub(crate) fn load_template_registry_with_cache_path(cache_path: &Path) -> Result<TemplateRegistry> {
    let registry_file = cache_path.join("templates.json");
    let meta_file = cache_path.join("templates.meta.json");

    // Check if cached registry exists and is fresh
    if registry_file.exists() && meta_file.exists() {
        if let Ok(meta_content) = fs::read_to_string(&meta_file) {
            if let Ok(meta) = serde_json::from_str::<TemplateRegistryMeta>(&meta_content) {
                let now = current_timestamp();

                // Check if cache is still valid
                if now < meta.expires_at {
                    // Use cached registry
                    let content = fs::read_to_string(&registry_file)
                        .context("Failed to read cached template registry")?;
                    return serde_json::from_str(&content)
                        .context("Failed to parse cached template registry");
                }
            }
        }
    }

    // Fetch fresh registry
    fetch_and_cache_template_registry_with_cache_path(cache_path)
}

/// Force refresh the template registry from remote
#[allow(dead_code)]
pub fn refresh_template_registry() -> Result<TemplateRegistry> {
    fetch_and_cache_template_registry_with_cache_path(&get_cache_path()?)
}

/// Fetch template registry from remote URL and cache it
fn fetch_and_cache_template_registry_with_cache_path(cache_path: &Path) -> Result<TemplateRegistry> {
    let registry_url = get_template_registry_url();

    // Use reqwest to fetch
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("Failed to create HTTP client")?;

    let response = client
        .get(&registry_url)
        .send()
        .context("Failed to fetch template registry")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to fetch template registry: HTTP {} from {}",
            response.status(),
            registry_url
        );
    }

    let content = response.text().context("Failed to read template registry response")?;
    let registry: TemplateRegistry = serde_json::from_str(&content)
        .context("Failed to parse template registry JSON")?;

    // Cache registry
    fs::create_dir_all(cache_path)
        .context("Failed to create cache directory")?;
    fs::write(cache_path.join("templates.json"), &content)
        .context("Failed to write template registry cache")?;

    // Write metadata with expiration
    let now = current_timestamp();
    let ttl = get_cache_ttl();
    let meta = TemplateRegistryMeta {
        fetched_at: now,
        expires_at: now + ttl,
    };

    fs::write(
        cache_path.join("templates.meta.json"),
        serde_json::to_string_pretty(&meta)?,
    )
    .context("Failed to write template registry metadata")?;

    Ok(registry)
}

/// Get template registry URL from environment or default
pub fn get_template_registry_url() -> String {
    std::env::var("CDM_TEMPLATE_REGISTRY_URL").unwrap_or_else(|_| {
        "https://raw.githubusercontent.com/cdm-lang/cdm/refs/heads/main/templates.json".to_string()
    })
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

/// Lookup a template in the registry by name
pub fn lookup_template<'a>(registry: &'a TemplateRegistry, name: &str) -> Option<&'a RegistryTemplate> {
    registry.templates.get(name)
}

/// Get a specific version of a template, or the latest if version is None
pub fn get_template_version<'a>(
    template: &'a RegistryTemplate,
    version: Option<&str>,
) -> Option<&'a RegistryTemplateVersion> {
    match version {
        Some(v) => template.versions.get(v),
        None => template.versions.get(&template.latest),
    }
}

#[cfg(test)]
#[path = "template_registry/template_registry_tests.rs"]
mod template_registry_tests;
