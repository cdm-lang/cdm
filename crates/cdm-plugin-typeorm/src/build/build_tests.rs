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

// ts_type tests

#[test]
fn test_field_ts_type_string() {
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
                name: "metadata".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "JSON".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({
                    "ts_type": "MyCustomType"
                }),
                entity_id: None,
            },
        ],
        config: serde_json::json!({}),
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
    assert!(content.contains("metadata!: MyCustomType"));
}

#[test]
fn test_field_ts_type_with_import() {
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
                name: "data".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "JSON".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({
                    "ts_type": {
                        "type": "CustomData",
                        "import": "./types/custom"
                    }
                }),
                entity_id: None,
            },
        ],
        config: serde_json::json!({}),
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
    assert!(content.contains("data!: CustomData"));
    assert!(content.contains("import { CustomData } from \"./types/custom\""));
}

#[test]
fn test_field_ts_type_with_default_import() {
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
                name: "config".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "JSON".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({
                    "ts_type": {
                        "type": "AppConfig",
                        "import": "./types/config",
                        "default": true
                    }
                }),
                entity_id: None,
            },
        ],
        config: serde_json::json!({}),
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
    assert!(content.contains("config!: AppConfig"));
    assert!(content.contains("import AppConfig from \"./types/config\""));
}

#[test]
fn test_type_alias_ts_type() {
    use cdm_plugin_interface::TypeAliasDefinition;

    let mut models = HashMap::new();
    let mut type_aliases = HashMap::new();

    // Create type alias with ts_type config
    type_aliases.insert(
        "Metadata".to_string(),
        TypeAliasDefinition {
            name: "Metadata".to_string(),
            alias_type: TypeExpression::Identifier {
                name: "JSON".to_string(),
            },
            config: serde_json::json!({
                "ts_type": {
                    "type": "MetadataType",
                    "import": "./types/metadata"
                }
            }),
            entity_id: None,
        },
    );

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
                name: "metadata".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "Metadata".to_string(),
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

    let schema = Schema {
        models,
        type_aliases,
    };
    let config = serde_json::json!({});
    let utils = Utils;

    let files = build(schema, config, &utils);

    let content = &files[0].content;
    assert!(content.contains("metadata!: MetadataType"));
    assert!(content.contains("import { MetadataType } from \"./types/metadata\""));
}

#[test]
fn test_field_ts_type_precedence_over_type_alias() {
    use cdm_plugin_interface::TypeAliasDefinition;

    let mut models = HashMap::new();
    let mut type_aliases = HashMap::new();

    // Create type alias with ts_type config
    type_aliases.insert(
        "Metadata".to_string(),
        TypeAliasDefinition {
            name: "Metadata".to_string(),
            alias_type: TypeExpression::Identifier {
                name: "JSON".to_string(),
            },
            config: serde_json::json!({
                "ts_type": {
                    "type": "AliasMetadataType",
                    "import": "./types/alias-metadata"
                }
            }),
            entity_id: None,
        },
    );

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
                name: "metadata".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "Metadata".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({
                    "ts_type": {
                        "type": "FieldMetadataType",
                        "import": "./types/field-metadata"
                    }
                }),
                entity_id: None,
            },
        ],
        config: serde_json::json!({}),
        entity_id: None,
    };
    models.insert("User".to_string(), user_model);

    let schema = Schema {
        models,
        type_aliases,
    };
    let config = serde_json::json!({});
    let utils = Utils;

    let files = build(schema, config, &utils);

    let content = &files[0].content;
    // Field-level ts_type should take precedence
    assert!(content.contains("metadata!: FieldMetadataType"));
    assert!(content.contains("import { FieldMetadataType } from \"./types/field-metadata\""));
    // Should NOT contain the type alias import
    assert!(!content.contains("AliasMetadataType"));
    assert!(!content.contains("alias-metadata"));
}

