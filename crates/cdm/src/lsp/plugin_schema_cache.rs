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
        // Resolve plugin path (cache_only to avoid blocking on network requests in LSP)
        let wasm_path = crate::plugin_resolver::resolve_plugin_path_cache_only(import).ok()?;

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
            &validation_result.removal_names,
            &validation_result.field_removals,
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
    use crate::plugin_validation::ConfigLevel;

    // =========================================================================
    // STRUCT TESTS
    // =========================================================================

    #[test]
    fn test_settings_field_struct() {
        let field = SettingsField {
            name: "test_field".to_string(),
            type_expr: Some("string".to_string()),
            optional: false,
            default_value: Some(serde_json::json!("default")),
            literal_values: vec!["a".to_string(), "b".to_string()],
            is_boolean: false,
        };

        assert_eq!(field.name, "test_field");
        assert_eq!(field.type_expr, Some("string".to_string()));
        assert!(!field.optional);
        assert_eq!(field.default_value, Some(serde_json::json!("default")));
        assert_eq!(field.literal_values.len(), 2);
        assert!(!field.is_boolean);
    }

    #[test]
    fn test_settings_field_debug() {
        let field = SettingsField {
            name: "debug_field".to_string(),
            type_expr: None,
            optional: true,
            default_value: None,
            literal_values: vec![],
            is_boolean: true,
        };

        let debug_str = format!("{:?}", field);
        assert!(debug_str.contains("debug_field"));
        assert!(debug_str.contains("is_boolean: true"));
    }

    #[test]
    fn test_settings_field_clone() {
        let field = SettingsField {
            name: "clone_test".to_string(),
            type_expr: Some("number".to_string()),
            optional: true,
            default_value: Some(serde_json::json!(42)),
            literal_values: vec!["x".to_string()],
            is_boolean: false,
        };

        let cloned = field.clone();
        assert_eq!(cloned.name, field.name);
        assert_eq!(cloned.type_expr, field.type_expr);
        assert_eq!(cloned.optional, field.optional);
        assert_eq!(cloned.default_value, field.default_value);
        assert_eq!(cloned.literal_values, field.literal_values);
        assert_eq!(cloned.is_boolean, field.is_boolean);
    }

    #[test]
    fn test_plugin_settings_schema_struct() {
        let schema = PluginSettingsSchema {
            global_settings: vec![SettingsField {
                name: "global".to_string(),
                type_expr: None,
                optional: false,
                default_value: None,
                literal_values: vec![],
                is_boolean: false,
            }],
            type_alias_settings: vec![],
            model_settings: vec![SettingsField {
                name: "model".to_string(),
                type_expr: None,
                optional: false,
                default_value: None,
                literal_values: vec![],
                is_boolean: false,
            }],
            field_settings: vec![],
            has_build: true,
            has_migrate: false,
        };

        assert_eq!(schema.global_settings.len(), 1);
        assert_eq!(schema.type_alias_settings.len(), 0);
        assert_eq!(schema.model_settings.len(), 1);
        assert_eq!(schema.field_settings.len(), 0);
        assert!(schema.has_build);
        assert!(!schema.has_migrate);
    }

    #[test]
    fn test_plugin_settings_schema_debug() {
        let schema = PluginSettingsSchema {
            global_settings: vec![],
            type_alias_settings: vec![],
            model_settings: vec![],
            field_settings: vec![],
            has_build: true,
            has_migrate: true,
        };

        let debug_str = format!("{:?}", schema);
        assert!(debug_str.contains("has_build: true"));
        assert!(debug_str.contains("has_migrate: true"));
    }

    #[test]
    fn test_plugin_settings_schema_clone() {
        let schema = PluginSettingsSchema {
            global_settings: vec![SettingsField {
                name: "test".to_string(),
                type_expr: None,
                optional: false,
                default_value: None,
                literal_values: vec![],
                is_boolean: false,
            }],
            type_alias_settings: vec![],
            model_settings: vec![],
            field_settings: vec![],
            has_build: true,
            has_migrate: false,
        };

        let cloned = schema.clone();
        assert_eq!(cloned.global_settings.len(), 1);
        assert_eq!(cloned.has_build, schema.has_build);
        assert_eq!(cloned.has_migrate, schema.has_migrate);
    }

    // =========================================================================
    // fields_for_level TESTS
    // =========================================================================

    #[test]
    fn test_fields_for_level_global() {
        let schema = PluginSettingsSchema {
            global_settings: vec![SettingsField {
                name: "global_field".to_string(),
                type_expr: None,
                optional: false,
                default_value: None,
                literal_values: vec![],
                is_boolean: false,
            }],
            type_alias_settings: vec![],
            model_settings: vec![],
            field_settings: vec![],
            has_build: false,
            has_migrate: false,
        };

        let fields = schema.fields_for_level(&ConfigLevel::Global);
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].name, "global_field");
    }

    #[test]
    fn test_fields_for_level_type_alias() {
        let schema = PluginSettingsSchema {
            global_settings: vec![],
            type_alias_settings: vec![
                SettingsField {
                    name: "type_alias_field_1".to_string(),
                    type_expr: Some("string".to_string()),
                    optional: false,
                    default_value: None,
                    literal_values: vec![],
                    is_boolean: false,
                },
                SettingsField {
                    name: "type_alias_field_2".to_string(),
                    type_expr: Some("number".to_string()),
                    optional: true,
                    default_value: Some(serde_json::json!(0)),
                    literal_values: vec![],
                    is_boolean: false,
                },
            ],
            model_settings: vec![],
            field_settings: vec![],
            has_build: false,
            has_migrate: false,
        };

        let fields = schema.fields_for_level(&ConfigLevel::TypeAlias {
            name: "TestType".to_string(),
        });
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name, "type_alias_field_1");
        assert_eq!(fields[1].name, "type_alias_field_2");
    }

    #[test]
    fn test_fields_for_level_model() {
        let schema = PluginSettingsSchema {
            global_settings: vec![],
            type_alias_settings: vec![],
            model_settings: vec![SettingsField {
                name: "model_field".to_string(),
                type_expr: None,
                optional: false,
                default_value: None,
                literal_values: vec![],
                is_boolean: false,
            }],
            field_settings: vec![],
            has_build: false,
            has_migrate: false,
        };

        let fields = schema.fields_for_level(&ConfigLevel::Model {
            name: "TestModel".to_string(),
        });
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].name, "model_field");
    }

    #[test]
    fn test_fields_for_level_field() {
        let schema = PluginSettingsSchema {
            global_settings: vec![],
            type_alias_settings: vec![],
            model_settings: vec![],
            field_settings: vec![SettingsField {
                name: "field_setting".to_string(),
                type_expr: None,
                optional: false,
                default_value: None,
                literal_values: vec![],
                is_boolean: false,
            }],
            has_build: false,
            has_migrate: false,
        };

        let fields = schema.fields_for_level(&ConfigLevel::Field {
            model: "TestModel".to_string(),
            field: "test_field".to_string(),
        });
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].name, "field_setting");
    }

    #[test]
    fn test_fields_for_level_returns_slice() {
        let schema = PluginSettingsSchema {
            global_settings: vec![
                SettingsField {
                    name: "a".to_string(),
                    type_expr: None,
                    optional: false,
                    default_value: None,
                    literal_values: vec![],
                    is_boolean: false,
                },
                SettingsField {
                    name: "b".to_string(),
                    type_expr: None,
                    optional: false,
                    default_value: None,
                    literal_values: vec![],
                    is_boolean: false,
                },
                SettingsField {
                    name: "c".to_string(),
                    type_expr: None,
                    optional: false,
                    default_value: None,
                    literal_values: vec![],
                    is_boolean: false,
                },
            ],
            type_alias_settings: vec![],
            model_settings: vec![],
            field_settings: vec![],
            has_build: false,
            has_migrate: false,
        };

        let fields = schema.fields_for_level(&ConfigLevel::Global);
        // Test that we can iterate and it's the correct slice
        let names: Vec<_> = fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["a", "b", "c"]);
    }

    // =========================================================================
    // CACHE TESTS
    // =========================================================================

    #[test]
    fn test_cache_new() {
        let cache = PluginSchemaCache::new();
        // Should be empty initially
        let inner = cache.cache.read().unwrap();
        assert!(inner.is_empty());
    }

    #[test]
    fn test_cache_default() {
        // Test Default::default() implementation
        let cache = PluginSchemaCache::default();
        let inner = cache.cache.read().unwrap();
        assert!(inner.is_empty());
    }

    #[test]
    fn test_cache_clone() {
        let cache = PluginSchemaCache::new();

        // Insert an entry
        {
            let mut inner = cache.cache.write().unwrap();
            inner.insert(
                "plugin1".to_string(),
                CachedPluginSchema {
                    schema: PluginSettingsSchema {
                        global_settings: Vec::new(),
                        type_alias_settings: Vec::new(),
                        model_settings: Vec::new(),
                        field_settings: Vec::new(),
                        has_build: true,
                        has_migrate: false,
                    },
                    loaded_at: Instant::now(),
                },
            );
        }

        // Clone the cache
        let cloned = cache.clone();

        // Both should have the same plugin
        assert!(cache.cache.read().unwrap().contains_key("plugin1"));
        assert!(cloned.cache.read().unwrap().contains_key("plugin1"));
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

    #[test]
    fn test_cache_invalidate_nonexistent() {
        let cache = PluginSchemaCache::new();

        // Should not panic when invalidating non-existent plugin
        cache.invalidate("nonexistent_plugin");

        // Cache should still be empty
        assert!(cache.cache.read().unwrap().is_empty());
    }

    #[test]
    fn test_cache_clear() {
        let cache = PluginSchemaCache::new();

        // Insert multiple entries
        {
            let mut inner = cache.cache.write().unwrap();
            for i in 0..5 {
                inner.insert(
                    format!("plugin_{}", i),
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
        }

        // Verify entries exist
        assert_eq!(cache.cache.read().unwrap().len(), 5);

        // Clear the cache
        cache.clear();

        // Verify cache is empty
        assert!(cache.cache.read().unwrap().is_empty());
    }

    #[test]
    fn test_cache_clear_empty() {
        let cache = PluginSchemaCache::new();

        // Should not panic when clearing empty cache
        cache.clear();

        assert!(cache.cache.read().unwrap().is_empty());
    }

    #[test]
    fn test_cache_invalidate_partial() {
        let cache = PluginSchemaCache::new();

        // Insert multiple entries
        {
            let mut inner = cache.cache.write().unwrap();
            inner.insert(
                "keep_me".to_string(),
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
            inner.insert(
                "remove_me".to_string(),
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

        // Invalidate only one
        cache.invalidate("remove_me");

        // Verify correct entry remains
        let inner = cache.cache.read().unwrap();
        assert!(inner.contains_key("keep_me"));
        assert!(!inner.contains_key("remove_me"));
        assert_eq!(inner.len(), 1);
    }

    // =========================================================================
    // HELPER FUNCTION TESTS
    // =========================================================================

    #[test]
    fn test_extract_literal_values_single_literal() {
        use cdm_utils::ParsedType;

        let parsed = ParsedType::Literal("postgres".to_string());
        let values = extract_literal_values(&parsed);
        assert_eq!(values, vec!["postgres"]);
    }

    #[test]
    fn test_extract_literal_values_union_of_literals() {
        use cdm_utils::ParsedType;

        let parsed = ParsedType::Union(vec![
            ParsedType::Literal("active".to_string()),
            ParsedType::Literal("inactive".to_string()),
            ParsedType::Literal("pending".to_string()),
        ]);
        let values = extract_literal_values(&parsed);
        assert_eq!(values, vec!["active", "inactive", "pending"]);
    }

    #[test]
    fn test_extract_literal_values_mixed_union() {
        use cdm_utils::{ParsedType, PrimitiveType};

        // Union with some literals and some non-literals
        let parsed = ParsedType::Union(vec![
            ParsedType::Literal("literal_value".to_string()),
            ParsedType::Primitive(PrimitiveType::String),
            ParsedType::Literal("another_literal".to_string()),
        ]);
        let values = extract_literal_values(&parsed);
        // Should only return literals
        assert_eq!(values, vec!["literal_value", "another_literal"]);
    }

    #[test]
    fn test_extract_literal_values_primitive() {
        use cdm_utils::{ParsedType, PrimitiveType};

        let parsed = ParsedType::Primitive(PrimitiveType::String);
        let values = extract_literal_values(&parsed);
        assert!(values.is_empty());
    }

    #[test]
    fn test_extract_literal_values_array() {
        use cdm_utils::{ParsedType, PrimitiveType};

        let parsed = ParsedType::Array(Box::new(ParsedType::Primitive(PrimitiveType::String)));
        let values = extract_literal_values(&parsed);
        assert!(values.is_empty());
    }

    #[test]
    fn test_extract_literal_values_reference() {
        use cdm_utils::ParsedType;

        let parsed = ParsedType::Reference("CustomType".to_string());
        let values = extract_literal_values(&parsed);
        assert!(values.is_empty());
    }

    #[test]
    fn test_extract_literal_values_null() {
        use cdm_utils::ParsedType;

        let parsed = ParsedType::Null;
        let values = extract_literal_values(&parsed);
        assert!(values.is_empty());
    }

    #[test]
    fn test_extract_literal_values_union_no_literals() {
        use cdm_utils::{ParsedType, PrimitiveType};

        let parsed = ParsedType::Union(vec![
            ParsedType::Primitive(PrimitiveType::String),
            ParsedType::Primitive(PrimitiveType::Number),
        ]);
        let values = extract_literal_values(&parsed);
        assert!(values.is_empty());
    }

    #[test]
    fn test_is_boolean_type_true() {
        use cdm_utils::{ParsedType, PrimitiveType};

        let parsed = ParsedType::Primitive(PrimitiveType::Boolean);
        assert!(is_boolean_type(&parsed));
    }

    #[test]
    fn test_is_boolean_type_string() {
        use cdm_utils::{ParsedType, PrimitiveType};

        let parsed = ParsedType::Primitive(PrimitiveType::String);
        assert!(!is_boolean_type(&parsed));
    }

    #[test]
    fn test_is_boolean_type_number() {
        use cdm_utils::{ParsedType, PrimitiveType};

        let parsed = ParsedType::Primitive(PrimitiveType::Number);
        assert!(!is_boolean_type(&parsed));
    }

    #[test]
    fn test_is_boolean_type_literal() {
        use cdm_utils::ParsedType;

        let parsed = ParsedType::Literal("true".to_string());
        assert!(!is_boolean_type(&parsed));
    }

    #[test]
    fn test_is_boolean_type_reference() {
        use cdm_utils::ParsedType;

        let parsed = ParsedType::Reference("Boolean".to_string());
        assert!(!is_boolean_type(&parsed));
    }

    #[test]
    fn test_is_boolean_type_array() {
        use cdm_utils::{ParsedType, PrimitiveType};

        let parsed = ParsedType::Array(Box::new(ParsedType::Primitive(PrimitiveType::Boolean)));
        assert!(!is_boolean_type(&parsed));
    }

    #[test]
    fn test_is_boolean_type_union() {
        use cdm_utils::{ParsedType, PrimitiveType};

        // Even a union with boolean is not a boolean type
        let parsed = ParsedType::Union(vec![
            ParsedType::Primitive(PrimitiveType::Boolean),
            ParsedType::Primitive(PrimitiveType::String),
        ]);
        assert!(!is_boolean_type(&parsed));
    }

    #[test]
    fn test_is_boolean_type_null() {
        use cdm_utils::ParsedType;

        let parsed = ParsedType::Null;
        assert!(!is_boolean_type(&parsed));
    }

    // =========================================================================
    // CACHED PLUGIN SCHEMA TESTS
    // =========================================================================

    #[test]
    fn test_cached_plugin_schema_timestamp() {
        let before = Instant::now();

        let cached = CachedPluginSchema {
            schema: PluginSettingsSchema {
                global_settings: Vec::new(),
                type_alias_settings: Vec::new(),
                model_settings: Vec::new(),
                field_settings: Vec::new(),
                has_build: false,
                has_migrate: false,
            },
            loaded_at: Instant::now(),
        };

        let after = Instant::now();

        // loaded_at should be between before and after
        assert!(cached.loaded_at >= before);
        assert!(cached.loaded_at <= after);
    }
}
