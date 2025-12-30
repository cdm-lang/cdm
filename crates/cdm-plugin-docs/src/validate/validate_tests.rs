use super::*;
use serde_json::json;

#[test]
fn test_validate_global_config_empty() {
    let config = json!({});
    let utils = Utils {};
    let errors = validate_config(ConfigLevel::Global, config, &utils);

    assert!(errors.is_empty(), "Empty global config should be valid");
}

#[test]
fn test_validate_global_config_valid_markdown() {
    let config = json!({
        "format": "markdown",
        "include_examples": true,
        "include_inheritance": false,
        "title": "My Documentation"
    });

    let utils = Utils {};
    let errors = validate_config(ConfigLevel::Global, config, &utils);

    assert!(errors.is_empty(), "Valid markdown config should not produce errors");
}

#[test]
fn test_validate_global_config_valid_html() {
    let config = json!({
        "format": "html"
    });

    let utils = Utils {};
    let errors = validate_config(ConfigLevel::Global, config, &utils);

    assert!(errors.is_empty(), "Valid HTML format should not produce errors");
}

#[test]
fn test_validate_global_config_valid_json() {
    let config = json!({
        "format": "json"
    });

    let utils = Utils {};
    let errors = validate_config(ConfigLevel::Global, config, &utils);

    assert!(errors.is_empty(), "Valid JSON format should not produce errors");
}

#[test]
fn test_validate_global_config_invalid_format() {
    let config = json!({
        "format": "pdf"
    });

    let utils = Utils {};
    let errors = validate_config(ConfigLevel::Global, config, &utils);

    assert_eq!(errors.len(), 1, "Invalid format should produce exactly one error");
    assert_eq!(errors[0].severity, Severity::Error);
    assert!(errors[0].message.contains("markdown"));
    assert!(errors[0].message.contains("html"));
    assert!(errors[0].message.contains("json"));
}

#[test]
fn test_validate_global_config_invalid_include_examples_type() {
    let config = json!({
        "include_examples": "yes"
    });

    let utils = Utils {};
    let errors = validate_config(ConfigLevel::Global, config, &utils);

    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].severity, Severity::Error);
    assert!(errors[0].message.contains("boolean"));
    assert!(errors[0].message.contains("include_examples"));
}

#[test]
fn test_validate_global_config_invalid_include_inheritance_type() {
    let config = json!({
        "include_inheritance": 1
    });

    let utils = Utils {};
    let errors = validate_config(ConfigLevel::Global, config, &utils);

    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("boolean"));
    assert!(errors[0].message.contains("include_inheritance"));
}

#[test]
fn test_validate_global_config_invalid_title_type() {
    let config = json!({
        "title": 123
    });

    let utils = Utils {};
    let errors = validate_config(ConfigLevel::Global, config, &utils);

    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("string"));
    assert!(errors[0].message.contains("title"));
}

#[test]
fn test_validate_global_config_multiple_errors() {
    let config = json!({
        "format": "xml",
        "include_examples": "true",
        "title": false
    });

    let utils = Utils {};
    let errors = validate_config(ConfigLevel::Global, config, &utils);

    assert_eq!(errors.len(), 3, "Should report all three errors");
}

#[test]
fn test_validate_model_config_empty() {
    let config = json!({});
    let utils = Utils {};
    let errors = validate_config(
        ConfigLevel::Model {
            name: "User".to_string(),
        },
        config,
        &utils,
    );

    assert!(errors.is_empty(), "Empty model config should be valid");
}

#[test]
fn test_validate_model_config_valid() {
    let config = json!({
        "description": "A user model",
        "example": "{\"id\": 1, \"name\": \"John\"}",
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

    assert!(errors.is_empty(), "Valid model config should not produce errors");
}

#[test]
fn test_validate_model_config_invalid_description_type() {
    let config = json!({
        "description": 123
    });

    let utils = Utils {};
    let errors = validate_config(
        ConfigLevel::Model {
            name: "User".to_string(),
        },
        config,
        &utils,
    );

    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("description"));
    assert!(errors[0].message.contains("string"));
    assert_eq!(errors[0].path.len(), 2);
    assert_eq!(errors[0].path[0].kind, "model");
    assert_eq!(errors[0].path[0].name, "User");
}

#[test]
fn test_validate_model_config_invalid_example_type() {
    let config = json!({
        "example": ["not", "a", "string"]
    });

    let utils = Utils {};
    let errors = validate_config(
        ConfigLevel::Model {
            name: "Post".to_string(),
        },
        config,
        &utils,
    );

    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("example"));
    assert!(errors[0].message.contains("string"));
}

#[test]
fn test_validate_model_config_invalid_hidden_type() {
    let config = json!({
        "hidden": "no"
    });

    let utils = Utils {};
    let errors = validate_config(
        ConfigLevel::Model {
            name: "Admin".to_string(),
        },
        config,
        &utils,
    );

    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("hidden"));
    assert!(errors[0].message.contains("boolean"));
}

