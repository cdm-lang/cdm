use cdm_plugin_interface::{TypeAliasDefinition, TypeExpression, JSON};
use std::collections::HashMap;

/// TypeMapper handles conversion of CDM types to TypeORM/PostgreSQL column types
/// and TypeScript types for entity properties.
pub struct TypeMapper<'a> {
    default_string_length: i64,
    type_aliases: &'a HashMap<String, TypeAliasDefinition>,
    models: Vec<String>,
}

impl<'a> TypeMapper<'a> {
    pub fn new(
        config: &JSON,
        type_aliases: &'a HashMap<String, TypeAliasDefinition>,
        model_names: Vec<String>,
    ) -> Self {
        let default_string_length = config
            .get("default_string_length")
            .and_then(|v| v.as_i64())
            .unwrap_or(255);

        TypeMapper {
            default_string_length,
            type_aliases,
            models: model_names,
        }
    }

    /// Check if a type expression references a model (for relation detection)
    pub fn is_model_reference(&self, type_expr: &TypeExpression) -> Option<String> {
        match type_expr {
            TypeExpression::Identifier { name } => {
                if self.models.contains(name) {
                    Some(name.clone())
                } else {
                    None
                }
            }
            TypeExpression::Array { element_type } => self.is_model_reference(element_type),
            _ => None,
        }
    }

    /// Check if type is an array of models
    #[allow(dead_code)]
    pub fn is_model_array(&self, type_expr: &TypeExpression) -> bool {
        matches!(type_expr, TypeExpression::Array { element_type } if self.is_model_reference(element_type).is_some())
    }

    /// Map a CDM type expression to a PostgreSQL column type for TypeORM
    pub fn map_to_column_type(&self, type_expr: &TypeExpression) -> String {
        match type_expr {
            TypeExpression::Identifier { name } => {
                match name.as_str() {
                    "string" => format!("varchar({})", self.default_string_length),
                    "number" => "double precision".to_string(),
                    "boolean" => "boolean".to_string(),
                    "JSON" => "jsonb".to_string(),
                    // Check if it's a type alias
                    _ => {
                        if let Some(type_alias) = self.type_aliases.get(name) {
                            // Check for explicit column_type override in type alias config
                            if let Some(col_type) =
                                type_alias.config.get("column_type").and_then(|t| t.as_str())
                            {
                                return col_type.to_string();
                            }
                            // Otherwise, recursively resolve the underlying type
                            self.map_to_column_type(&type_alias.alias_type)
                        } else if self.models.contains(name) {
                            // Model references don't have a column type (they're relations)
                            // Return empty - caller should check is_model_reference first
                            String::new()
                        } else {
                            // Unknown type - default to jsonb
                            "jsonb".to_string()
                        }
                    }
                }
            }

            TypeExpression::Array { element_type } => {
                // Check if it's an array of models (relation, no column)
                if self.is_model_reference(element_type).is_some() {
                    return String::new();
                }
                // PostgreSQL array type
                let inner_type = self.map_to_column_type(element_type);
                format!("{}[]", inner_type)
            }

            TypeExpression::Union { types } => {
                // Check if all types are string literals (enum-like)
                let all_string_literals = types.iter().all(|t| {
                    matches!(t, TypeExpression::StringLiteral { .. })
                });

                if all_string_literals {
                    // Could be an enum, but TypeORM handles this differently
                    // For now, use varchar
                    format!("varchar({})", self.default_string_length)
                } else {
                    // Mixed union - use jsonb
                    "jsonb".to_string()
                }
            }

            TypeExpression::StringLiteral { .. } => {
                // Single string literal type
                format!("varchar({})", self.default_string_length)
            }
        }
    }

    /// Map a CDM type expression to a TypeScript type for entity properties
    pub fn map_to_typescript_type(&self, type_expr: &TypeExpression) -> String {
        match type_expr {
            TypeExpression::Identifier { name } => {
                match name.as_str() {
                    "string" => "string".to_string(),
                    "number" => "number".to_string(),
                    "boolean" => "boolean".to_string(),
                    "JSON" => "Record<string, unknown>".to_string(),
                    // Type alias or model reference - use the name as-is
                    _ => {
                        if let Some(type_alias) = self.type_aliases.get(name) {
                            // Recursively resolve to get the underlying TypeScript type
                            self.map_to_typescript_type(&type_alias.alias_type)
                        } else {
                            // Model reference or unknown - use the name as TypeScript type
                            name.clone()
                        }
                    }
                }
            }

            TypeExpression::Array { element_type } => {
                let inner_type = self.map_to_typescript_type(element_type);
                format!("{}[]", inner_type)
            }

            TypeExpression::Union { types } => {
                let type_strings: Vec<String> = types
                    .iter()
                    .map(|t| self.map_to_typescript_type(t))
                    .collect();
                type_strings.join(" | ")
            }

            TypeExpression::StringLiteral { value } => {
                format!("\"{}\"", escape_string(value))
            }
        }
    }
}

/// Escape special characters in a string for TypeScript string literals
fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
#[path = "type_mapper/type_mapper_tests.rs"]
mod type_mapper_tests;
