//! Template resolution logic for CDM templates
//!
//! This module provides unified functions for resolving templates from different sources:
//! - Registry templates (e.g., `sql/postgres-types`, `cdm/auth`)
//! - Git templates (`git:https://github.com/org/repo.git`)
//! - Local templates (`./templates/shared`)

use anyhow::{Context, Result};
use cdm_plugin_interface::JSON;
use cdm_utils::{EntityIdSource, Span};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Template manifest structure (cdm-template.json)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TemplateManifest {
    /// Template identifier (e.g., "cdm/auth")
    pub name: String,
    /// Semantic version
    pub version: String,
    /// Human-readable description
    pub description: String,
    /// Path to main CDM file (relative to manifest).
    /// Optional for export-only templates that only expose named exports.
    #[serde(default)]
    pub entry: Option<String>,
    /// Named export paths for selective importing
    #[serde(default)]
    pub exports: HashMap<String, String>,
}

/// Represents a template import directive
#[derive(Debug, Clone)]
pub struct TemplateImport {
    /// The namespace to use for accessing template definitions
    pub namespace: String,
    /// Where to load the template from
    pub source: TemplateSource,
    /// Optional configuration (version, git_ref, git_path, etc.)
    pub config: Option<JSON>,
    /// Source location for error messages
    pub span: Span,
    /// Path of the file containing this import (for relative path resolution)
    pub source_file: PathBuf,
}

/// Represents a template extends directive (merged import)
#[derive(Debug, Clone)]
pub struct TemplateExtends {
    /// Where to load the template from
    pub source: TemplateSource,
    /// Optional configuration (version, git_ref, git_path, etc.)
    pub config: Option<JSON>,
    /// Source location for error messages
    pub span: Span,
    /// Path of the file containing this extends (for relative path resolution)
    pub source_file: PathBuf,
}

/// Source of a template
#[derive(Debug, Clone)]
pub enum TemplateSource {
    /// Registry template (e.g., "sql/postgres-types", "cdm/auth")
    Registry { name: String },
    /// Git repository
    Git { url: String },
    /// Local path (relative to importing file)
    Local { path: String },
}

/// Loaded template with its manifest and content
#[derive(Debug, Clone)]
pub struct LoadedTemplate {
    /// Template manifest
    pub manifest: TemplateManifest,
    /// Absolute path to the template directory
    pub path: PathBuf,
    /// Absolute path to the entry file.
    /// None for export-only templates that have no main entry point.
    pub entry_path: Option<PathBuf>,
}

/// Extract template imports from a parsed AST
pub fn extract_template_imports(
    root: tree_sitter::Node,
    source: &str,
    source_file: &Path,
) -> Vec<TemplateImport> {
    let mut imports = Vec::new();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        if child.kind() == "template_import" {
            if let Some(import) = parse_template_import(child, source, source_file) {
                imports.push(import);
            }
        }
    }

    imports
}

/// Extract template extends from a parsed AST
pub fn extract_template_extends(
    root: tree_sitter::Node,
    source: &str,
    source_file: &Path,
) -> Vec<TemplateExtends> {
    let mut extends = Vec::new();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        if child.kind() == "extends_template" {
            if let Some(ext) = parse_template_extends(child, source, source_file) {
                extends.push(ext);
            }
        }
    }

    extends
}

/// Parse a single template_import node
fn parse_template_import(
    node: tree_sitter::Node,
    source: &str,
    source_file: &Path,
) -> Option<TemplateImport> {
    let namespace = node
        .child_by_field_name("namespace")?
        .utf8_text(source.as_bytes())
        .ok()?
        .to_string();

    let source_node = node.child_by_field_name("source")?;
    let template_source = parse_template_source(source_node, source)?;

    let config = node
        .child_by_field_name("config")
        .and_then(|c| parse_object_literal(c, source));

    let start = node.start_position();
    let end = node.end_position();

    Some(TemplateImport {
        namespace,
        source: template_source,
        config,
        span: Span {
            start: cdm_utils::Position {
                line: start.row,
                column: start.column,
            },
            end: cdm_utils::Position {
                line: end.row,
                column: end.column,
            },
        },
        source_file: source_file.to_path_buf(),
    })
}

