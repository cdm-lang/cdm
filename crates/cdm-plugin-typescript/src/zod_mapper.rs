use cdm_plugin_interface::TypeExpression;
use std::collections::HashSet;

/// Maps a CDM type expression to a Zod schema string.
/// This is a convenience wrapper that doesn't handle lazy types.
#[cfg(test)]
pub fn map_type_to_zod(type_expr: &TypeExpression, strict_nulls: bool) -> String {
    map_type_to_zod_with_lazy(type_expr, strict_nulls, &HashSet::new())
}

/// Maps a CDM type expression to a Zod schema string, with support for lazy evaluation.
/// Types in `lazy_types` will be wrapped with `z.lazy(() => ...)` to handle circular references.
pub fn map_type_to_zod_with_lazy(
    type_expr: &TypeExpression,
    strict_nulls: bool,
    lazy_types: &HashSet<String>,
) -> String {
    match type_expr {
        TypeExpression::Identifier { name } => {
            map_builtin_type_to_zod_with_lazy(name, strict_nulls, lazy_types)
        }
        TypeExpression::Array { element_type } => {
            format!(
                "z.array({})",
                map_type_to_zod_with_lazy(element_type, strict_nulls, lazy_types)
            )
        }
        TypeExpression::Map { value_type, key_type } => {
            // Zod uses z.record(keySchema, valueSchema) for maps
            let key_zod = map_key_type_to_zod(key_type, strict_nulls, lazy_types);
            let value_zod = map_type_to_zod_with_lazy(value_type, strict_nulls, lazy_types);
            format!("z.record({}, {})", key_zod, value_zod)
        }
        TypeExpression::Union { types } => {
            if types.len() == 1 {
                map_type_to_zod_with_lazy(&types[0], strict_nulls, lazy_types)
            } else {
                let type_strings: Vec<String> = types
                    .iter()
                    .map(|t| map_type_to_zod_with_lazy(t, strict_nulls, lazy_types))
                    .collect();
                format!("z.union([{}])", type_strings.join(", "))
            }
        }
        TypeExpression::StringLiteral { value } => {
            format!("z.literal(\"{}\")", escape_string(value))
        }
        TypeExpression::NumberLiteral { value } => {
            if value.fract() == 0.0 {
                format!("z.literal({})", *value as i64)
            } else {
                format!("z.literal({})", value)
            }
        }
    }
}

/// Maps a CDM key type expression to a Zod key schema
fn map_key_type_to_zod(
    type_expr: &TypeExpression,
    strict_nulls: bool,
    lazy_types: &HashSet<String>,
) -> String {
    match type_expr {
        TypeExpression::Identifier { name } => {
            match name.as_str() {
                "string" => "z.string()".to_string(),
                "number" => "z.number()".to_string(),
                // Type alias - reference schema
                other => {
                    let schema_name = format!("{}Schema", other);
                    if lazy_types.contains(other) {
                        format!("z.lazy(() => {})", schema_name)
                    } else {
                        schema_name
                    }
                }
            }
        }
        TypeExpression::Union { types } => {
            if types.len() == 1 {
                map_key_type_to_zod(&types[0], strict_nulls, lazy_types)
            } else {
                let type_strings: Vec<String> = types
                    .iter()
                    .map(|t| map_key_type_to_zod(t, strict_nulls, lazy_types))
                    .collect();
                format!("z.union([{}])", type_strings.join(", "))
            }
        }
        TypeExpression::StringLiteral { value } => {
            format!("z.literal(\"{}\")", escape_string(value))
        }
        TypeExpression::NumberLiteral { value } => {
            if value.fract() == 0.0 {
                format!("z.literal({})", *value as i64)
            } else {
                format!("z.literal({})", value)
            }
        }
        // Array and Map types are not valid keys - fallback to string
        _ => "z.string()".to_string(),
    }
}

/// Maps CDM built-in types to Zod schema types, with support for lazy evaluation.
fn map_builtin_type_to_zod_with_lazy(
    name: &str,
    strict_nulls: bool,
    lazy_types: &HashSet<String>,
) -> String {
    match name {
        "string" => "z.string()".to_string(),
        "number" => "z.number()".to_string(),
        "boolean" => "z.boolean()".to_string(),
        "JSON" => {
            if strict_nulls {
                "z.record(z.string(), z.unknown()).or(z.array(z.unknown()))".to_string()
            } else {
                "z.any()".to_string()
            }
        }
        // User-defined types reference their schema
        other => {
            let schema_name = format!("{}Schema", other);
            if lazy_types.contains(other) {
                format!("z.lazy(() => {})", schema_name)
            } else {
                schema_name
            }
        }
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
#[path = "zod_mapper/zod_mapper_tests.rs"]
mod zod_mapper_tests;
