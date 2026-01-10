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

// Hooks tests

fn create_test_schema_with_hooks() -> Schema {
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
        config: serde_json::json!({
            "hooks": {
                "before_insert": "setDefaults",
                "after_load": "computeFields"
            }
        }),
        entity_id: None,
    };
    models.insert("User".to_string(), user_model);

    Schema {
        models,
        type_aliases: HashMap::new(),
    }
}

#[test]
fn test_build_generates_hooks() {
    let schema = create_test_schema_with_hooks();
    let config = serde_json::json!({});
    let utils = Utils;

    let files = build(schema, config, &utils);

    assert_eq!(files.len(), 1);
    assert!(files[0].content.contains("@BeforeInsert()"));
    assert!(files[0].content.contains("setDefaults()"));
    assert!(files[0].content.contains("@AfterLoad()"));
    assert!(files[0].content.contains("computeFields()"));
}

#[test]
fn test_build_generates_hook_method_body() {
    let schema = create_test_schema_with_hooks();
    let config = serde_json::json!({});
    let utils = Utils;

    let files = build(schema, config, &utils);

    // Check that method bodies are generated
    assert!(files[0].content.contains("// Implementation required"));
}

#[test]
fn test_build_generates_hook_imports() {
    let schema = create_test_schema_with_hooks();
    let config = serde_json::json!({});
    let utils = Utils;

    let files = build(schema, config, &utils);

    assert!(files[0].content.contains("BeforeInsert"));
    assert!(files[0].content.contains("AfterLoad"));
    assert!(files[0].content.contains("from \"typeorm\""));
}

#[test]
fn test_build_generates_all_hook_types() {
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
        ],
        config: serde_json::json!({
            "hooks": {
                "before_insert": "beforeInsertHandler",
                "after_insert": "afterInsertHandler",
                "before_update": "beforeUpdateHandler",
                "after_update": "afterUpdateHandler",
                "before_remove": "beforeRemoveHandler",
                "after_remove": "afterRemoveHandler",
                "after_load": "afterLoadHandler",
                "before_soft_remove": "beforeSoftRemoveHandler",
                "after_soft_remove": "afterSoftRemoveHandler",
                "after_recover": "afterRecoverHandler"
            }
        }),
        entity_id: None,
    };
    models.insert("User".to_string(), user_model);

    let schema = Schema {
        models,
        type_aliases: HashMap::new(),
    };
    let config = serde_json::json!({});
    let utils = Utils;

    let files = build(schema, config, &utils);

    assert!(files[0].content.contains("@BeforeInsert()"));
    assert!(files[0].content.contains("@AfterInsert()"));
    assert!(files[0].content.contains("@BeforeUpdate()"));
    assert!(files[0].content.contains("@AfterUpdate()"));
    assert!(files[0].content.contains("@BeforeRemove()"));
    assert!(files[0].content.contains("@AfterRemove()"));
    assert!(files[0].content.contains("@AfterLoad()"));
    assert!(files[0].content.contains("@BeforeSoftRemove()"));
    assert!(files[0].content.contains("@AfterSoftRemove()"));
    assert!(files[0].content.contains("@AfterRecover()"));
}

#[test]
fn test_build_hooks_appear_after_fields() {
    let schema = create_test_schema_with_hooks();
    let config = serde_json::json!({});
    let utils = Utils;

    let files = build(schema, config, &utils);

    let content = &files[0].content;
    let email_pos = content.find("email").unwrap();
    let hook_pos = content.find("@BeforeInsert").unwrap();

    // Hook should appear after the email field
    assert!(hook_pos > email_pos);
}

// Hook with import tests

fn create_test_schema_with_hook_imports() -> Schema {
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
        ],
        config: serde_json::json!({
            "hooks": {
                "before_insert": {
                    "method": "setDefaults",
                    "import": "./hooks/userHooks"
                },
                "after_load": {
                    "method": "computeFields",
                    "import": "./hooks/compute"
                }
            }
        }),
        entity_id: None,
    };
    models.insert("User".to_string(), user_model);

    Schema {
        models,
        type_aliases: HashMap::new(),
    }
}

#[test]
fn test_build_generates_hook_with_import() {
    let schema = create_test_schema_with_hook_imports();
    let config = serde_json::json!({});
    let utils = Utils;

    let files = build(schema, config, &utils);

    assert_eq!(files.len(), 1);
    let content = &files[0].content;

    // Check that hooks delegate to imported functions
    assert!(content.contains("setDefaults.call(this)"));
    assert!(content.contains("computeFields.call(this)"));
}

#[test]
fn test_build_generates_hook_function_imports() {
    let schema = create_test_schema_with_hook_imports();
    let config = serde_json::json!({});
    let utils = Utils;

    let files = build(schema, config, &utils);

    let content = &files[0].content;

    // Check that function imports are generated
    assert!(content.contains("import { setDefaults } from \"./hooks/userHooks\""));
    assert!(content.contains("import { computeFields } from \"./hooks/compute\""));
}

#[test]
fn test_build_generates_mixed_hook_formats() {
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
        ],
        config: serde_json::json!({
            "hooks": {
                "before_insert": "stubMethod",
                "after_load": {
                    "method": "computeFields",
                    "import": "./hooks/compute"
                }
            }
        }),
        entity_id: None,
    };
    models.insert("User".to_string(), user_model);

    let schema = Schema {
        models,
        type_aliases: HashMap::new(),
    };
    let config = serde_json::json!({});
    let utils = Utils;

    let files = build(schema, config, &utils);

    let content = &files[0].content;

    // Stub method should have "Implementation required" comment
    assert!(content.contains("stubMethod()"));
    assert!(content.contains("// Implementation required"));

    // Imported method should call the function
    assert!(content.contains("computeFields.call(this)"));
    assert!(content.contains("import { computeFields } from \"./hooks/compute\""));
}

#[test]
fn test_build_groups_imports_by_path() {
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
        ],
        config: serde_json::json!({
            "hooks": {
                "before_insert": {
                    "method": "setDefaults",
                    "import": "./hooks"
                },
                "after_load": {
                    "method": "computeFields",
                    "import": "./hooks"
                }
            }
        }),
        entity_id: None,
    };
    models.insert("User".to_string(), user_model);

    let schema = Schema {
        models,
        type_aliases: HashMap::new(),
    };
    let config = serde_json::json!({});
    let utils = Utils;

    let files = build(schema, config, &utils);

    let content = &files[0].content;

    // Should group imports from same path
    assert!(content.contains("import { computeFields, setDefaults } from \"./hooks\""));
}
