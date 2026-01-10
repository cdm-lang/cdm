use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::{Diagnostic, Severity, PluginRunner, ResolvedSchema, validate, node_span};
use crate::diagnostics::{
    E401_PLUGIN_NOT_FOUND, E402_INVALID_PLUGIN_CONFIG, E403_MISSING_PLUGIN_EXPORT,
    E404_PLUGIN_EXECUTION_FAILED,
};
use cdm_utils::Span;
use serde_json::Value as JSON;

/// Structured plugin configuration data extracted from the AST
#[derive(Debug, Clone)]
pub struct ExtractedPluginConfigs {
    /// Type alias configs: (type_name) -> (plugin_name -> config)
    pub type_alias_configs: HashMap<String, HashMap<String, JSON>>,
    /// Model configs: (model_name) -> (plugin_name -> config)
    pub model_configs: HashMap<String, HashMap<String, JSON>>,
    /// Field configs: (model_name, field_name) -> (plugin_name -> config)
    pub field_configs: HashMap<(String, String), HashMap<String, JSON>>,
    /// Field default values: (model_name, field_name) -> default_value
    pub field_defaults: HashMap<(String, String), JSON>,
}

#[cfg(test)]
#[path = "plugin_validation/plugin_validation_tests.rs"]
mod plugin_validation_tests;

/// Information about a plugin import (@plugin directive)
#[derive(Debug, Clone)]
pub struct PluginImport {
    pub name: String,
    pub source: Option<PluginSource>,
    pub global_config: Option<JSON>,
    /// The span of the entire plugin import (including config block)
    pub span: Span,
    /// The span of just the plugin name (for targeted error highlighting)
    pub name_span: Span,
    /// The absolute path of the CDM file this import is from (for resolving relative paths)
    pub source_file: PathBuf,
}

/// Source location for a plugin
#[derive(Debug, Clone)]
pub enum PluginSource {
    Git { url: String, path: Option<String> },
    Path { path: String },
}

/// A plugin configuration block at any level (global/model/field)
#[derive(Debug, Clone)]
pub struct PluginConfig {
    pub plugin_name: String,
    pub level: ConfigLevel,
    pub config: JSON,
    pub span: Span,
}

/// Configuration level matching cdm-plugin-interface::ConfigLevel
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigLevel {
    Global,
    TypeAlias { name: String },
    Model { name: String },
    Field { model: String, field: String },
}

/// In-memory cache for loaded plugins during validation session
pub struct PluginCache {
    plugins: HashMap<String, CachedPlugin>,
    /// If true, only use cached plugins, don't download missing ones
    cache_only: bool,
}

pub(crate) struct CachedPlugin {
    runner: PluginRunner,
    resolved_schema: Option<ResolvedSchema>,  // Parsed schema for Level 1 validation
}

impl PluginCache {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            cache_only: false,
        }
    }

    /// Create a cache that only uses cached plugins, never downloads
    pub fn new_cache_only() -> Self {
        Self {
            plugins: HashMap::new(),
            cache_only: true,
        }
    }

    /// Load plugin and cache, or return cached version.
    /// Returns None and adds E401 diagnostic if plugin can't be loaded.
    pub(crate) fn load_plugin(
        &mut self,
        import: &PluginImport,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<&mut CachedPlugin> {
        // Check cache first
        if self.plugins.contains_key(&import.name) {
            return self.plugins.get_mut(&import.name);
        }

        // Resolve plugin location
        let wasm_path = match self.resolve_plugin_path(import) {
            Ok(path) => path,
            Err(msg) => {
                diagnostics.push(Diagnostic {
                    message: format!("{}: Plugin not found: '{}' - {}", E401_PLUGIN_NOT_FOUND, import.name, msg),
                    severity: Severity::Error,
                    span: import.name_span,
                });
                return None;
            }
        };

        // Load WASM module
        let mut runner = match PluginRunner::new(&wasm_path) {
            Ok(r) => r,
            Err(e) => {
                diagnostics.push(Diagnostic {
                    message: format!("{}: Failed to load plugin '{}': {}", E401_PLUGIN_NOT_FOUND, import.name, e),
                    severity: Severity::Error,
                    span: import.name_span,
                });
                return None;
            }
        };

        // Get plugin's schema.cdm
        let schema_cdm = match runner.schema() {
            Ok(s) => s,
            Err(e) => {
                diagnostics.push(Diagnostic {
                    message: format!(
                        "{}: Plugin '{}' missing required export '_schema': {}",
                        E403_MISSING_PLUGIN_EXPORT, import.name, e
                    ),
                    severity: Severity::Error,
                    span: import.name_span,
                });
                return None;
            }
        };

        // Parse plugin schema for Level 1 validation
        let validation_result = validate(&schema_cdm, &[]);
        let resolved_schema = if validation_result.has_errors() || validation_result.tree.is_none() {
            // Schema parse/validation failed - store None and continue
            // We'll skip Level 1 validation for this plugin
            None
        } else {
            // Build ResolvedSchema from the parsed tree
            use crate::resolved_schema::build_resolved_schema;
            Some(build_resolved_schema(
                &validation_result.symbol_table,
                &validation_result.model_fields,
                &[],
                &[],
            ))
        };

        // Cache and return
        let cached = CachedPlugin {
            runner,
            resolved_schema,
        };
        self.plugins.insert(import.name.clone(), cached);
        self.plugins.get_mut(&import.name)
    }

    fn resolve_plugin_path(&self, import: &PluginImport) -> Result<PathBuf, String> {
        if self.cache_only {
            crate::plugin_resolver::resolve_plugin_path_cache_only(import)
                .map_err(|e| e.to_string())
        } else {
            crate::plugin_resolver::resolve_plugin_path(import)
                .map_err(|e| e.to_string())
        }
    }
}

