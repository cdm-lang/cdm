use anyhow::{Context, Result};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::template_registry;

/// List available templates from registry or cached templates
pub fn list_templates(cached: bool) -> Result<()> {
    if cached {
        // List cached templates
        let cached_templates = list_cached_templates()?;

        if cached_templates.is_empty() {
            println!("No cached templates found");
            println!("\nTo cache a template, use: cdm template cache <name>");
            return Ok(());
        }

        println!("Cached templates:\n");
        for (name, version, path) in cached_templates {
            println!("  {}@{}", name, version);
            println!("    Path: {}", path.display());
            println!();
        }
    } else {
        // List registry templates
        let registry = template_registry::load_template_registry()?;

        println!("Available templates from registry:");
        println!("(Registry updated: {})\n", registry.updated_at);

        let mut templates: Vec<_> = registry.templates.iter().collect();
        templates.sort_by_key(|(name, _)| *name);

        for (name, template) in templates {
            let official = if template.official { " [official]" } else { "" };
            println!("  {}{}", name, official);
            println!("    {}", template.description);
            println!("    Latest: {} ({} versions available)", template.latest, template.versions.len());
            println!();
        }

        println!("Use 'cdm template info <name>' for details about a specific template");
        println!("Use 'cdm template list --cached' to see cached templates");
    }

    Ok(())
}

/// Show information about a specific template
pub fn template_info(name: &str, show_versions: bool) -> Result<()> {
    let registry = template_registry::load_template_registry()?;

    let template = registry
        .templates
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Template '{}' not found in registry", name))?;

    println!("Template: {}", name);
    println!("Description: {}", template.description);
    println!("Repository: {}", template.repository);
    println!("Official: {}", if template.official { "yes" } else { "no" });
    println!("Latest version: {}", template.latest);
    println!("Total versions: {}", template.versions.len());

    if show_versions {
        println!("\nAvailable versions:");
        let mut versions: Vec<_> = template.versions.keys().collect();

        // Try to sort by semver
        versions.sort_by(|a, b| {
            match (semver::Version::parse(a), semver::Version::parse(b)) {
                (Ok(va), Ok(vb)) => vb.cmp(&va), // Reverse order (newest first)
                _ => b.cmp(a), // Fallback to string comparison
            }
        });

        for version in versions {
            let ver_info = &template.versions[version];
            let is_latest = version == &template.latest;
            let latest_marker = if is_latest { " (latest)" } else { "" };
            println!("  {}{}", version, latest_marker);
            println!("    Git URL: {}", ver_info.git_url);
            println!("    Git ref: {}", ver_info.git_ref);
            if let Some(ref path) = ver_info.git_path {
                println!("    Git path: {}", path);
            }
        }
    } else {
        println!("\nUse --versions flag to see all available versions");
    }

    // Check if template is cached
    if is_template_cached(name, &template.latest)? {
        println!("\n✓ Latest version is cached locally");
    } else {
        println!("\nℹ Not cached. Use 'cdm template cache {}' to cache it", name);
    }

    Ok(())
}

/// Cache a template for offline use
pub fn cache_template_cmd(name: Option<&str>, all: bool) -> Result<()> {
    if all {
        // Cache all templates used in current project
        cache_project_templates()?;
    } else if let Some(template_name) = name {
        // Cache specific template
        cache_single_template(template_name)?;
    } else {
        anyhow::bail!("Must specify template name or --all flag");
    }

    Ok(())
}

/// Cache a single template (latest version)
fn cache_single_template(name: &str) -> Result<()> {
    // Extract base template name (strip subpath exports like "/postgres")
    let base_name = extract_base_template_name(name);

    let registry = template_registry::load_template_registry()?;

    let template = registry
        .templates
        .get(&base_name)
        .ok_or_else(|| anyhow::anyhow!("Template '{}' not found in registry", base_name))?;

    // Use latest version
    let version = &template.latest;

    // Check if already cached
    if is_template_cached(&base_name, version)? {
        println!("✓ Template {}@{} is already cached", base_name, version);
        return Ok(());
    }

    let ver_info = template.versions.get(version)
        .ok_or_else(|| anyhow::anyhow!("Version {} not found for template {}", version, base_name))?;

    println!("Caching {}@{}...", base_name, version);

    // Use git clone to cache the template
    cache_template_from_git(&base_name, version, &ver_info.git_url, &ver_info.git_ref, ver_info.git_path.as_deref())?;

    println!("✓ Successfully cached {}@{}", base_name, version);

    Ok(())
}

