use super::*;
use cdm_plugin_interface::{Delta, FieldDefinition, ModelDefinition, Schema, TypeExpression, Utils};
use std::collections::HashMap;

fn create_test_schema() -> Schema {
    let mut models = HashMap::new();

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
fn test_migrate_empty_deltas() {
    let schema = create_test_schema();
    let deltas = vec![];
    let config = serde_json::json!({});
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);

    assert!(files.is_empty());
}

#[test]
fn test_migrate_model_added() {
    let schema = create_test_schema();
    let user_model = schema.models.get("User").unwrap().clone();

    let deltas = vec![Delta::ModelAdded {
        name: "User".to_string(),
        after: user_model,
    }];

    let config = serde_json::json!({
        "table_name_format": "snake_case",
        "pluralize_table_names": true
    });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);

    assert_eq!(files.len(), 1);
    assert!(files[0].path.ends_with(".ts"));
    assert!(files[0].content.contains("MigrationInterface"));
    assert!(files[0].content.contains("CREATE TABLE"));
    assert!(files[0].content.contains("users"));
}

#[test]
fn test_migrate_model_removed() {
    let schema = create_test_schema();
    let user_model = schema.models.get("User").unwrap().clone();

    let deltas = vec![Delta::ModelRemoved {
        name: "User".to_string(),
        before: user_model,
    }];

    let config = serde_json::json!({
        "table_name_format": "snake_case",
        "pluralize_table_names": true
    });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);

    assert_eq!(files.len(), 1);
    assert!(files[0].content.contains("DROP TABLE"));
}

#[test]
fn test_migrate_generates_typescript_class() {
    let schema = create_test_schema();
    let user_model = schema.models.get("User").unwrap().clone();

    let deltas = vec![Delta::ModelAdded {
        name: "User".to_string(),
        after: user_model,
    }];

    let config = serde_json::json!({});
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);

    assert!(files[0].content.contains("import { MigrationInterface, QueryRunner }"));
    assert!(files[0].content.contains("export class"));
    assert!(files[0].content.contains("implements MigrationInterface"));
    assert!(files[0].content.contains("async up(queryRunner: QueryRunner)"));
    assert!(files[0].content.contains("async down(queryRunner: QueryRunner)"));
}

#[test]
fn test_migrate_field_added() {
    let schema = create_test_schema();
    let new_field = FieldDefinition {
        name: "name".to_string(),
        field_type: TypeExpression::Identifier {
            name: "string".to_string(),
        },
        optional: true,
        default: None,
        config: serde_json::json!({}),
        entity_id: None,
    };

    let deltas = vec![Delta::FieldAdded {
        model: "User".to_string(),
        field: "name".to_string(),
        after: new_field,
    }];

    let config = serde_json::json!({});
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);

    assert!(files[0].content.contains("ADD COLUMN"));
}

#[test]
fn test_migrate_field_removed() {
    let schema = create_test_schema();
    let email_field = schema.models.get("User").unwrap().fields[1].clone();

    let deltas = vec![Delta::FieldRemoved {
        model: "User".to_string(),
        field: "email".to_string(),
        before: email_field,
    }];

    let config = serde_json::json!({});
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);

    assert!(files[0].content.contains("DROP COLUMN"));
}

#[test]
fn test_migration_name_from_model_added() {
    let deltas = vec![Delta::ModelAdded {
        name: "Post".to_string(),
        after: ModelDefinition {
            name: "Post".to_string(),
            parents: vec![],
            fields: vec![],
            config: serde_json::json!({}),
            entity_id: None,
        },
    }];

    let name = generate_migration_name(&deltas);
    assert_eq!(name, "AddPost");
}

#[test]
fn test_migration_name_from_field_added() {
    let deltas = vec![Delta::FieldAdded {
        model: "User".to_string(),
        field: "avatar".to_string(),
        after: FieldDefinition {
            name: "avatar".to_string(),
            field_type: TypeExpression::Identifier {
                name: "string".to_string(),
            },
            optional: true,
            default: None,
            config: serde_json::json!({}),
            entity_id: None,
        },
    }];

    let name = generate_migration_name(&deltas);
    assert_eq!(name, "AddAvatarToUser");
}
