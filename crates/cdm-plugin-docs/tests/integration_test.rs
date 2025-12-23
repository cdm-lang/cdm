/// Integration tests for the docs plugin
/// These tests verify the plugin works correctly when compiled to WASM
use cdm_plugin_api::{ConfigLevel, Schema, Severity, Utils};
use serde_json::json;

// Import the plugin functions directly for testing
// In real WASM, these would be called via FFI
use cdm_plugin_docs::{build, validate_config};

#[test]
fn test_validate_global_config_valid() {
    let config = json!({
        "format": "markdown",
        "include_examples": true,
        "title": "My Docs"
    });

    let utils = Utils {};
    let errors = validate_config(ConfigLevel::Global, config, &utils);

    assert!(errors.is_empty(), "Valid config should not produce errors");
}

#[test]
fn test_validate_global_config_invalid_format() {
    let config = json!({
        "format": "pdf"  // Invalid format
    });

    let utils = Utils {};
    let errors = validate_config(ConfigLevel::Global, config, &utils);

    assert_eq!(errors.len(), 1, "Should produce exactly one error");
    assert_eq!(errors[0].severity, Severity::Error);
    assert!(errors[0].message.contains("markdown"));
}

#[test]
fn test_validate_global_config_invalid_boolean() {
    let config = json!({
        "include_examples": "yes"  // Should be boolean
    });

    let utils = Utils {};
    let errors = validate_config(ConfigLevel::Global, config, &utils);

    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("boolean"));
}

#[test]
fn test_validate_model_config_valid() {
    let config = json!({
        "description": "A user model",
        "example": "{\"id\": 1}",
        "hidden": false
    });

    let utils = Utils {};
    let errors = validate_config(
        ConfigLevel::Model {
            name: "User".to_string(),
        },
        config,
        &utils,
    );

    assert!(errors.is_empty());
}

#[test]
fn test_validate_model_config_invalid_types() {
    let config = json!({
        "description": 123,  // Should be string
        "hidden": "no"       // Should be boolean
    });

    let utils = Utils {};
    let errors = validate_config(
        ConfigLevel::Model {
            name: "User".to_string(),
        },
        config,
        &utils,
    );

    assert_eq!(errors.len(), 2);
}

#[test]
fn test_validate_field_config_valid() {
    let config = json!({
        "description": "User's email address",
        "example": "user@example.com",
        "deprecated": true
    });

    let utils = Utils {};
    let errors = validate_config(
        ConfigLevel::Field {
            model: "User".to_string(),
            field: "email".to_string(),
        },
        config,
        &utils,
    );

    assert!(errors.is_empty());
}

#[test]
fn test_validate_field_config_invalid() {
    let config = json!({
        "deprecated": "yes"  // Should be boolean
    });

    let utils = Utils {};
    let errors = validate_config(
        ConfigLevel::Field {
            model: "User".to_string(),
            field: "email".to_string(),
        },
        config,
        &utils,
    );

    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("boolean"));
}

#[test]
fn test_generate_markdown_basic() {
    let schema = create_test_schema();
    let config = json!({
        "format": "markdown",
        "include_examples": false,
        "include_inheritance": false
    });

    let utils = Utils {};
    let outputs = build(schema, config, &utils);

    assert_eq!(outputs.len(), 1, "Should generate one file");
    assert_eq!(outputs[0].path, "schema.md");
    assert!(outputs[0].content.contains("# Schema Documentation"));
    assert!(outputs[0].content.contains("User"));
}

#[test]
fn test_generate_markdown_with_title() {
    let schema = create_test_schema();
    let config = json!({
        "format": "markdown",
        "title": "My Custom Title"
    });

    let utils = Utils {};
    let outputs = build(schema, config, &utils);

    assert!(outputs[0].content.contains("# My Custom Title"));
}

