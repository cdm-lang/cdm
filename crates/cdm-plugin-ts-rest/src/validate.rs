use cdm_plugin_interface::{ConfigLevel, PathSegment, Severity, Utils, ValidationError, JSON};
use std::collections::HashSet;

/// Valid HTTP methods for ts-rest routes
const VALID_METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE"];

/// Common status codes that don't trigger warnings
const COMMON_STATUS_CODES: &[u16] = &[200, 201, 204, 400, 401, 403, 404, 409, 422, 500, 502, 503];

/// Validates plugin configuration at different levels
pub fn validate_config(level: ConfigLevel, config: JSON, _utils: &Utils) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    match level {
        ConfigLevel::Global => {
            validate_global_config(&config, &mut errors);
        }
        ConfigLevel::TypeAlias { .. } => {
            // No type alias settings for ts-rest plugin
        }
        ConfigLevel::Model { .. } => {
            // No model settings for ts-rest plugin
        }
        ConfigLevel::Field { .. } => {
            // No field settings for ts-rest plugin
        }
    }

    errors
}

/// Valid strategies for import configuration
const VALID_IMPORT_STRATEGIES: &[&str] = &["single", "per_model"];

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

fn validate_global_config(config: &JSON, errors: &mut Vec<ValidationError>) {
    // Note: build_output is handled by CDM, not by plugins.
    // CDM filters it out before passing config to plugins.

    // Validate schema_import if provided
    if let Some(schema_import) = config.get("schema_import") {
        validate_import_config(schema_import, "schema_import", errors);
    }

    // Validate routes
    match config.get("routes") {
        Some(routes) => {
            if let Some(routes_obj) = routes.as_object() {
                // V002: routes must contain at least one route
                if routes_obj.is_empty() {
                    errors.push(ValidationError {
                        path: vec![PathSegment {
                            kind: "global".to_string(),
                            name: "routes".to_string(),
                        }],
                        message: "routes must contain at least one route".to_string(),
                        severity: Severity::Error,
                    });
                } else {
                    // Validate each route
                    for (route_name, route_config) in routes_obj {
                        validate_route(route_name, route_config, errors);
                    }

                    // V501/V502: Check for route conflicts
                    validate_route_conflicts(routes_obj, errors);
                }
            } else {
                errors.push(ValidationError {
                    path: vec![PathSegment {
                        kind: "global".to_string(),
                        name: "routes".to_string(),
                    }],
                    message: "routes must be an object".to_string(),
                    severity: Severity::Error,
                });
            }
        }
        None => {
            // V002: routes is required
            errors.push(ValidationError {
                path: vec![PathSegment {
                    kind: "global".to_string(),
                    name: "routes".to_string(),
                }],
                message: "routes is required".to_string(),
                severity: Severity::Error,
            });
        }
    }

    // Validate base_path if provided
    if let Some(base_path) = config.get("base_path") {
        if let Some(base_path_str) = base_path.as_str() {
            if !base_path_str.is_empty() && !base_path_str.starts_with('/') {
                errors.push(ValidationError {
                    path: vec![PathSegment {
                        kind: "global".to_string(),
                        name: "base_path".to_string(),
                    }],
                    message: "base_path must start with '/'".to_string(),
                    severity: Severity::Error,
                });
            }
        }
    }
}

