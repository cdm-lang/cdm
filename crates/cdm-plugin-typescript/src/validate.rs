use cdm_plugin_interface::{ConfigLevel, PathSegment, Severity, Utils, ValidationError, JSON};

pub fn validate_config(
    level: ConfigLevel,
    config: JSON,
    _utils: &Utils,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    match level {
        ConfigLevel::Global => {
            validate_global_config(&config, &mut errors);
        }
        ConfigLevel::TypeAlias { name } => {
            validate_type_alias_config(&config, &name, &mut errors);
        }
        ConfigLevel::Model { name } => {
            validate_model_config(&config, &name, &mut errors);
        }
        ConfigLevel::Field { model, field } => {
            validate_field_config(&config, &model, &field, &mut errors);
        }
    }

    errors
}

fn validate_global_config(config: &JSON, errors: &mut Vec<ValidationError>) {
    // Validate output_format
    if let Some(format) = config.get("output_format") {
        if let Some(format_str) = format.as_str() {
            if !["interface", "class", "type"].contains(&format_str) {
                errors.push(ValidationError {
                    path: vec![PathSegment {
                        kind: "global".to_string(),
                        name: "output_format".to_string(),
                    }],
                    message: "output_format must be 'interface', 'class', or 'type'".to_string(),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Validate file_strategy
    if let Some(strategy) = config.get("file_strategy") {
        if let Some(strategy_str) = strategy.as_str() {
            if !["single", "per_model"].contains(&strategy_str) {
                errors.push(ValidationError {
                    path: vec![PathSegment {
                        kind: "global".to_string(),
                        name: "file_strategy".to_string(),
                    }],
                    message: "file_strategy must be 'single' or 'per_model'".to_string(),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Validate optional_strategy
    if let Some(strategy) = config.get("optional_strategy") {
        if let Some(strategy_str) = strategy.as_str() {
            if !["native", "union_undefined"].contains(&strategy_str) {
                errors.push(ValidationError {
                    path: vec![PathSegment {
                        kind: "global".to_string(),
                        name: "optional_strategy".to_string(),
                    }],
                    message: "optional_strategy must be 'native' or 'union_undefined'".to_string(),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Validate type_name_format
    if let Some(format) = config.get("type_name_format") {
        if let Some(format_str) = format.as_str() {
            if !["preserve", "pascal", "camel", "snake", "kebab", "constant"].contains(&format_str) {
                errors.push(ValidationError {
                    path: vec![PathSegment {
                        kind: "global".to_string(),
                        name: "type_name_format".to_string(),
                    }],
                    message: "type_name_format must be 'preserve', 'pascal', 'camel', 'snake', 'kebab', or 'constant'".to_string(),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Validate field_name_format
    if let Some(format) = config.get("field_name_format") {
        if let Some(format_str) = format.as_str() {
            if !["preserve", "pascal", "camel", "snake", "kebab", "constant"].contains(&format_str) {
                errors.push(ValidationError {
                    path: vec![PathSegment {
                        kind: "global".to_string(),
                        name: "field_name_format".to_string(),
                    }],
                    message: "field_name_format must be 'preserve', 'pascal', 'camel', 'snake', 'kebab', or 'constant'".to_string(),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Validate single_file_name ends with .ts
    if let Some(file_name) = config.get("single_file_name") {
        if let Some(file_name_str) = file_name.as_str() {
            if !file_name_str.ends_with(".ts") {
                errors.push(ValidationError {
                    path: vec![PathSegment {
                        kind: "global".to_string(),
                        name: "single_file_name".to_string(),
                    }],
                    message: "single_file_name must end with '.ts'".to_string(),
                    severity: Severity::Error,
                });
            }
        }
    }
}

fn validate_type_alias_config(
    config: &JSON,
    alias_name: &str,
    errors: &mut Vec<ValidationError>,
) {
    // Validate export_name is a valid TypeScript identifier
    if let Some(export_name) = config.get("export_name") {
        if let Some(export_name_str) = export_name.as_str() {
            if !is_valid_typescript_identifier(export_name_str) {
                errors.push(ValidationError {
                    path: vec![
                        PathSegment {
                            kind: "type_alias".to_string(),
                            name: alias_name.to_string(),
                        },
                        PathSegment {
                            kind: "config".to_string(),
                            name: "export_name".to_string(),
                        },
                    ],
                    message: format!("'{}' is not a valid TypeScript identifier", export_name_str),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Note: We don't validate type_override syntax as it could be complex TypeScript types
    // The TypeScript compiler will catch any invalid types
}

fn validate_model_config(config: &JSON, model_name: &str, errors: &mut Vec<ValidationError>) {
    // Validate output_format override
    if let Some(format) = config.get("output_format") {
        if let Some(format_str) = format.as_str() {
            if !["interface", "class", "type"].contains(&format_str) {
                errors.push(ValidationError {
                    path: vec![
                        PathSegment {
                            kind: "model".to_string(),
                            name: model_name.to_string(),
                        },
                        PathSegment {
                            kind: "config".to_string(),
                            name: "output_format".to_string(),
                        },
                    ],
                    message: "output_format must be 'interface', 'class', or 'type'".to_string(),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Validate export_name is a valid TypeScript identifier
    if let Some(export_name) = config.get("export_name") {
        if let Some(export_name_str) = export_name.as_str() {
            if !is_valid_typescript_identifier(export_name_str) {
                errors.push(ValidationError {
                    path: vec![
                        PathSegment {
                            kind: "model".to_string(),
                            name: model_name.to_string(),
                        },
                        PathSegment {
                            kind: "config".to_string(),
                            name: "export_name".to_string(),
                        },
                    ],
                    message: format!("'{}' is not a valid TypeScript identifier", export_name_str),
                    severity: Severity::Error,
                });
            }
        }
    }
}

fn validate_field_config(config: &JSON, model_name: &str, field_name: &str, errors: &mut Vec<ValidationError>) {
    // Validate field_name override is a valid TypeScript identifier
    if let Some(override_name) = config.get("field_name") {
        if let Some(override_name_str) = override_name.as_str() {
            if !is_valid_typescript_identifier(override_name_str) {
                errors.push(ValidationError {
                    path: vec![
                        PathSegment {
                            kind: "model".to_string(),
                            name: model_name.to_string(),
                        },
                        PathSegment {
                            kind: "field".to_string(),
                            name: field_name.to_string(),
                        },
                        PathSegment {
                            kind: "config".to_string(),
                            name: "field_name".to_string(),
                        },
                    ],
                    message: format!("'{}' is not a valid TypeScript identifier", override_name_str),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Note: We don't validate type_override syntax as it could be complex TypeScript types
    // The TypeScript compiler will catch any invalid types
}

fn is_valid_typescript_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    let mut chars = s.chars();
    let first = chars.next().unwrap();

    // First character must be letter, underscore, or dollar sign
    if !first.is_alphabetic() && first != '_' && first != '$' {
        return false;
    }

    // Remaining characters can be letters, digits, underscores, or dollar signs
    for ch in chars {
        if !ch.is_alphanumeric() && ch != '_' && ch != '$' {
            return false;
        }
    }

    // Check against TypeScript reserved keywords
    let reserved = [
        "break", "case", "catch", "class", "const", "continue", "debugger", "default",
        "delete", "do", "else", "enum", "export", "extends", "false", "finally",
        "for", "function", "if", "import", "in", "instanceof", "new", "null",
        "return", "super", "switch", "this", "throw", "true", "try", "typeof",
        "var", "void", "while", "with", "implements", "interface", "let", "package",
        "private", "protected", "public", "static", "yield", "any", "boolean",
        "constructor", "declare", "get", "module", "require", "number", "set",
        "string", "symbol", "type", "from", "of", "async", "await",
    ];

    !reserved.contains(&s)
}


#[cfg(test)]
#[path = "validate/validate_tests.rs"]
mod validate_tests;