/// Parse a single extends_template node
fn parse_template_extends(
    node: tree_sitter::Node,
    source: &str,
    source_file: &Path,
) -> Option<TemplateExtends> {
    let source_node = node.child_by_field_name("source")?;
    let template_source = parse_template_source(source_node, source)?;

    let config = node
        .child_by_field_name("config")
        .and_then(|c| parse_object_literal(c, source));

    let start = node.start_position();
    let end = node.end_position();

    Some(TemplateExtends {
        source: template_source,
        config,
        span: Span {
            start: cdm_utils::Position {
                line: start.row,
                column: start.column,
            },
            end: cdm_utils::Position {
                line: end.row,
                column: end.column,
            },
        },
        source_file: source_file.to_path_buf(),
    })
}

/// Parse a string_literal node into TemplateSource enum
///
/// The source string determines the type:
/// - Starts with "git:" → Git source (URL after "git:")
/// - Starts with "./" or "../" → Local path
/// - Otherwise → Registry name
fn parse_template_source(node: tree_sitter::Node, source: &str) -> Option<TemplateSource> {
    // Node is a string_literal, extract the text and strip quotes
    let text = node.utf8_text(source.as_bytes()).ok()?;

    // Strip surrounding quotes
    let value = if text.len() >= 2 && text.starts_with('"') && text.ends_with('"') {
        &text[1..text.len()-1]
    } else {
        return None;
    };

    // Determine source type based on string content
    if let Some(url) = value.strip_prefix("git:") {
        Some(TemplateSource::Git { url: url.to_string() })
    } else if value.starts_with("./") || value.starts_with("../") {
        Some(TemplateSource::Local { path: value.to_string() })
    } else {
        Some(TemplateSource::Registry { name: value.to_string() })
    }
}

/// Parse an object_literal node into JSON
fn parse_object_literal(node: tree_sitter::Node, source: &str) -> Option<JSON> {
    // Reuse existing parsing logic from plugin_validation
    crate::plugin_validation::parse_object_literal_to_json(node, source)
}

/// Resolve a template import to a loaded template
pub fn resolve_template(import: &TemplateImport) -> Result<LoadedTemplate> {
    resolve_template_from_source(&import.source, &import.config, &import.source_file)
}

/// Resolve a template extends to a loaded template
pub fn resolve_template_extends(extends: &TemplateExtends) -> Result<LoadedTemplate> {
    resolve_template_from_source(&extends.source, &extends.config, &extends.source_file)
}

/// Resolve a template from its source
pub fn resolve_template_from_source(
    source: &TemplateSource,
    config: &Option<JSON>,
    source_file: &Path,
) -> Result<LoadedTemplate> {
    match source {
        TemplateSource::Local { path } => resolve_local_template(path, source_file),
        TemplateSource::Git { url } => resolve_git_template(url, config),
        TemplateSource::Registry { name } => resolve_registry_template(name, config),
    }
}

/// Resolve a local template
fn resolve_local_template(path: &str, source_file: &Path) -> Result<LoadedTemplate> {
    let source_dir = source_file
        .parent()
        .context("Failed to get source file directory")?;
    let template_path = source_dir.join(path);

    // Check if this is a direct CDM file reference
    if path.ends_with(".cdm") {
        return resolve_local_template_file(&template_path);
    }

    // Otherwise, treat as a directory with a manifest
    resolve_local_template_dir(&template_path)
}