/// Main plugin validation function - call after semantic validation
///
/// If `cache_only` is true, only uses cached plugins and won't download missing ones.
/// This is useful for LSP where we don't want to block on network requests.
pub fn validate_plugins(
    tree: &tree_sitter::Tree,
    source: &str,
    main_file_path: &Path,
    ancestor_sources: &[(String, PathBuf)],  // (source, file_path) pairs
    diagnostics: &mut Vec<Diagnostic>,
    cache_only: bool,
) {
    let root = tree.root_node();

    // Step 1: Extract plugin imports from ancestors (furthest ancestor first)
    let mut all_plugin_imports = Vec::new();
    for (ancestor_source, ancestor_path) in ancestor_sources.iter().rev() {
        let ancestor_imports = extract_plugin_imports_from_source(ancestor_source, ancestor_path);
        all_plugin_imports.extend(ancestor_imports);
    }

    // Step 2: Extract plugin imports from main file
    let plugin_imports = extract_plugin_imports(root, source, main_file_path);
    all_plugin_imports.extend(plugin_imports);

    // Step 3: Extract all plugin configurations
    let plugin_configs = extract_plugin_configs(root, source);

    // Early return only if both imports AND configs are empty
    if all_plugin_imports.is_empty() && plugin_configs.is_empty() {
        return;
    }

    // Step 4: Create plugin cache
    let mut cache = if cache_only {
        PluginCache::new_cache_only()
    } else {
        PluginCache::new()
    };

    // Step 5: Load all plugins (fail fast on E401)
    for import in &all_plugin_imports {
        cache.load_plugin(import, diagnostics);
    }

    // If any plugins failed to load, stop (fail fast)
    if diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return;
    }

    // Step 6: Validate global configs from plugin imports
    for import in &all_plugin_imports {
        if let Some(global_config) = &import.global_config {
            if let Some(cached_plugin) = cache.plugins.get_mut(&import.name) {
                let config = PluginConfig {
                    plugin_name: import.name.clone(),
                    level: ConfigLevel::Global,
                    config: global_config.clone(),
                    span: import.span,
                };
                validate_config_with_plugin(&config, cached_plugin, diagnostics);
            }
        }
    }

    // Step 7: Validate model/field level configs
    for config in &plugin_configs {
        if let Some(cached_plugin) = cache.plugins.get_mut(&config.plugin_name) {
            // Call plugin validate function
            validate_config_with_plugin(config, cached_plugin, diagnostics);
        } else {
            // Plugin used but not imported
            diagnostics.push(Diagnostic {
                message: format!(
                    "{}: Plugin '{}' used but not imported. Add '@{}' at top of file",
                    E402_INVALID_PLUGIN_CONFIG, config.plugin_name, config.plugin_name
                ),
                severity: Severity::Error,
                span: config.span,
            });
        }
    }
}

/// Extract plugin imports from a source string (for ancestors)
fn extract_plugin_imports_from_source(source: &str, source_file_path: &Path) -> Vec<PluginImport> {
    // Parse the source using tree-sitter
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&grammar::LANGUAGE.into()).expect("Failed to load CDM grammar");

    if let Some(tree) = parser.parse(source, None) {
        extract_plugin_imports(tree.root_node(), source, source_file_path)
    } else {
        Vec::new()
    }
}

