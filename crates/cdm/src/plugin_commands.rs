use anyhow::{Context, Result};
use std::path::Path;

use crate::registry;
use crate::plugin_cache;

/// List available plugins from registry or cached plugins
pub fn list_plugins(cached: bool) -> Result<()> {
    if cached {
        // List cached plugins
        let cached_plugins = plugin_cache::list_cached_plugins()?;

        if cached_plugins.is_empty() {
            println!("No cached plugins found");
            println!("\nTo cache a plugin, use: cdm plugin cache <name>");
            return Ok(());
        }

        println!("Cached plugins:\n");
        for (name, version, meta) in cached_plugins {
            println!("  {}@{}", name, version);
            println!("    Downloaded: {}", meta.downloaded_at);
            match &meta.source {
                plugin_cache::CacheSource::Registry { registry_url } => {
                    println!("    Source: Registry ({})", registry_url);
                }
                plugin_cache::CacheSource::Git { url, commit } => {
                    println!("    Source: Git ({}, commit: {})", url, commit);
                }
                plugin_cache::CacheSource::Local { path } => {
                    println!("    Source: Local ({})", path);
                }
            }
            println!();
        }
    } else {
        // List registry plugins
        let registry = registry::load_registry()?;

        println!("Available plugins from registry:");
        println!("(Registry updated: {})\n", registry.updated_at);

        let mut plugins: Vec<_> = registry.plugins.iter().collect();
        plugins.sort_by_key(|(name, _)| *name);

        for (name, plugin) in plugins {
            let official = if plugin.official { " [official]" } else { "" };
            println!("  {}{}", name, official);
            println!("    {}", plugin.description);
            println!("    Latest: {} ({} versions available)", plugin.latest, plugin.versions.len());
            println!();
        }

        println!("Use 'cdm plugin info <name>' for details about a specific plugin");
        println!("Use 'cdm plugin list --cached' to see cached plugins");
    }

    Ok(())
}

/// Show information about a specific plugin
pub fn plugin_info(name: &str, show_versions: bool) -> Result<()> {
    let registry = registry::load_registry()?;

    let plugin = registry
        .plugins
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Plugin '{}' not found in registry", name))?;

    println!("Plugin: {}", name);
    println!("Description: {}", plugin.description);
    println!("Repository: {}", plugin.repository);
    println!("Official: {}", if plugin.official { "yes" } else { "no" });
    println!("Latest version: {}", plugin.latest);
    println!("Total versions: {}", plugin.versions.len());

    if show_versions {
        println!("\nAvailable versions:");
        let mut versions: Vec<_> = plugin.versions.keys().collect();

        // Try to sort by semver
        versions.sort_by(|a, b| {
            match (semver::Version::parse(a), semver::Version::parse(b)) {
                (Ok(va), Ok(vb)) => vb.cmp(&va), // Reverse order (newest first)
                _ => b.cmp(a), // Fallback to string comparison
            }
        });

        for version in versions {
            let ver_info = &plugin.versions[version];
            let is_latest = version == &plugin.latest;
            let latest_marker = if is_latest { " (latest)" } else { "" };
            println!("  {}{}", version, latest_marker);
            println!("    URL: {}", ver_info.wasm_url);
            println!("    Checksum: {}", ver_info.checksum);
        }
    } else {
        println!("\nUse --versions flag to see all available versions");
    }

    // Check if plugin is cached
    if let Ok(Some(_)) = plugin_cache::get_cached_plugin(name, &plugin.latest) {
        println!("\n✓ Latest version is cached locally");
    } else {
        println!("\nℹ Not cached. Use 'cdm plugin cache {}' to cache it", name);
    }

    Ok(())
}

/// Cache a plugin for offline use
pub fn cache_plugin_cmd(name: Option<&str>, all: bool) -> Result<()> {
    if all {
        // Cache all plugins used in current project
        cache_project_plugins()?;
    } else if let Some(plugin_name) = name {
        // Cache specific plugin
        cache_single_plugin(plugin_name)?;
    } else {
        anyhow::bail!("Must specify plugin name or --all flag");
    }

    Ok(())
}