fn validate_route(route_name: &str, route_config: &JSON, errors: &mut Vec<ValidationError>) {
    let route_path = vec![
        PathSegment {
            kind: "global".to_string(),
            name: "routes".to_string(),
        },
        PathSegment {
            kind: "route".to_string(),
            name: route_name.to_string(),
        },
    ];

    // V003: method is required
    let method = match route_config.get("method") {
        Some(m) => {
            if let Some(method_str) = m.as_str() {
                let method_upper = method_str.to_uppercase();
                // V301: Validate method is valid
                if !VALID_METHODS.contains(&method_upper.as_str()) {
                    let mut path = route_path.clone();
                    path.push(PathSegment {
                        kind: "field".to_string(),
                        name: "method".to_string(),
                    });
                    errors.push(ValidationError {
                        path,
                        message: format!(
                            "invalid method '{}'. Must be GET, POST, PUT, PATCH, or DELETE",
                            method_str
                        ),
                        severity: Severity::Error,
                    });
                }
                Some(method_upper)
            } else {
                let mut path = route_path.clone();
                path.push(PathSegment {
                    kind: "field".to_string(),
                    name: "method".to_string(),
                });
                errors.push(ValidationError {
                    path,
                    message: "method must be a string".to_string(),
                    severity: Severity::Error,
                });
                None
            }
        }
        None => {
            errors.push(ValidationError {
                path: route_path.clone(),
                message: format!("Route '{}' is missing required field 'method'", route_name),
                severity: Severity::Error,
            });
            None
        }
    };

    // V004: path is required
    let path_value = match route_config.get("path") {
        Some(p) => {
            if let Some(path_str) = p.as_str() {
                if !path_str.starts_with('/') {
                    let mut path = route_path.clone();
                    path.push(PathSegment {
                        kind: "field".to_string(),
                        name: "path".to_string(),
                    });
                    errors.push(ValidationError {
                        path,
                        message: "path must start with '/'".to_string(),
                        severity: Severity::Error,
                    });
                }
                Some(path_str.to_string())
            } else {
                let mut path = route_path.clone();
                path.push(PathSegment {
                    kind: "field".to_string(),
                    name: "path".to_string(),
                });
                errors.push(ValidationError {
                    path,
                    message: "path must be a string".to_string(),
                    severity: Severity::Error,
                });
                None
            }
        }
        None => {
            errors.push(ValidationError {
                path: route_path.clone(),
                message: format!("Route '{}' is missing required field 'path'", route_name),
                severity: Severity::Error,
            });
            None
        }
    };

    // V005: responses is required
    match route_config.get("responses") {
        Some(responses) => {
            if let Some(responses_obj) = responses.as_object() {
                if responses_obj.is_empty() {
                    let mut path = route_path.clone();
                    path.push(PathSegment {
                        kind: "field".to_string(),
                        name: "responses".to_string(),
                    });
                    errors.push(ValidationError {
                        path,
                        message: "responses must contain at least one response".to_string(),
                        severity: Severity::Error,
                    });
                } else {
                    validate_responses(route_name, responses_obj, &route_path, errors);
                }
            } else {
                let mut path = route_path.clone();
                path.push(PathSegment {
                    kind: "field".to_string(),
                    name: "responses".to_string(),
                });
                errors.push(ValidationError {
                    path,
                    message: "responses must be an object".to_string(),
                    severity: Severity::Error,
                });
            }
        }
        None => {
            errors.push(ValidationError {
                path: route_path.clone(),
                message: format!(
                    "Route '{}' is missing required field 'responses'",
                    route_name
                ),
                severity: Severity::Error,
            });
        }
    }

    // Validate optional fields
    validate_optional_string_field(route_config, "summary", &route_path, errors);
    validate_optional_string_field(route_config, "description", &route_path, errors);
    validate_optional_string_field(route_config, "pathParams", &route_path, errors);
    validate_optional_string_field(route_config, "query", &route_path, errors);
    // body can be string (model name) or null (explicitly no body)
    validate_optional_string_or_null_field(route_config, "body", &route_path, errors);

    // Validate tags is array of strings if present
    if let Some(tags) = route_config.get("tags") {
        if let Some(tags_array) = tags.as_array() {
            for (i, tag) in tags_array.iter().enumerate() {
                if !tag.is_string() {
                    let mut path = route_path.clone();
                    path.push(PathSegment {
                        kind: "field".to_string(),
                        name: format!("tags[{}]", i),
                    });
                    errors.push(ValidationError {
                        path,
                        message: "tag must be a string".to_string(),
                        severity: Severity::Error,
                    });
                }
            }
        } else {
            let mut path = route_path.clone();
            path.push(PathSegment {
                kind: "field".to_string(),
                name: "tags".to_string(),
            });
            errors.push(ValidationError {
                path,
                message: "tags must be an array of strings".to_string(),
                severity: Severity::Error,
            });
        }
    }

    // V302/V303: Warn about unusual body usage
    // body: null means explicitly no body (suppress warning)
    // body: "Model" means has body
    // no body field means no body (triggers warning for POST/PUT/PATCH)
    let body_value = route_config.get("body");
    let has_body = body_value.map(|v| !v.is_null()).unwrap_or(false);
    let body_explicitly_null = body_value.map(|v| v.is_null()).unwrap_or(false);
    if let Some(ref method) = method {
        match method.as_str() {
            "GET" if has_body => {
                errors.push(ValidationError {
                    path: route_path.clone(),
                    message: format!("Route '{}': GET request with body is unusual", route_name),
                    severity: Severity::Warning,
                });
            }
            "DELETE" if has_body => {
                errors.push(ValidationError {
                    path: route_path.clone(),
                    message: format!(
                        "Route '{}': DELETE request with body is unusual",
                        route_name
                    ),
                    severity: Severity::Warning,
                });
            }
            // V304/V305/V306: Warn about missing body (unless explicitly set to null)
            "POST" if !has_body && !body_explicitly_null => {
                errors.push(ValidationError {
                    path: route_path.clone(),
                    message: format!("Route '{}': POST request without body", route_name),
                    severity: Severity::Warning,
                });
            }
            "PUT" if !has_body && !body_explicitly_null => {
                errors.push(ValidationError {
                    path: route_path.clone(),
                    message: format!("Route '{}': PUT request without body", route_name),
                    severity: Severity::Warning,
                });
            }
            "PATCH" if !has_body && !body_explicitly_null => {
                errors.push(ValidationError {
                    path: route_path.clone(),
                    message: format!("Route '{}': PATCH request without body", route_name),
                    severity: Severity::Warning,
                });
            }
            _ => {}
        }
    }

    // V202: Check path params without pathParams model
    if let Some(ref path_str) = path_value {
        let path_params = extract_path_params(path_str);
        if !path_params.is_empty() && route_config.get("pathParams").is_none() {
            let mut path = route_path.clone();
            path.push(PathSegment {
                kind: "field".to_string(),
                name: "path".to_string(),
            });
            errors.push(ValidationError {
                path,
                message: format!(
                    "path '{}' contains parameter(s) {:?} but no pathParams model is specified",
                    path_str, path_params
                ),
                severity: Severity::Error,
            });
        }
    }
}