#[test]
fn test_ts_type_imports_grouped_by_path() {
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
                name: "metadata".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "JSON".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({
                    "ts_type": {
                        "type": "Metadata",
                        "import": "./types"
                    }
                }),
                entity_id: None,
            },
            FieldDefinition {
                name: "config".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "JSON".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({
                    "ts_type": {
                        "type": "Config",
                        "import": "./types"
                    }
                }),
                entity_id: None,
            },
        ],
        config: serde_json::json!({}),
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
    // Should have a single grouped import for both types
    assert!(content.contains("import { Config, Metadata } from \"./types\""));
}

#[test]
fn test_ts_type_mixed_default_and_named_imports() {
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
                name: "defaultType".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "JSON".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({
                    "ts_type": {
                        "type": "DefaultType",
                        "import": "./types/default",
                        "default": true
                    }
                }),
                entity_id: None,
            },
            FieldDefinition {
                name: "namedType".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "JSON".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({
                    "ts_type": {
                        "type": "NamedType",
                        "import": "./types/named"
                    }
                }),
                entity_id: None,
            },
        ],
        config: serde_json::json!({}),
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
    assert!(content.contains("import DefaultType from \"./types/default\""));
    assert!(content.contains("import { NamedType } from \"./types/named\""));
}

// Primary key type tests

#[test]
fn test_primary_key_with_type_override() {
    let mut models = HashMap::new();

    let user_model = ModelDefinition {
        name: "User".to_string(),
        parents: vec![],
        fields: vec![FieldDefinition {
            name: "id".to_string(),
            field_type: TypeExpression::Identifier {
                name: "number".to_string(),
            },
            optional: false,
            default: None,
            config: serde_json::json!({
                "primary": { "generation": "increment" },
                "type": "bigint"
            }),
            entity_id: None,
        }],
        config: serde_json::json!({}),
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
    assert!(
        content.contains("@PrimaryGeneratedColumn(\"increment\", { type: \"bigint\" })"),
        "Should include type option in PrimaryGeneratedColumn. Content: {}",
        content
    );
}

#[test]
fn test_primary_column_with_type_override() {
    let mut models = HashMap::new();

    let user_model = ModelDefinition {
        name: "User".to_string(),
        parents: vec![],
        fields: vec![FieldDefinition {
            name: "id".to_string(),
            field_type: TypeExpression::Identifier {
                name: "string".to_string(),
            },
            optional: false,
            default: None,
            config: serde_json::json!({
                "primary": {},
                "type": "uuid"
            }),
            entity_id: None,
        }],
        config: serde_json::json!({}),
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
    assert!(
        content.contains("@PrimaryColumn({ type: \"uuid\" })"),
        "Should include type option in PrimaryColumn. Content: {}",
        content
    );
}

// Field-level join_column tests

fn create_schema_with_relation(relation_config: serde_json::Value, join_column: Option<serde_json::Value>, join_table: Option<serde_json::Value>) -> Schema {
    let mut models = HashMap::new();

    // Create User model (target entity for relations)
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
        config: serde_json::json!({}),
        entity_id: None,
    };
    models.insert("User".to_string(), user_model);

    // Create Identity model with relation to User
    let mut field_config = serde_json::json!({
        "relation": relation_config
    });
    if let Some(jc) = join_column {
        field_config["join_column"] = jc;
    }
    if let Some(jt) = join_table {
        field_config["join_table"] = jt;
    }

    let identity_model = ModelDefinition {
        name: "Identity".to_string(),
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
                name: "user".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "User".to_string(),
                },
                optional: false,
                default: None,
                config: field_config,
                entity_id: None,
            },
        ],
        config: serde_json::json!({}),
        entity_id: None,
    };
    models.insert("Identity".to_string(), identity_model);

    Schema {
        models,
        type_aliases: HashMap::new(),
    }
}

#[test]
fn test_field_level_join_column() {
    let schema = create_schema_with_relation(
        serde_json::json!({
            "type": "many_to_one",
            "inverse_side": "identities",
            "on_delete": "CASCADE"
        }),
        Some(serde_json::json!({ "name": "user_id" })),
        None,
    );
    let config = serde_json::json!({});
    let utils = Utils;

    let files = build(schema, config, &utils);

    // Find the Identity.ts file
    let identity_file = files.iter().find(|f| f.path == "Identity.ts").expect("Identity.ts not found");
    let content = &identity_file.content;

    assert!(
        content.contains("@JoinColumn({ name: \"user_id\" })"),
        "Should include JoinColumn decorator. Content: {}",
        content
    );
    assert!(
        content.contains("JoinColumn"),
        "Should import JoinColumn. Content: {}",
        content
    );
}