/// Resolve a direct CDM file as a template (no manifest required)
fn resolve_local_template_file(file_path: &Path) -> Result<LoadedTemplate> {
    if !file_path.exists() {
        anyhow::bail!(
            "Template file not found: {}",
            file_path.display()
        );
    }

    // Extract the file name for the manifest
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("template.cdm")
        .to_string();

    let template_dir = file_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    // Create a synthetic manifest for direct file imports
    let manifest = TemplateManifest {
        name: file_name.trim_end_matches(".cdm").to_string(),
        version: "0.0.0".to_string(),
        description: format!("Direct import from {}", file_path.display()),
        entry: Some(file_name),
        exports: HashMap::new(),
    };

    Ok(LoadedTemplate {
        manifest,
        path: template_dir
            .canonicalize()
            .unwrap_or_else(|_| template_dir.clone()),
        entry_path: Some(
            file_path
                .canonicalize()
                .unwrap_or_else(|_| file_path.to_path_buf()),
        ),
    })
}

/// Resolve a template directory with a manifest
fn resolve_local_template_dir(template_dir: &Path) -> Result<LoadedTemplate> {
    // Read manifest
    let manifest_path = template_dir.join("cdm-template.json");
    if !manifest_path.exists() {
        anyhow::bail!(
            "No cdm-template.json found in template directory: {}\n\
            Templates must have a cdm-template.json manifest file",
            template_dir.display()
        );
    }

    let manifest = load_manifest(&manifest_path)?;

    // Entry is optional - some templates only expose named exports
    let entry_path = if let Some(ref entry) = manifest.entry {
        let path = template_dir.join(entry);
        if !path.exists() {
            anyhow::bail!(
                "Template entry file not found: {}\n\
                Specified in cdm-template.json as: {}",
                path.display(),
                entry
            );
        }
        Some(path.canonicalize().unwrap_or(path))
    } else {
        None
    };

    Ok(LoadedTemplate {
        manifest,
        path: template_dir
            .canonicalize()
            .unwrap_or_else(|_| template_dir.to_path_buf()),
        entry_path,
    })
}

/// Resolve a git template
fn resolve_git_template(url: &str, config: &Option<JSON>) -> Result<LoadedTemplate> {
    use crate::git_plugin;
    use crate::registry;

    let cache_path = registry::get_cache_path()?;

    // Extract git ref from config
    let git_ref = config
        .as_ref()
        .and_then(|c| c.get("git_ref"))
        .and_then(|v| v.as_str())
        .unwrap_or("main");

    // Extract git path from config
    let git_path = config
        .as_ref()
        .and_then(|c| c.get("git_path"))
        .and_then(|v| v.as_str());

    // Clone or update git repository (reuse plugin caching)
    let repo_path = git_plugin::clone_git_plugin_with_cache_path(url, git_ref, &cache_path)
        .map_err(|e| anyhow::anyhow!("Failed to clone git repository '{}': {}", url, e))?;

    // Navigate to subdirectory if specified
    let template_dir = if let Some(path) = git_path {
        repo_path.join(path)
    } else {
        repo_path
    };

    // Read manifest
    let manifest_path = template_dir.join("cdm-template.json");
    if !manifest_path.exists() {
        anyhow::bail!(
            "No cdm-template.json found in git repository: {}\n\
            Git URL: {}\n\
            {}",
            template_dir.display(),
            url,
            if git_path.is_some() {
                format!("Git path: {}", git_path.unwrap())
            } else {
                String::new()
            }
        );
    }

    let manifest = load_manifest(&manifest_path)?;

    // Entry is optional - some templates only expose named exports
    let entry_path = if let Some(ref entry) = manifest.entry {
        let path = template_dir.join(entry);
        if !path.exists() {
            anyhow::bail!(
                "Template entry file not found: {}\n\
                Specified in cdm-template.json as: {}",
                path.display(),
                entry
            );
        }
        Some(path)
    } else {
        None
    };

    Ok(LoadedTemplate {
        manifest,
        path: template_dir,
        entry_path,
    })
}

