use cdm_plugin_interface::TypeExpression;

/// Maps a CDM type expression to a TypeScript type string
pub fn map_type_to_typescript(type_expr: &TypeExpression, strict_nulls: bool) -> String {
    match type_expr {
        TypeExpression::Identifier { name } => map_builtin_type(name, strict_nulls),
        TypeExpression::Array { element_type } => {
            format!("{}[]", map_type_to_typescript(element_type, strict_nulls))
        }
        TypeExpression::Map { value_type, key_type } => {
            let key_ts = map_key_type_to_typescript(key_type, strict_nulls);
            let value_ts = map_type_to_typescript(value_type, strict_nulls);
            format!("Record<{}, {}>", key_ts, value_ts)
        }
        TypeExpression::Union { types } => {
            let type_strings: Vec<String> = types
                .iter()
                .map(|t| map_type_to_typescript(t, strict_nulls))
                .collect();
            type_strings.join(" | ")
        }
        TypeExpression::StringLiteral { value } => {
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

/// Maps a CDM key type expression to a TypeScript type for Record keys
fn map_key_type_to_typescript(type_expr: &TypeExpression, strict_nulls: bool) -> String {
    match type_expr {
        TypeExpression::Identifier { name } => {
            match name.as_str() {
                "string" => "string".to_string(),
                "number" => "number".to_string(),
                // Type alias - use as-is
                other => other.to_string(),
            }
        }
        TypeExpression::Union { types } => {
            let type_strings: Vec<String> = types
                .iter()
                .map(|t| map_key_type_to_typescript(t, strict_nulls))
                .collect();
            type_strings.join(" | ")
        }
        TypeExpression::StringLiteral { value } => {
            format!("\"{}\"", escape_string(value))
        }
        TypeExpression::NumberLiteral { value } => {
            if value.fract() == 0.0 {
                format!("{}", *value as i64)
            } else {
                format!("{}", value)
            }
        }
        // Array and Map types are not valid keys - fallback to string
        _ => "string".to_string(),
    }
}

/// Maps CDM built-in types to TypeScript types
fn map_builtin_type(name: &str, strict_nulls: bool) -> String {
    match name {
        "string" => "string".to_string(),
        "number" => "number".to_string(),
        "boolean" => "boolean".to_string(),
        "JSON" => {
            if strict_nulls {
                "Record<string, unknown> | unknown[]".to_string()
            } else {
                "any".to_string()
            }
        }
        // User-defined types are passed through as-is
        other => other.to_string(),
    }
}

/// Escapes special characters in string literals
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