/// Cache a single plugin (latest version)
fn cache_single_plugin(name: &str) -> Result<()> {
    let registry = registry::load_registry()?;

    let plugin = registry
        .plugins
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Plugin '{}' not found in registry", name))?;

    // Use latest version
    let version = &plugin.latest;

    // Check if already cached
    if let Ok(Some(cached_path)) = plugin_cache::get_cached_plugin(name, version) {
        println!("✓ Plugin {}@{} is already cached at {}", name, version, cached_path.display());
        return Ok(());
    }

    let ver_info = &plugin.versions[version];

    println!("Caching {}@{}...", name, version);

    plugin_cache::cache_plugin(name, version, &ver_info.wasm_url, &ver_info.checksum)?;

    println!("✓ Successfully cached {}@{}", name, version);

    Ok(())
}

/// Cache all plugins referenced in the current project
fn cache_project_plugins() -> Result<()> {
    // Look for schema.cdm or any .cdm files in current directory
    let current_dir = std::env::current_dir()?;
    let schema_path = current_dir.join("schema.cdm");

    if !schema_path.exists() {
        // Try to find any .cdm files
        let cdm_files: Vec<_> = std::fs::read_dir(&current_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "cdm")
                    .unwrap_or(false)
            })
            .collect();

        if cdm_files.is_empty() {
            anyhow::bail!(
                "No .cdm files found in current directory.\nRun this command from your CDM project root."
            );
        }

        println!("Found {} .cdm file(s) in current directory", cdm_files.len());

        // Parse all .cdm files and extract plugins
        let mut all_plugin_names = std::collections::HashSet::new();

        for entry in cdm_files {
            let path = entry.path();
            if let Ok(imports) = extract_plugin_imports_from_file(&path) {
                for import in imports {
                    // Only cache registry plugins (skip local paths and git)
                    if import.source.is_none() {
                        all_plugin_names.insert(import.name.clone());
                    }
                }
            }
        }

        if all_plugin_names.is_empty() {
            println!("No registry plugins found in .cdm files");
            return Ok(());
        }

        println!("Caching {} plugin(s)...\n", all_plugin_names.len());

        for plugin_name in all_plugin_names {
            cache_single_plugin(&plugin_name)?;
            println!();
        }

        println!("✓ All plugins cached");
    } else {
        // Use schema.cdm
        let imports = extract_plugin_imports_from_file(&schema_path)?;

        let registry_plugins: Vec<_> = imports
            .iter()
            .filter(|import| import.source.is_none())
            .collect();

        if registry_plugins.is_empty() {
            println!("No registry plugins found in schema.cdm");
            return Ok(());
        }

        println!("Caching {} plugin(s) from schema.cdm...\n", registry_plugins.len());

        for import in registry_plugins {
            cache_single_plugin(&import.name)?;
            println!();
        }

        println!("✓ All plugins cached");
    }

    Ok(())
}

/// Extract plugin imports from a CDM file
fn extract_plugin_imports_from_file(path: &Path) -> Result<Vec<crate::PluginImport>> {
    use crate::extract_plugin_imports;

    let source = std::fs::read_to_string(path)
        .context(format!("Failed to read file: {}", path.display()))?;

    // Parse the source using tree-sitter
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&grammar::LANGUAGE.into())
        .expect("Failed to load CDM grammar");

    let tree = parser
        .parse(&source, None)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse CDM file: {}", path.display()))?;

    Ok(extract_plugin_imports(tree.root_node(), &source, path))
}

/// Clear plugin cache
pub fn clear_cache_cmd(name: Option<&str>) -> Result<()> {
    if let Some(plugin_name) = name {
        println!("Clearing cache for '{}'...", plugin_name);
        plugin_cache::clear_plugin_cache(plugin_name)?;
        println!("✓ Cleared cache for '{}'", plugin_name);
    } else {
        println!("Clearing all plugin caches and registry...");

        // Ask for confirmation
        println!("This will remove all cached plugins and registry data. Continue? (y/N): ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if input.trim().to_lowercase() == "y" {
            plugin_cache::clear_all_cache()?;
            println!("✓ Cleared all plugin caches and registry");
        } else {
            println!("Cancelled");
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "plugin_commands/plugin_commands_tests.rs"]
mod plugin_commands_tests;