/// Resolve a template from cache or download it
fn resolve_cached_or_download_template(
    name: &str,
    version: &str,
    download_url: &str,
    checksum: &str,
) -> Result<LoadedTemplate> {
    use crate::registry;
    use flate2::read::GzDecoder;
    use tar::Archive;

    let cache_path = registry::get_cache_path()?;
    let templates_dir = cache_path.join("templates");
    let template_dir = templates_dir.join(format!("{}@{}", name.replace('/', "_"), version));

    // Check if already cached
    let extracted_dir = find_cached_template_root(&template_dir);
    if let Some(ref cached_path) = extracted_dir {
        // Template is cached, load from disk
        return resolve_local_template_dir(cached_path);
    }

    // Not cached, download and extract
    std::fs::create_dir_all(&template_dir)?;

    eprintln!("Downloading template {}@{} from {}...", name, version, download_url);

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .context("Failed to create HTTP client")?;

    let response = client
        .get(download_url)
        .send()
        .context("Failed to download template")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "HTTP error {} while downloading template from {}",
            response.status(),
            download_url
        );
    }

    let bytes = response.bytes().context("Failed to read response bytes")?;

    // Verify checksum
    verify_template_checksum(&bytes, checksum)?;

    // Extract tar.gz archive
    let decoder = GzDecoder::new(bytes.as_ref());
    let mut archive = Archive::new(decoder);

    archive
        .unpack(&template_dir)
        .context("Failed to extract template archive")?;

    // Find the extracted template root
    let extracted_root = find_cached_template_root(&template_dir)
        .ok_or_else(|| anyhow::anyhow!("Could not find cdm-template.json in extracted archive"))?;

    // Load the template from the extracted directory
    resolve_local_template_dir(&extracted_root)
}

/// Find the root directory of a cached template (where cdm-template.json is)
fn find_cached_template_root(template_dir: &Path) -> Option<PathBuf> {
    if !template_dir.exists() {
        return None;
    }

    // Check if cdm-template.json is directly in template_dir
    if template_dir.join("cdm-template.json").exists() {
        return Some(template_dir.to_path_buf());
    }

    // Look in immediate subdirectories (tar extracts into a subdirectory)
    if let Ok(entries) = std::fs::read_dir(template_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("cdm-template.json").exists() {
                return Some(path);
            }
        }
    }

    None
}

