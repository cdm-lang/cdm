use crate::build::build;
use cdm_plugin_interface::{
    FieldDefinition, ModelDefinition, Schema, TypeExpression, Utils,
};
use serde_json::json;
use std::collections::HashMap;

fn utils() -> Utils {
    Utils
}

fn create_test_schema() -> Schema {
    let mut models = HashMap::new();
    let type_aliases = HashMap::new();

    // User model
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "id".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: None,
                },
                FieldDefinition {
                    name: "email".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: None,
                },
                FieldDefinition {
                    name: "name".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: None,
                },
            ],
            config: json!({}),
            entity_id: None,
        },
    );

    // GetUserPathParams model
    models.insert(
        "GetUserPathParams".to_string(),
        ModelDefinition {
            name: "GetUserPathParams".to_string(),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "id".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: json!({}),
                entity_id: None,
            }],
            config: json!({}),
            entity_id: None,
        },
    );

    // ListUsersQuery model
    models.insert(
        "ListUsersQuery".to_string(),
        ModelDefinition {
            name: "ListUsersQuery".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "limit".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "number".to_string(),
                    },
                    optional: true,
                    default: None,
                    config: json!({}),
                    entity_id: None,
                },
                FieldDefinition {
                    name: "offset".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "number".to_string(),
                    },
                    optional: true,
                    default: None,
                    config: json!({}),
                    entity_id: None,
                },
            ],
            config: json!({}),
            entity_id: None,
        },
    );

    // CreateUserBody model
    models.insert(
        "CreateUserBody".to_string(),
        ModelDefinition {
            name: "CreateUserBody".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "email".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: None,
                },
                FieldDefinition {
                    name: "name".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: None,
                },
            ],
            config: json!({}),
            entity_id: None,
        },
    );

    // NotFoundError model
    models.insert(
        "NotFoundError".to_string(),
        ModelDefinition {
            name: "NotFoundError".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "code".to_string(),
                    field_type: TypeExpression::StringLiteral {
                        value: "NOT_FOUND".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: None,
                },
                FieldDefinition {
                    name: "message".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: None,
                },
            ],
            config: json!({}),
            entity_id: None,
        },
    );

    // ValidationError model
    models.insert(
        "ValidationError".to_string(),
        ModelDefinition {
            name: "ValidationError".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "code".to_string(),
                    field_type: TypeExpression::StringLiteral {
                        value: "VALIDATION_ERROR".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: None,
                },
                FieldDefinition {
                    name: "message".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: None,
                },
            ],
            config: json!({}),
            entity_id: None,
        },
    );

    Schema {
        models,
        type_aliases,
    }
}

// ============================================================================
// Basic Build Tests
// ============================================================================

#[test]
fn test_build_generates_single_file() {
    let schema = create_test_schema();
    let config = json!({
        "routes": {
            "getUser": {
                "method": "GET",
                "path": "/users/:id",
                "pathParams": "GetUserPathParams",
                "responses": {
                    "200": "User",
                    "404": "NotFoundError"
                }
            }
        }
    });

    let files = build(schema, config, &utils());
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "contract.ts");
}

#[test]
fn test_build_includes_header_comment() {
    let schema = create_test_schema();
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

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("Generated by CDM @tsRest plugin"));
    assert!(content.contains("DO NOT EDIT"));
}

#[test]
fn test_build_imports_ts_rest() {
    let schema = create_test_schema();
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

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("import { initContract } from '@ts-rest/core'"));
    assert!(content.contains("import { z } from 'zod'"));
}

