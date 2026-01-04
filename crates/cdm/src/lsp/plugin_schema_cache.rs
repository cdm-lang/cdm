//! Plugin schema cache for LSP completion
//!
//! This module provides caching for loaded plugin schemas, allowing
//! completions to suggest plugin configuration fields based on each
//! plugin's schema.cdm definitions.
//!
//! Note: We use a simplified thread-safe representation (`PluginSettingsSchema`)
//! instead of `ResolvedSchema` because the latter contains `RefCell` which is not
//! `Send + Sync` and can't be shared across async boundaries.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tower_lsp::lsp_types::Url;
use tree_sitter::Parser;

use crate::plugin_validation::{PluginImport, extract_plugin_imports};
use crate::resolved_schema::build_resolved_schema;
use crate::PluginRunner;

/// A simplified, thread-safe representation of a plugin's settings schema.
/// Contains only the information needed for completions.
#[derive(Debug, Clone)]
pub struct PluginSettingsSchema {
    /// GlobalSettings model fields
    pub global_settings: Vec<SettingsField>,
    /// TypeAliasSettings model fields
    pub type_alias_settings: Vec<SettingsField>,
    /// ModelSettings model fields
    pub model_settings: Vec<SettingsField>,
    /// FieldSettings model fields
    pub field_settings: Vec<SettingsField>,
    /// Whether the plugin has a _build function (supports build_output)
    pub has_build: bool,
    /// Whether the plugin has a _migrate function (supports migrations_output)
    pub has_migrate: bool,
}

/// A simplified field definition for completion purposes
#[derive(Debug, Clone)]
pub struct SettingsField {
    pub name: String,
    /// The type expression as a string (e.g., "string", "\"postgres\" | \"sqlite\"")
    pub type_expr: Option<String>,
    pub optional: bool,
    /// Default value as JSON
    pub default_value: Option<serde_json::Value>,
    /// Parsed literal values for enum-like types
    pub literal_values: Vec<String>,
    /// Whether this is a boolean type
    pub is_boolean: bool,
}

impl PluginSettingsSchema {
    /// Get the settings fields for a given config level
    pub fn fields_for_level(&self, level: &crate::plugin_validation::ConfigLevel) -> &[SettingsField] {
        use crate::plugin_validation::ConfigLevel;
        match level {
            ConfigLevel::Global => &self.global_settings,
            ConfigLevel::TypeAlias { .. } => &self.type_alias_settings,
            ConfigLevel::Model { .. } => &self.model_settings,
            ConfigLevel::Field { .. } => &self.field_settings,
        }
    }
}

/// Thread-safe cache for loaded plugin schemas
#[derive(Clone)]
pub struct PluginSchemaCache {
    cache: Arc<RwLock<HashMap<String, CachedPluginSchema>>>,
}

struct CachedPluginSchema {
    /// The simplified settings schema
    schema: PluginSettingsSchema,
    /// When this schema was loaded (for potential TTL-based invalidation)
    #[allow(dead_code)]
    loaded_at: Instant,
}

impl Default for PluginSchemaCache {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginSchemaCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get a plugin's schema, loading it if not cached.
    ///
    /// # Arguments
    /// * `plugin_name` - The name of the plugin (e.g., "sql", "typescript")
    /// * `document_uri` - The URI of the current document (for resolving relative paths)
    /// * `document_text` - The text of the current document (to extract plugin imports)
    ///
    /// # Returns
    /// The plugin's settings schema if available, or None if the plugin couldn't be loaded
    pub fn get_or_load(
        &self,
        plugin_name: &str,
        document_uri: &Url,
        document_text: &str,
    ) -> Option<PluginSettingsSchema> {
        // Check cache first
        {
            let cache = self.cache.read().ok()?;
            if let Some(cached) = cache.get(plugin_name) {
                return Some(cached.schema.clone());
            }
        }

        // Not cached - load the schema
        let schema = self.load_plugin_schema(plugin_name, document_uri, document_text)?;

        // Cache and return
        {
            if let Ok(mut cache) = self.cache.write() {
                cache.insert(
                    plugin_name.to_string(),
                    CachedPluginSchema {
                        schema: schema.clone(),
                        loaded_at: Instant::now(),
                    },
                );
            }
        }

        Some(schema)
    }

    /// Load a plugin's schema by finding its import and loading the WASM
    fn load_plugin_schema(
        &self,
        plugin_name: &str,
        document_uri: &Url,
        document_text: &str,
    ) -> Option<PluginSettingsSchema> {
        // Parse the document to get the tree
        let mut parser = Parser::new();
        parser.set_language(&grammar::LANGUAGE.into()).ok()?;
        let tree = parser.parse(document_text, None)?;

        // Get the file path from the URI
        let file_path = document_uri.to_file_path().ok().unwrap_or_else(|| PathBuf::from("."));

        // Extract plugin imports
        let imports = extract_plugin_imports(tree.root_node(), document_text, &file_path);

        // Find the import for this plugin
        let import = imports.into_iter().find(|i| i.name == plugin_name)?;

        // Load the plugin and get its schema
        self.load_schema_from_import(&import)
    }