fn validate_responses(
    route_name: &str,
    responses: &serde_json::Map<String, JSON>,
    route_path: &[PathSegment],
    errors: &mut Vec<ValidationError>,
) {
    let mut has_success_response = false;

    for (status_code_str, response) in responses {
        // V401: Validate status code is valid
        match status_code_str.parse::<u16>() {
            Ok(code) => {
                if code < 100 || code > 599 {
                    let mut path = route_path.to_vec();
                    path.push(PathSegment {
                        kind: "field".to_string(),
                        name: format!("responses.{}", status_code_str),
                    });
                    errors.push(ValidationError {
                        path,
                        message: format!(
                            "invalid status code '{}'. Must be between 100 and 599",
                            status_code_str
                        ),
                        severity: Severity::Error,
                    });
                } else {
                    // Check for success response (2xx)
                    if code >= 200 && code < 300 {
                        has_success_response = true;
                    }

                    // V402: Warn about unusual status codes
                    if !COMMON_STATUS_CODES.contains(&code) {
                        let mut path = route_path.to_vec();
                        path.push(PathSegment {
                            kind: "field".to_string(),
                            name: format!("responses.{}", status_code_str),
                        });
                        errors.push(ValidationError {
                            path,
                            message: format!("unusual status code '{}'", status_code_str),
                            severity: Severity::Warning,
                        });
                    }
                }
            }
            Err(_) => {
                let mut path = route_path.to_vec();
                path.push(PathSegment {
                    kind: "field".to_string(),
                    name: format!("responses.{}", status_code_str),
                });
                errors.push(ValidationError {
                    path,
                    message: format!(
                        "invalid status code '{}'. Must be a number between 100 and 599",
                        status_code_str
                    ),
                    severity: Severity::Error,
                });
            }
        }

        // Validate response value is valid type
        validate_response_value(route_name, status_code_str, response, route_path, errors);
    }

    // V403: Warn if no success response
    if !has_success_response {
        errors.push(ValidationError {
            path: route_path.to_vec(),
            message: format!(
                "Route '{}': no success response (2xx) defined",
                route_name
            ),
            severity: Severity::Warning,
        });
    }
}