/// Verify checksum of downloaded data
fn verify_template_checksum(data: &[u8], expected_checksum: &str) -> Result<()> {
    use sha2::{Digest, Sha256};

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

/// Resolve a registry template
fn resolve_registry_template(name: &str, config: &Option<JSON>) -> Result<LoadedTemplate> {
    use crate::diagnostics::E601_TEMPLATE_NOT_FOUND;
    use crate::{template_registry, version_resolver};

    // Split the name into base template name and optional subpath export
    // e.g., "sql-types/postgres" -> ("sql-types", Some("postgres"))
    // e.g., "cdm/auth" -> ("cdm/auth", None) (scoped name, no subpath)
    // e.g., "cdm/auth/types" -> ("cdm/auth", Some("types"))
    let (base_name, subpath_export) = split_template_name_and_subpath(name);

    // Extract version constraint from config
    let version_constraint = config
        .as_ref()
        .and_then(|c| c.get("version"))
        .and_then(|v| v.as_str())
        .map(version_resolver::parse_version_constraint)
        .transpose()
        .map_err(|e| anyhow::anyhow!("Invalid version constraint: {}", e))?
        .unwrap_or(version_resolver::VersionConstraint::Latest);

    // Load the template registry
    let registry = template_registry::load_template_registry()
        .context("Failed to load template registry")?;

    // Look up the base template name in the registry
    let template = template_registry::lookup_template(&registry, &base_name)
        .ok_or_else(|| anyhow::anyhow!(
            "{}: Template not found: '{}' - Template '{}' not found in registry. Run 'cdm template cache {}' to download it.",
            E601_TEMPLATE_NOT_FOUND, base_name, base_name, base_name
        ))?;

    // Resolve version constraint to a specific version
    let resolved_version = resolve_template_version(&version_constraint, template)?;

    // Get the version info
    let version_info = template_registry::get_template_version(template, Some(&resolved_version))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Version '{}' not found for template '{}'",
                resolved_version,
                name
            )
        })?;

    // Resolve template from cache or download
    let mut loaded = resolve_cached_or_download_template(
        &base_name,
        &resolved_version,
        &version_info.download_url,
        &version_info.checksum,
    )
    .with_context(|| format!("Failed to resolve template '{}' version '{}'", base_name, resolved_version))?;

    // If a subpath export was specified, resolve it to the correct entry file
    if let Some(subpath) = subpath_export {
        let export_key = format!("./{}", subpath);
        let export_path = loaded.manifest.exports.get(&export_key)
            .or_else(|| {
                // Also try without the "./" prefix
                loaded.manifest.exports.get(&subpath)
            })
            .ok_or_else(|| anyhow::anyhow!(
                "Template '{}' does not export '{}'. Available exports: {}",
                base_name,
                subpath,
                if loaded.manifest.exports.is_empty() {
                    "none (only main entry available)".to_string()
                } else {
                    loaded.manifest.exports.keys().cloned().collect::<Vec<_>>().join(", ")
                }
            ))?;

        // Update the entry path to use the export
        loaded.entry_path = Some(loaded.path.join(export_path.trim_start_matches("./")));
    } else if loaded.entry_path.is_none() {
        // No subpath specified and no main entry - this is an export-only template
        let available_exports: Vec<_> = loaded.manifest.exports.keys().cloned().collect();
        anyhow::bail!(
            "Template '{}' has no main entry point. You must specify an export path.\n\
            Available exports: {}\n\
            Example: import name from \"{}/{}\"",
            base_name,
            available_exports.join(", "),
            base_name,
            available_exports.first().map(|s| s.trim_start_matches("./")).unwrap_or("export")
        );
    }

    Ok(loaded)
}

/// Resolve a version constraint to a specific version for templates
fn resolve_template_version(
    constraint: &crate::version_resolver::VersionConstraint,
    template: &crate::template_registry::RegistryTemplate,
) -> Result<String> {
    use crate::version_resolver::{version_matches, VersionConstraint};
    use semver::Version;

    match constraint {
        VersionConstraint::Latest => Ok(template.latest.clone()),
        VersionConstraint::Exact(v) => {
            if template.versions.contains_key(v) {
                Ok(v.clone())
            } else {
                anyhow::bail!("Exact version '{}' not found", v)
            }
        }
        _ => {
            // For caret, tilde, or range constraints, find the highest matching version
            let mut matching_versions: Vec<(Version, String)> = template
                .versions
                .keys()
                .filter(|v| version_matches(constraint, v))
                .filter_map(|v| Version::parse(v).ok().map(|parsed| (parsed, v.clone())))
                .collect();

            if matching_versions.is_empty() {
                anyhow::bail!(
                    "No version matching constraint '{}' found. Available versions: {}",
                    constraint,
                    template.versions.keys().cloned().collect::<Vec<_>>().join(", ")
                )
            }

            matching_versions.sort_by(|a, b| a.0.cmp(&b.0));
            Ok(matching_versions.last().unwrap().1.clone())
        }
    }
}