    /// Load schema from a plugin import
    fn load_schema_from_import(&self, import: &PluginImport) -> Option<PluginSettingsSchema> {
        // Resolve plugin path
        let wasm_path = crate::plugin_resolver::resolve_plugin_path(import).ok()?;

        // Load WASM and get schema
        let runner = PluginRunner::new(&wasm_path).ok()?;

        // Check plugin capabilities
        let has_build = runner.has_build().unwrap_or(false);
        let has_migrate = runner.has_migrate().unwrap_or(false);

        // Get schema (need mutable borrow for this)
        let mut runner = runner;
        let schema_cdm = runner.schema().ok()?;

        // Parse the schema
        let validation_result = crate::validate(&schema_cdm, &[]);
        if validation_result.has_errors() {
            return None;
        }

        // Build ResolvedSchema temporarily (just for extraction)
        let resolved = build_resolved_schema(
            &validation_result.symbol_table,
            &validation_result.model_fields,
            &[],
            &[],
        );

        // Extract simplified settings fields
        Some(PluginSettingsSchema {
            global_settings: extract_settings_fields(&resolved, "GlobalSettings"),
            type_alias_settings: extract_settings_fields(&resolved, "TypeAliasSettings"),
            model_settings: extract_settings_fields(&resolved, "ModelSettings"),
            field_settings: extract_settings_fields(&resolved, "FieldSettings"),
            has_build,
            has_migrate,
        })
    }

    /// Clear the cache for a specific plugin
    #[allow(dead_code)]
    pub fn invalidate(&self, plugin_name: &str) {
        if let Ok(mut cache) = self.cache.write() {
            cache.remove(plugin_name);
        }
    }

    /// Clear all cached schemas
    #[allow(dead_code)]
    pub fn clear(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
    }
}

/// Extract simplified settings fields from a resolved schema model
fn extract_settings_fields(schema: &cdm_utils::ResolvedSchema, model_name: &str) -> Vec<SettingsField> {
    let model = match schema.models.get(model_name) {
        Some(m) => m,
        None => return Vec::new(),
    };

    model.fields.iter().map(|field| {
        // Try to parse the type to extract literal values and check for boolean
        let (literal_values, is_boolean) = if let Ok(parsed) = field.parsed_type() {
            (extract_literal_values(&parsed), is_boolean_type(&parsed))
        } else {
            (Vec::new(), false)
        };

        SettingsField {
            name: field.name.clone(),
            type_expr: field.type_expr.clone(),
            optional: field.optional,
            default_value: field.default_value.clone(),
            literal_values,
            is_boolean,
        }
    }).collect()
}

/// Extract literal values from a parsed type (for enum-like unions)
fn extract_literal_values(parsed_type: &cdm_utils::ParsedType) -> Vec<String> {
    use cdm_utils::ParsedType;

    match parsed_type {
        ParsedType::Literal(s) => vec![s.clone()],
        ParsedType::Union(types) => types
            .iter()
            .filter_map(|t| match t {
                ParsedType::Literal(s) => Some(s.clone()),
                _ => None,
            })
            .collect(),
        _ => vec![],
    }
}

/// Check if a parsed type is boolean
fn is_boolean_type(parsed_type: &cdm_utils::ParsedType) -> bool {
    use cdm_utils::{ParsedType, PrimitiveType};

    matches!(parsed_type, ParsedType::Primitive(PrimitiveType::Boolean))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_new() {
        let cache = PluginSchemaCache::new();
        // Should be empty initially
        let inner = cache.cache.read().unwrap();
        assert!(inner.is_empty());
    }

    #[test]
    fn test_cache_invalidate() {
        let cache = PluginSchemaCache::new();

        // Manually insert a fake entry for testing
        {
            let mut inner = cache.cache.write().unwrap();
            inner.insert(
                "test_plugin".to_string(),
                CachedPluginSchema {
                    schema: PluginSettingsSchema {
                        global_settings: Vec::new(),
                        type_alias_settings: Vec::new(),
                        model_settings: Vec::new(),
                        field_settings: Vec::new(),
                        has_build: false,
                        has_migrate: false,
                    },
                    loaded_at: Instant::now(),
                },
            );
        }

        // Verify it exists
        assert!(cache.cache.read().unwrap().contains_key("test_plugin"));

        // Invalidate
        cache.invalidate("test_plugin");

        // Verify it's gone
        assert!(!cache.cache.read().unwrap().contains_key("test_plugin"));
    }
}