/// Extract plugin imports from a validation result
///
/// This is a convenience function that extracts plugin imports from a ValidationResult
/// by re-reading the source file and extracting imports from the parsed tree.
/// Used by build.rs and migrate.rs.
pub fn extract_plugin_imports_from_validation_result(
    validation_result: &crate::ValidationResult,
    main_path: &Path,
) -> anyhow::Result<Vec<PluginImport>> {
    use anyhow::Context;
    use std::fs;

    let parsed_tree = validation_result.tree.as_ref()
        .context("No parsed tree available")?;

    // We need to re-read the source file since tree was consumed
    let main_source = fs::read_to_string(main_path)
        .with_context(|| format!("Failed to read source file: {}", main_path.display()))?;

    let root = parsed_tree.root_node();
    Ok(extract_plugin_imports(root, &main_source, main_path))
}

/// Extract all plugin imports from AST (public for use in build.rs)
pub fn extract_plugin_imports(
    root: tree_sitter::Node,
    source: &str,
    source_file_path: &Path,
) -> Vec<PluginImport> {
    let mut imports = Vec::new();
    let mut cursor = root.walk();

    for node in root.children(&mut cursor) {
        if node.kind() == "plugin_import" {
            let name_node = node.child_by_field_name("name");
            let name = name_node
                .map(|n| node_text(n, source).to_string())
                .unwrap_or_default();

            // Use name node span if available, otherwise fall back to full node span
            let name_span = name_node.map(|n| node_span(n)).unwrap_or_else(|| node_span(node));

            let source_opt = node.child_by_field_name("source")
                .map(|s| parse_plugin_source(s, source));

            let global_config = node.child_by_field_name("config")
                .and_then(|c| parse_json_config(c, source));

            imports.push(PluginImport {
                name,
                source: source_opt,
                global_config,
                span: node_span(node),
                name_span,
                source_file: source_file_path.to_path_buf(),
            });
        }
    }

    imports
}

/// Parse a string_literal node into PluginSource
///
/// The source string determines the type:
/// - Starts with "git:" → Git source (URL after "git:")
/// - Starts with "./" or "../" → Local path
fn parse_plugin_source(node: tree_sitter::Node, source: &str) -> PluginSource {
    let text = node_text(node, source);

    // Strip surrounding quotes
    let value = if text.len() >= 2 && text.starts_with('"') && text.ends_with('"') {
        &text[1..text.len()-1]
    } else {
        return PluginSource::Path { path: String::new() };
    };

    // Determine source type based on string content
    if let Some(url) = value.strip_prefix("git:") {
        PluginSource::Git {
            url: url.to_string(),
            path: None, // Will be extracted from global config
        }
    } else {
        PluginSource::Path {
            path: value.to_string(),
        }
    }
}

/// Parse an object_literal node into JSON
///
/// This is public so it can be reused by template_resolver
pub fn parse_object_literal_to_json(node: tree_sitter::Node, source: &str) -> Option<JSON> {
    // The node is an object_literal, we need to parse it from the AST
    parse_value(node, source)
}

fn parse_json_config(node: tree_sitter::Node, source: &str) -> Option<JSON> {
    parse_object_literal_to_json(node, source)
}

/// Parse a CDM value node into a JSON value
fn parse_value(node: tree_sitter::Node, source: &str) -> Option<JSON> {
    match node.kind() {
        "object_literal" => {
            let mut map = serde_json::Map::new();
            let mut cursor = node.walk();

            for child in node.children(&mut cursor) {
                if child.kind() == "object_entry" {
                    let key = child.child_by_field_name("key")
                        .map(|k| {
                            let text = node_text(k, source);
                            // Remove quotes if present
                            if text.starts_with('"') && text.ends_with('"') {
                                text[1..text.len()-1].to_string()
                            } else {
                                text.to_string()
                            }
                        })?;

                    let value = child.child_by_field_name("value")
                        .and_then(|v| parse_value(v, source))?;

                    map.insert(key, value);
                }
            }

            Some(JSON::Object(map))
        }
        "array_literal" => {
            let mut arr = Vec::new();
            let mut cursor = node.walk();

            for child in node.children(&mut cursor) {
                if child.kind() != "[" && child.kind() != "]" && child.kind() != "," {
                    if let Some(value) = parse_value(child, source) {
                        arr.push(value);
                    }
                }
            }

            Some(JSON::Array(arr))
        }
        "string_literal" => {
            let text = node_text(node, source);
            // Remove surrounding quotes and parse escape sequences
            if text.len() >= 2 && text.starts_with('"') && text.ends_with('"') {
                Some(JSON::String(text[1..text.len()-1].to_string()))
            } else {
                None
            }
        }
        "number_literal" => {
            let text = node_text(node, source);
            if let Ok(n) = text.parse::<i64>() {
                Some(JSON::Number(n.into()))
            } else if let Ok(f) = text.parse::<f64>() {
                serde_json::Number::from_f64(f).map(JSON::Number)
            } else {
                None
            }
        }
        "boolean_literal" => {
            let text = node_text(node, source);
            match text {
                "true" => Some(JSON::Bool(true)),
                "false" => Some(JSON::Bool(false)),
                _ => None,
            }
        }
        "null_literal" => Some(JSON::Null),
        _ => None,
    }
}

