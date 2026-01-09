use super::*;
use cdm_plugin_interface::{FieldDefinition, ModelDefinition, Schema, TypeExpression, Utils};
use std::collections::HashMap;

fn create_test_schema() -> Schema {
    let mut models = HashMap::new();

    // Create User model
    let user_model = ModelDefinition {
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
                config: serde_json::json!({
                    "primary": { "generation": "uuid" }
                }),
                entity_id: None,
            },
            FieldDefinition {
                name: "email".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({ "unique": true }),
                entity_id: None,
            },
            FieldDefinition {
                name: "name".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: true,
                default: None,
                config: serde_json::json!({}),
                entity_id: None,
            },
        ],
        config: serde_json::json!({}),
        entity_id: None,
    };
    models.insert("User".to_string(), user_model);

    Schema {
        models,
        type_aliases: HashMap::new(),
    }
}

#[test]
fn test_build_generates_entity_file() {
    let schema = create_test_schema();
    let config = serde_json::json!({
        "entity_file_strategy": "per_model",
        "table_name_format": "snake_case",
        "pluralize_table_names": true
    });
    let utils = Utils;

    let files = build(schema, config, &utils);

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "User.ts");
}

#[test]
fn test_build_generates_entity_decorator() {
    let schema = create_test_schema();
    let config = serde_json::json!({
        "entity_file_strategy": "per_model",
        "table_name_format": "snake_case",
        "pluralize_table_names": true
    });
    let utils = Utils;

    let files = build(schema, config, &utils);

    assert!(files[0].content.contains("@Entity"));
    assert!(files[0].content.contains("\"users\""));
}

#[test]
fn test_build_generates_primary_key_decorator() {
    let schema = create_test_schema();
    let config = serde_json::json!({});
    let utils = Utils;

    let files = build(schema, config, &utils);

    assert!(files[0].content.contains("@PrimaryGeneratedColumn(\"uuid\")"));
}

#[test]
fn test_build_generates_column_decorator() {
    let schema = create_test_schema();
    let config = serde_json::json!({});
    let utils = Utils;

    let files = build(schema, config, &utils);

    assert!(files[0].content.contains("@Column"));
}

#[test]
fn test_build_generates_unique_column() {
    let schema = create_test_schema();
    let config = serde_json::json!({});
    let utils = Utils;

    let files = build(schema, config, &utils);

    // The email field should have unique: true
    assert!(files[0].content.contains("unique: true"));
}

#[test]
fn test_build_generates_optional_field() {
    let schema = create_test_schema();
    let config = serde_json::json!({});
    let utils = Utils;

    let files = build(schema, config, &utils);

    // The name field is optional, should have ? and nullable
    assert!(files[0].content.contains("name?:"));
}

#[test]
fn test_build_generates_typeorm_imports() {
    let schema = create_test_schema();
    let config = serde_json::json!({});
    let utils = Utils;

    let files = build(schema, config, &utils);

    assert!(files[0].content.contains("import {"));
    assert!(files[0].content.contains("Entity"));
    assert!(files[0].content.contains("Column"));
    assert!(files[0].content.contains("PrimaryGeneratedColumn"));
    assert!(files[0].content.contains("from \"typeorm\""));
}

#[test]
fn test_build_single_file_strategy() {
    let schema = create_test_schema();
    let config = serde_json::json!({
        "entity_file_strategy": "single",
        "entities_file_name": "all-entities.ts"
    });
    let utils = Utils;

    let files = build(schema, config, &utils);

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "all-entities.ts");
}

#[test]
fn test_build_skips_model() {
    let mut schema = create_test_schema();
    schema.models.get_mut("User").unwrap().config = serde_json::json!({ "skip": true });

    let config = serde_json::json!({});
    let utils = Utils;

    let files = build(schema, config, &utils);

    assert_eq!(files.len(), 0);
}
