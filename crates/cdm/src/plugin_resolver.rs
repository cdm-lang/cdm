//! Plugin resolution logic shared across validate, build, and migrate commands
//!
//! This module provides unified functions for resolving plugins from different sources:
//! - Registry plugins (no `from` clause)
//! - Git plugins (`from git:...`)
//! - Local plugins (`from ./path`)

use crate::plugin_validation::{PluginImport, PluginSource};
use anyhow::{Result, Context};
use cdm_plugin_interface::JSON;
use std::path::PathBuf;

/// Resolve plugin path based on import specification
///
/// This function handles all three plugin source types:
/// 1. Local path plugins - resolved relative to source file
/// 2. Git plugins - cloned and cached
/// 3. Registry plugins - downloaded and cached from CDM registry
pub fn resolve_plugin_path(import: &PluginImport) -> Result<PathBuf> {
    match &import.source {
        Some(PluginSource::Path { path }) => {
            let source_dir = import.source_file.parent()
                .context("Failed to get source file directory")?;
            let mut wasm_path = source_dir.join(path);

            if !wasm_path.extension().map_or(false, |e| e == "wasm") {
                wasm_path.set_extension("wasm");
            }

            if wasm_path.exists() {
                Ok(wasm_path)
            } else {
                anyhow::bail!("Plugin WASM file not found: {}", wasm_path.display())
            }
        }
        Some(PluginSource::Git { url }) => {
            resolve_git_plugin(url, &import.name, &import.global_config)
                .map_err(|e| anyhow::anyhow!("{}", e))
        }
        None => {
            // Check default location: ./plugins/{name}.wasm
            let local = PathBuf::from("./plugins")
                .join(&import.name)
                .with_extension("wasm");

            if local.exists() {
                Ok(local)
            } else {
                // Try to resolve from registry
                resolve_from_registry(&import.name, &import.global_config)
                    .map_err(|e| anyhow::anyhow!("{}", e))
            }
        }
    }
}

/// Resolve a plugin from the CDM registry
///
/// This function:
/// 1. Loads the plugin registry (with caching)
/// 2. Resolves the version constraint from config
/// 3. Checks if the plugin is already cached
/// 4. Downloads and caches the plugin if needed
/// 5. Returns the path to the cached WASM file
pub fn resolve_from_registry(plugin_name: &str, config: &Option<JSON>) -> Result<PathBuf, String> {
    use crate::{registry, plugin_cache, version_resolver};

    // Extract version constraint from config
    let version_constraint = config
        .as_ref()
        .and_then(|c| c.get("version"))
        .and_then(|v| v.as_str())
        .map(|s| version_resolver::parse_version_constraint(s))
        .transpose()
        .map_err(|e| format!("Invalid version constraint: {}", e))?
        .unwrap_or(version_resolver::VersionConstraint::Latest);

    // Load registry
    let registry = registry::load_registry()
        .map_err(|e| {
            format!(
                "Failed to load plugin registry: {}\nCheck your internet connection or set CDM_REGISTRY_URL environment variable",
                e
            )
        })?;

    // Find plugin in registry
    let plugin = registry.plugins.get(plugin_name).ok_or_else(|| {
        format!(
            "Plugin '{}' not found in registry.\nAvailable plugins: {}",
            plugin_name,
            registry.plugins.keys().take(5).cloned().collect::<Vec<_>>().join(", ")
        )
    })?;

    // Resolve version
    let version = version_resolver::resolve_version(&version_constraint, &plugin.versions)
        .ok_or_else(|| {
            format!(
                "No version matching '{}' found for plugin '{}'.\nAvailable versions: {}",
                version_constraint,
                plugin_name,
                plugin.versions.keys().take(5).cloned().collect::<Vec<_>>().join(", ")
            )
        })?;

    // Check cache first
    if let Ok(Some(cached_path)) = plugin_cache::get_cached_plugin(plugin_name, &version) {
        return Ok(cached_path);
    }

    // Download and cache
    let plugin_version = &plugin.versions[&version];
    let wasm_path = plugin_cache::cache_plugin(
        plugin_name,
        &version,
        &plugin_version.wasm_url,
        &plugin_version.checksum,
    )
    .map_err(|e| format!("Failed to download plugin '{}': {}", plugin_name, e))?;

    Ok(wasm_path)
}

/// Resolve a git plugin
///
/// This function:
/// 1. Clones or updates the git repository
/// 2. Extracts the WASM file from the repository
/// 3. Returns the path to the WASM file
pub fn resolve_git_plugin(url: &str, plugin_name: &str, config: &Option<JSON>) -> Result<PathBuf, String> {
    use crate::git_plugin;

    // Extract git ref from config (branch, tag, or commit)
    let git_ref = config
        .as_ref()
        .and_then(|c| c.get("git_ref"))
        .and_then(|v| v.as_str())
        .unwrap_or("main");

    // Clone or update git repository
    let repo_path = git_plugin::clone_git_plugin(url, git_ref)
        .map_err(|e| format!("Failed to clone git repository '{}': {}", url, e))?;

    // Extract WASM file
    let wasm_path = git_plugin::extract_wasm_from_repo(&repo_path, plugin_name)
        .map_err(|e| format!("Failed to extract WASM from git repository: {}", e))?;

    Ok(wasm_path)
}
