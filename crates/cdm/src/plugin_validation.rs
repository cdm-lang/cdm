use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::{Diagnostic, Severity, PluginRunner, ResolvedSchema, validate};
use cdm_utils::{Span, Position};
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
mod tests;

/// Information about a plugin import (@plugin directive)
#[derive(Debug, Clone)]
pub struct PluginImport {
    pub name: String,
    pub source: Option<PluginSource>,
    pub global_config: Option<JSON>,
    pub span: Span,
    /// The absolute path of the CDM file this import is from (for resolving relative paths)
    pub source_file: PathBuf,
}

/// Source location for a plugin
#[derive(Debug, Clone)]
pub enum PluginSource {
    Git { url: String },
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

/// Configuration level matching cdm-plugin-api::ConfigLevel
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigLevel {
    Global,
    Model { name: String },
    Field { model: String, field: String },
}

/// In-memory cache for loaded plugins during validation session
pub struct PluginCache {
    plugins: HashMap<String, CachedPlugin>,
}

struct CachedPlugin {
    runner: PluginRunner,
    schema_cdm: String,
    resolved_schema: Option<ResolvedSchema>,  // Parsed schema for Level 1 validation
}

impl PluginCache {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    /// Load plugin and cache, or return cached version.
    /// Returns None and adds E401 diagnostic if plugin can't be loaded.
    pub fn load_plugin(
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
                    message: format!("E401: Plugin not found: '{}' - {}", import.name, msg),
                    severity: Severity::Error,
                    span: import.span,
                });
                return None;
            }
        };

        // Load WASM module
        let mut runner = match PluginRunner::new(&wasm_path) {
            Ok(r) => r,
            Err(e) => {
                diagnostics.push(Diagnostic {
                    message: format!("E401: Failed to load plugin '{}': {}", import.name, e),
                    severity: Severity::Error,
                    span: import.span,
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
                        "E403: Plugin '{}' missing required export '_schema': {}",
                        import.name, e
                    ),
                    severity: Severity::Error,
                    span: import.span,
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
            schema_cdm,
            resolved_schema,
        };
        self.plugins.insert(import.name.clone(), cached);
        self.plugins.get_mut(&import.name)
    }

    fn resolve_plugin_path(&self, import: &PluginImport) -> Result<PathBuf, String> {
        match &import.source {
            Some(PluginSource::Path { path }) => {
                // Resolve relative to the source file's directory
                let source_dir = import.source_file.parent().unwrap_or_else(|| Path::new("."));
                let mut wasm_path = source_dir.join(path);

                if !wasm_path.extension().map_or(false, |e| e == "wasm") {
                    wasm_path.set_extension("wasm");
                }
                if wasm_path.exists() {
                    Ok(wasm_path)
                } else {
                    Err(format!("File not found: {}", wasm_path.display()))
                }
            }
            Some(PluginSource::Git { url }) => {
                Err(format!("Git plugin sources not yet supported: {}", url))
            }
            None => {
                // Check default locations: ./plugins/{name}.wasm
                let local = PathBuf::from("./plugins")
                    .join(&import.name)
                    .with_extension("wasm");
                if local.exists() {
                    Ok(local)
                } else {
                    Err(format!(
                        "Plugin '{}' not found. Specify 'from' or place in ./plugins/",
                        import.name
                    ))
                }
            }
        }
    }
}

/// Main plugin validation function - call after semantic validation
pub fn validate_plugins(
    tree: &tree_sitter::Tree,
    source: &str,
    main_file_path: &Path,
    ancestor_sources: &[(String, PathBuf)],  // (source, file_path) pairs
    diagnostics: &mut Vec<Diagnostic>,
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
    let mut cache = PluginCache::new();

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
                    "E402: Plugin '{}' used but not imported. Add '@{}' at top of file",
                    config.plugin_name, config.plugin_name
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
            let name = node.child_by_field_name("name")
                .map(|n| node_text(n, source).to_string())
                .unwrap_or_default();

            let source_opt = node.child_by_field_name("source")
                .map(|s| parse_plugin_source(s, source));

            let global_config = node.child_by_field_name("config")
                .and_then(|c| parse_json_config(c, source));

            imports.push(PluginImport {
                name,
                source: source_opt,
                global_config,
                span: node_span(node),
                source_file: source_file_path.to_path_buf(),
            });
        }
    }

    imports
}

fn parse_plugin_source(node: tree_sitter::Node, source: &str) -> PluginSource {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "git_reference" => {
                if let Some(url_node) = child.child_by_field_name("url") {
                    return PluginSource::Git {
                        url: node_text(url_node, source).to_string()
                    };
                }
            }
            "plugin_path" => {
                return PluginSource::Path {
                    path: node_text(child, source).to_string()
                };
            }
            _ => {}
        }
    }
    PluginSource::Path { path: String::new() }
}

fn parse_json_config(node: tree_sitter::Node, source: &str) -> Option<JSON> {
    // The node is an object_literal, we need to parse it from the AST
    parse_value(node, source)
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

/// Validate config using two-level validation:
/// Level 1: Validate against plugin's schema (structural)
/// Level 2: Call plugin's WASM validate function (semantic)
fn validate_config_with_plugin(
    config: &PluginConfig,
    cached_plugin: &mut CachedPlugin,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // LEVEL 1: Schema validation (structural)
    if let Some(ref resolved_schema) = cached_plugin.resolved_schema {
        let model_name = match &config.level {
            ConfigLevel::Global => "GlobalSettings",
            ConfigLevel::Model { .. } => "ModelSettings",
            ConfigLevel::Field { .. } => "FieldSettings",
        };

        let schema_errors = cdm_json_validator::validate_json(
            resolved_schema,
            &config.config,
            model_name,
        );

        // Convert validation errors to diagnostics
        for error in &schema_errors {
            let path_str = error.path.iter()
                .map(|seg| seg.name.as_str())
                .collect::<Vec<_>>()
                .join(".");

            let message = if path_str.is_empty() {
                format!("E402: {}", error.message)
            } else {
                format!("E402: {}: {}", path_str, error.message)
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
        ConfigLevel::Global => cdm_plugin_api::ConfigLevel::Global,
        ConfigLevel::Model { name } => {
            cdm_plugin_api::ConfigLevel::Model { name: name.clone() }
        }
        ConfigLevel::Field { model, field } => {
            cdm_plugin_api::ConfigLevel::Field {
                model: model.clone(),
                field: field.clone(),
            }
        }
    };

    // Call plugin's validate function
    // Returns empty array if plugin doesn't have validate_config
    match cached_plugin.runner.validate(api_level, config.config.clone()) {
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
                        cdm_plugin_api::Severity::Error => Severity::Error,
                        cdm_plugin_api::Severity::Warning => Severity::Warning,
                    },
                    span: config.span,
                });
            }
        }
        Err(e) => {
            diagnostics.push(Diagnostic {
                message: format!(
                    "E404: Plugin execution failed for '{}': {}",
                    config.plugin_name, e
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

fn node_span(node: tree_sitter::Node) -> Span {
    let start = node.start_position();
    let end = node.end_position();
    Span {
        start: Position {
            line: start.row,
            column: start.column,
        },
        end: Position {
            line: end.row,
            column: end.column,
        },
    }
}
