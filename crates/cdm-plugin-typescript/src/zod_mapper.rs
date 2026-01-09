use cdm_plugin_interface::TypeExpression;

/// Maps a CDM type expression to a Zod schema string
pub fn map_type_to_zod(type_expr: &TypeExpression, strict_nulls: bool) -> String {
    match type_expr {
        TypeExpression::Identifier { name } => map_builtin_type_to_zod(name, strict_nulls),
        TypeExpression::Array { element_type } => {
            format!("z.array({})", map_type_to_zod(element_type, strict_nulls))
        }
        TypeExpression::Union { types } => {
            if types.len() == 1 {
                map_type_to_zod(&types[0], strict_nulls)
            } else {
                let type_strings: Vec<String> = types
                    .iter()
                    .map(|t| map_type_to_zod(t, strict_nulls))
                    .collect();
                format!("z.union([{}])", type_strings.join(", "))
            }
        }
        TypeExpression::StringLiteral { value } => {
            format!("z.literal(\"{}\")", escape_string(value))
        }
    }
}

/// Maps CDM built-in types to Zod schema types
fn map_builtin_type_to_zod(name: &str, strict_nulls: bool) -> String {
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
        other => format!("{}Schema", other),
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