/// Extract the base template name from a name that may include subpath exports.
///
/// Examples:
/// - "sql-types" -> "sql-types"
/// - "sql-types/postgres" -> "sql-types"
/// - "sql-types/postgres.cdm" -> "sql-types"
/// - "sql-types/postgres/v2" -> "sql-types"
/// - "cdm/auth" -> "cdm/auth" (scoped name)
/// - "cdm/auth/types" -> "cdm/auth"
///
/// Heuristic: A template name contains a dash (e.g., "sql-types"), while a scope
/// is a short, simple word without dashes (e.g., "cdm", "org").
fn extract_base_template_name(name: &str) -> String {
    // Remove any .cdm extension
    let name = name.trim_end_matches(".cdm");

    let parts: Vec<&str> = name.split('/').collect();

    if parts.len() >= 3 {
        let first_part = parts[0];
        // If first part has a dash, it's a template name, not a scope
        if first_part.contains('-') {
            // "sql-types/postgres/v2" -> "sql-types"
            parts[0].to_string()
        } else {
            // "cdm/auth/types" -> "cdm/auth"
            format!("{}/{}", parts[0], parts[1])
        }
    } else if parts.len() == 2 {
        let first_part = parts[0];
        // If first part has a dash, it's a template name (not a scope)
        if first_part.contains('-') {
            first_part.to_string()
        } else {
            // Scoped name like "cdm/auth"
            name.to_string()
        }
    } else {
        name.to_string()
    }
}

/// Cache a template from a git repository
fn cache_template_from_git(
    name: &str,
    version: &str,
    git_url: &str,
    git_ref: &str,
    git_path: Option<&str>,
) -> Result<()> {
    use crate::git_plugin;
    use crate::registry;

    let cache_path = registry::get_cache_path()?;

    // Clone or update git repository
    let repo_path = git_plugin::clone_git_plugin_with_cache_path(git_url, git_ref, &cache_path)
        .map_err(|e| anyhow::anyhow!("Failed to clone git repository '{}': {}", git_url, e))?;

    // Navigate to subdirectory if specified
    let template_dir = if let Some(path) = git_path {
        repo_path.join(path)
    } else {
        repo_path
    };

    // Verify cdm-template.json exists
    let manifest_path = template_dir.join("cdm-template.json");
    if !manifest_path.exists() {
        anyhow::bail!(
            "No cdm-template.json found in template: {}\n\
            Git URL: {}\n\
            Git ref: {}\n\
            {}",
            template_dir.display(),
            git_url,
            git_ref,
            if let Some(p) = git_path {
                format!("Git path: {}", p)
            } else {
                String::new()
            }
        );
    }

    // Record the cache metadata
    let metadata_dir = cache_path.join("template_metadata").join(name.replace('/', "_"));
    std::fs::create_dir_all(&metadata_dir)?;

    let cached_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let metadata = serde_json::json!({
        "name": name,
        "version": version,
        "git_url": git_url,
        "git_ref": git_ref,
        "git_path": git_path,
        "cached_at": cached_at,
        "template_path": template_dir.to_string_lossy()
    });

    std::fs::write(
        metadata_dir.join(format!("{}.json", version)),
        serde_json::to_string_pretty(&metadata)?,
    )?;

    Ok(())
}