fn validate_response_value(
    _route_name: &str,
    status_code: &str,
    response: &JSON,
    route_path: &[PathSegment],
    errors: &mut Vec<ValidationError>,
) {
    // Valid response types: string, null, or array of strings
    match response {
        JSON::String(_) => {
            // Valid: single model reference like "User" or "User[]"
        }
        JSON::Null => {
            // Valid: no response body (e.g., 204 No Content)
        }
        JSON::Array(arr) => {
            // Valid: union of models like ["ValidationError", "NotFoundError"]
            for (i, item) in arr.iter().enumerate() {
                if !item.is_string() {
                    let mut path = route_path.to_vec();
                    path.push(PathSegment {
                        kind: "field".to_string(),
                        name: format!("responses.{}[{}]", status_code, i),
                    });
                    errors.push(ValidationError {
                        path,
                        message: "response union items must be model name strings".to_string(),
                        severity: Severity::Error,
                    });
                }
            }
        }
        _ => {
            let mut path = route_path.to_vec();
            path.push(PathSegment {
                kind: "field".to_string(),
                name: format!("responses.{}", status_code),
            });
            errors.push(ValidationError {
                path,
                message:
                    "response must be a model name (string), null, or array of model names"
                        .to_string(),
                severity: Severity::Error,
            });
        }
    }
}

fn validate_optional_string_field(
    config: &JSON,
    field_name: &str,
    route_path: &[PathSegment],
    errors: &mut Vec<ValidationError>,
) {
    if let Some(value) = config.get(field_name) {
        if !value.is_string() {
            let mut path = route_path.to_vec();
            path.push(PathSegment {
                kind: "field".to_string(),
                name: field_name.to_string(),
            });
            errors.push(ValidationError {
                path,
                message: format!("{} must be a string", field_name),
                severity: Severity::Error,
            });
        }
    }
}

fn validate_optional_string_or_null_field(
    config: &JSON,
    field_name: &str,
    route_path: &[PathSegment],
    errors: &mut Vec<ValidationError>,
) {
    if let Some(value) = config.get(field_name) {
        if !value.is_string() && !value.is_null() {
            let mut path = route_path.to_vec();
            path.push(PathSegment {
                kind: "field".to_string(),
                name: field_name.to_string(),
            });
            errors.push(ValidationError {
                path,
                message: format!("{} must be a string or null", field_name),
                severity: Severity::Error,
            });
        }
    }
}

fn validate_route_conflicts(
    routes: &serde_json::Map<String, JSON>,
    errors: &mut Vec<ValidationError>,
) {
    let route_entries: Vec<(&String, &JSON)> = routes.iter().collect();

    for i in 0..route_entries.len() {
        for j in (i + 1)..route_entries.len() {
            let (name_a, config_a) = route_entries[i];
            let (name_b, config_b) = route_entries[j];

            let method_a = config_a
                .get("method")
                .and_then(|m| m.as_str())
                .map(|s| s.to_uppercase());
            let method_b = config_b
                .get("method")
                .and_then(|m| m.as_str())
                .map(|s| s.to_uppercase());

            let path_a = config_a.get("path").and_then(|p| p.as_str());
            let path_b = config_b.get("path").and_then(|p| p.as_str());

            if let (Some(m_a), Some(m_b), Some(p_a), Some(p_b)) =
                (&method_a, &method_b, path_a, path_b)
            {
                if m_a == m_b {
                    // V501: Identical method and path
                    if p_a == p_b {
                        errors.push(ValidationError {
                            path: vec![PathSegment {
                                kind: "global".to_string(),
                                name: "routes".to_string(),
                            }],
                            message: format!(
                                "Routes '{}' and '{}' have identical method and path: {} {}",
                                name_a, name_b, m_a, p_a
                            ),
                            severity: Severity::Error,
                        });
                    }
                    // V502: Potentially ambiguous paths
                    else if paths_are_ambiguous(p_a, p_b) {
                        errors.push(ValidationError {
                            path: vec![PathSegment {
                                kind: "global".to_string(),
                                name: "routes".to_string(),
                            }],
                            message: format!(
                                "Routes '{}' and '{}' have potentially ambiguous paths: {} vs {}",
                                name_a, name_b, p_a, p_b
                            ),
                            severity: Severity::Warning,
                        });
                    }
                }
            }
        }
    }
}

