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

    // Validate single_file_name ends with .rs
    if let Some(file_name) = config.get("single_file_name") {
        if let Some(file_name_str) = file_name.as_str() {
            if !file_name_str.ends_with(".rs") {
                errors.push(ValidationError {
                    path: vec![PathSegment {
                        kind: "global".to_string(),
                        name: "single_file_name".to_string(),
                    }],
                    message: "single_file_name must end with '.rs'".to_string(),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Validate type_name_format
    if let Some(format) = config.get("type_name_format") {
        if let Some(format_str) = format.as_str() {
            if !["preserve", "pascal", "camel", "snake", "kebab", "constant"]
                .contains(&format_str)
            {
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
            if !["preserve", "pascal", "camel", "snake", "kebab", "constant"]
                .contains(&format_str)
            {
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

    // Validate number_type
    if let Some(number_type) = config.get("number_type") {
        if let Some(number_type_str) = number_type.as_str() {
            if !["f64", "f32", "i32", "i64", "u32", "u64"].contains(&number_type_str) {
                errors.push(ValidationError {
                    path: vec![PathSegment {
                        kind: "global".to_string(),
                        name: "number_type".to_string(),
                    }],
                    message: "number_type must be 'f64', 'f32', 'i32', 'i64', 'u32', or 'u64'"
                        .to_string(),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Validate map_type
    if let Some(map_type) = config.get("map_type") {
        if let Some(map_type_str) = map_type.as_str() {
            if !["HashMap", "BTreeMap"].contains(&map_type_str) {
                errors.push(ValidationError {
                    path: vec![PathSegment {
                        kind: "global".to_string(),
                        name: "map_type".to_string(),
                    }],
                    message: "map_type must be 'HashMap' or 'BTreeMap'".to_string(),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Validate visibility
    if let Some(visibility) = config.get("visibility") {
        if let Some(visibility_str) = visibility.as_str() {
            if !["pub", "pub_crate", "private"].contains(&visibility_str) {
                errors.push(ValidationError {
                    path: vec![PathSegment {
                        kind: "global".to_string(),
                        name: "visibility".to_string(),
                    }],
                    message: "visibility must be 'pub', 'pub_crate', or 'private'".to_string(),
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
    // Validate export_name is a valid Rust identifier
    if let Some(export_name) = config.get("export_name") {
        if let Some(export_name_str) = export_name.as_str() {
            if !is_valid_rust_identifier(export_name_str) {
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
                    message: format!("'{}' is not a valid Rust identifier", export_name_str),
                    severity: Severity::Error,
                });
            }
        }
    }
}

fn validate_model_config(config: &JSON, model_name: &str, errors: &mut Vec<ValidationError>) {
    // Validate struct_name is a valid Rust identifier
    if let Some(struct_name) = config.get("struct_name") {
        if let Some(struct_name_str) = struct_name.as_str() {
            if !is_valid_rust_identifier(struct_name_str) {
                errors.push(ValidationError {
                    path: vec![
                        PathSegment {
                            kind: "model".to_string(),
                            name: model_name.to_string(),
                        },
                        PathSegment {
                            kind: "config".to_string(),
                            name: "struct_name".to_string(),
                        },
                    ],
                    message: format!("'{}' is not a valid Rust identifier", struct_name_str),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Validate visibility override
    if let Some(visibility) = config.get("visibility") {
        if let Some(visibility_str) = visibility.as_str() {
            if !["pub", "pub_crate", "private"].contains(&visibility_str) {
                errors.push(ValidationError {
                    path: vec![
                        PathSegment {
                            kind: "model".to_string(),
                            name: model_name.to_string(),
                        },
                        PathSegment {
                            kind: "config".to_string(),
                            name: "visibility".to_string(),
                        },
                    ],
                    message: "visibility must be 'pub', 'pub_crate', or 'private'".to_string(),
                    severity: Severity::Error,
                });
            }
        }
    }
}

fn validate_field_config(
    config: &JSON,
    model_name: &str,
    field_name: &str,
    errors: &mut Vec<ValidationError>,
) {
    // Validate field_name override is a valid Rust identifier
    if let Some(override_name) = config.get("field_name") {
        if let Some(override_name_str) = override_name.as_str() {
            if !is_valid_rust_identifier(override_name_str) {
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
                    message: format!("'{}' is not a valid Rust identifier", override_name_str),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Validate visibility override
    if let Some(visibility) = config.get("visibility") {
        if let Some(visibility_str) = visibility.as_str() {
            if !["pub", "pub_crate", "private"].contains(&visibility_str) {
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
                            name: "visibility".to_string(),
                        },
                    ],
                    message: "visibility must be 'pub', 'pub_crate', or 'private'".to_string(),
                    severity: Severity::Error,
                });
            }
        }
    }
}

pub const RUST_KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
    "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
    "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
    "use", "where", "while", "async", "await", "dyn", "abstract", "become", "box", "do",
    "final", "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "try",
];

pub fn escape_rust_keyword(name: &str) -> String {
    if RUST_KEYWORDS.contains(&name) {
        format!("r#{}", name)
    } else {
        name.to_string()
    }
}

fn is_valid_rust_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    let mut chars = s.chars();
    let first = chars.next().unwrap();

    // First character must be letter or underscore
    if !first.is_alphabetic() && first != '_' {
        return false;
    }

    // Remaining characters can be letters, digits, or underscores
    for ch in chars {
        if !ch.is_alphanumeric() && ch != '_' {
            return false;
        }
    }

    !RUST_KEYWORDS.contains(&s)
}


#[cfg(test)]
#[path = "validate/validate_tests.rs"]
mod validate_tests;
