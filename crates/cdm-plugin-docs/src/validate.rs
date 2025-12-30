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

fn validate_global_config(config: &JSON, errors: &mut Vec<ValidationError>) {
    // Validate format field
    if let Some(format) = config.get("format") {
        if let Some(format_str) = format.as_str() {
            if !["markdown", "html", "json"].contains(&format_str) {
                errors.push(ValidationError {
                    path: vec![PathSegment {
                        kind: "global".to_string(),
                        name: "format".to_string(),
                    }],
                    message: format!(
                        "Invalid format '{}'. Must be one of: markdown, html, json",
                        format_str
                    ),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Validate boolean fields
    for field_name in &["include_examples", "include_inheritance"] {
        if let Some(value) = config.get(field_name) {
            if !value.is_boolean() {
                errors.push(ValidationError {
                    path: vec![PathSegment {
                        kind: "global".to_string(),
                        name: field_name.to_string(),
                    }],
                    message: format!("Field '{}' must be a boolean", field_name),
                    severity: Severity::Error,
                });
            }
        }
    }

    // Validate title field
    if let Some(title) = config.get("title") {
        if !title.is_string() {
            errors.push(ValidationError {
                path: vec![PathSegment {
                    kind: "global".to_string(),
                    name: "title".to_string(),
                }],
                message: "Field 'title' must be a string".to_string(),
                severity: Severity::Error,
            });
        }
    }
}

fn validate_model_config(config: &JSON, model_name: &str, errors: &mut Vec<ValidationError>) {
    // Validate description field
    if let Some(description) = config.get("description") {
        if !description.is_string() {
            errors.push(ValidationError {
                path: vec![
                    PathSegment {
                        kind: "model".to_string(),
                        name: model_name.to_string(),
                    },
                    PathSegment {
                        kind: "config".to_string(),
                        name: "description".to_string(),
                    },
                ],
                message: "Field 'description' must be a string".to_string(),
                severity: Severity::Error,
            });
        }
    }

    // Validate example field
    if let Some(example) = config.get("example") {
        if !example.is_string() {
            errors.push(ValidationError {
                path: vec![
                    PathSegment {
                        kind: "model".to_string(),
                        name: model_name.to_string(),
                    },
                    PathSegment {
                        kind: "config".to_string(),
                        name: "example".to_string(),
                    },
                ],
                message: "Field 'example' must be a string".to_string(),
                severity: Severity::Error,
            });
        }
    }

    // Validate hidden field
    if let Some(hidden) = config.get("hidden") {
        if !hidden.is_boolean() {
            errors.push(ValidationError {
                path: vec![
                    PathSegment {
                        kind: "model".to_string(),
                        name: model_name.to_string(),
                    },
                    PathSegment {
                        kind: "config".to_string(),
                        name: "hidden".to_string(),
                    },
                ],
                message: "Field 'hidden' must be a boolean".to_string(),
                severity: Severity::Error,
            });
        }
    }
}

fn validate_field_config(
    config: &JSON,
    model_name: &str,
    field_name: &str,
    errors: &mut Vec<ValidationError>,
) {
    // Validate description field
    if let Some(description) = config.get("description") {
        if !description.is_string() {
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
                        name: "description".to_string(),
                    },
                ],
                message: "Field 'description' must be a string".to_string(),
                severity: Severity::Error,
            });
        }
    }

    // Validate example field
    if let Some(example) = config.get("example") {
        if !example.is_string() {
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
                        name: "example".to_string(),
                    },
                ],
                message: "Field 'example' must be a string".to_string(),
                severity: Severity::Error,
            });
        }
    }

    // Validate deprecated field
    if let Some(deprecated) = config.get("deprecated") {
        if !deprecated.is_boolean() {
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
                        name: "deprecated".to_string(),
                    },
                ],
                message: "Field 'deprecated' must be a boolean".to_string(),
                severity: Severity::Error,
            });
        }
    }
}

#[cfg(test)]
#[path = "validate/validate_tests.rs"]
mod validate_tests;