/// Split a template name into base name and optional subpath export.
///
/// Registry template names can include subpath exports:
/// - "sql-types" -> ("sql-types", None)
/// - "sql-types/postgres" -> ("sql-types", Some("postgres"))
/// - "cdm/auth" -> ("cdm/auth", None) - scoped name, no subpath
/// - "cdm/auth/types" -> ("cdm/auth", Some("types")) - scoped name with subpath
///
/// Heuristic: A template name contains a dash (e.g., "sql-types"), while a scope
/// is a short, simple word without dashes (e.g., "cdm", "org").
fn split_template_name_and_subpath(name: &str) -> (String, Option<String>) {
    // Remove any .cdm extension that might have been included
    let name = name.trim_end_matches(".cdm");

    let parts: Vec<&str> = name.split('/').collect();

    if parts.len() >= 3 {
        // "cdm/auth/types" or "sql-types/postgres/v2"
        let first_part = parts[0];
        // If first part has a dash, it's a template name, not a scope
        if first_part.contains('-') {
            // "sql-types/postgres/v2" -> template "sql-types", subpath "postgres/v2"
            let base_name = parts[0].to_string();
            let subpath = parts[1..].join("/");
            (base_name, Some(subpath))
        } else {
            // "cdm/auth/types" -> scoped name "cdm/auth", subpath "types"
            let base_name = format!("{}/{}", parts[0], parts[1]);
            let subpath = parts[2..].join("/");
            (base_name, Some(subpath))
        }
    } else if parts.len() == 2 {
        // Could be "sql-types/postgres" (template + subpath) or "cdm/auth" (scoped name)
        let first_part = parts[0];
        let second_part = parts[1];

        // Heuristic: if first part contains a dash, it's a template name (not a scope)
        // e.g., "sql-types" is a template, "cdm" is a scope
        if first_part.contains('-') {
            // "sql-types/postgres" -> template "sql-types" with subpath "postgres"
            (first_part.to_string(), Some(second_part.to_string()))
        } else {
            // "cdm/auth" -> scoped template name, no subpath
            (name.to_string(), None)
        }
    } else {
        // Single part, no subpath
        (name.to_string(), None)
    }
}

/// Load and parse a template manifest file
fn load_manifest(path: &Path) -> Result<TemplateManifest> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;

    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse template manifest: {}", path.display()))
}

/// Derive an EntityIdSource from a TemplateSource.
///
/// This function maps template sources to entity ID sources for tagging entity IDs
/// with their origin. This prevents ID collisions when multiple templates use the
/// same numeric IDs independently.
///
/// # Arguments
///
/// * `source` - The template source to derive from
/// * `config` - Optional configuration (for git_path extraction)
/// * `source_file` - The file containing the import (for relative path resolution)
/// * `project_root` - Root of the project (for canonicalizing local paths)
///
/// # Returns
///
/// An EntityIdSource corresponding to the template source type.
pub fn get_entity_id_source(
    source: &TemplateSource,
    config: &Option<JSON>,
    source_file: &Path,
    project_root: &Path,
) -> EntityIdSource {
    match source {
        TemplateSource::Registry { name } => EntityIdSource::Registry { name: name.clone() },
        TemplateSource::Git { url } => {
            // Extract git_path from config if present
            let git_path = config
                .as_ref()
                .and_then(|c| c.get("git_path"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            EntityIdSource::Git {
                url: url.clone(),
                path: git_path,
            }
        }
        TemplateSource::Local { path } => {
            // Compute the path relative to project root
            let source_dir = source_file.parent().unwrap_or(Path::new("."));
            let template_dir = source_dir.join(path);

            // Try to make it relative to project root, fall back to the path as-is
            let relative_path = template_dir
                .strip_prefix(project_root)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| path.clone());

            EntityIdSource::LocalTemplate { path: relative_path }
        }
    }
}

/// Get EntityIdSource from a TemplateImport
pub fn get_import_entity_id_source(import: &TemplateImport, project_root: &Path) -> EntityIdSource {
    get_entity_id_source(&import.source, &import.config, &import.source_file, project_root)
}

/// Get EntityIdSource from a TemplateExtends
pub fn get_extends_entity_id_source(extends: &TemplateExtends, project_root: &Path) -> EntityIdSource {
    get_entity_id_source(&extends.source, &extends.config, &extends.source_file, project_root)
}

#[cfg(test)]
#[path = "template_resolver/template_resolver_tests.rs"]
mod template_resolver_tests;