/// Cache all templates referenced in the current project
fn cache_project_templates() -> Result<()> {
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

        // Parse all .cdm files and extract templates
        let mut all_template_names = std::collections::HashSet::new();

        for entry in cdm_files {
            let path = entry.path();
            if let Ok(imports) = extract_template_imports_from_file(&path) {
                for import in imports {
                    // Only cache registry templates (skip local paths and git)
                    if let crate::TemplateSource::Registry { name } = import.source {
                        all_template_names.insert(name);
                    }
                }
            }
        }

        if all_template_names.is_empty() {
            println!("No registry templates found in .cdm files");
            return Ok(());
        }

        println!("Caching {} template(s)...\n", all_template_names.len());

        for template_name in all_template_names {
            cache_single_template(&template_name)?;
            println!();
        }

        println!("✓ All templates cached");
    } else {
        // Use schema.cdm
        let imports = extract_template_imports_from_file(&schema_path)?;

        let registry_templates: Vec<_> = imports
            .iter()
            .filter_map(|import| {
                if let crate::TemplateSource::Registry { name } = &import.source {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        if registry_templates.is_empty() {
            println!("No registry templates found in schema.cdm");
            return Ok(());
        }

        println!("Caching {} template(s) from schema.cdm...\n", registry_templates.len());

        for template_name in registry_templates {
            cache_single_template(&template_name)?;
            println!();
        }

        println!("✓ All templates cached");
    }

    Ok(())
}

/// Extract template imports from a CDM file
fn extract_template_imports_from_file(path: &Path) -> Result<Vec<crate::TemplateImport>> {
    use crate::extract_template_imports;

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

    Ok(extract_template_imports(tree.root_node(), &source, path))
}

/// List cached templates
fn list_cached_templates() -> Result<Vec<(String, String, std::path::PathBuf)>> {
    use crate::registry;

    let cache_path = registry::get_cache_path()?;
    let metadata_dir = cache_path.join("template_metadata");

    if !metadata_dir.exists() {
        return Ok(Vec::new());
    }

    let mut cached = Vec::new();

    for entry in std::fs::read_dir(&metadata_dir)? {
        let entry = entry?;
        let template_dir = entry.path();

        if template_dir.is_dir() {
            for version_file in std::fs::read_dir(&template_dir)? {
                let version_file = version_file?;
                let path = version_file.path();

                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(metadata) = serde_json::from_str::<serde_json::Value>(&content) {
                            let name = metadata.get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();
                            let version = metadata.get("version")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();
                            let template_path = metadata.get("template_path")
                                .and_then(|v| v.as_str())
                                .map(std::path::PathBuf::from)
                                .unwrap_or_default();

                            cached.push((name, version, template_path));
                        }
                    }
                }
            }
        }
    }

    Ok(cached)
}

/// Check if a template is cached
fn is_template_cached(name: &str, version: &str) -> Result<bool> {
    use crate::registry;

    let cache_path = registry::get_cache_path()?;
    let metadata_file = cache_path
        .join("template_metadata")
        .join(name.replace('/', "_"))
        .join(format!("{}.json", version));

    if !metadata_file.exists() {
        return Ok(false);
    }

    // Also verify the template directory still exists
    let content = std::fs::read_to_string(&metadata_file)?;
    let metadata: serde_json::Value = serde_json::from_str(&content)?;

    if let Some(template_path) = metadata.get("template_path").and_then(|v| v.as_str()) {
        let path = std::path::Path::new(template_path);
        Ok(path.exists() && path.join("cdm-template.json").exists())
    } else {
        Ok(false)
    }
}

/// Clear template cache
pub fn clear_template_cache_cmd(name: Option<&str>) -> Result<()> {
    use crate::registry;

    let cache_path = registry::get_cache_path()?;
    let metadata_dir = cache_path.join("template_metadata");

    if let Some(template_name) = name {
        let template_dir = metadata_dir.join(template_name.replace('/', "_"));
        if template_dir.exists() {
            println!("Clearing cache for '{}'...", template_name);
            std::fs::remove_dir_all(&template_dir)?;
            println!("✓ Cleared cache for '{}'", template_name);
        } else {
            println!("Template '{}' is not cached", template_name);
        }
    } else {
        println!("Clearing all template caches...");

        // Ask for confirmation
        println!("This will remove all cached template metadata. Continue? (y/N): ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if input.trim().to_lowercase() == "y" {
            if metadata_dir.exists() {
                std::fs::remove_dir_all(&metadata_dir)?;
            }
            println!("✓ Cleared all template caches");
        } else {
            println!("Cancelled");
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "template_commands/template_commands_tests.rs"]
mod template_commands_tests;
