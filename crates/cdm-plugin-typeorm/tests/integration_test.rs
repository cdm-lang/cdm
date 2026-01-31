/// Integration tests for the TypeORM plugin ts_type feature
/// These tests verify the complete flow from schema to generated TypeScript
use cdm_plugin_interface::{
    FieldDefinition, ModelDefinition, Schema, TypeAliasDefinition, TypeExpression, Utils,
};
use serde_json::json;
use std::collections::HashMap;

use cdm_plugin_typeorm::build;

// ============================================================================
// ts_type String Format Tests
// ============================================================================

#[test]
fn test_ts_type_string_generates_custom_type() {
    let schema = create_schema_with_field_ts_type_string();
    let config = json!({});
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    assert_eq!(outputs.len(), 1);
    let content = &outputs[0].content;

    // Should use custom type instead of default mapping
    assert!(
        content.contains("metadata!: CustomMetadata"),
        "Should generate custom type. Content: {}",
        content
    );
    // Should NOT have any import for string-only ts_type
    assert!(
        !content.contains("import CustomMetadata"),
        "Should not generate import for string ts_type"
    );
}

// ============================================================================
// ts_type Object Format Tests (Named Import)
// ============================================================================

#[test]
fn test_ts_type_object_generates_named_import() {
    let schema = create_schema_with_field_ts_type_object();
    let config = json!({});
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    assert_eq!(outputs.len(), 1);
    let content = &outputs[0].content;

    // Should use custom type
    assert!(
        content.contains("data!: CustomData"),
        "Should generate custom type. Content: {}",
        content
    );
    // Should generate named import
    assert!(
        content.contains("import { CustomData } from \"./types/custom\""),
        "Should generate named import. Content: {}",
        content
    );
}

// ============================================================================
// ts_type Object Format Tests (Default Import)
// ============================================================================

#[test]
fn test_ts_type_object_generates_default_import() {
    let schema = create_schema_with_field_ts_type_default_import();
    let config = json!({});
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    assert_eq!(outputs.len(), 1);
    let content = &outputs[0].content;

    // Should use custom type
    assert!(
        content.contains("config!: AppConfig"),
        "Should generate custom type. Content: {}",
        content
    );
    // Should generate default import
    assert!(
        content.contains("import AppConfig from \"./config\""),
        "Should generate default import. Content: {}",
        content
    );
}

// ============================================================================
// Type Alias ts_type Tests
// ============================================================================

#[test]
fn test_type_alias_ts_type_applies_to_fields() {
    let schema = create_schema_with_type_alias_ts_type();
    let config = json!({});
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    assert_eq!(outputs.len(), 1);
    let content = &outputs[0].content;

    // Should use type from type alias config
    assert!(
        content.contains("metadata!: AliasType"),
        "Should use type alias ts_type. Content: {}",
        content
    );
    // Should generate import from type alias config
    assert!(
        content.contains("import { AliasType } from \"./types/alias\""),
        "Should generate import from type alias. Content: {}",
        content
    );
}

// ============================================================================
// Precedence Tests
// ============================================================================

#[test]
fn test_field_ts_type_takes_precedence_over_type_alias() {
    let schema = create_schema_with_both_field_and_alias_ts_type();
    let config = json!({});
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    assert_eq!(outputs.len(), 1);
    let content = &outputs[0].content;

    // Field-level should win
    assert!(
        content.contains("metadata!: FieldOverride"),
        "Field ts_type should take precedence. Content: {}",
        content
    );
    assert!(
        content.contains("import { FieldOverride } from \"./types/field\""),
        "Should import from field config. Content: {}",
        content
    );
    // Type alias import should NOT be present
    assert!(
        !content.contains("AliasType"),
        "Type alias ts_type should be overridden. Content: {}",
        content
    );
    assert!(
        !content.contains("./types/alias"),
        "Type alias import should not be present. Content: {}",
        content
    );
}

// ============================================================================
// Import Grouping Tests
// ============================================================================

#[test]
fn test_multiple_imports_from_same_path_are_grouped() {
    let schema = create_schema_with_multiple_imports_same_path();
    let config = json!({});
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    assert_eq!(outputs.len(), 1);
    let content = &outputs[0].content;

    // Should have single grouped import
    assert!(
        content.contains("import { TypeA, TypeB } from \"./types\""),
        "Should group imports from same path. Content: {}",
        content
    );
    // Should NOT have separate imports
    let import_count = content.matches("from \"./types\"").count();
    assert_eq!(
        import_count, 1,
        "Should have exactly one import from ./types. Content: {}",
        content
    );
}