#[test]
fn test_field_level_join_column_with_referenced_column() {
    let schema = create_schema_with_relation(
        serde_json::json!({
            "type": "many_to_one",
            "inverse_side": "identities"
        }),
        Some(serde_json::json!({
            "name": "user_id",
            "referenced_column": "uuid"
        })),
        None,
    );
    let config = serde_json::json!({});
    let utils = Utils;

    let files = build(schema, config, &utils);

    let identity_file = files.iter().find(|f| f.path == "Identity.ts").expect("Identity.ts not found");
    let content = &identity_file.content;

    assert!(
        content.contains("@JoinColumn({ name: \"user_id\", referencedColumnName: \"uuid\" })"),
        "Should include JoinColumn with referencedColumnName. Content: {}",
        content
    );
}

#[test]
fn test_field_level_join_column_precedence_over_nested() {
    // Both field-level and nested join_column specified - field-level should win
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
        config: serde_json::json!({}),
        entity_id: None,
    };
    models.insert("User".to_string(), user_model);

    let identity_model = ModelDefinition {
        name: "Identity".to_string(),
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
                name: "user".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "User".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({
                    "relation": {
                        "type": "many_to_one",
                        "inverse_side": "identities",
                        "join_column": { "name": "nested_user_id" }
                    },
                    "join_column": { "name": "field_level_user_id" }
                }),
                entity_id: None,
            },
        ],
        config: serde_json::json!({}),
        entity_id: None,
    };
    models.insert("Identity".to_string(), identity_model);

    let schema = Schema {
        models,
        type_aliases: HashMap::new(),
    };
    let config = serde_json::json!({});
    let utils = Utils;

    let files = build(schema, config, &utils);

    let identity_file = files.iter().find(|f| f.path == "Identity.ts").expect("Identity.ts not found");
    let content = &identity_file.content;

    // Field-level should take precedence
    assert!(
        content.contains("@JoinColumn({ name: \"field_level_user_id\" })"),
        "Field-level join_column should take precedence. Content: {}",
        content
    );
    assert!(
        !content.contains("nested_user_id"),
        "Nested join_column should not appear. Content: {}",
        content
    );
}

#[test]
fn test_field_level_join_table() {
    // Create schema with ManyToMany relation and field-level join_table
    let mut models = HashMap::new();

    let tag_model = ModelDefinition {
        name: "Tag".to_string(),
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
        config: serde_json::json!({}),
        entity_id: None,
    };
    models.insert("Tag".to_string(), tag_model);

    let post_model = ModelDefinition {
        name: "Post".to_string(),
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
                name: "tags".to_string(),
                field_type: TypeExpression::Array {
                    element_type: Box::new(TypeExpression::Identifier {
                        name: "Tag".to_string(),
                    }),
                },
                optional: false,
                default: None,
                config: serde_json::json!({
                    "relation": {
                        "type": "many_to_many",
                        "inverse_side": "posts"
                    },
                    "join_table": {
                        "name": "post_tags",
                        "join_column": { "name": "post_id" },
                        "inverse_join_column": { "name": "tag_id" }
                    }
                }),
                entity_id: None,
            },
        ],
        config: serde_json::json!({}),
        entity_id: None,
    };
    models.insert("Post".to_string(), post_model);

    let schema = Schema {
        models,
        type_aliases: HashMap::new(),
    };
    let config = serde_json::json!({});
    let utils = Utils;

    let files = build(schema, config, &utils);

    let post_file = files.iter().find(|f| f.path == "Post.ts").expect("Post.ts not found");
    let content = &post_file.content;

    assert!(
        content.contains("@JoinTable({"),
        "Should include JoinTable decorator. Content: {}",
        content
    );
    assert!(
        content.contains("name: \"post_tags\""),
        "Should include table name. Content: {}",
        content
    );
    assert!(
        content.contains("JoinTable"),
        "Should import JoinTable. Content: {}",
        content
    );
}

