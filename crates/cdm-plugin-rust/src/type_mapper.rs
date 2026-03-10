use cdm_plugin_interface::TypeExpression;

/// Configuration for the type mapper
pub struct TypeMapperConfig {
    pub number_type: String,
    pub map_type: String,
}

impl Default for TypeMapperConfig {
    fn default() -> Self {
        Self {
            number_type: "f64".to_string(),
            map_type: "HashMap".to_string(),
        }
    }
}

/// Maps a CDM type expression to a Rust type string.
/// For union types, this returns an empty string since unions need
/// enum generation handled by build.rs.
pub fn map_type_to_rust(type_expr: &TypeExpression, config: &TypeMapperConfig) -> String {
    match type_expr {
        TypeExpression::Identifier { name } => map_builtin_type(name, config),
        TypeExpression::Array { element_type } => {
            format!("Vec<{}>", map_type_to_rust(element_type, config))
        }
        TypeExpression::Map {
            value_type,
            key_type,
        } => {
            let key_rust = map_type_to_rust(key_type, config);
            let value_rust = map_type_to_rust(value_type, config);
            format!("{}<{}, {}>", config.map_type, key_rust, value_rust)
        }
        TypeExpression::Union { .. } => {
            // Union types cannot be directly represented in Rust.
            // build.rs handles enum generation and provides the type name.
            String::new()
        }
        TypeExpression::StringLiteral { value } => {
            // String literals in type position - return the literal for reference
            format!("\"{}\"", escape_string(value))
        }
        TypeExpression::NumberLiteral { value } => {
            if value.fract() == 0.0 {
                format!("{}", *value as i64)
            } else {
                format!("{}", value)
            }
        }
    }
}

/// Maps CDM built-in types to Rust types
pub fn map_builtin_type(name: &str, config: &TypeMapperConfig) -> String {
    match name {
        "string" => "String".to_string(),
        "number" => config.number_type.clone(),
        "boolean" => "bool".to_string(),
        "JSON" => "serde_json::Value".to_string(),
        // User-defined types are passed through as-is
        other => other.to_string(),
    }
}

/// Escapes special characters in string literals
pub fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Checks if a union type expression consists entirely of string literals
pub fn is_string_literal_union(type_expr: &TypeExpression) -> bool {
    match type_expr {
        TypeExpression::Union { types } => {
            types
                .iter()
                .all(|t| matches!(t, TypeExpression::StringLiteral { .. }))
        }
        _ => false,
    }
}

/// Checks if a union type expression consists entirely of type identifiers
pub fn is_type_reference_union(type_expr: &TypeExpression) -> bool {
    match type_expr {
        TypeExpression::Union { types } => {
            types
                .iter()
                .all(|t| matches!(t, TypeExpression::Identifier { .. }))
        }
        _ => false,
    }
}

/// Checks if a type expression is a union type
pub fn is_union_type(type_expr: &TypeExpression) -> bool {
    matches!(type_expr, TypeExpression::Union { .. })
}


#[cfg(test)]
#[path = "type_mapper/type_mapper_tests.rs"]
mod type_mapper_tests;
