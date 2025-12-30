use super::*;
use serde_json::json;

#[test]
fn test_validate_empty_config() {
    let config = json!({});
    let utils = Utils;

    let errors = validate_config(ConfigLevel::Global, config, &utils);
    assert!(errors.is_empty());
}

#[test]
fn test_validate_global_dialect() {
    let utils = Utils;

    // Valid dialects
    let config = json!({ "dialect": "postgresql" });
    let errors = validate_config(ConfigLevel::Global, config, &utils);
    assert!(errors.is_empty());

    let config = json!({ "dialect": "sqlite" });
    let errors = validate_config(ConfigLevel::Global, config, &utils);
    assert!(errors.is_empty());

    // Invalid dialect
    let config = json!({ "dialect": "mysql" });
    let errors = validate_config(ConfigLevel::Global, config, &utils);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("dialect"));
}

#[test]
fn test_validate_global_table_name_format() {
    let utils = Utils;

    let config = json!({ "table_name_format": "invalid" });
    let errors = validate_config(ConfigLevel::Global, config, &utils);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("table_name_format"));
}

#[test]
fn test_validate_global_column_name_format() {
    let utils = Utils;

    let config = json!({ "column_name_format": "invalid" });
    let errors = validate_config(ConfigLevel::Global, config, &utils);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("column_name_format"));
}

#[test]
fn test_validate_global_default_string_length() {
    let utils = Utils;

    // Valid
    let config = json!({ "default_string_length": 255 });
    let errors = validate_config(ConfigLevel::Global, config, &utils);
    assert!(errors.is_empty());

    // Invalid (zero)
    let config = json!({ "default_string_length": 0 });
    let errors = validate_config(ConfigLevel::Global, config, &utils);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("greater than 0"));

    // Invalid (negative)
    let config = json!({ "default_string_length": -1 });
    let errors = validate_config(ConfigLevel::Global, config, &utils);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("greater than 0"));
}

#[test]
fn test_validate_global_number_type() {
    let utils = Utils;

    let config = json!({ "number_type": "invalid" });
    let errors = validate_config(ConfigLevel::Global, config, &utils);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("number_type"));
}

#[test]
fn test_validate_global_schema_sqlite() {
    let utils = Utils;

    // Schema with SQLite should error
    let config = json!({
        "dialect": "sqlite",
        "schema": "public"
    });
    let errors = validate_config(ConfigLevel::Global, config, &utils);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("PostgreSQL"));
}

#[test]
fn test_validate_model_indexes() {
    let utils = Utils;

    // Valid index
    let config = json!({
        "indexes": [
            { "fields": ["id"], "primary": true }
        ]
    });
    let errors = validate_config(
        ConfigLevel::Model {
            name: "User".to_string(),
        },
        config,
        &utils,
    );
    assert!(errors.is_empty());

    // Empty fields array
    let config = json!({
        "indexes": [
            { "fields": [] }
        ]
    });
    let errors = validate_config(
        ConfigLevel::Model {
            name: "User".to_string(),
        },
        config,
        &utils,
    );
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("at least one field"));

    // Missing fields
    let config = json!({
        "indexes": [
            { "primary": true }
        ]
    });
    let errors = validate_config(
        ConfigLevel::Model {
            name: "User".to_string(),
        },
        config,
        &utils,
    );
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("'fields' array"));
}

#[test]
fn test_validate_model_multiple_primary_keys() {
    let utils = Utils;

    let config = json!({
        "indexes": [
            { "fields": ["id"], "primary": true },
            { "fields": ["email"], "primary": true }
        ]
    });
    let errors = validate_config(
        ConfigLevel::Model {
            name: "User".to_string(),
        },
        config,
        &utils,
    );
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("one primary key"));
}

#[test]
fn test_validate_model_constraint_check() {
    let utils = Utils;

    // Valid check constraint
    let config = json!({
        "constraints": [
            {
                "type": "check",
                "fields": ["age"],
                "expression": "age >= 18"
            }
        ]
    });
    let errors = validate_config(
        ConfigLevel::Model {
            name: "User".to_string(),
        },
        config,
        &utils,
    );
    assert!(errors.is_empty());

    // Missing expression
    let config = json!({
        "constraints": [
            {
                "type": "check",
                "fields": ["age"]
            }
        ]
    });
    let errors = validate_config(
        ConfigLevel::Model {
            name: "User".to_string(),
        },
        config,
        &utils,
    );
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("expression"));
}

