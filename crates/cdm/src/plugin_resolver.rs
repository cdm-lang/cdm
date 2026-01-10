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
use std::fs;

/// Resolve plugin path based on import specification
///
/// This function handles all three plugin source types:
/// 1. Local path plugins - resolved from cdm-plugin.json manifest
/// 2. Git plugins - cloned and cached
/// 3. Registry plugins - downloaded and cached from CDM registry
pub fn resolve_plugin_path(import: &PluginImport) -> Result<PathBuf> {
    use crate::registry;
    let cache_path = registry::get_cache_path()?;
    resolve_plugin_path_with_cache_path(import, &cache_path, false)
}

/// Resolve plugin path using only cached plugins (no downloads)
///
/// This is useful for LSP/editor integration where we don't want to block
/// on network requests. Returns an error if the plugin is not in cache.
pub fn resolve_plugin_path_cache_only(import: &PluginImport) -> Result<PathBuf> {
    use crate::registry;
    let cache_path = registry::get_cache_path()?;
    resolve_plugin_path_with_cache_path(import, &cache_path, true)
}

/// Resolve plugin path with explicit cache path (for testing)
pub(crate) fn resolve_plugin_path_with_cache_path(import: &PluginImport, cache_path: &std::path::Path, cache_only: bool) -> Result<PathBuf> {
    match &import.source {
        Some(PluginSource::Path { path }) => {
            let source_dir = import.source_file.parent()
                .context("Failed to get source file directory")?;
            let plugin_dir = source_dir.join(path);

            // Read cdm-plugin.json manifest
            let manifest_path = plugin_dir.join("cdm-plugin.json");
            if !manifest_path.exists() {
                anyhow::bail!(
                    "No cdm-plugin.json found in plugin directory: {}\n\
                    Local plugins must have a cdm-plugin.json file with a 'wasm.file' field",
                    plugin_dir.display()
                );
            }

            let manifest_content = fs::read_to_string(&manifest_path)
                .with_context(|| format!("Failed to read cdm-plugin.json at {}", manifest_path.display()))?;

            let manifest: serde_json::Value = serde_json::from_str(&manifest_content)
                .with_context(|| format!("Failed to parse cdm-plugin.json at {}", manifest_path.display()))?;

            // Get WASM file path from manifest
            let wasm_file = manifest
                .get("wasm")
                .and_then(|w| w.get("file"))
                .and_then(|f| f.as_str())
                .ok_or_else(|| anyhow::anyhow!(
                    "No 'wasm.file' field found in cdm-plugin.json at {}",
                    manifest_path.display()
                ))?;

            let wasm_path = plugin_dir.join(wasm_file);
            if !wasm_path.exists() {
                anyhow::bail!(
                    "WASM file not found: {}\n\
                    Specified in cdm-plugin.json as: {}\n\
                    Manifest location: {}",
                    wasm_path.display(),
                    wasm_file,
                    manifest_path.display()
                );
            }

            Ok(wasm_path)
        }
        Some(PluginSource::Git { url, path }) => {
            // Extract git_path from global config if provided, otherwise use path from source
            let git_path = import.global_config.as_ref()
                .and_then(|c| c.get("git_path"))
                .and_then(|p| p.as_str())
                .map(|s| s.to_string())
                .or_else(|| path.clone());

            resolve_git_plugin_with_cache_path(url, &import.global_config, git_path.as_deref(), cache_path)
                .map_err(|e| anyhow::anyhow!("{}", e))
        }
        None => {
            // Check default location: ./plugins/{name}.wasm
            let local = PathBuf::from("./plugins")
                .join(&import.name)
                .with_extension("wasm");

            if local.exists() {
                Ok(local)
            } else if cache_only {
                // In cache_only mode, just check cache without downloading
                resolve_from_registry_cache_only(&import.name, &import.global_config, cache_path)
                    .map_err(|e| anyhow::anyhow!("{}", e))
            } else {
                // Try to resolve from registry (may download)
                resolve_from_registry(&import.name, &import.global_config)
                    .map_err(|e| anyhow::anyhow!("{}", e))
            }
        }
    }
}