#[test]
fn test_nested_join_column_still_works() {
    // Ensure backward compatibility - nested join_column inside relation still works
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
        config: serde_json::json!({}),
        entity_id: None,
    };
    models.insert("User".to_string(), user_model);

    let identity_model = ModelDefinition {
        name: "Identity".to_string(),
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
                name: "user".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "User".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({
                    "relation": {
                        "type": "many_to_one",
                        "inverse_side": "identities",
                        "join_column": { "name": "legacy_user_id" }
                    }
                }),
                entity_id: None,
            },
        ],
        config: serde_json::json!({}),
        entity_id: None,
    };
    models.insert("Identity".to_string(), identity_model);

    let schema = Schema {
        models,
        type_aliases: HashMap::new(),
    };
    let config = serde_json::json!({});
    let utils = Utils;

    let files = build(schema, config, &utils);

    let identity_file = files.iter().find(|f| f.path == "Identity.ts").expect("Identity.ts not found");
    let content = &identity_file.content;

    // Nested join_column should still work for backward compatibility
    assert!(
        content.contains("@JoinColumn({ name: \"legacy_user_id\" })"),
        "Nested join_column should still work. Content: {}",
        content
    );
}

// Table name override tests

#[test]
fn test_table_name_override() {
    // Test that @typeorm { table_name: "custom_table" } works (consistent with SQL plugin)
    // Use a completely custom name to ensure we're reading table_name, not just pluralizing
    let mut models = HashMap::new();

    let repo_model = ModelDefinition {
        name: "Repo".to_string(),
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
            "table_name": "project_repos"  // Custom name that differs from pluralized "repos"
        }),
        entity_id: None,
    };
    models.insert("Repo".to_string(), repo_model);

    let schema = Schema {
        models,
        type_aliases: HashMap::new(),
    };
    let config = serde_json::json!({
        "pluralize_table_names": true
    });
    let utils = Utils;

    let files = build(schema, config, &utils);

    let content = &files[0].content;
    // Without the fix, this would generate "repos" (pluralized model name)
    // With table_name support, it should use "project_repos"
    assert!(
        content.contains("@Entity({ name: \"project_repos\" })"),
        "Should use custom table name from 'table_name' field. Content: {}",
        content
    );
}

#[test]
fn test_table_name_override_bypasses_pluralization() {
    // Test that table_name completely bypasses the pluralization logic
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
            "table_name": "app_users"
        }),
        entity_id: None,
    };
    models.insert("User".to_string(), user_model);

    let schema = Schema {
        models,
        type_aliases: HashMap::new(),
    };
    let config = serde_json::json!({
        "pluralize_table_names": true
    });
    let utils = Utils;

    let files = build(schema, config, &utils);

    let content = &files[0].content;
    assert!(
        content.contains("@Entity({ name: \"app_users\" })"),
        "Should use custom table name from 'table_name' field. Content: {}",
        content
    );
}

// Definite assignment tests

#[test]
fn test_definite_assignment_default_behavior() {
    // By default, non-optional fields should have ! (definite assignment assertion)
    let schema = create_test_schema();
    let config = serde_json::json!({});
    let utils = Utils;

    let files = build(schema, config, &utils);

    let content = &files[0].content;
    // email is non-optional, should have !
    assert!(
        content.contains("email!: string"),
        "Non-optional field should have definite assignment assertion. Content: {}",
        content
    );
    // id is non-optional primary key, should have !
    assert!(
        content.contains("id!: string"),
        "Non-optional primary key should have definite assignment assertion. Content: {}",
        content
    );
}

#[test]
fn test_definite_assignment_optional_fields_unchanged() {
    // Optional fields should have ? not !
    let schema = create_test_schema();
    let config = serde_json::json!({});
    let utils = Utils;

    let files = build(schema, config, &utils);

    let content = &files[0].content;
    // name is optional, should have ? not !
    assert!(
        content.contains("name?: string"),
        "Optional field should have ? marker, not !. Content: {}",
        content
    );
    assert!(
        !content.contains("name!:"),
        "Optional field should NOT have definite assignment assertion. Content: {}",
        content
    );
}