#[test]
fn test_default_and_named_imports_are_separate() {
    let schema = create_schema_with_mixed_import_types();
    let config = json!({});
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    assert_eq!(outputs.len(), 1);
    let content = &outputs[0].content;

    // Should have default import
    assert!(
        content.contains("import DefaultType from \"./types/default\""),
        "Should have default import. Content: {}",
        content
    );
    // Should have named import
    assert!(
        content.contains("import { NamedType } from \"./types/named\""),
        "Should have named import. Content: {}",
        content
    );
}

// ============================================================================
// Integration with Other Features
// ============================================================================

#[test]
fn test_ts_type_works_with_relations() {
    let schema = create_schema_with_relation_and_ts_type();
    let config = json!({});
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    // Should have both User and Post files
    assert_eq!(outputs.len(), 2);

    let user_file = outputs.iter().find(|f| f.path == "User.ts").unwrap();
    let content = &user_file.content;

    // Regular field with ts_type should work
    assert!(
        content.contains("metadata!: UserMeta"),
        "Should apply ts_type to regular field. Content: {}",
        content
    );
    // Relation should still work normally
    assert!(
        content.contains("posts!: Post[]"),
        "Relation should work. Content: {}",
        content
    );
}

#[test]
fn test_ts_type_works_with_primary_key() {
    let schema = create_schema_with_primary_and_ts_type();
    let config = json!({});
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    assert_eq!(outputs.len(), 1);
    let content = &outputs[0].content;

    // Primary key should have custom type
    assert!(
        content.contains("id!: CustomId"),
        "Should apply ts_type to primary key. Content: {}",
        content
    );
    assert!(
        content.contains("@PrimaryGeneratedColumn"),
        "Should still have primary decorator. Content: {}",
        content
    );
}

// ============================================================================
// Helper Functions
// ============================================================================

fn create_schema_with_field_ts_type_string() -> Schema {
    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            fields: vec![
                create_primary_field("id"),
                FieldDefinition {
                    name: "metadata".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "JSON".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({
                        "ts_type": "CustomMetadata"
                    }),
                    entity_id: None,
                },
            ],
            parents: vec![],
            config: json!({}),
            entity_id: None,
        },
    );

    Schema {
        models,
        type_aliases: HashMap::new(),
    }
}

fn create_schema_with_field_ts_type_object() -> Schema {
    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            fields: vec![
                create_primary_field("id"),
                FieldDefinition {
                    name: "data".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "JSON".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({
                        "ts_type": {
                            "type": "CustomData",
                            "import": "./types/custom"
                        }
                    }),
                    entity_id: None,
                },
            ],
            parents: vec![],
            config: json!({}),
            entity_id: None,
        },
    );

    Schema {
        models,
        type_aliases: HashMap::new(),
    }
}

fn create_schema_with_field_ts_type_default_import() -> Schema {
    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            fields: vec![
                create_primary_field("id"),
                FieldDefinition {
                    name: "config".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "JSON".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({
                        "ts_type": {
                            "type": "AppConfig",
                            "import": "./config",
                            "default": true
                        }
                    }),
                    entity_id: None,
                },
            ],
            parents: vec![],
            config: json!({}),
            entity_id: None,
        },
    );

    Schema {
        models,
        type_aliases: HashMap::new(),
    }
}

fn create_schema_with_type_alias_ts_type() -> Schema {
    let mut models = HashMap::new();
    let mut type_aliases = HashMap::new();

    type_aliases.insert(
        "Metadata".to_string(),
        TypeAliasDefinition {
            name: "Metadata".to_string(),
            alias_type: TypeExpression::Identifier {
                name: "JSON".to_string(),
            },
            config: json!({
                "ts_type": {
                    "type": "AliasType",
                    "import": "./types/alias"
                }
            }),
            entity_id: None,
        },
    );

    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            fields: vec![
                create_primary_field("id"),
                FieldDefinition {
                    name: "metadata".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "Metadata".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: None,
                },
            ],
            parents: vec![],
            config: json!({}),
            entity_id: None,
        },
    );

    Schema {
        models,
        type_aliases,
    }
}

