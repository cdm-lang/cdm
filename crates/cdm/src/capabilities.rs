use crate::{FileResolver, PluginRunner};
use crate::plugin_validation::extract_plugin_imports_from_validation_result;
use anyhow::Result;
use serde::Serialize;
use std::path::Path;

/// Information about a single plugin's capabilities
#[derive(Debug, Serialize)]
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

    // Validate the tree (consumes tree)
    let validation_result = crate::validate_tree(tree).map_err(|diagnostics| {
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
        // Try to load the plugin and check its capabilities
        let (has_build, has_migrate) = match PluginRunner::from_import(plugin_import) {
            Ok(runner) => {
                let build = runner.has_build().unwrap_or(false);
                let migrate = runner.has_migrate().unwrap_or(false);
                (build, migrate)
            }
            Err(_) => {
                // If we can't load the plugin, assume no capabilities
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
    // Tests would go here
}