#[test]
fn test_validate_model_constraint_foreign_key() {
    let utils = Utils;

    // Valid foreign key constraint
    let config = json!({
        "constraints": [
            {
                "type": "foreign_key",
                "fields": ["user_id"],
                "reference": {
                    "table": "User",
                    "column": "id"
                }
            }
        ]
    });
    let errors = validate_config(
        ConfigLevel::Model {
            name: "Post".to_string(),
        },
        config,
        &utils,
    );
    assert!(errors.is_empty());

    // Missing reference
    let config = json!({
        "constraints": [
            {
                "type": "foreign_key",
                "fields": ["user_id"]
            }
        ]
    });
    let errors = validate_config(
        ConfigLevel::Model {
            name: "Post".to_string(),
        },
        config,
        &utils,
    );
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("reference"));
}

#[test]
fn test_validate_field_reference() {
    let utils = Utils;

    // Valid reference
    let config = json!({
        "references": {
            "table": "User",
            "column": "id",
            "on_delete": "cascade"
        }
    });
    let errors = validate_config(
        ConfigLevel::Field {
            model: "Post".to_string(),
            field: "user_id".to_string(),
        },
        config,
        &utils,
    );
    assert!(errors.is_empty());

    // Missing table
    let config = json!({
        "references": {
            "column": "id"
        }
    });
    let errors = validate_config(
        ConfigLevel::Field {
            model: "Post".to_string(),
            field: "user_id".to_string(),
        },
        config,
        &utils,
    );
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("'table' field"));

    // Empty table
    let config = json!({
        "references": {
            "table": ""
        }
    });
    let errors = validate_config(
        ConfigLevel::Field {
            model: "Post".to_string(),
            field: "user_id".to_string(),
        },
        config,
        &utils,
    );
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("cannot be empty"));
}

#[test]
fn test_validate_field_reference_actions() {
    let utils = Utils;

    // Invalid on_delete
    let config = json!({
        "references": {
            "table": "User",
            "on_delete": "invalid"
        }
    });
    let errors = validate_config(
        ConfigLevel::Field {
            model: "Post".to_string(),
            field: "user_id".to_string(),
        },
        config,
        &utils,
    );
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("on_delete"));

    // Invalid on_update
    let config = json!({
        "references": {
            "table": "User",
            "on_update": "invalid"
        }
    });
    let errors = validate_config(
        ConfigLevel::Field {
            model: "Post".to_string(),
            field: "user_id".to_string(),
        },
        config,
        &utils,
    );
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("on_update"));
}

#[test]
fn test_validate_field_relationship() {
    let utils = Utils;

    // Valid one_to_many
    let config = json!({
        "relationship": {
            "type": "one_to_many"
        }
    });
    let errors = validate_config(
        ConfigLevel::Field {
            model: "User".to_string(),
            field: "posts".to_string(),
        },
        config,
        &utils,
    );
    assert!(errors.is_empty());

    // Valid many_to_many with through
    let config = json!({
        "relationship": {
            "type": "many_to_many",
            "through": "UserGroup"
        }
    });
    let errors = validate_config(
        ConfigLevel::Field {
            model: "User".to_string(),
            field: "groups".to_string(),
        },
        config,
        &utils,
    );
    assert!(errors.is_empty());

    // many_to_many without through
    let config = json!({
        "relationship": {
            "type": "many_to_many"
        }
    });
    let errors = validate_config(
        ConfigLevel::Field {
            model: "User".to_string(),
            field: "groups".to_string(),
        },
        config,
        &utils,
    );
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("'through'"));

    // Invalid relationship type
    let config = json!({
        "relationship": {
            "type": "invalid"
        }
    });
    let errors = validate_config(
        ConfigLevel::Field {
            model: "User".to_string(),
            field: "posts".to_string(),
        },
        config,
        &utils,
    );
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("relationship type"));

    // Missing type
    let config = json!({
        "relationship": {}
    });
    let errors = validate_config(
        ConfigLevel::Field {
            model: "User".to_string(),
            field: "posts".to_string(),
        },
        config,
        &utils,
    );
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("'type' field"));
}
