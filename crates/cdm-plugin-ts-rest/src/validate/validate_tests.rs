use crate::validate::{
    collect_model_references, extract_path_params, is_array_response, strip_array_suffix,
    validate_config,
};
use cdm_plugin_interface::{ConfigLevel, Severity, Utils};
use serde_json::json;

fn utils() -> Utils {
    Utils
}

// ============================================================================
// Schema Import Validation Tests
// ============================================================================

#[test]
fn test_schema_import_valid_single_strategy() {
    let config = json!({
        "schema_import": {
            "strategy": "single",
            "path": "./schemas"
        },
        "routes": {
            "getUser": {
                "method": "GET",
                "path": "/users",
                "responses": { "200": "User" }
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors
        .iter()
        .find(|e| e.message.contains("schema_import"));
    assert!(error.is_none(), "Expected no schema_import errors");
}

#[test]
fn test_schema_import_valid_per_model_strategy() {
    let config = json!({
        "schema_import": {
            "strategy": "per_model",
            "path": "./models"
        },
        "routes": {
            "getUser": {
                "method": "GET",
                "path": "/users",
                "responses": { "200": "User" }
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors
        .iter()
        .find(|e| e.message.contains("schema_import"));
    assert!(error.is_none(), "Expected no schema_import errors");
}

#[test]
fn test_schema_import_invalid_strategy() {
    let config = json!({
        "schema_import": {
            "strategy": "invalid",
            "path": "./schemas"
        },
        "routes": {
            "getUser": {
                "method": "GET",
                "path": "/users",
                "responses": { "200": "User" }
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors
        .iter()
        .find(|e| e.message.contains("invalid strategy") && e.message.contains("invalid"));
    assert!(error.is_some(), "Expected error for invalid strategy");
}

#[test]
fn test_schema_import_missing_path() {
    let config = json!({
        "schema_import": {
            "strategy": "single"
        },
        "routes": {
            "getUser": {
                "method": "GET",
                "path": "/users",
                "responses": { "200": "User" }
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors
        .iter()
        .find(|e| e.message.contains("schema_import.path"));
    assert!(error.is_some(), "Expected error for missing path");
}

// ============================================================================
// Path Parameter Extraction Tests
// ============================================================================

#[test]
fn test_extract_path_params_single() {
    let params = extract_path_params("/users/:id");
    assert_eq!(params, vec!["id"]);
}

#[test]
fn test_extract_path_params_multiple() {
    let params = extract_path_params("/users/:userId/posts/:postId");
    assert_eq!(params, vec!["userId", "postId"]);
}

#[test]
fn test_extract_path_params_none() {
    let params = extract_path_params("/users");
    assert!(params.is_empty());
}

#[test]
fn test_extract_path_params_underscore() {
    let params = extract_path_params("/users/:user_id");
    assert_eq!(params, vec!["user_id"]);
}

// ============================================================================
// Array Response Helper Tests
// ============================================================================

#[test]
fn test_strip_array_suffix() {
    assert_eq!(strip_array_suffix("User[]"), "User");
    assert_eq!(strip_array_suffix("User"), "User");
    assert_eq!(strip_array_suffix("ValidationError[]"), "ValidationError");
}

#[test]
fn test_is_array_response() {
    assert!(is_array_response("User[]"));
    assert!(!is_array_response("User"));
}

// ============================================================================
// Model Reference Collection Tests
// ============================================================================

#[test]
fn test_collect_model_references() {
    let routes = json!({
        "getUser": {
            "method": "GET",
            "path": "/users/:id",
            "pathParams": "GetUserPathParams",
            "responses": {
                "200": "User",
                "404": "NotFoundError"
            }
        },
        "listUsers": {
            "method": "GET",
            "path": "/users",
            "query": "ListUsersQuery",
            "responses": {
                "200": "User[]"
            }
        },
        "createUser": {
            "method": "POST",
            "path": "/users",
            "body": "CreateUserBody",
            "responses": {
                "201": "User",
                "400": ["ValidationError", "InvalidInputError"]
            }
        }
    });

    let refs = collect_model_references(&routes);

    assert!(refs.contains("GetUserPathParams"));
    assert!(refs.contains("User"));
    assert!(refs.contains("NotFoundError"));
    assert!(refs.contains("ListUsersQuery"));
    assert!(refs.contains("CreateUserBody"));
    assert!(refs.contains("ValidationError"));
    assert!(refs.contains("InvalidInputError"));
}

// Note: build_output validation was removed - CDM handles this, not plugins.
// See docs/7-plugin-development.md for details on reserved config keys.

// ============================================================================
// V002: routes must contain at least one route
// ============================================================================

#[test]
fn test_v002_routes_required() {
    let config = json!({});

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors.iter().find(|e| e.message.contains("routes is required"));
    assert!(error.is_some(), "Expected V002 error");
}

#[test]
fn test_v002_routes_empty() {
    let config = json!({
        "routes": {}
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors
        .iter()
        .find(|e| e.message.contains("routes must contain at least one route"));
    assert!(error.is_some(), "Expected V002 error for empty routes");
}

// ============================================================================
// V003: Route missing method
// ============================================================================

#[test]
fn test_v003_method_required() {
    let config = json!({
        "routes": {
            "getUser": {
                "path": "/users/:id",
                "responses": { "200": "User" }
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors
        .iter()
        .find(|e| e.message.contains("missing required field 'method'"));
    assert!(error.is_some(), "Expected V003 error");
}

// ============================================================================
// V004: Route missing path
// ============================================================================

#[test]
fn test_v004_path_required() {
    let config = json!({
        "routes": {
            "getUser": {
                "method": "GET",
                "responses": { "200": "User" }
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors
        .iter()
        .find(|e| e.message.contains("missing required field 'path'"));
    assert!(error.is_some(), "Expected V004 error");
}

// ============================================================================
// V005: Route missing responses
// ============================================================================

#[test]
fn test_v005_responses_required() {
    let config = json!({
        "routes": {
            "getUser": {
                "method": "GET",
                "path": "/users/:id"
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors
        .iter()
        .find(|e| e.message.contains("missing required field 'responses'"));
    assert!(error.is_some(), "Expected V005 error");
}

// ============================================================================
// V202: Path params without pathParams model
// ============================================================================

#[test]
fn test_v202_path_params_without_model() {
    let config = json!({
        "routes": {
            "getUser": {
                "method": "GET",
                "path": "/users/:id",
                "responses": { "200": "User" }
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors
        .iter()
        .find(|e| e.message.contains("no pathParams model is specified"));
    assert!(error.is_some(), "Expected V202 error");
}

#[test]
fn test_v202_path_params_with_model() {
    let config = json!({
        "routes": {
            "getUser": {
                "method": "GET",
                "path": "/users/:id",
                "pathParams": "GetUserPathParams",
                "responses": { "200": "User" }
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors
        .iter()
        .find(|e| e.message.contains("no pathParams model is specified"));
    assert!(error.is_none());
}

// ============================================================================
// V301: Invalid HTTP method
// ============================================================================

#[test]
fn test_v301_invalid_method() {
    let config = json!({
        "routes": {
            "getUser": {
                "method": "INVALID",
                "path": "/users/:id",
                "pathParams": "GetUserParams",
                "responses": { "200": "User" }
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors.iter().find(|e| e.message.contains("invalid method"));
    assert!(error.is_some(), "Expected V301 error");
}

#[test]
fn test_v301_valid_methods() {
    for method in &["GET", "POST", "PUT", "PATCH", "DELETE", "get", "post"] {
        let config = json!({
                "routes": {
                "testRoute": {
                    "method": method,
                    "path": "/test",
                    "body": "TestBody",
                    "responses": { "200": "TestResponse" }
                }
            }
        });

        let errors = validate_config(ConfigLevel::Global, config, &utils());
        let error = errors.iter().find(|e| e.message.contains("invalid method"));
        assert!(error.is_none(), "Method {} should be valid", method);
    }
}

// ============================================================================
// V302/V303: Unusual body usage warnings
// ============================================================================

#[test]
fn test_v302_get_with_body_warning() {
    let config = json!({
        "routes": {
            "getUser": {
                "method": "GET",
                "path": "/users",
                "body": "GetUsersBody",
                "responses": { "200": "User" }
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let warning = errors
        .iter()
        .find(|e| e.message.contains("GET request with body is unusual"));
    assert!(warning.is_some(), "Expected V302 warning");
    assert_eq!(warning.unwrap().severity, Severity::Warning);
}

#[test]
fn test_v303_delete_with_body_warning() {
    let config = json!({
        "routes": {
            "deleteUser": {
                "method": "DELETE",
                "path": "/users/:id",
                "pathParams": "DeleteUserParams",
                "body": "DeleteUserBody",
                "responses": { "204": null }
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let warning = errors
        .iter()
        .find(|e| e.message.contains("DELETE request with body is unusual"));
    assert!(warning.is_some(), "Expected V303 warning");
}

// ============================================================================
// V304/V305/V306: Missing body warnings
// ============================================================================

#[test]
fn test_v304_post_without_body_warning() {
    let config = json!({
        "routes": {
            "createUser": {
                "method": "POST",
                "path": "/users",
                "responses": { "201": "User" }
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let warning = errors
        .iter()
        .find(|e| e.message.contains("POST request without body"));
    assert!(warning.is_some(), "Expected V304 warning");
}

#[test]
fn test_v304_post_with_body_null_no_warning() {
    // body: null explicitly indicates no body, suppressing the warning
    let config = json!({
        "routes": {
            "logout": {
                "method": "POST",
                "path": "/logout",
                "body": null,
                "responses": { "204": null }
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let warning = errors
        .iter()
        .find(|e| e.message.contains("POST request without body"));
    assert!(warning.is_none(), "body: null should suppress the warning");
}

#[test]
fn test_v305_put_without_body_warning() {
    let config = json!({
        "routes": {
            "updateUser": {
                "method": "PUT",
                "path": "/users/:id",
                "pathParams": "UpdateUserParams",
                "responses": { "200": "User" }
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let warning = errors
        .iter()
        .find(|e| e.message.contains("PUT request without body"));
    assert!(warning.is_some(), "Expected V305 warning");
}

#[test]
fn test_v306_patch_without_body_warning() {
    let config = json!({
        "routes": {
            "patchUser": {
                "method": "PATCH",
                "path": "/users/:id",
                "pathParams": "PatchUserParams",
                "responses": { "200": "User" }
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let warning = errors
        .iter()
        .find(|e| e.message.contains("PATCH request without body"));
    assert!(warning.is_some(), "Expected V306 warning");
}

// ============================================================================
// V401: Invalid status code
// ============================================================================

#[test]
fn test_v401_invalid_status_code_non_numeric() {
    let config = json!({
        "routes": {
            "getUser": {
                "method": "GET",
                "path": "/users",
                "responses": { "abc": "User" }
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors.iter().find(|e| e.message.contains("invalid status code"));
    assert!(error.is_some(), "Expected V401 error for non-numeric status code");
}

#[test]
fn test_v401_invalid_status_code_out_of_range() {
    let config = json!({
        "routes": {
            "getUser": {
                "method": "GET",
                "path": "/users",
                "responses": { "99": "User" }
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors
        .iter()
        .find(|e| e.message.contains("Must be between 100 and 599"));
    assert!(error.is_some(), "Expected V401 error for out of range status code");
}

// ============================================================================
// V402: Unusual status code warning
// ============================================================================

#[test]
fn test_v402_unusual_status_code_warning() {
    let config = json!({
        "routes": {
            "getUser": {
                "method": "GET",
                "path": "/users",
                "responses": {
                    "200": "User",
                    "418": "TeapotError"
                }
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let warning = errors
        .iter()
        .find(|e| e.message.contains("unusual status code '418'"));
    assert!(warning.is_some(), "Expected V402 warning for unusual status code");
    assert_eq!(warning.unwrap().severity, Severity::Warning);
}

// ============================================================================
// V403: No success response warning
// ============================================================================

#[test]
fn test_v403_no_success_response_warning() {
    let config = json!({
        "routes": {
            "getUser": {
                "method": "GET",
                "path": "/users",
                "responses": {
                    "400": "BadRequestError",
                    "500": "InternalError"
                }
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let warning = errors
        .iter()
        .find(|e| e.message.contains("no success response (2xx) defined"));
    assert!(warning.is_some(), "Expected V403 warning");
}

// ============================================================================
// V501: Identical route conflict
// ============================================================================

#[test]
fn test_v501_identical_routes() {
    let config = json!({
        "routes": {
            "getUser1": {
                "method": "GET",
                "path": "/users/:id",
                "pathParams": "GetUserParams",
                "responses": { "200": "User" }
            },
            "getUser2": {
                "method": "GET",
                "path": "/users/:id",
                "pathParams": "GetUserParams",
                "responses": { "200": "User" }
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors
        .iter()
        .find(|e| e.message.contains("have identical method and path"));
    assert!(error.is_some(), "Expected V501 error");
}

// ============================================================================
// V502: Ambiguous routes warning
// ============================================================================

#[test]
fn test_v502_ambiguous_routes_different_param_names() {
    let config = json!({
        "routes": {
            "getUser1": {
                "method": "GET",
                "path": "/users/:id",
                "pathParams": "GetUserParams1",
                "responses": { "200": "User" }
            },
            "getUser2": {
                "method": "GET",
                "path": "/users/:userId",
                "pathParams": "GetUserParams2",
                "responses": { "200": "User" }
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let warning = errors
        .iter()
        .find(|e| e.message.contains("potentially ambiguous paths"));
    assert!(warning.is_some(), "Expected V502 warning");
}

#[test]
fn test_v502_ambiguous_routes_param_vs_literal() {
    let config = json!({
        "routes": {
            "getUser": {
                "method": "GET",
                "path": "/users/:id",
                "pathParams": "GetUserParams",
                "responses": { "200": "User" }
            },
            "getCurrentUser": {
                "method": "GET",
                "path": "/users/me",
                "responses": { "200": "User" }
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let warning = errors
        .iter()
        .find(|e| e.message.contains("potentially ambiguous paths"));
    assert!(warning.is_some(), "Expected V502 warning for param vs literal");
}

// ============================================================================
// Valid configuration tests
// ============================================================================

#[test]
fn test_valid_complete_config() {
    let config = json!({
        "base_path": "/api/v1",
        "routes": {
            "getUser": {
                "method": "GET",
                "path": "/users/:id",
                "summary": "Get a user by ID",
                "description": "Returns a single user",
                "tags": ["users"],
                "pathParams": "GetUserPathParams",
                "responses": {
                    "200": "User",
                    "404": "NotFoundError"
                }
            },
            "listUsers": {
                "method": "GET",
                "path": "/users",
                "summary": "List all users",
                "query": "ListUsersQuery",
                "responses": {
                    "200": "User[]"
                }
            },
            "createUser": {
                "method": "POST",
                "path": "/users",
                "summary": "Create a new user",
                "body": "CreateUserBody",
                "responses": {
                    "201": "User",
                    "400": ["ValidationError", "InvalidInputError"]
                }
            },
            "deleteUser": {
                "method": "DELETE",
                "path": "/users/:id",
                "pathParams": "GetUserPathParams",
                "responses": {
                    "204": null,
                    "404": "NotFoundError"
                }
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let actual_errors: Vec<_> = errors.iter().filter(|e| e.severity == Severity::Error).collect();
    assert!(
        actual_errors.is_empty(),
        "Expected no errors for valid config, got: {:?}",
        actual_errors
    );
}

// ============================================================================
// Response type validation tests
// ============================================================================

#[test]
fn test_invalid_response_type() {
    let config = json!({
        "routes": {
            "getUser": {
                "method": "GET",
                "path": "/users",
                "responses": {
                    "200": 123
                }
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors
        .iter()
        .find(|e| e.message.contains("response must be a model name"));
    assert!(error.is_some(), "Expected error for invalid response type");
}

#[test]
fn test_invalid_response_union_item() {
    let config = json!({
        "routes": {
            "getUser": {
                "method": "GET",
                "path": "/users",
                "responses": {
                    "200": "User",
                    "400": ["ValidationError", 123]
                }
            }
        }
    });

    let errors = validate_config(ConfigLevel::Global, config, &utils());
    let error = errors
        .iter()
        .find(|e| e.message.contains("response union items must be model name strings"));
    assert!(error.is_some(), "Expected error for invalid union item");
}
