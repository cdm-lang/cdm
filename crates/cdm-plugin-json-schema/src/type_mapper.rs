use cdm_plugin_interface::{TypeExpression, JSON};
use serde_json::{json, Map, Value};

pub struct TypeMapper {
    /// Global union mode setting
    pub union_mode: String,
}

impl TypeMapper {
    pub fn new(union_mode: String) -> Self {
        Self {
            union_mode,
        }
    }

    /// Map a CDM TypeExpression to a JSON Schema type definition
    pub fn map_type(
        &mut self,
        type_expr: &TypeExpression,
        type_alias_union_mode: Option<&str>,
        field_config: &JSON,
    ) -> Value {
        // Check for custom_type override in field config
        if let Some(custom) = field_config.get("custom_type") {
            if let Some(custom_str) = custom.as_str() {
                return json!({ "type": custom_str });
            }
        }

        match type_expr {
            TypeExpression::Identifier { name } => {
                self.map_identifier(name)
            }
            TypeExpression::Array { element_type } => {
                let items = self.map_type(element_type, type_alias_union_mode, &json!({}));
                json!({
                    "type": "array",
                    "items": items
                })
            }
            TypeExpression::Map { value_type, key_type: _ } => {
                // JSON Schema represents maps as objects with additionalProperties
                let value_schema = self.map_type(value_type, type_alias_union_mode, &json!({}));
                json!({
                    "type": "object",
                    "additionalProperties": value_schema
                })
            }
            TypeExpression::Union { types } => {
                self.map_union(types, type_alias_union_mode)
            }
            TypeExpression::StringLiteral { value } => {
                json!({ "const": value })
            }
            TypeExpression::NumberLiteral { value } => {
                json!({ "const": value })
            }
        }
    }

    /// Map a simple type identifier to JSON Schema
    fn map_identifier(&self, name: &str) -> Value {
        match name {
            "string" => json!({ "type": "string" }),
            "number" => json!({ "type": "number" }),
            "boolean" => json!({ "type": "boolean" }),
            "JSON" => json!({}), // No type restriction for arbitrary JSON
            _ => {
                // This is a model or type alias reference
                // In JSON Schema, we use $ref to reference other schemas
                json!({ "$ref": format!("#/$defs/{}", name) })
            }
        }
    }

    /// Map a union type to JSON Schema
    fn map_union(&self, types: &[TypeExpression], type_alias_union_mode: Option<&str>) -> Value {
        // Determine which union mode to use (type alias override or global default)
        let mode = type_alias_union_mode.unwrap_or(&self.union_mode);

        // Check if this is a string literal-only union
        let all_string_literals = types.iter().all(|t| matches!(t, TypeExpression::StringLiteral { .. }));

        if all_string_literals && mode == "enum" {
            // Use enum for string literal unions
            let values: Vec<Value> = types
                .iter()
                .filter_map(|t| {
                    if let TypeExpression::StringLiteral { value } = t {
                        Some(json!(value))
                    } else {
                        None
                    }
                })
                .collect();

            json!({
                "type": "string",
                "enum": values
            })
        } else {
            // Use oneOf for mixed unions or when explicitly requested
            let schemas: Vec<Value> = types
                .iter()
                .map(|t| {
                    match t {
                        TypeExpression::StringLiteral { value } => json!({ "const": value }),
                        _ => self.map_type_readonly(t, None, &json!({})),
                    }
                })
                .collect();

            json!({ "oneOf": schemas })
        }
    }

    /// Read-only version of map_type (doesn't modify self)
    fn map_type_readonly(&self, type_expr: &TypeExpression, type_alias_union_mode: Option<&str>, _field_config: &JSON) -> Value {
        match type_expr {
            TypeExpression::Identifier { name } => {
                self.map_identifier(name)
            }
            TypeExpression::Array { element_type } => {
                let items = self.map_type_readonly(element_type, type_alias_union_mode, &json!({}));
                json!({
                    "type": "array",
                    "items": items
                })
            }
            TypeExpression::Map { value_type, key_type: _ } => {
                let value_schema = self.map_type_readonly(value_type, type_alias_union_mode, &json!({}));
                json!({
                    "type": "object",
                    "additionalProperties": value_schema
                })
            }
            TypeExpression::Union { types } => {
                self.map_union(types, type_alias_union_mode)
            }
            TypeExpression::StringLiteral { value } => {
                json!({ "const": value })
            }
            TypeExpression::NumberLiteral { value } => {
                json!({ "const": value })
            }
        }
    }
}

/// Apply field-level JSON Schema constraints from config
pub fn apply_field_constraints(mut schema: Map<String, Value>, field_config: &JSON) -> Map<String, Value> {
    // String constraints
    if let Some(pattern) = field_config.get("pattern") {
        schema.insert("pattern".to_string(), pattern.clone());
    }
    if let Some(min_length) = field_config.get("min_length") {
        schema.insert("minLength".to_string(), min_length.clone());
    }
    if let Some(max_length) = field_config.get("max_length") {
        schema.insert("maxLength".to_string(), max_length.clone());
    }
    if let Some(format) = field_config.get("format") {
        schema.insert("format".to_string(), format.clone());
    }

    // Number constraints
    if let Some(minimum) = field_config.get("minimum") {
        schema.insert("minimum".to_string(), minimum.clone());
    }
    if let Some(maximum) = field_config.get("maximum") {
        schema.insert("maximum".to_string(), maximum.clone());
    }
    if let Some(exclusive_min) = field_config.get("exclusive_minimum") {
        schema.insert("exclusiveMinimum".to_string(), exclusive_min.clone());
    }
    if let Some(exclusive_max) = field_config.get("exclusive_maximum") {
        schema.insert("exclusiveMaximum".to_string(), exclusive_max.clone());
    }

    // Description
    if let Some(description) = field_config.get("description") {
        schema.insert("description".to_string(), description.clone());
    }

    // Examples
    if let Some(examples) = field_config.get("examples") {
        if let Some(examples_arr) = examples.as_array() {
            schema.insert("examples".to_string(), json!(examples_arr));
        }
    }

    schema
}

#[cfg(test)]
#[path = "type_mapper/type_mapper_tests.rs"]
mod type_mapper_tests;
