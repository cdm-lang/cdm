use super::*;
use cdm_plugin_interface::{FieldDefinition, ModelDefinition, TypeExpression};
use std::collections::HashMap;

#[test]
fn test_build_empty_schema() {
    let schema = Schema {
        type_aliases: HashMap::new(),
        models: HashMap::new(),
    };

    let config = serde_json::json!({});
    let utils = Utils;

    let files = build(schema, config, &utils);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "schema.json");
}

#[test]
fn test_build_single_model() {
    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "id".to_string(),
                    field_type: TypeExpression::Identifier { name: "string".to_string() },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: None,
                },
                FieldDefinition {
                    name: "name".to_string(),
                    field_type: TypeExpression::Identifier { name: "string".to_string() },
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

    let schema = Schema {
        type_aliases: HashMap::new(),
        models,
    };

    let config = serde_json::json!({});
    let utils = Utils;

    let files = build(schema, config, &utils);
    assert_eq!(files.len(), 1);

    let content: Value = serde_json::from_str(&files[0].content).unwrap();
    assert!(content.get("properties").is_some());
}