#[test]
fn test_definite_assignment_global_false() {
    // When global definite_assignment is false, non-optional fields should NOT have !
    let schema = create_test_schema();
    let config = serde_json::json!({
        "definite_assignment": false
    });
    let utils = Utils;

    let files = build(schema, config, &utils);

    let content = &files[0].content;
    // email is non-optional but definite_assignment is false, should NOT have !
    assert!(
        content.contains("email: string"),
        "Non-optional field should NOT have ! when definite_assignment is false. Content: {}",
        content
    );
    assert!(
        !content.contains("email!:"),
        "Non-optional field should NOT have ! when definite_assignment is false. Content: {}",
        content
    );
}

#[test]
fn test_definite_assignment_model_level_override_false() {
    // Model-level definite_assignment: false should override global default (true)
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
            "definite_assignment": false
        }),
        entity_id: None,
    };
    models.insert("User".to_string(), user_model);

    let schema = Schema {
        models,
        type_aliases: HashMap::new(),
    };
    let config = serde_json::json!({});  // Global default is true
    let utils = Utils;

    let files = build(schema, config, &utils);

    let content = &files[0].content;
    // Model-level false should override global true
    assert!(
        content.contains("email: string"),
        "Model-level definite_assignment: false should override global. Content: {}",
        content
    );
    assert!(
        !content.contains("email!:"),
        "Model-level definite_assignment: false should override global. Content: {}",
        content
    );
}

#[test]
fn test_definite_assignment_field_level_override_false() {
    // Field-level definite_assignment: false should override model/global
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
                config: serde_json::json!({
                    "definite_assignment": false
                }),
                entity_id: None,
            },
            FieldDefinition {
                name: "name".to_string(),
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

    let schema = Schema {
        models,
        type_aliases: HashMap::new(),
    };
    let config = serde_json::json!({});  // Global default is true
    let utils = Utils;

    let files = build(schema, config, &utils);

    let content = &files[0].content;
    // Field-level false should override global true
    assert!(
        content.contains("email: string"),
        "Field-level definite_assignment: false should override global. Content: {}",
        content
    );
    // name should still have ! (uses global default)
    assert!(
        content.contains("name!: string"),
        "Field without override should use global setting. Content: {}",
        content
    );
}

#[test]
fn test_definite_assignment_field_level_override_true() {
    // Field-level definite_assignment: true should override model-level false
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
                config: serde_json::json!({
                    "definite_assignment": true
                }),
                entity_id: None,
            },
            FieldDefinition {
                name: "name".to_string(),
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
            "definite_assignment": false
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
    // Field-level true should override model-level false
    assert!(
        content.contains("email!: string"),
        "Field-level definite_assignment: true should override model-level false. Content: {}",
        content
    );
    // name should NOT have ! (uses model-level false)
    assert!(
        content.contains("name: string"),
        "Field without override should use model setting. Content: {}",
        content
    );
    assert!(
        !content.contains("name!:"),
        "Field without override should use model setting. Content: {}",
        content
    );
}

#[test]
fn test_definite_assignment_relation_fields() {
    // Relation fields should also respect definite_assignment
    let schema = create_schema_with_relation(
        serde_json::json!({
            "type": "many_to_one",
            "inverse_side": "identities"
        }),
        None,
        None,
    );
    let config = serde_json::json!({});
    let utils = Utils;

    let files = build(schema, config, &utils);

    let identity_file = files.iter().find(|f| f.path == "Identity.ts").expect("Identity.ts not found");
    let content = &identity_file.content;

    // user is non-optional relation, should have !
    assert!(
        content.contains("user!: User"),
        "Non-optional relation field should have definite assignment assertion. Content: {}",
        content
    );
}

// Date column decorator tests

