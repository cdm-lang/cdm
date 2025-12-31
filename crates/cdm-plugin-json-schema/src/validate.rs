use cdm_plugin_interface::{ConfigLevel, PathSegment, Severity, Utils, ValidationError, JSON};

/// Validates plugin configuration at different levels
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
        ConfigLevel::TypeAlias { ref name } => {
            validate_type_alias_config(&config, name, &mut errors);
        }
        ConfigLevel::Model { ref name } => {
            validate_model_config(&config, name, &mut errors);
        }
        ConfigLevel::Field {
            ref model,
            ref field,
        } => {
            validate_field_config(&config, model, field, &mut errors);
        }
    }

    errors
}

fn validate_global_config(_config: &JSON, _errors: &mut Vec<ValidationError>) {
    // Validate root_model exists if specified (we can't check this without the schema, so skip)

    // All other fields are validated by the schema itself, so no additional validation needed
    // The schema validation happens before this function is called
}

fn validate_type_alias_config(_config: &JSON, _type_name: &str, _errors: &mut Vec<ValidationError>) {
    // Schema validation handles:
    // - description is string
    // - union_mode is "enum" | "oneOf"
    // - skip is boolean
    // No additional semantic validation needed
}

fn validate_model_config(_config: &JSON, _model_name: &str, _errors: &mut Vec<ValidationError>) {
    // Schema validation handles:
    // - title, description are strings
    // - additional_properties is boolean
    // - skip is boolean
    // - relationship_mode is "reference" | "inline"
    // - is_root is boolean
    // No additional semantic validation needed
}

fn validate_field_config(
    config: &JSON,
    model_name: &str,
    field_name: &str,
    errors: &mut Vec<ValidationError>,
) {
    // Validate pattern is a valid regex if provided
    if let Some(pattern) = config.get("pattern") {
        if let Some(pattern_str) = pattern.as_str() {
            if let Err(e) = regex::Regex::new(pattern_str) {
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
                            name: "pattern".to_string(),
                        },
                    ],
                    message: format!("Invalid regex pattern: {}", e),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Validate min_length <= max_length if both provided
    if let (Some(min), Some(max)) = (config.get("min_length"), config.get("max_length")) {
        if let (Some(min_val), Some(max_val)) = (min.as_f64(), max.as_f64()) {
            if min_val > max_val {
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
                            name: "min_length".to_string(),
                        },
                    ],
                    message: format!(
                        "min_length ({}) cannot be greater than max_length ({})",
                        min_val, max_val
                    ),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Validate minimum <= maximum if both provided
    if let (Some(min), Some(max)) = (config.get("minimum"), config.get("maximum")) {
        if let (Some(min_val), Some(max_val)) = (min.as_f64(), max.as_f64()) {
            if min_val > max_val {
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
                            name: "minimum".to_string(),
                        },
                    ],
                    message: format!(
                        "minimum ({}) cannot be greater than maximum ({})",
                        min_val, max_val
                    ),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Validate exclusive_minimum <= exclusive_maximum if both provided
    if let (Some(min), Some(max)) = (config.get("exclusive_minimum"), config.get("exclusive_maximum")) {
        if let (Some(min_val), Some(max_val)) = (min.as_f64(), max.as_f64()) {
            if min_val >= max_val {
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
                            name: "exclusive_minimum".to_string(),
                        },
                    ],
                    message: format!(
                        "exclusive_minimum ({}) must be less than exclusive_maximum ({})",
                        min_val, max_val
                    ),
                    severity: Severity::Error,
                });
            }
        }
    }
}

#[cfg(test)]
#[path = "validate/validate_tests.rs"]
mod validate_tests;