/// Extract plugin configs and default values in a structured format for storage in FieldInfo/Definition
///
/// This function is used by validate.rs to populate plugin_configs and default_value fields
/// during the initial parsing phase, so they're available throughout the compilation pipeline.
pub fn extract_structured_plugin_configs(
    root: tree_sitter::Node,
    source: &str,
) -> ExtractedPluginConfigs {
    let mut type_alias_configs: HashMap<String, HashMap<String, JSON>> = HashMap::new();
    let mut model_configs: HashMap<String, HashMap<String, JSON>> = HashMap::new();
    let mut field_configs: HashMap<(String, String), HashMap<String, JSON>> = HashMap::new();
    let mut field_defaults: HashMap<(String, String), JSON> = HashMap::new();

    let mut cursor = root.walk();

    for node in root.children(&mut cursor) {
        match node.kind() {
            "type_alias" => {
                let type_name = node.child_by_field_name("name")
                    .map(|n| node_text(n, source).to_string())
                    .unwrap_or_default();

                // Extract plugin configs from type alias plugins block
                // Type aliases use "plugins" field, not "body"
                if let Some(plugins) = node.child_by_field_name("plugins") {
                    let mut configs_for_type = HashMap::new();
                    extract_plugin_block_into_map(plugins, source, &mut configs_for_type);
                    if !configs_for_type.is_empty() {
                        type_alias_configs.insert(type_name, configs_for_type);
                    }
                }
            }
            "model_definition" => {
                let model_name = node.child_by_field_name("name")
                    .map(|n| node_text(n, source).to_string())
                    .unwrap_or_default();

                if let Some(body) = node.child_by_field_name("body") {
                    extract_model_configs_structured(
                        body,
                        source,
                        &model_name,
                        &mut model_configs,
                        &mut field_configs,
                        &mut field_defaults,
                    );
                }
            }
            _ => {}
        }
    }

    ExtractedPluginConfigs {
        type_alias_configs,
        model_configs,
        field_configs,
        field_defaults,
    }
}