#[test]
fn test_validate_model_config_multiple_errors() {
    let config = json!({
        "description": true,
        "example": null,
        "hidden": "yes"
    });

    let utils = Utils {};
    let errors = validate_config(
        ConfigLevel::Model {
            name: "Product".to_string(),
        },
        config,
        &utils,
    );

    assert_eq!(errors.len(), 3, "Should report all three type errors");
}

#[test]
fn test_validate_field_config_empty() {
    let config = json!({});
    let utils = Utils {};
    let errors = validate_config(
        ConfigLevel::Field {
            model: "User".to_string(),
            field: "email".to_string(),
        },
        config,
        &utils,
    );

    assert!(errors.is_empty(), "Empty field config should be valid");
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

    assert!(errors.is_empty(), "Valid field config should not produce errors");
}

#[test]
fn test_validate_field_config_invalid_description_type() {
    let config = json!({
        "description": 456
    });

    let utils = Utils {};
    let errors = validate_config(
        ConfigLevel::Field {
            model: "User".to_string(),
            field: "name".to_string(),
        },
        config,
        &utils,
    );

    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("description"));
    assert!(errors[0].message.contains("string"));
    assert_eq!(errors[0].path.len(), 3);
    assert_eq!(errors[0].path[0].kind, "model");
    assert_eq!(errors[0].path[0].name, "User");
    assert_eq!(errors[0].path[1].kind, "field");
    assert_eq!(errors[0].path[1].name, "name");
}

#[test]
fn test_validate_field_config_invalid_example_type() {
    let config = json!({
        "example": {"key": "value"}
    });

    let utils = Utils {};
    let errors = validate_config(
        ConfigLevel::Field {
            model: "Post".to_string(),
            field: "content".to_string(),
        },
        config,
        &utils,
    );

    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("example"));
    assert!(errors[0].message.contains("string"));
}

#[test]
fn test_validate_field_config_invalid_deprecated_type() {
    let config = json!({
        "deprecated": "yes"
    });

    let utils = Utils {};
    let errors = validate_config(
        ConfigLevel::Field {
            model: "User".to_string(),
            field: "oldEmail".to_string(),
        },
        config,
        &utils,
    );

    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("deprecated"));
    assert!(errors[0].message.contains("boolean"));
}

#[test]
fn test_validate_field_config_multiple_errors() {
    let config = json!({
        "description": [],
        "example": 123,
        "deprecated": "true"
    });

    let utils = Utils {};
    let errors = validate_config(
        ConfigLevel::Field {
            model: "Order".to_string(),
            field: "status".to_string(),
        },
        config,
        &utils,
    );

    assert_eq!(errors.len(), 3, "Should report all three type errors");
}

#[test]
fn test_validate_config_path_includes_correct_segments() {
    let config = json!({
        "description": 999
    });

    let utils = Utils {};
    let errors = validate_config(
        ConfigLevel::Field {
            model: "Customer".to_string(),
            field: "address".to_string(),
        },
        config,
        &utils,
    );

    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].path.len(), 3);

    // Verify path segments
    assert_eq!(errors[0].path[0].kind, "model");
    assert_eq!(errors[0].path[0].name, "Customer");

    assert_eq!(errors[0].path[1].kind, "field");
    assert_eq!(errors[0].path[1].name, "address");

    assert_eq!(errors[0].path[2].kind, "config");
    assert_eq!(errors[0].path[2].name, "description");
}

#[test]
fn test_validate_global_allows_unknown_fields() {
    let config = json!({
        "format": "markdown",
        "unknown_field": "should be ignored",
        "another_unknown": 123
    });

    let utils = Utils {};
    let errors = validate_config(ConfigLevel::Global, config, &utils);

    // Should only validate known fields, ignore unknown ones
    assert!(errors.is_empty(), "Unknown fields should be ignored");
}

#[test]
fn test_validate_model_allows_unknown_fields() {
    let config = json!({
        "description": "Valid description",
        "custom_field": "ignored"
    });

    let utils = Utils {};
    let errors = validate_config(
        ConfigLevel::Model {
            name: "Product".to_string(),
        },
        config,
        &utils,
    );

    assert!(errors.is_empty(), "Unknown fields should be ignored");
}

#[test]
fn test_validate_field_allows_unknown_fields() {
    let config = json!({
        "deprecated": false,
        "extra_metadata": {"key": "value"}
    });

    let utils = Utils {};
    let errors = validate_config(
        ConfigLevel::Field {
            model: "Item".to_string(),
            field: "quantity".to_string(),
        },
        config,
        &utils,
    );

    assert!(errors.is_empty(), "Unknown fields should be ignored");
}
