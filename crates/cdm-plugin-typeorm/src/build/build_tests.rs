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
    assert!(content.contains("metadata: MyCustomType"));
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
    assert!(content.contains("data: CustomData"));
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
    assert!(content.contains("config: AppConfig"));
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
    assert!(content.contains("metadata: MetadataType"));
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
    assert!(content.contains("metadata: FieldMetadataType"));
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
