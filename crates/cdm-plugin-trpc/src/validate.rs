use cdm_plugin_interface::{ConfigLevel, PathSegment, Severity, Utils, ValidationError, JSON};
use std::collections::HashSet;

/// Valid procedure types for tRPC
const VALID_PROCEDURE_TYPES: &[&str] = &["query", "mutation", "subscription"];

/// Valid import strategies
const VALID_IMPORT_STRATEGIES: &[&str] = &["single", "per_model"];

/// Validates plugin configuration at different levels
pub fn validate_config(level: ConfigLevel, config: JSON, _utils: &Utils) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    match level {
        ConfigLevel::Global => {
            validate_global_config(&config, &mut errors);
        }
        ConfigLevel::TypeAlias { .. } => {
            // No type alias settings for tRPC plugin
        }
        ConfigLevel::Model { .. } => {
            // No model settings for tRPC plugin
        }
        ConfigLevel::Field { .. } => {
            // No field settings for tRPC plugin
        }
    }

    errors
}

fn validate_global_config(config: &JSON, errors: &mut Vec<ValidationError>) {
    // Note: build_output is handled by CDM, not by plugins.
    // CDM filters it out before passing config to plugins.

    // Validate schema_import if provided
    if let Some(schema_import) = config.get("schema_import") {
        validate_import_config(schema_import, "schema_import", errors);
    }

    // Validate procedures
    match config.get("procedures") {
        Some(procedures) => {
            if let Some(procedures_obj) = procedures.as_object() {
                if procedures_obj.is_empty() {
                    errors.push(ValidationError {
                        path: vec![PathSegment {
                            kind: "global".to_string(),
                            name: "procedures".to_string(),
                        }],
                        message: "procedures must contain at least one procedure".to_string(),
                        severity: Severity::Error,
                    });
                } else {
                    // Validate each procedure
                    for (procedure_name, procedure_config) in procedures_obj {
                        validate_procedure(procedure_name, procedure_config, errors);
                    }
                }
            } else {
                errors.push(ValidationError {
                    path: vec![PathSegment {
                        kind: "global".to_string(),
                        name: "procedures".to_string(),
                    }],
                    message: "procedures must be an object".to_string(),
                    severity: Severity::Error,
                });
            }
        }
        None => {
            errors.push(ValidationError {
                path: vec![PathSegment {
                    kind: "global".to_string(),
                    name: "procedures".to_string(),
                }],
                message: "procedures is required".to_string(),
                severity: Severity::Error,
            });
        }
    }
}

fn validate_import_config(config: &JSON, field_name: &str, errors: &mut Vec<ValidationError>) {
    if !config.is_object() {
        errors.push(ValidationError {
            path: vec![PathSegment {
                kind: "global".to_string(),
                name: field_name.to_string(),
            }],
            message: format!("{} must be an object", field_name),
            severity: Severity::Error,
        });
        return;
    }

    // Validate strategy
    match config.get("strategy") {
        Some(strategy) => {
            if let Some(strategy_str) = strategy.as_str() {
                if !VALID_IMPORT_STRATEGIES.contains(&strategy_str) {
                    errors.push(ValidationError {
                        path: vec![PathSegment {
                            kind: "global".to_string(),
                            name: format!("{}.strategy", field_name),
                        }],
                        message: format!(
                            "invalid strategy '{}'. Must be 'single' or 'per_model'",
                            strategy_str
                        ),
                        severity: Severity::Error,
                    });
                }
            } else {
                errors.push(ValidationError {
                    path: vec![PathSegment {
                        kind: "global".to_string(),
                        name: format!("{}.strategy", field_name),
                    }],
                    message: format!("{}.strategy must be a string", field_name),
                    severity: Severity::Error,
                });
            }
        }
        None => {
            errors.push(ValidationError {
                path: vec![PathSegment {
                    kind: "global".to_string(),
                    name: format!("{}.strategy", field_name),
                }],
                message: format!("{}.strategy is required", field_name),
                severity: Severity::Error,
            });
        }
    }

    // Validate path
    match config.get("path") {
        Some(path) => {
            if !path.is_string() {
                errors.push(ValidationError {
                    path: vec![PathSegment {
                        kind: "global".to_string(),
                        name: format!("{}.path", field_name),
                    }],
                    message: format!("{}.path must be a string", field_name),
                    severity: Severity::Error,
                });
            }
        }
        None => {
            errors.push(ValidationError {
                path: vec![PathSegment {
                    kind: "global".to_string(),
                    name: format!("{}.path", field_name),
                }],
                message: format!("{}.path is required", field_name),
                severity: Severity::Error,
            });
        }
    }
}