fn create_schema_with_both_field_and_alias_ts_type() -> Schema {
    let mut models = HashMap::new();
    let mut type_aliases = HashMap::new();

    type_aliases.insert(
        "Metadata".to_string(),
        TypeAliasDefinition {
            name: "Metadata".to_string(),
            alias_type: TypeExpression::Identifier {
                name: "JSON".to_string(),
            },
            config: json!({
                "ts_type": {
                    "type": "AliasType",
                    "import": "./types/alias"
                }
            }),
            entity_id: None,
        },
    );

    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            fields: vec![
                create_primary_field("id"),
                FieldDefinition {
                    name: "metadata".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "Metadata".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({
                        "ts_type": {
                            "type": "FieldOverride",
                            "import": "./types/field"
                        }
                    }),
                    entity_id: None,
                },
            ],
            parents: vec![],
            config: json!({}),
            entity_id: None,
        },
    );

    Schema {
        models,
        type_aliases,
    }
}

fn create_schema_with_multiple_imports_same_path() -> Schema {
    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            fields: vec![
                create_primary_field("id"),
                FieldDefinition {
                    name: "fieldA".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "JSON".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({
                        "ts_type": {
                            "type": "TypeA",
                            "import": "./types"
                        }
                    }),
                    entity_id: None,
                },
                FieldDefinition {
                    name: "fieldB".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "JSON".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({
                        "ts_type": {
                            "type": "TypeB",
                            "import": "./types"
                        }
                    }),
                    entity_id: None,
                },
            ],
            parents: vec![],
            config: json!({}),
            entity_id: None,
        },
    );

    Schema {
        models,
        type_aliases: HashMap::new(),
    }
}

fn create_schema_with_mixed_import_types() -> Schema {
    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            fields: vec![
                create_primary_field("id"),
                FieldDefinition {
                    name: "defaultField".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "JSON".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({
                        "ts_type": {
                            "type": "DefaultType",
                            "import": "./types/default",
                            "default": true
                        }
                    }),
                    entity_id: None,
                },
                FieldDefinition {
                    name: "namedField".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "JSON".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({
                        "ts_type": {
                            "type": "NamedType",
                            "import": "./types/named"
                        }
                    }),
                    entity_id: None,
                },
            ],
            parents: vec![],
            config: json!({}),
            entity_id: None,
        },
    );

    Schema {
        models,
        type_aliases: HashMap::new(),
    }
}

fn create_schema_with_relation_and_ts_type() -> Schema {
    let mut models = HashMap::new();

    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            fields: vec![
                create_primary_field("id"),
                FieldDefinition {
                    name: "metadata".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "JSON".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({
                        "ts_type": {
                            "type": "UserMeta",
                            "import": "./types/user"
                        }
                    }),
                    entity_id: None,
                },
                FieldDefinition {
                    name: "posts".to_string(),
                    field_type: TypeExpression::Array {
                        element_type: Box::new(TypeExpression::Identifier {
                            name: "Post".to_string(),
                        }),
                    },
                    optional: false,
                    default: None,
                    config: json!({
                        "relation": {
                            "type": "one_to_many",
                            "inverse_side": "author"
                        }
                    }),
                    entity_id: None,
                },
            ],
            parents: vec![],
            config: json!({}),
            entity_id: None,
        },
    );

    models.insert(
        "Post".to_string(),
        ModelDefinition {
            name: "Post".to_string(),
            fields: vec![
                create_primary_field("id"),
                FieldDefinition {
                    name: "author".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "User".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({
                        "relation": {
                            "type": "many_to_one",
                            "inverse_side": "posts"
                        }
                    }),
                    entity_id: None,
                },
            ],
            parents: vec![],
            config: json!({}),
            entity_id: None,
        },
    );

    Schema {
        models,
        type_aliases: HashMap::new(),
    }
}

fn create_schema_with_primary_and_ts_type() -> Schema {
    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            fields: vec![FieldDefinition {
                name: "id".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: json!({
                    "primary": { "generation": "uuid" },
                    "ts_type": {
                        "type": "CustomId",
                        "import": "./types/id"
                    }
                }),
                entity_id: None,
            }],
            parents: vec![],
            config: json!({}),
            entity_id: None,
        },
    );

    Schema {
        models,
        type_aliases: HashMap::new(),
    }
}

fn create_primary_field(name: &str) -> FieldDefinition {
    FieldDefinition {
        name: name.to_string(),
        field_type: TypeExpression::Identifier {
            name: "string".to_string(),
        },
        optional: false,
        default: None,
        config: json!({
            "primary": { "generation": "uuid" }
        }),
        entity_id: None,
    }
}