/// Resolve a plugin from cache only (no network requests)
///
/// This function only checks the local plugin cache. If the plugin isn't cached,
/// it returns an error suggesting to run `cdm build` to download it.
fn resolve_from_registry_cache_only(plugin_name: &str, config: &Option<JSON>, cache_path: &std::path::Path) -> Result<PathBuf, String> {
    use crate::{plugin_cache, version_resolver};

    // Extract version constraint from config
    let version_constraint = config
        .as_ref()
        .and_then(|c| c.get("version"))
        .and_then(|v| v.as_str());

    // List cached plugins to find matching version
    let cached_plugins = plugin_cache::list_cached_plugins_with_cache_path(cache_path)
        .map_err(|e| format!("Failed to read plugin cache: {}", e))?;

    // Find any cached version of this plugin
    let matching_plugins: Vec<_> = cached_plugins
        .iter()
        .filter(|(name, _, _)| name == plugin_name)
        .collect();

    if matching_plugins.is_empty() {
        return Err(format!(
            "Plugin '{}' not found in cache. Run 'cdm build' or 'cdm plugin install {}' to download it.",
            plugin_name, plugin_name
        ));
    }

    // If specific version requested, try to find it
    if let Some(version_str) = version_constraint {
        if let Ok(constraint) = version_resolver::parse_version_constraint(version_str) {
            for (_, version, _) in &matching_plugins {
                if version_resolver::version_matches(&constraint, version) {
                    if let Ok(Some(path)) = plugin_cache::get_cached_plugin_with_cache_path(plugin_name, version, cache_path) {
                        return Ok(path);
                    }
                }
            }
            return Err(format!(
                "Plugin '{}' version '{}' not found in cache. Run 'cdm build' to download it.",
                plugin_name, version_str
            ));
        }
    }

    // Return latest cached version (highest version number)
    let mut versions: Vec<_> = matching_plugins.iter().map(|(_, v, _)| v.as_str()).collect();
    versions.sort_by(|a, b| version_resolver::compare_versions(b, a));

    if let Some(version) = versions.first() {
        if let Ok(Some(path)) = plugin_cache::get_cached_plugin_with_cache_path(plugin_name, version, cache_path) {
            return Ok(path);
        }
    }

    Err(format!(
        "Plugin '{}' cache corrupted. Run 'cdm plugin clear-cache' and then 'cdm build' to re-download.",
        plugin_name
    ))
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
    let version = version_resolver::resolve_version(&version_constraint, &plugin.versions, Some(&plugin.latest))
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

/// Resolve a git plugin with explicit cache path (for testing)
pub(crate) fn resolve_git_plugin_with_cache_path(
    url: &str,
    config: &Option<JSON>,
    git_path: Option<&str>,
    cache_path: &std::path::Path,
) -> Result<PathBuf, String> {
    use crate::git_plugin;

    // Extract git ref from config (branch, tag, or commit)
    let git_ref = config
        .as_ref()
        .and_then(|c| c.get("git_ref"))
        .and_then(|v| v.as_str())
        .unwrap_or("main");

    // Clone or update git repository
    let repo_path = git_plugin::clone_git_plugin_with_cache_path(url, git_ref, cache_path)
        .map_err(|e| format!("Failed to clone git repository '{}': {}", url, e))?;

    // Extract WASM file (optionally from subdirectory)
    let wasm_path = git_plugin::extract_wasm_from_repo(&repo_path, git_path)
        .map_err(|e| format!("Failed to extract WASM from git repository: {}", e))?;

    Ok(wasm_path)
}

#[cfg(test)]
#[path = "plugin_resolver/plugin_resolver_tests.rs"]
mod plugin_resolver_tests;