/// Extract plugin configs from a model body into structured maps
fn extract_model_configs_structured(
    body: tree_sitter::Node,
    source: &str,
    model_name: &str,
    model_configs: &mut HashMap<String, HashMap<String, JSON>>,
    field_configs: &mut HashMap<(String, String), HashMap<String, JSON>>,
    field_defaults: &mut HashMap<(String, String), JSON>,
) {
    let mut cursor = body.walk();

    for child in body.children(&mut cursor) {
        match child.kind() {
            "plugin_config" => {
                // Model-level: @sql { table: "users" }
                if let Some((name, value)) = parse_plugin_config_node(child, source) {
                    model_configs
                        .entry(model_name.to_string())
                        .or_insert_with(HashMap::new)
                        .insert(name, value);
                }
            }
            "field_definition" | "field_override" => {
                let field_name = child.child_by_field_name("name")
                    .map(|n| node_text(n, source).to_string())
                    .unwrap_or_default();

                // Extract default value
                if let Some(default_node) = child.child_by_field_name("default") {
                    if let Some(default_value) = parse_value(default_node, source) {
                        field_defaults.insert((model_name.to_string(), field_name.clone()), default_value);
                    }
                }

                // Extract plugin configs
                if let Some(plugins) = child.child_by_field_name("plugins") {
                    let mut configs_for_field = HashMap::new();
                    extract_plugin_block_into_map(plugins, source, &mut configs_for_field);
                    if !configs_for_field.is_empty() {
                        field_configs.insert((model_name.to_string(), field_name), configs_for_field);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Extract plugin configs from a plugin block into a map (plugin_name -> config)
fn extract_plugin_block_into_map(
    block: tree_sitter::Node,
    source: &str,
    configs: &mut HashMap<String, JSON>,
) {
    let mut cursor = block.walk();
    for child in block.children(&mut cursor) {
        if child.kind() == "plugin_config" {
            if let Some((name, value)) = parse_plugin_config_node(child, source) {
                configs.insert(name, value);
            }
        }
    }
}

/// Extract all plugin configurations from models and fields
fn extract_plugin_configs(
    root: tree_sitter::Node,
    source: &str,
) -> Vec<PluginConfig> {
    let mut configs = Vec::new();
    let mut cursor = root.walk();

    for node in root.children(&mut cursor) {
        match node.kind() {
            "type_alias" => {
                let alias_name = node.child_by_field_name("name")
                    .map(|n| node_text(n, source))
                    .unwrap_or("");

                if let Some(plugins) = node.child_by_field_name("plugins") {
                    extract_plugin_block(
                        plugins,
                        source,
                        ConfigLevel::TypeAlias {
                            name: alias_name.to_string(),
                        },
                        &mut configs,
                    );
                }
            }
            "model_definition" => {
                let model_name = node.child_by_field_name("name")
                    .map(|n| node_text(n, source))
                    .unwrap_or("");

                if let Some(body) = node.child_by_field_name("body") {
                    extract_model_configs(body, source, model_name, &mut configs);
                }
            }
            _ => {}
        }
    }

    configs
}

fn extract_model_configs(
    body: tree_sitter::Node,
    source: &str,
    model_name: &str,
    configs: &mut Vec<PluginConfig>,
) {
    let mut cursor = body.walk();

    for child in body.children(&mut cursor) {
        match child.kind() {
            "plugin_config" => {
                // Model-level: @sql { table: "users" }
                if let Some((name, value)) = parse_plugin_config_node(child, source) {
                    configs.push(PluginConfig {
                        plugin_name: name,
                        level: ConfigLevel::Model {
                            name: model_name.to_string(),
                        },
                        config: value,
                        span: node_span(child),
                    });
                }
            }
            "field_definition" => {
                // Field-level inline: id: number { @sql { ... } }
                if let Some(plugins) = child.child_by_field_name("plugins") {
                    let field_name = child.child_by_field_name("name")
                        .map(|n| node_text(n, source))
                        .unwrap_or("");

                    extract_plugin_block(
                        plugins,
                        source,
                        ConfigLevel::Field {
                            model: model_name.to_string(),
                            field: field_name.to_string(),
                        },
                        configs,
                    );
                }
            }
            "field_override" => {
                // Field override: email { @validation { ... } }
                if let Some(plugins) = child.child_by_field_name("plugins") {
                    let field_name = child.child_by_field_name("name")
                        .map(|n| node_text(n, source))
                        .unwrap_or("");

                    extract_plugin_block(
                        plugins,
                        source,
                        ConfigLevel::Field {
                            model: model_name.to_string(),
                            field: field_name.to_string(),
                        },
                        configs,
                    );
                }
            }
            _ => {}
        }
    }
}

fn extract_plugin_block(
    block: tree_sitter::Node,
    source: &str,
    level: ConfigLevel,
    configs: &mut Vec<PluginConfig>,
) {
    let mut cursor = block.walk();
    for child in block.children(&mut cursor) {
        if child.kind() == "plugin_config" {
            if let Some((name, value)) = parse_plugin_config_node(child, source) {
                configs.push(PluginConfig {
                    plugin_name: name,
                    level: level.clone(),
                    config: value,
                    span: node_span(child),
                });
            }
        }
    }
}

fn parse_plugin_config_node(
    node: tree_sitter::Node,
    source: &str,
) -> Option<(String, JSON)> {
    let name = node.child_by_field_name("name")
        .map(|n| node_text(n, source).to_string())?;

    let value = node.child_by_field_name("config")
        .and_then(|c| parse_json_config(c, source))?;

    Some((name, value))
}

/// Filter out reserved config keys that CDM uses internally
/// These keys are processed by CDM itself and should not be validated by plugins
fn filter_reserved_config_keys(config: &serde_json::Value) -> serde_json::Value {
    if let Some(obj) = config.as_object() {
        let mut filtered = obj.clone();
        // CDM-internal keys that plugins shouldn't see or validate
        filtered.remove("build_output");       // Handled by CDM for build command
        filtered.remove("migrations_output");  // Handled by CDM for migrate command
        filtered.remove("version");            // Plugin version constraint
        filtered.remove("git_ref");            // Git plugin source ref
        filtered.remove("git_path");           // Git plugin path within repo
        serde_json::Value::Object(filtered)
    } else {
        config.clone()
    }
}


/// Validate config using two-level validation:
/// Level 1: Validate against plugin's schema (structural)
/// Level 2: Call plugin's WASM validate function (semantic)
fn validate_config_with_plugin(
    config: &PluginConfig,
    cached_plugin: &mut CachedPlugin,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let model_name = match &config.level {
        ConfigLevel::Global => "GlobalSettings",
        ConfigLevel::TypeAlias { .. } => "TypeAliasSettings",
        ConfigLevel::Model { .. } => "ModelSettings",
        ConfigLevel::Field { .. } => "FieldSettings",
    };

    // Filter out reserved config keys
    let filtered_config = if matches!(config.level, ConfigLevel::Global) {
        filter_reserved_config_keys(&config.config)
    } else {
        config.config.clone()
    };

    // Apply defaults if schema is available
    let config_with_defaults = if let Some(ref resolved_schema) = cached_plugin.resolved_schema {
        cdm_json_validator::apply_defaults(
            resolved_schema,
            &filtered_config,
            model_name,
        )
    } else {
        filtered_config.clone()
    };

    // LEVEL 1: Schema validation (structural)
    if let Some(ref resolved_schema) = cached_plugin.resolved_schema {
        let schema_errors = cdm_json_validator::validate_json(
            resolved_schema,
            &config_with_defaults,
            model_name,
        );

        // Convert validation errors to diagnostics
        for error in &schema_errors {
            let path_str = error.path.iter()
                .map(|seg| seg.name.as_str())
                .collect::<Vec<_>>()
                .join(".");

            let message = if path_str.is_empty() {
                format!("{}: {}", E402_INVALID_PLUGIN_CONFIG, error.message)
            } else {
                format!("{}: {}: {}", E402_INVALID_PLUGIN_CONFIG, path_str, error.message)
            };

            diagnostics.push(Diagnostic {
                message,
                severity: Severity::Error,
                span: config.span,
            });
        }

        // Fail fast: If Level 1 fails, don't run Level 2
        if !schema_errors.is_empty() {
            return;
        }
    }

    // LEVEL 2: Plugin semantic validation (if plugin has _validate_config)
    let api_level = match &config.level {
        ConfigLevel::Global => cdm_plugin_interface::ConfigLevel::Global,
        ConfigLevel::TypeAlias { name } => {
            cdm_plugin_interface::ConfigLevel::TypeAlias { name: name.clone() }
        }
        ConfigLevel::Model { name } => {
            cdm_plugin_interface::ConfigLevel::Model { name: name.clone() }
        }
        ConfigLevel::Field { model, field } => {
            cdm_plugin_interface::ConfigLevel::Field {
                model: model.clone(),
                field: field.clone(),
            }
        }
    };

    // Call plugin's validate function with defaults applied
    // Returns empty array if plugin doesn't have validate_config
    match cached_plugin.runner.validate(api_level, config_with_defaults) {
        Ok(errors) => {
            for error in errors {
                let path_str = error.path.iter()
                    .map(|seg| seg.name.as_str())
                    .collect::<Vec<_>>()
                    .join(".");

                // Plugin-returned errors are displayed as-is (no E402 prefix)
                let message = if path_str.is_empty() {
                    error.message.clone()
                } else {
                    format!("{}: {}", path_str, error.message)
                };

                diagnostics.push(Diagnostic {
                    message,
                    severity: match error.severity {
                        cdm_plugin_interface::Severity::Error => Severity::Error,
                        cdm_plugin_interface::Severity::Warning => Severity::Warning,
                    },
                    span: config.span,
                });
            }
        }
        Err(e) => {
            diagnostics.push(Diagnostic {
                message: format!(
                    "{}: Plugin execution failed for '{}': {}",
                    E404_PLUGIN_EXECUTION_FAILED, config.plugin_name, e
                ),
                severity: Severity::Error,
                span: config.span,
            });
        }
    }
}

// Helper functions

fn node_text<'a>(node: tree_sitter::Node, source: &'a str) -> &'a str {
    node.utf8_text(source.as_bytes()).unwrap_or("")
}