#[test]
fn test_generate_markdown_with_examples() {
    let schema = create_test_schema_with_examples();
    let config = json!({
        "format": "markdown",
        "include_examples": true
    });

    let utils = Utils {};
    let outputs = build(schema, config, &utils);

    assert!(outputs[0].content.contains("**Example:**"));
}

#[test]
fn test_generate_markdown_hidden_models() {
    let schema = create_test_schema_with_hidden();
    let config = json!({
        "format": "markdown"
    });

    let utils = Utils {};
    let outputs = build(schema, config, &utils);

    // Hidden models should not appear in output
    assert!(!outputs[0].content.contains("HiddenModel"));
    assert!(outputs[0].content.contains("VisibleModel"));
}

#[test]
fn test_generate_html() {
    let schema = create_test_schema();
    let config = json!({
        "format": "html"
    });

    let utils = Utils {};
    let outputs = build(schema, config, &utils);

    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].path, "schema.html");
    assert!(outputs[0].content.contains("<!DOCTYPE html>"));
    assert!(outputs[0].content.contains("</html>"));
}

#[test]
fn test_generate_json() {
    let schema = create_test_schema();
    let config = json!({
        "format": "json"
    });

    let utils = Utils {};
    let outputs = build(schema, config, &utils);

    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].path, "schema.json");

    // Should be valid JSON
    let parsed: serde_json::Value =
        serde_json::from_str(&outputs[0].content).expect("Should be valid JSON");
    assert!(parsed.is_object());
}

#[test]
fn test_generate_with_deprecated_fields() {
    let schema = create_test_schema_with_deprecated();
    let config = json!({
        "format": "markdown"
    });

    let utils = Utils {};
    let outputs = build(schema, config, &utils);

    // Deprecated fields should be marked with strikethrough
    assert!(outputs[0].content.contains("~~oldField~~"));
}

// Helper functions to create test schemas

fn create_test_schema() -> Schema {
    use cdm_plugin_api::{FieldDefinition, ModelDefinition, TypeExpression};
    use std::collections::HashMap;

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            fields: vec![
                FieldDefinition {
                    name: "id".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "UUID".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                },
                FieldDefinition {
                    name: "email".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({
                        "description": "User's email address"
                    }),
                },
            ],
            parents: vec![],
            config: json!({
                "description": "A user in the system"
            }),
        },
    );

    Schema {
        models,
        type_aliases: HashMap::new(),
    }
}

fn create_test_schema_with_examples() -> Schema {
    use cdm_plugin_api::ModelDefinition;
    use std::collections::HashMap;

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            fields: vec![],
            parents: vec![],
            config: json!({
                "example": "{\"id\": \"123\", \"email\": \"test@example.com\"}"
            }),
        },
    );

    Schema {
        models,
        type_aliases: HashMap::new(),
    }
}

fn create_test_schema_with_hidden() -> Schema {
    use cdm_plugin_api::ModelDefinition;
    use std::collections::HashMap;

    let mut models = HashMap::new();
    models.insert(
        "HiddenModel".to_string(),
        ModelDefinition {
            name: "HiddenModel".to_string(),
            fields: vec![],
            parents: vec![],
            config: json!({
                "hidden": true
            }),
        },
    );
    models.insert(
        "VisibleModel".to_string(),
        ModelDefinition {
            name: "VisibleModel".to_string(),
            fields: vec![],
            parents: vec![],
            config: json!({}),
        },
    );

    Schema {
        models,
        type_aliases: HashMap::new(),
    }
}

fn create_test_schema_with_deprecated() -> Schema {
    use cdm_plugin_api::{FieldDefinition, ModelDefinition, TypeExpression};
    use std::collections::HashMap;

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            fields: vec![FieldDefinition {
                name: "oldField".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: true,
                default: None,
                config: json!({
                    "deprecated": true
                }),
            }],
            parents: vec![],
            config: json!({}),
        },
    );

    Schema {
        models,
        type_aliases: HashMap::new(),
    }
}
