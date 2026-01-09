use super::*;
use cdm_plugin_interface::{ConfigLevel, Utils};

#[test]
fn test_validate_global_config_valid() {
    let config = serde_json::json!({
        "entity_file_strategy": "per_model",
        "table_name_format": "snake_case",
        "column_name_format": "snake_case"
    });
    let utils = Utils;

    let errors = validate_config(ConfigLevel::Global, config, &utils);

    assert!(errors.is_empty());
}

#[test]
fn test_validate_global_config_invalid_file_strategy() {
    let config = serde_json::json!({
        "entity_file_strategy": "invalid"
    });
    let utils = Utils;

    let errors = validate_config(ConfigLevel::Global, config, &utils);

    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("entity_file_strategy"));
}

#[test]
fn test_validate_global_config_invalid_table_name_format() {
    let config = serde_json::json!({
        "table_name_format": "invalid"
    });
    let utils = Utils;

    let errors = validate_config(ConfigLevel::Global, config, &utils);

    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("table_name_format"));
}

#[test]
fn test_validate_global_config_empty_import_path() {
    let config = serde_json::json!({
        "typeorm_import_path": ""
    });
    let utils = Utils;

    let errors = validate_config(ConfigLevel::Global, config, &utils);

    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("typeorm_import_path"));
}

#[test]
fn test_validate_model_config_valid_index() {
    let config = serde_json::json!({
        "indexes": [
            { "fields": ["email"], "unique": true }
        ]
    });
    let utils = Utils;

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
fn test_validate_model_config_empty_index_fields() {
    let config = serde_json::json!({
        "indexes": [
            { "fields": [] }
        ]
    });
    let utils = Utils;

    let errors = validate_config(
        ConfigLevel::Model {
            name: "User".to_string(),
        },
        config,
        &utils,
    );

    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("at least one field"));
}

#[test]
fn test_validate_model_config_missing_index_fields() {
    let config = serde_json::json!({
        "indexes": [
            { "unique": true }
        ]
    });
    let utils = Utils;

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
fn test_validate_field_config_valid_primary() {
    let config = serde_json::json!({
        "primary": { "generation": "uuid" }
    });
    let utils = Utils;

    let errors = validate_config(
        ConfigLevel::Field {
            model: "User".to_string(),
            field: "id".to_string(),
        },
        config,
        &utils,
    );

    assert!(errors.is_empty());
}

#[test]
fn test_validate_field_config_invalid_primary_generation() {
    let config = serde_json::json!({
        "primary": { "generation": "invalid" }
    });
    let utils = Utils;

    let errors = validate_config(
        ConfigLevel::Field {
            model: "User".to_string(),
            field: "id".to_string(),
        },
        config,
        &utils,
    );

    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("primary.generation"));
}

#[test]
fn test_validate_field_config_valid_relation() {
    let config = serde_json::json!({
        "relation": {
            "type": "many_to_one",
            "inverse_side": "posts"
        }
    });
    let utils = Utils;

    let errors = validate_config(
        ConfigLevel::Field {
            model: "Post".to_string(),
            field: "author".to_string(),
        },
        config,
        &utils,
    );

    assert!(errors.is_empty());
}

#[test]
fn test_validate_field_config_invalid_relation_type() {
    let config = serde_json::json!({
        "relation": {
            "type": "invalid"
        }
    });
    let utils = Utils;

    let errors = validate_config(
        ConfigLevel::Field {
            model: "Post".to_string(),
            field: "author".to_string(),
        },
        config,
        &utils,
    );

    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("relation.type"));
}

#[test]
fn test_validate_field_config_missing_relation_type() {
    let config = serde_json::json!({
        "relation": {
            "inverse_side": "posts"
        }
    });
    let utils = Utils;

    let errors = validate_config(
        ConfigLevel::Field {
            model: "Post".to_string(),
            field: "author".to_string(),
        },
        config,
        &utils,
    );

    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("must have a 'type' field"));
}

#[test]
fn test_validate_field_config_primary_and_relation_error() {
    let config = serde_json::json!({
        "primary": { "generation": "uuid" },
        "relation": { "type": "many_to_one" }
    });
    let utils = Utils;

    let errors = validate_config(
        ConfigLevel::Field {
            model: "Post".to_string(),
            field: "author".to_string(),
        },
        config,
        &utils,
    );

    assert!(errors.iter().any(|e| e.message.contains("cannot have both")));
}

#[test]
fn test_validate_field_config_invalid_inverse_side() {
    let config = serde_json::json!({
        "relation": {
            "type": "many_to_one",
            "inverse_side": ""
        }
    });
    let utils = Utils;

    let errors = validate_config(
        ConfigLevel::Field {
            model: "Post".to_string(),
            field: "author".to_string(),
        },
        config,
        &utils,
    );

    assert!(errors.iter().any(|e| e.message.contains("inverse_side")));
}

#[test]
fn test_validate_field_config_invalid_on_delete() {
    let config = serde_json::json!({
        "relation": {
            "type": "many_to_one",
            "on_delete": "INVALID"
        }
    });
    let utils = Utils;

    let errors = validate_config(
        ConfigLevel::Field {
            model: "Post".to_string(),
            field: "author".to_string(),
        },
        config,
        &utils,
    );

    assert!(errors.iter().any(|e| e.message.contains("on_delete")));
}

#[test]
fn test_validate_many_to_many_with_join_table() {
    let config = serde_json::json!({
        "relation": {
            "type": "many_to_many",
            "inverse_side": "posts",
            "join_table": {
                "name": "post_tags"
            }
        }
    });
    let utils = Utils;

    let errors = validate_config(
        ConfigLevel::Field {
            model: "Post".to_string(),
            field: "tags".to_string(),
        },
        config,
        &utils,
    );

    assert!(errors.is_empty());
}

#[test]
fn test_validate_many_to_many_empty_join_table_name() {
    let config = serde_json::json!({
        "relation": {
            "type": "many_to_many",
            "join_table": {
                "name": ""
            }
        }
    });
    let utils = Utils;

    let errors = validate_config(
        ConfigLevel::Field {
            model: "Post".to_string(),
            field: "tags".to_string(),
        },
        config,
        &utils,
    );

    assert!(errors.iter().any(|e| e.message.contains("join_table name")));
}

#[test]
fn test_is_valid_identifier() {
    assert!(is_valid_identifier("validName"));
    assert!(is_valid_identifier("_private"));
    assert!(is_valid_identifier("$jquery"));
    assert!(is_valid_identifier("name123"));

    assert!(!is_valid_identifier(""));
    assert!(!is_valid_identifier("123invalid"));
    assert!(!is_valid_identifier("has-hyphen"));
    assert!(!is_valid_identifier("has space"));
    assert!(!is_valid_identifier("class")); // reserved word
    assert!(!is_valid_identifier("function")); // reserved word
}