fn validate_procedure(
    procedure_name: &str,
    procedure_config: &JSON,
    errors: &mut Vec<ValidationError>,
) {
    let procedure_path = vec![
        PathSegment {
            kind: "global".to_string(),
            name: "procedures".to_string(),
        },
        PathSegment {
            kind: "procedure".to_string(),
            name: procedure_name.to_string(),
        },
    ];

    // Validate type is required
    match procedure_config.get("type") {
        Some(procedure_type) => {
            if let Some(type_str) = procedure_type.as_str() {
                if !VALID_PROCEDURE_TYPES.contains(&type_str) {
                    let mut path = procedure_path.clone();
                    path.push(PathSegment {
                        kind: "field".to_string(),
                        name: "type".to_string(),
                    });
                    errors.push(ValidationError {
                        path,
                        message: format!(
                            "invalid procedure type '{}'. Must be 'query', 'mutation', or 'subscription'",
                            type_str
                        ),
                        severity: Severity::Error,
                    });
                }
            } else {
                let mut path = procedure_path.clone();
                path.push(PathSegment {
                    kind: "field".to_string(),
                    name: "type".to_string(),
                });
                errors.push(ValidationError {
                    path,
                    message: "type must be a string".to_string(),
                    severity: Severity::Error,
                });
            }
        }
        None => {
            errors.push(ValidationError {
                path: procedure_path.clone(),
                message: format!(
                    "Procedure '{}' is missing required field 'type'",
                    procedure_name
                ),
                severity: Severity::Error,
            });
        }
    }

    // Validate output is required
    match procedure_config.get("output") {
        Some(output) => {
            if !output.is_string() {
                let mut path = procedure_path.clone();
                path.push(PathSegment {
                    kind: "field".to_string(),
                    name: "output".to_string(),
                });
                errors.push(ValidationError {
                    path,
                    message: "output must be a string".to_string(),
                    severity: Severity::Error,
                });
            }
        }
        None => {
            errors.push(ValidationError {
                path: procedure_path.clone(),
                message: format!(
                    "Procedure '{}' is missing required field 'output'",
                    procedure_name
                ),
                severity: Severity::Error,
            });
        }
    }

    // Validate optional input field
    if let Some(input) = procedure_config.get("input") {
        if !input.is_string() {
            let mut path = procedure_path.clone();
            path.push(PathSegment {
                kind: "field".to_string(),
                name: "input".to_string(),
            });
            errors.push(ValidationError {
                path,
                message: "input must be a string".to_string(),
                severity: Severity::Error,
            });
        }
    }

    // Validate optional error field
    if let Some(error) = procedure_config.get("error") {
        if !error.is_string() {
            let mut path = procedure_path;
            path.push(PathSegment {
                kind: "field".to_string(),
                name: "error".to_string(),
            });
            errors.push(ValidationError {
                path,
                message: "error must be a string".to_string(),
                severity: Severity::Error,
            });
        }
    }
}

/// Strip array suffix from model name
/// e.g., "User[]" -> "User"
pub fn strip_array_suffix(model_name: &str) -> &str {
    model_name.strip_suffix("[]").unwrap_or(model_name)
}

/// Check if model name indicates an array output
pub fn is_array_output(model_name: &str) -> bool {
    model_name.ends_with("[]")
}

/// Check if output is void
pub fn is_void_output(model_name: &str) -> bool {
    model_name == "void"
}

/// Collect all unique model references from the procedures configuration
pub fn collect_model_references(procedures: &JSON) -> HashSet<String> {
    let mut models = HashSet::new();

    if let Some(procedures_obj) = procedures.as_object() {
        for (_procedure_name, procedure_config) in procedures_obj {
            // Collect input
            if let Some(input) = procedure_config.get("input").and_then(|v| v.as_str()) {
                models.insert(strip_array_suffix(input).to_string());
            }

            // Collect output (unless void)
            if let Some(output) = procedure_config.get("output").and_then(|v| v.as_str()) {
                if !is_void_output(output) {
                    models.insert(strip_array_suffix(output).to_string());
                }
            }

            // Collect error
            if let Some(error) = procedure_config.get("error").and_then(|v| v.as_str()) {
                models.insert(strip_array_suffix(error).to_string());
            }
        }
    }

    models
}

#[cfg(test)]
#[path = "validate/validate_tests.rs"]
mod validate_tests;