/// Extract path parameters from a path string
/// e.g., "/users/:id/posts/:postId" -> ["id", "postId"]
pub fn extract_path_params(path: &str) -> Vec<String> {
    let mut params = Vec::new();
    for segment in path.split('/') {
        if let Some(param) = segment.strip_prefix(':') {
            // Extract just the parameter name (alphanumeric and underscore)
            let param_name: String = param
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !param_name.is_empty() {
                params.push(param_name);
            }
        }
    }
    params
}

/// Check if two paths are potentially ambiguous
/// e.g., "/users/:id" and "/users/:userId" are ambiguous (same pattern)
/// e.g., "/users/:id" and "/users/me" may conflict
fn paths_are_ambiguous(path_a: &str, path_b: &str) -> bool {
    let segments_a: Vec<&str> = path_a.split('/').collect();
    let segments_b: Vec<&str> = path_b.split('/').collect();

    if segments_a.len() != segments_b.len() {
        return false;
    }

    let mut has_param_difference = false;

    for (seg_a, seg_b) in segments_a.iter().zip(segments_b.iter()) {
        let is_param_a = seg_a.starts_with(':');
        let is_param_b = seg_b.starts_with(':');

        if is_param_a && is_param_b {
            // Both are params - potentially ambiguous if they differ
            if seg_a != seg_b {
                has_param_difference = true;
            }
        } else if is_param_a != is_param_b {
            // One is param, one is literal - could be ambiguous
            // e.g., /users/:id vs /users/me
            has_param_difference = true;
        } else if seg_a != seg_b {
            // Both are literals but different - not ambiguous
            return false;
        }
    }

    has_param_difference
}

/// Strip array suffix from model name
/// e.g., "User[]" -> "User"
pub fn strip_array_suffix(model_name: &str) -> &str {
    model_name.strip_suffix("[]").unwrap_or(model_name)
}

/// Check if model name indicates an array response
pub fn is_array_response(model_name: &str) -> bool {
    model_name.ends_with("[]")
}

/// Collect all unique model references from the routes configuration
pub fn collect_model_references(routes: &JSON) -> HashSet<String> {
    let mut models = HashSet::new();

    if let Some(routes_obj) = routes.as_object() {
        for (_route_name, route_config) in routes_obj {
            // Collect pathParams
            if let Some(path_params) = route_config.get("pathParams").and_then(|v| v.as_str()) {
                models.insert(path_params.to_string());
            }

            // Collect query
            if let Some(query) = route_config.get("query").and_then(|v| v.as_str()) {
                models.insert(query.to_string());
            }

            // Collect body
            if let Some(body) = route_config.get("body").and_then(|v| v.as_str()) {
                models.insert(body.to_string());
            }

            // Collect responses
            if let Some(responses) = route_config.get("responses").and_then(|v| v.as_object()) {
                for (_status_code, response) in responses {
                    match response {
                        JSON::String(model_name) => {
                            models.insert(strip_array_suffix(model_name).to_string());
                        }
                        JSON::Array(arr) => {
                            for item in arr {
                                if let Some(model_name) = item.as_str() {
                                    models.insert(strip_array_suffix(model_name).to_string());
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    models
}

#[cfg(test)]
#[path = "validate/validate_tests.rs"]
mod validate_tests;