fn create_test_schema_with_date_columns() -> Schema {
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
                name: "createdAt".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({ "create_date": true }),
                entity_id: None,
            },
            FieldDefinition {
                name: "updatedAt".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({ "update_date": true }),
                entity_id: None,
            },
            FieldDefinition {
                name: "deletedAt".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: true,
                default: None,
                config: serde_json::json!({ "delete_date": true }),
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
fn test_build_generates_create_date_column() {
    let schema = create_test_schema_with_date_columns();
    let config = serde_json::json!({});
    let utils = Utils;
    let files = build(schema, config, &utils);

    let content = &files[0].content;
    assert!(
        content.contains("@CreateDateColumn()"),
        "Should generate @CreateDateColumn decorator. Content: {}",
        content
    );
}

#[test]
fn test_build_generates_update_date_column() {
    let schema = create_test_schema_with_date_columns();
    let config = serde_json::json!({});
    let utils = Utils;
    let files = build(schema, config, &utils);

    let content = &files[0].content;
    assert!(
        content.contains("@UpdateDateColumn()"),
        "Should generate @UpdateDateColumn decorator. Content: {}",
        content
    );
}

#[test]
fn test_build_generates_delete_date_column() {
    let schema = create_test_schema_with_date_columns();
    let config = serde_json::json!({});
    let utils = Utils;
    let files = build(schema, config, &utils);

    let content = &files[0].content;
    assert!(
        content.contains("@DeleteDateColumn({ nullable: true })"),
        "Should generate @DeleteDateColumn decorator with nullable. Content: {}",
        content
    );
}

#[test]
fn test_build_date_column_imports() {
    let schema = create_test_schema_with_date_columns();
    let config = serde_json::json!({});
    let utils = Utils;
    let files = build(schema, config, &utils);

    let content = &files[0].content;
    assert!(
        content.contains("CreateDateColumn"),
        "Should import CreateDateColumn. Content: {}",
        content
    );
    assert!(
        content.contains("UpdateDateColumn"),
        "Should import UpdateDateColumn. Content: {}",
        content
    );
    assert!(
        content.contains("DeleteDateColumn"),
        "Should import DeleteDateColumn. Content: {}",
        content
    );
    assert!(
        content.contains("from \"typeorm\""),
        "Should import from typeorm. Content: {}",
        content
    );
}

#[test]
fn test_build_date_column_does_not_use_column_decorator() {
    let schema = create_test_schema_with_date_columns();
    let config = serde_json::json!({});
    let utils = Utils;
    let files = build(schema, config, &utils);

    let content = &files[0].content;
    // The only @Column-like decorators should be the date-specific ones
    // The import line should NOT include plain "Column"
    assert!(
        !content.contains("import { Column"),
        "Should NOT import plain Column when only date columns are used. Content: {}",
        content
    );
}

#[test]
fn test_build_create_date_column_with_type_override() {
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
                name: "createdAt".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({
                    "create_date": true,
                    "type": "timestamp with time zone"
                }),
                entity_id: None,
            },
        ],
        config: serde_json::json!({}),
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
    assert!(
        content.contains("@CreateDateColumn({ type: \"timestamp with time zone\" })"),
        "Should include type option in CreateDateColumn. Content: {}",
        content
    );
}

#[test]
fn test_build_delete_date_column_optional() {
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
                name: "deletedAt".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: true,
                default: None,
                config: serde_json::json!({ "delete_date": true }),
                entity_id: None,
            },
        ],
        config: serde_json::json!({}),
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
    assert!(
        content.contains("@DeleteDateColumn({ nullable: true })"),
        "Optional delete date field should have nullable. Content: {}",
        content
    );
    assert!(
        content.contains("deletedAt?:"),
        "Optional delete date field should have ? marker. Content: {}",
        content
    );
}

#[test]
fn test_build_create_date_false_uses_regular_column() {
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
                name: "createdAt".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({ "create_date": false }),
                entity_id: None,
            },
        ],
        config: serde_json::json!({}),
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
    assert!(
        !content.contains("@CreateDateColumn"),
        "create_date: false should NOT generate @CreateDateColumn. Content: {}",
        content
    );
    assert!(
        content.contains("@Column()"),
        "create_date: false should fall through to regular @Column. Content: {}",
        content
    );
}