#[test]
fn test_build_imports_schemas() {
    let schema = create_test_schema();
    let config = json!({
        "routes": {
            "getUser": {
                "method": "GET",
                "path": "/users/:id",
                "pathParams": "GetUserPathParams",
                "responses": {
                    "200": "User",
                    "404": "NotFoundError"
                }
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("GetUserPathParamsSchema"));
    assert!(content.contains("UserSchema"));
    assert!(content.contains("NotFoundErrorSchema"));
    assert!(content.contains("from './schemas'"));
}

// ============================================================================
// Route Generation Tests
// ============================================================================

#[test]
fn test_build_generates_route_with_method() {
    let schema = create_test_schema();
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

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("method: 'GET'"));
}

#[test]
fn test_build_generates_route_with_path() {
    let schema = create_test_schema();
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

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("path: '/users/:id'"));
}

#[test]
fn test_build_applies_base_path() {
    let schema = create_test_schema();
    let config = json!({
        "base_path": "/api/v1",
        "routes": {
            "getUser": {
                "method": "GET",
                "path": "/users/:id",
                "pathParams": "GetUserPathParams",
                "responses": { "200": "User" }
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("path: '/api/v1/users/:id'"));
}

#[test]
fn test_build_generates_path_params() {
    let schema = create_test_schema();
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

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("pathParams: GetUserPathParamsSchema"));
}

#[test]
fn test_build_generates_query_params() {
    let schema = create_test_schema();
    let config = json!({
        "routes": {
            "listUsers": {
                "method": "GET",
                "path": "/users",
                "query": "ListUsersQuery",
                "responses": { "200": "User[]" }
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("query: ListUsersQuerySchema"));
}

#[test]
fn test_build_generates_body() {
    let schema = create_test_schema();
    let config = json!({
        "routes": {
            "createUser": {
                "method": "POST",
                "path": "/users",
                "body": "CreateUserBody",
                "responses": { "201": "User" }
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("body: CreateUserBodySchema"));
}

#[test]
fn test_build_generates_summary() {
    let schema = create_test_schema();
    let config = json!({
        "routes": {
            "getUser": {
                "method": "GET",
                "path": "/users/:id",
                "summary": "Get a user by ID",
                "pathParams": "GetUserPathParams",
                "responses": { "200": "User" }
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("summary: 'Get a user by ID'"));
}

// ============================================================================
// Response Generation Tests
// ============================================================================

#[test]
fn test_build_generates_single_response() {
    let schema = create_test_schema();
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

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("200: UserSchema"));
}

#[test]
fn test_build_generates_array_response() {
    let schema = create_test_schema();
    let config = json!({
        "routes": {
            "listUsers": {
                "method": "GET",
                "path": "/users",
                "responses": { "200": "User[]" }
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("200: z.array(UserSchema)"));
}

#[test]
fn test_build_generates_union_response() {
    let schema = create_test_schema();
    let config = json!({
        "routes": {
            "createUser": {
                "method": "POST",
                "path": "/users",
                "body": "CreateUserBody",
                "responses": {
                    "201": "User",
                    "400": ["ValidationError", "NotFoundError"]
                }
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("z.union([ValidationErrorSchema, NotFoundErrorSchema])"));
}

#[test]
fn test_build_generates_void_response() {
    let schema = create_test_schema();
    let config = json!({
        "routes": {
            "deleteUser": {
                "method": "DELETE",
                "path": "/users/:id",
                "pathParams": "GetUserPathParams",
                "responses": { "204": null }
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("204: z.void()"));
}

// ============================================================================
// Contract Structure Tests
// ============================================================================

#[test]
fn test_build_creates_contract_router() {
    let schema = create_test_schema();
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

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("const c = initContract()"));
    assert!(content.contains("export const contract = c.router({"));
}

#[test]
fn test_build_with_multiple_routes() {
    let schema = create_test_schema();
    let config = json!({
        "base_path": "/api/v1",
        "routes": {
            "getUser": {
                "method": "GET",
                "path": "/users/:id",
                "summary": "Get a user by ID",
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
                    "400": "ValidationError"
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

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    // Verify all routes are generated
    assert!(content.contains("getUser:"));
    assert!(content.contains("listUsers:"));
    assert!(content.contains("createUser:"));
    assert!(content.contains("deleteUser:"));

    // Verify base_path is applied
    assert!(content.contains("path: '/api/v1/users/:id'"));
    assert!(content.contains("path: '/api/v1/users'"));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_build_with_empty_routes() {
    let schema = create_test_schema();
    let config = json!({
        "routes": {}
    });

    let files = build(schema, config, &utils());
    assert!(files.is_empty());
}

#[test]
fn test_build_without_routes() {
    let schema = create_test_schema();
    let config = json!({});

    let files = build(schema, config, &utils());
    assert!(files.is_empty());
}

#[test]
fn test_build_escapes_special_chars_in_summary() {
    let schema = create_test_schema();
    let config = json!({
        "routes": {
            "getUser": {
                "method": "GET",
                "path": "/users/:id",
                "summary": "Get user's profile",
                "pathParams": "GetUserPathParams",
                "responses": { "200": "User" }
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    // The apostrophe should be escaped
    assert!(content.contains("Get user\\'s profile"));
}

// Note: test_build_output_path_formatting was removed because build_output
// is now handled by CDM, not plugins. Plugins return relative paths like
// "contract.ts" and CDM prepends the configured output directory.

// ============================================================================
// Schema Import Configuration Tests
// ============================================================================

#[test]
fn test_build_default_schema_import_path() {
    // Default behavior: imports from './schemas'
    let schema = create_test_schema();
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

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("from './schemas'"));
}

#[test]
fn test_build_custom_schema_import_single_strategy() {
    // Custom single file path for schema imports
    let schema = create_test_schema();
    let config = json!({
        "schema_import": {
            "strategy": "single",
            "path": "./generated/zod-schemas"
        },
        "routes": {
            "getUser": {
                "method": "GET",
                "path": "/users/:id",
                "pathParams": "GetUserPathParams",
                "responses": { "200": "User" }
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    assert!(content.contains("from './generated/zod-schemas'"));
    assert!(!content.contains("from './schemas'"));
}

#[test]
fn test_build_schema_import_per_model_strategy() {
    // Per-model strategy: imports from individual files in directory
    let schema = create_test_schema();
    let config = json!({
        "schema_import": {
            "strategy": "per_model",
            "path": "./models"
        },
        "routes": {
            "getUser": {
                "method": "GET",
                "path": "/users/:id",
                "pathParams": "GetUserPathParams",
                "responses": {
                    "200": "User",
                    "404": "NotFoundError"
                }
            }
        }
    });

    let files = build(schema, config, &utils());
    let content = &files[0].content;

    // Should import from individual model files
    assert!(content.contains("from './models/User'"));
    assert!(content.contains("from './models/GetUserPathParams'"));
    assert!(content.contains("from './models/NotFoundError'"));
    // Should NOT have a single schemas import
    assert!(!content.contains("from './schemas'"));
}

