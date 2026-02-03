use super::*;
use cdm_plugin_interface::{EntityId, FieldDefinition, ModelDefinition};
use std::collections::HashMap;

fn local_id(id: u64) -> Option<EntityId> {
    Some(EntityId::local(id))
}

#[test]
fn test_migrate_empty() {
    let schema = Schema {
        type_aliases: HashMap::new(),
        models: HashMap::new(),
    };

    let config = serde_json::json!({});
    let utils = Utils;

    let files = migrate(schema, vec![], config, &utils);
    assert!(files.is_empty());
}

#[test]
fn test_migrate_first_time_migration_with_models() {
    // BUG TEST: When this is the first migration (no previous schema),
    // the deltas should contain ModelAdded for all current models,
    // and the SQL plugin should generate CREATE TABLE statements.

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "id".to_string(),
                    field_type: cdm_plugin_interface::TypeExpression::Identifier {
                        name: "number".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: serde_json::json!({}),
                    entity_id: local_id(2),
                },
                FieldDefinition {
                    name: "email".to_string(),
                    field_type: cdm_plugin_interface::TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: serde_json::json!({}),
                    entity_id: local_id(3),
                },
            ],
            config: serde_json::json!({}),
        },
    );
    models.insert(
        "Post".to_string(),
        ModelDefinition {
            name: "Post".to_string(),
            entity_id: local_id(10),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "title".to_string(),
                    field_type: cdm_plugin_interface::TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: serde_json::json!({}),
                    entity_id: local_id(11),
                },
            ],
            config: serde_json::json!({}),
        },
    );

    let schema = Schema {
        type_aliases: HashMap::new(),
        models: models.clone(),
    };

    // Simulate first-time migration: deltas should have ModelAdded for both models
    let deltas = vec![
        Delta::ModelAdded {
            name: "User".to_string(),
            after: models.get("User").unwrap().clone(),
        },
        Delta::ModelAdded {
            name: "Post".to_string(),
            after: models.get("Post").unwrap().clone(),
        },
    ];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);

    // Should generate migration files (not empty!)
    assert_eq!(files.len(), 2, "Expected 2 migration files (up and down) for first-time migration");

    // Check up migration contains CREATE TABLE for both models
    assert!(files[0].path.contains("up"));
    assert!(files[0].content.contains("CREATE TABLE"));
    assert!(files[0].content.contains("\"user\""));
    assert!(files[0].content.contains("\"post\""));
    assert!(files[0].content.contains("\"id\""));
    assert!(files[0].content.contains("\"email\""));
    assert!(files[0].content.contains("\"title\""));

    // Check down migration contains DROP TABLE for both models
    assert!(files[1].path.contains("down"));
    assert!(files[1].content.contains("DROP TABLE"));
    assert!(files[1].content.contains("\"user\""));
    assert!(files[1].content.contains("\"post\""));
}

#[test]
fn test_migrate_with_deltas() {
    let schema = Schema {
        type_aliases: HashMap::new(),
        models: HashMap::new(),
    };

    let deltas = vec![Delta::ModelAdded {
        name: "User".to_string(),
        after: ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![],
            config: serde_json::json!({}),
        },
    }];

    let config = serde_json::json!({});
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2); // up and down migrations

    // Check up migration
    assert!(files[0].path.contains("up"));
    assert!(files[0].content.contains("CREATE TABLE"));

    // Check down migration
    assert!(files[1].path.contains("down"));
    assert!(files[1].content.contains("DROP TABLE"));
}

#[test]
fn test_migrate_field_added_postgres() {
    use cdm_plugin_interface::TypeExpression;

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "name".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({}),
                entity_id: local_id(2),
            }],
            config: serde_json::json!({}),
        },
    );

    let schema = Schema {
        type_aliases: HashMap::new(),
        models,
    };

    let deltas = vec![Delta::FieldAdded {
        model: "User".to_string(),
        field: "email".to_string(),
        after: FieldDefinition {
            name: "email".to_string(),
            field_type: TypeExpression::Identifier {
                name: "string".to_string(),
            },
            optional: false,
            default: None,
            config: serde_json::json!({}),
            entity_id: local_id(3),
        },
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Check up migration contains ADD COLUMN
    assert!(files[0].content.contains("ADD COLUMN"));
    assert!(files[0].content.contains("\"email\""));

    // Check down migration contains DROP COLUMN
    assert!(files[1].content.contains("DROP COLUMN"));
    assert!(files[1].content.contains("\"email\""));
}

#[test]
fn test_migrate_field_renamed_postgres() {
    use cdm_plugin_interface::TypeExpression;

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "full_name".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({}),
                entity_id: local_id(2),
            }],
            config: serde_json::json!({}),
        },
    );

    let schema = Schema {
        type_aliases: HashMap::new(),
        models,
    };

    let deltas = vec![Delta::FieldRenamed {
        model: "User".to_string(),
        old_name: "name".to_string(),
        new_name: "full_name".to_string(),
        id: local_id(2),
        before: FieldDefinition {
            name: "name".to_string(),
            field_type: TypeExpression::Identifier {
                name: "string".to_string(),
            },
            optional: false,
            default: None,
            config: serde_json::json!({}),
            entity_id: local_id(2),
        },
        after: FieldDefinition {
            name: "full_name".to_string(),
            field_type: TypeExpression::Identifier {
                name: "string".to_string(),
            },
            optional: false,
            default: None,
            config: serde_json::json!({}),
            entity_id: local_id(2),
        },
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Check up migration renames column
    assert!(files[0].content.contains("RENAME COLUMN"));
    assert!(files[0].content.contains("\"name\""));
    assert!(files[0].content.contains("\"full_name\""));
}

#[test]
fn test_migrate_model_removed_postgres() {
    let schema = Schema {
        type_aliases: HashMap::new(),
        models: HashMap::new(),
    };

    let deltas = vec![Delta::ModelRemoved {
        name: "User".to_string(),
        before: ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![],
            config: serde_json::json!({}),
        },
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Check up migration drops table
    assert!(files[0].content.contains("DROP TABLE"));

    // Check down migration creates table
    assert!(files[1].content.contains("CREATE TABLE"));
}

#[test]
fn test_migrate_field_type_changed_sqlite() {
    use cdm_plugin_interface::TypeExpression;

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "age".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({}),
                entity_id: local_id(2),
            }],
            config: serde_json::json!({}),
        },
    );

    let schema = Schema {
        type_aliases: HashMap::new(),
        models,
    };

    let deltas = vec![Delta::FieldTypeChanged {
        model: "User".to_string(),
        field: "age".to_string(),
        before: TypeExpression::Identifier {
            name: "number".to_string(),
        },
        after: TypeExpression::Identifier {
            name: "string".to_string(),
        },
    }];

    let config = serde_json::json!({ "dialect": "sqlite", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // SQLite doesn't support ALTER COLUMN TYPE, so should have comments
    assert!(files[0].content.contains("SQLite does not support"));
    assert!(files[0].content.contains("Manual migration required"));
}

#[test]
fn test_migrate_optionality_changed_postgres() {
    use cdm_plugin_interface::TypeExpression;

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "email".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: true,
                default: None,
                config: serde_json::json!({}),
                entity_id: local_id(2),
            }],
            config: serde_json::json!({}),
        },
    );

    let schema = Schema {
        type_aliases: HashMap::new(),
        models,
    };

    let deltas = vec![Delta::FieldOptionalityChanged {
        model: "User".to_string(),
        field: "email".to_string(),
        before: false, // was required
        after: true,   // became optional
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Check up migration drops NOT NULL
    assert!(files[0].content.contains("DROP NOT NULL"));

    // Check down migration sets NOT NULL
    assert!(files[1].content.contains("SET NOT NULL"));
}

#[test]
fn test_migrate_default_changed_postgres() {
    use cdm_plugin_interface::{TypeExpression, Value};

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "status".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: Some(Value::String("active".to_string())),
                config: serde_json::json!({}),
                entity_id: local_id(2),
            }],
            config: serde_json::json!({}),
        },
    );

    let schema = Schema {
        type_aliases: HashMap::new(),
        models,
    };

    let deltas = vec![Delta::FieldDefaultChanged {
        model: "User".to_string(),
        field: "status".to_string(),
        before: Some(Value::String("pending".to_string())),
        after: Some(Value::String("active".to_string())),
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Check up migration sets new default
    assert!(files[0].content.contains("SET DEFAULT"));
    assert!(files[0].content.contains("'active'"));

    // Check down migration sets old default
    assert!(files[1].content.contains("SET DEFAULT"));
    assert!(files[1].content.contains("'pending'"));
}

#[test]
fn test_migrate_default_removed_postgres() {
    use cdm_plugin_interface::{TypeExpression, Value};

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "status".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({}),
                entity_id: local_id(2),
            }],
            config: serde_json::json!({}),
        },
    );

    let schema = Schema {
        type_aliases: HashMap::new(),
        models,
    };

    let deltas = vec![Delta::FieldDefaultChanged {
        model: "User".to_string(),
        field: "status".to_string(),
        before: Some(Value::String("active".to_string())),
        after: None,
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Check up migration drops default
    assert!(files[0].content.contains("DROP DEFAULT"));

    // Check down migration restores default
    assert!(files[1].content.contains("SET DEFAULT"));
    assert!(files[1].content.contains("'active'"));
}

#[test]
fn test_migrate_model_renamed_postgres() {
    let schema = Schema {
        type_aliases: HashMap::new(),
        models: HashMap::new(),
    };

    let deltas = vec![Delta::ModelRenamed {
        old_name: "User".to_string(),
        new_name: "Account".to_string(),
        id: local_id(1),
        before: ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![],
            config: serde_json::json!({}),
        },
        after: ModelDefinition {
            name: "Account".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![],
            config: serde_json::json!({}),
        },
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Check up migration renames table
    assert!(files[0].content.contains("RENAME TO"));
    assert!(files[0].content.contains("\"user\""));
    assert!(files[0].content.contains("\"account\""));

    // Check down migration reverses rename
    assert!(files[1].content.contains("RENAME TO"));
    assert!(files[1].content.contains("\"account\""));
    assert!(files[1].content.contains("\"user\""));
}

#[test]
fn test_migrate_model_renamed_sqlite() {
    let schema = Schema {
        type_aliases: HashMap::new(),
        models: HashMap::new(),
    };

    let deltas = vec![Delta::ModelRenamed {
        old_name: "User".to_string(),
        new_name: "Account".to_string(),
        id: local_id(1),
        before: ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![],
            config: serde_json::json!({}),
        },
        after: ModelDefinition {
            name: "Account".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![],
            config: serde_json::json!({}),
        },
    }];

    let config = serde_json::json!({ "dialect": "sqlite", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Check SQLite format (no schema prefix)
    assert!(files[0].content.contains("ALTER TABLE \"user\" RENAME TO \"account\""));
    assert!(files[1].content.contains("ALTER TABLE \"account\" RENAME TO \"user\""));
}

#[test]
fn test_migrate_field_renamed_sqlite() {
    use cdm_plugin_interface::TypeExpression;

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "full_name".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({}),
                entity_id: local_id(2),
            }],
            config: serde_json::json!({}),
        },
    );

    let schema = Schema {
        type_aliases: HashMap::new(),
        models,
    };

    let deltas = vec![Delta::FieldRenamed {
        model: "User".to_string(),
        old_name: "name".to_string(),
        new_name: "full_name".to_string(),
        id: local_id(2),
        before: FieldDefinition {
            name: "name".to_string(),
            field_type: TypeExpression::Identifier {
                name: "string".to_string(),
            },
            optional: false,
            default: None,
            config: serde_json::json!({}),
            entity_id: local_id(2),
        },
        after: FieldDefinition {
            name: "full_name".to_string(),
            field_type: TypeExpression::Identifier {
                name: "string".to_string(),
            },
            optional: false,
            default: None,
            config: serde_json::json!({}),
            entity_id: local_id(2),
        },
    }];

    let config = serde_json::json!({ "dialect": "sqlite", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // SQLite supports RENAME COLUMN in 3.25.0+
    assert!(files[0].content.contains("RENAME COLUMN"));
    assert!(files[0].content.contains("\"name\""));
    assert!(files[0].content.contains("\"full_name\""));
}

#[test]
fn test_migrate_field_type_changed_postgres() {
    use cdm_plugin_interface::TypeExpression;

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "age".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({}),
                entity_id: local_id(2),
            }],
            config: serde_json::json!({}),
        },
    );

    let schema = Schema {
        type_aliases: HashMap::new(),
        models,
    };

    let deltas = vec![Delta::FieldTypeChanged {
        model: "User".to_string(),
        field: "age".to_string(),
        before: TypeExpression::Identifier {
            name: "number".to_string(),
        },
        after: TypeExpression::Identifier {
            name: "string".to_string(),
        },
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // PostgreSQL supports ALTER COLUMN TYPE
    assert!(files[0].content.contains("ALTER COLUMN"));
    assert!(files[0].content.contains("TYPE"));
    assert!(files[0].content.contains("VARCHAR(255)"));

    // Down migration reverts type
    assert!(files[1].content.contains("DOUBLE PRECISION"));
}

#[test]
fn test_migrate_with_schema_prefix_postgres() {
    let schema = Schema {
        type_aliases: HashMap::new(),
        models: HashMap::new(),
    };

    let deltas = vec![Delta::ModelAdded {
        name: "User".to_string(),
        after: ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![],
            config: serde_json::json!({}),
        },
    }];

    let config = serde_json::json!({
        "dialect": "postgresql",
        "schema": "public",
        "pluralize_table_names": false
    });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Check up migration includes schema prefix
    assert!(files[0].content.contains("\"public\".\"user\""));

    // Check down migration includes schema prefix
    assert!(files[1].content.contains("DROP TABLE \"public\".\"user\""));
}

#[test]
fn test_migrate_with_name_formatting() {
    use cdm_plugin_interface::TypeExpression;

    let mut models = HashMap::new();
    models.insert(
        "UserProfile".to_string(),
        ModelDefinition {
            name: "UserProfile".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "firstName".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({}),
                entity_id: local_id(2),
            }],
            config: serde_json::json!({}),
        },
    );

    let schema = Schema {
        type_aliases: HashMap::new(),
        models,
    };

    let deltas = vec![Delta::FieldAdded {
        model: "UserProfile".to_string(),
        field: "lastName".to_string(),
        after: FieldDefinition {
            name: "lastName".to_string(),
            field_type: TypeExpression::Identifier {
                name: "string".to_string(),
            },
            optional: false,
            default: None,
            config: serde_json::json!({}),
            entity_id: local_id(3),
        },
    }];

    let config = serde_json::json!({
        "dialect": "postgresql",
        "table_name_format": "snake_case",
        "column_name_format": "snake_case",
        "pluralize_table_names": false
    });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Check that names are formatted as snake_case
    assert!(files[0].content.contains("\"user_profile\""));
    assert!(files[0].content.contains("\"last_name\""));
}

#[test]
fn test_migrate_optionality_changed_required_to_optional() {
    use cdm_plugin_interface::TypeExpression;

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "email".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: true,
                default: None,
                config: serde_json::json!({}),
                entity_id: local_id(2),
            }],
            config: serde_json::json!({}),
        },
    );

    let schema = Schema {
        type_aliases: HashMap::new(),
        models,
    };

    let deltas = vec![Delta::FieldOptionalityChanged {
        model: "User".to_string(),
        field: "email".to_string(),
        before: false, // was required
        after: true,   // became optional
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Field became optional, so DROP NOT NULL
    assert!(files[0].content.contains("DROP NOT NULL"));
    assert!(files[1].content.contains("SET NOT NULL"));
}

#[test]
fn test_migrate_optionality_changed_optional_to_required() {
    use cdm_plugin_interface::TypeExpression;

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "email".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({}),
                entity_id: local_id(2),
            }],
            config: serde_json::json!({}),
        },
    );

    let schema = Schema {
        type_aliases: HashMap::new(),
        models,
    };

    let deltas = vec![Delta::FieldOptionalityChanged {
        model: "User".to_string(),
        field: "email".to_string(),
        before: true,  // was optional
        after: false,  // became required
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Field became required, so SET NOT NULL
    assert!(files[0].content.contains("SET NOT NULL"));
    assert!(files[1].content.contains("DROP NOT NULL"));
}

#[test]
fn test_migrate_sqlite_limitations() {
    use cdm_plugin_interface::{TypeExpression, Value};

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "status".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({}),
                entity_id: local_id(2),
            }],
            config: serde_json::json!({}),
        },
    );

    let schema = Schema {
        type_aliases: HashMap::new(),
        models,
    };

    // Test multiple SQLite limitations
    let deltas = vec![
        Delta::FieldOptionalityChanged {
            model: "User".to_string(),
            field: "status".to_string(),
            before: false,
            after: true,
        },
        Delta::FieldDefaultChanged {
            model: "User".to_string(),
            field: "status".to_string(),
            before: None,
            after: Some(Value::String("active".to_string())),
        },
    ];

    let config = serde_json::json!({ "dialect": "sqlite", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // All should have manual migration warnings
    let up_content = &files[0].content;
    assert!(up_content.contains("SQLite does not support"));
    assert!(up_content.contains("Manual migration required"));
}

#[test]
fn test_migrate_multiple_deltas() {
    use cdm_plugin_interface::TypeExpression;

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "id".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "number".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: serde_json::json!({}),
                    entity_id: local_id(2),
                },
                FieldDefinition {
                    name: "email".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: serde_json::json!({}),
                    entity_id: local_id(3),
                },
            ],
            config: serde_json::json!({}),
        },
    );

    let schema = Schema {
        type_aliases: HashMap::new(),
        models,
    };

    let deltas = vec![
        Delta::FieldAdded {
            model: "User".to_string(),
            field: "name".to_string(),
            after: FieldDefinition {
                name: "name".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({}),
                entity_id: local_id(4),
            },
        },
        Delta::FieldRemoved {
            model: "User".to_string(),
            field: "email".to_string(),
            before: FieldDefinition {
                name: "email".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({}),
                entity_id: local_id(3),
            },
        },
    ];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Up migration should have both operations
    assert!(files[0].content.contains("ADD COLUMN"));
    assert!(files[0].content.contains("DROP COLUMN"));

    // Down migration should reverse both operations
    assert!(files[1].content.contains("DROP COLUMN \"name\""));
    assert!(files[1].content.contains("ADD COLUMN \"email\""));
}

#[test]
fn test_migrate_file_naming_default() {
    // Test default naming when no migration_name is provided
    let schema = Schema {
        type_aliases: HashMap::new(),
        models: HashMap::new(),
    };

    let deltas = vec![Delta::ModelAdded {
        name: "User".to_string(),
        after: ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![],
            config: serde_json::json!({}),
        },
    }];

    // Test PostgreSQL file naming with default name
    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;
    let files = migrate(schema.clone(), deltas.clone(), config, &utils);
    assert_eq!(files[0].path, "001_migration.up.postgres.sql");
    assert_eq!(files[1].path, "001_migration.down.postgres.sql");

    // Test SQLite file naming with default name
    let config = serde_json::json!({ "dialect": "sqlite", "pluralize_table_names": false });
    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files[0].path, "001_migration.up.sqlite.sql");
    assert_eq!(files[1].path, "001_migration.down.sqlite.sql");
}

#[test]
fn test_migrate_file_naming_with_custom_name() {
    // Test that migration_name from config is used for file naming
    let schema = Schema {
        type_aliases: HashMap::new(),
        models: HashMap::new(),
    };

    let deltas = vec![Delta::ModelAdded {
        name: "User".to_string(),
        after: ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![],
            config: serde_json::json!({}),
        },
    }];

    // Test PostgreSQL with custom migration name
    let config = serde_json::json!({
        "dialect": "postgresql",
        "pluralize_table_names": false,
        "migration_name": "002_add_users_table"
    });
    let utils = Utils;
    let files = migrate(schema.clone(), deltas.clone(), config, &utils);
    assert_eq!(files[0].path, "002_add_users_table.up.postgres.sql");
    assert_eq!(files[1].path, "002_add_users_table.down.postgres.sql");

    // Test SQLite with custom migration name
    let config = serde_json::json!({
        "dialect": "sqlite",
        "pluralize_table_names": false,
        "migration_name": "003_initial_schema"
    });
    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files[0].path, "003_initial_schema.up.sqlite.sql");
    assert_eq!(files[1].path, "003_initial_schema.down.sqlite.sql");
}

#[test]
fn test_migrate_model_with_indexes_added() {
    let schema = Schema {
        type_aliases: HashMap::new(),
        models: HashMap::new(),
    };

    let deltas = vec![Delta::ModelAdded {
        name: "User".to_string(),
        after: ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![],
            config: serde_json::json!({
                "indexes": [
                    {
                        "fields": ["email"],
                        "unique": true
                    },
                    {
                        "fields": ["created_at"]
                    }
                ]
            }),
        },
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Check up migration
    let expected_up = "-- Migration generated by CDM SQL Plugin
-- Dialect: PostgreSQL

CREATE TABLE \"user\" (
  UNIQUE (\"email\")
);

CREATE INDEX \"idx_user_1\" ON \"user\" (\"created_at\");

";
    assert_eq!(files[0].content, expected_up);

    // Check down migration
    let expected_down = "-- Rollback migration generated by CDM SQL Plugin
-- Dialect: PostgreSQL

DROP TABLE \"user\";

";
    assert_eq!(files[1].content, expected_down);
}

#[test]
fn test_migrate_model_with_primary_key_added() {
    let schema = Schema {
        type_aliases: HashMap::new(),
        models: HashMap::new(),
    };

    let deltas = vec![Delta::ModelAdded {
        name: "User".to_string(),
        after: ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![],
            config: serde_json::json!({
                "indexes": [
                    {
                        "fields": ["id"],
                        "primary": true
                    }
                ]
            }),
        },
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Check up migration
    let expected_up = "-- Migration generated by CDM SQL Plugin
-- Dialect: PostgreSQL

CREATE TABLE \"user\" (
  PRIMARY KEY (\"id\")
);

";
    assert_eq!(files[0].content, expected_up);

    // Check down migration
    let expected_down = "-- Rollback migration generated by CDM SQL Plugin
-- Dialect: PostgreSQL

DROP TABLE \"user\";

";
    assert_eq!(files[1].content, expected_down);
}

#[test]
fn test_migrate_model_with_composite_index_added() {
    let schema = Schema {
        type_aliases: HashMap::new(),
        models: HashMap::new(),
    };

    let deltas = vec![Delta::ModelAdded {
        name: "Post".to_string(),
        after: ModelDefinition {
            name: "Post".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![],
            config: serde_json::json!({
                "indexes": [
                    {
                        "name": "idx_user_created",
                        "fields": ["user_id", "created_at"]
                    }
                ]
            }),
        },
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Check up migration
    let expected_up = "-- Migration generated by CDM SQL Plugin
-- Dialect: PostgreSQL

CREATE TABLE \"post\" (
);

CREATE INDEX \"idx_user_created\" ON \"post\" (\"user_id\", \"created_at\");

";
    assert_eq!(files[0].content, expected_up);

    // Check down migration
    let expected_down = "-- Rollback migration generated by CDM SQL Plugin
-- Dialect: PostgreSQL

DROP TABLE \"post\";

";
    assert_eq!(files[1].content, expected_down);
}

#[test]
fn test_migrate_model_with_partial_index_postgres() {
    let schema = Schema {
        type_aliases: HashMap::new(),
        models: HashMap::new(),
    };

    let deltas = vec![Delta::ModelAdded {
        name: "User".to_string(),
        after: ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![],
            config: serde_json::json!({
                "indexes": [
                    {
                        "name": "idx_active_users",
                        "fields": ["email"],
                        "where": "active = TRUE"
                    }
                ]
            }),
        },
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Check up migration
    let expected_up = "-- Migration generated by CDM SQL Plugin
-- Dialect: PostgreSQL

CREATE TABLE \"user\" (
);

CREATE INDEX \"idx_active_users\" ON \"user\" (\"email\") WHERE active = TRUE;

";
    assert_eq!(files[0].content, expected_up);

    // Check down migration
    let expected_down = "-- Rollback migration generated by CDM SQL Plugin
-- Dialect: PostgreSQL

DROP TABLE \"user\";

";
    assert_eq!(files[1].content, expected_down);
}

#[test]
fn test_migrate_model_with_index_method_postgres() {
    let schema = Schema {
        type_aliases: HashMap::new(),
        models: HashMap::new(),
    };

    let deltas = vec![Delta::ModelAdded {
        name: "Document".to_string(),
        after: ModelDefinition {
            name: "Document".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![],
            config: serde_json::json!({
                "indexes": [
                    {
                        "name": "idx_content_gin",
                        "fields": ["content"],
                        "method": "gin"
                    }
                ]
            }),
        },
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Check up migration
    let expected_up = "-- Migration generated by CDM SQL Plugin
-- Dialect: PostgreSQL

CREATE TABLE \"document\" (
);

CREATE INDEX \"idx_content_gin\" ON \"document\" (\"content\") USING GIN;

";
    assert_eq!(files[0].content, expected_up);

    // Check down migration
    let expected_down = "-- Rollback migration generated by CDM SQL Plugin
-- Dialect: PostgreSQL

DROP TABLE \"document\";

";
    assert_eq!(files[1].content, expected_down);
}

#[test]
fn test_migrate_model_with_multiple_constraint_types() {
    let schema = Schema {
        type_aliases: HashMap::new(),
        models: HashMap::new(),
    };

    let deltas = vec![Delta::ModelAdded {
        name: "User".to_string(),
        after: ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![],
            config: serde_json::json!({
                "indexes": [
                    {
                        "fields": ["id"],
                        "primary": true
                    },
                    {
                        "fields": ["email"],
                        "unique": true
                    },
                    {
                        "fields": ["username"],
                        "unique": true
                    },
                    {
                        "fields": ["created_at"]
                    }
                ]
            }),
        },
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Check up migration
    let expected_up = "-- Migration generated by CDM SQL Plugin
-- Dialect: PostgreSQL

CREATE TABLE \"user\" (
  PRIMARY KEY (\"id\"),
  UNIQUE (\"email\"),
  UNIQUE (\"username\")
);

CREATE INDEX \"idx_user_3\" ON \"user\" (\"created_at\");

";
    assert_eq!(files[0].content, expected_up);

    // Check down migration
    let expected_down = "-- Rollback migration generated by CDM SQL Plugin
-- Dialect: PostgreSQL

DROP TABLE \"user\";

";
    assert_eq!(files[1].content, expected_down);
}

#[test]
fn test_migrate_model_config_changed_indexes() {
    use cdm_plugin_interface::TypeExpression;

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "email".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({}),
                entity_id: local_id(2),
            }],
            config: serde_json::json!({
                "indexes": [
                    {
                        "fields": ["email"],
                        "unique": true
                    }
                ]
            }),
        },
    );

    let schema = Schema {
        type_aliases: HashMap::new(),
        models,
    };

    // Simulate config change (indexes added/removed/modified)
    let deltas = vec![Delta::ModelConfigChanged {
        model: "User".to_string(),
        before: serde_json::json!({}),
        after: serde_json::json!({
            "indexes": [
                {
                    "fields": ["email"],
                    "unique": true
                }
            ]
        }),
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Check that index creation generates proper SQL
    let up_content = &files[0].content;
    assert!(up_content.contains("CREATE UNIQUE INDEX"), "Expected CREATE UNIQUE INDEX in up migration, got: {}", up_content);
    assert!(up_content.contains("\"user\""), "Expected table name in up migration, got: {}", up_content);
    assert!(up_content.contains("\"email\""), "Expected field name in up migration, got: {}", up_content);

    // Check that down migration has DROP INDEX
    let down_content = &files[1].content;
    assert!(down_content.contains("DROP INDEX"), "Expected DROP INDEX in down migration, got: {}", down_content);
    assert!(down_content.contains("idx_user_0"), "Expected index name in down migration, got: {}", down_content);
}

#[test]
fn test_migrate_sqlite_indexes() {
    let schema = Schema {
        type_aliases: HashMap::new(),
        models: HashMap::new(),
    };

    let deltas = vec![Delta::ModelAdded {
        name: "User".to_string(),
        after: ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![],
            config: serde_json::json!({
                "indexes": [
                    {
                        "fields": ["id"],
                        "primary": true
                    },
                    {
                        "fields": ["email"],
                        "unique": true
                    },
                    {
                        "fields": ["created_at"]
                    }
                ]
            }),
        },
    }];

    let config = serde_json::json!({ "dialect": "sqlite", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Check SQLite-specific syntax
    let up_content = &files[0].content;
    assert!(up_content.contains("CREATE TABLE \"user\""));
    assert!(up_content.contains("PRIMARY KEY"));
    assert!(up_content.contains("UNIQUE"));
    assert!(up_content.contains("CREATE INDEX"));

    // SQLite should not have WHERE clause support in this context
    assert!(!up_content.contains("WHERE"));
}

#[test]
fn test_migrate_model_removed_with_indexes() {
    let schema = Schema {
        type_aliases: HashMap::new(),
        models: HashMap::new(),
    };

    let deltas = vec![Delta::ModelRemoved {
        name: "User".to_string(),
        before: ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![],
            config: serde_json::json!({
                "indexes": [
                    {
                        "fields": ["id"],
                        "primary": true
                    },
                    {
                        "name": "idx_email",
                        "fields": ["email"]
                    }
                ]
            }),
        },
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Up migration should drop table (and all indexes with it)
    let up_content = &files[0].content;
    assert!(up_content.contains("DROP TABLE"));

    // Down migration should recreate table with indexes
    let down_content = &files[1].content;
    assert!(down_content.contains("CREATE TABLE"));
    assert!(down_content.contains("PRIMARY KEY"));
    assert!(down_content.contains("CREATE INDEX"));
    assert!(down_content.contains("\"idx_email\""));
}

#[test]
fn test_migrate_composite_primary_key() {
    let schema = Schema {
        type_aliases: HashMap::new(),
        models: HashMap::new(),
    };

    let deltas = vec![Delta::ModelAdded {
        name: "UserRole".to_string(),
        after: ModelDefinition {
            name: "UserRole".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![],
            config: serde_json::json!({
                "indexes": [
                    {
                        "fields": ["user_id", "role_id"],
                        "primary": true
                    }
                ]
            }),
        },
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Check up migration
    let expected_up = "-- Migration generated by CDM SQL Plugin
-- Dialect: PostgreSQL

CREATE TABLE \"user_role\" (
  PRIMARY KEY (\"user_id\", \"role_id\")
);

";
    assert_eq!(files[0].content, expected_up);

    // Check down migration
    let expected_down = "-- Rollback migration generated by CDM SQL Plugin
-- Dialect: PostgreSQL

DROP TABLE \"user_role\";

";
    assert_eq!(files[1].content, expected_down);
}

#[test]
fn test_migrate_composite_unique_constraint() {
    let schema = Schema {
        type_aliases: HashMap::new(),
        models: HashMap::new(),
    };

    let deltas = vec![Delta::ModelAdded {
        name: "Product".to_string(),
        after: ModelDefinition {
            name: "Product".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![],
            config: serde_json::json!({
                "indexes": [
                    {
                        "fields": ["sku", "store_id"],
                        "unique": true
                    }
                ]
            }),
        },
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Check up migration
    let expected_up = "-- Migration generated by CDM SQL Plugin
-- Dialect: PostgreSQL

CREATE TABLE \"product\" (
  UNIQUE (\"sku\", \"store_id\")
);

";
    assert_eq!(files[0].content, expected_up);

    // Check down migration
    let expected_down = "-- Rollback migration generated by CDM SQL Plugin
-- Dialect: PostgreSQL

DROP TABLE \"product\";

";
    assert_eq!(files[1].content, expected_down);
}

#[test]
fn test_migrate_with_schema_prefix_and_indexes() {
    let schema = Schema {
        type_aliases: HashMap::new(),
        models: HashMap::new(),
    };

    let deltas = vec![Delta::ModelAdded {
        name: "User".to_string(),
        after: ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![],
            config: serde_json::json!({
                "indexes": [
                    {
                        "name": "idx_email",
                        "fields": ["email"]
                    }
                ]
            }),
        },
    }];

    let config = serde_json::json!({
        "dialect": "postgresql",
        "schema": "public",
        "pluralize_table_names": false
    });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Check up migration
    let expected_up = "-- Migration generated by CDM SQL Plugin
-- Dialect: PostgreSQL

CREATE TABLE \"public\".\"user\" (
);

CREATE INDEX \"idx_email\" ON \"public\".\"user\" (\"email\");

";
    assert_eq!(files[0].content, expected_up);

    // Check down migration
    let expected_down = "-- Rollback migration generated by CDM SQL Plugin
-- Dialect: PostgreSQL

DROP TABLE \"public\".\"user\";

";
    assert_eq!(files[1].content, expected_down);
}

#[test]
fn test_migrate_multiple_indexes_different_types() {
    let schema = Schema {
        type_aliases: HashMap::new(),
        models: HashMap::new(),
    };

    let deltas = vec![Delta::ModelAdded {
        name: "Document".to_string(),
        after: ModelDefinition {
            name: "Document".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![],
            config: serde_json::json!({
                "indexes": [
                    {
                        "fields": ["id"],
                        "primary": true
                    },
                    {
                        "name": "idx_title",
                        "fields": ["title"]
                    },
                    {
                        "name": "idx_content_gin",
                        "fields": ["content"],
                        "method": "gin"
                    },
                    {
                        "name": "idx_published",
                        "fields": ["published_at"],
                        "where": "published_at IS NOT NULL"
                    }
                ]
            }),
        },
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Check up migration
    let expected_up = "-- Migration generated by CDM SQL Plugin
-- Dialect: PostgreSQL

CREATE TABLE \"document\" (
  PRIMARY KEY (\"id\")
);

CREATE INDEX \"idx_title\" ON \"document\" (\"title\");
CREATE INDEX \"idx_content_gin\" ON \"document\" (\"content\") USING GIN;
CREATE INDEX \"idx_published\" ON \"document\" (\"published_at\") WHERE published_at IS NOT NULL;

";
    assert_eq!(files[0].content, expected_up);

    // Check down migration
    let expected_down = "-- Rollback migration generated by CDM SQL Plugin
-- Dialect: PostgreSQL

DROP TABLE \"document\";

";
    assert_eq!(files[1].content, expected_down);
}

#[test]
fn test_get_schema_prefix_none() {
    let config = serde_json::json!({ "dialect": "postgresql" });
    let prefix = get_schema_prefix(&config, Dialect::PostgreSQL);
    assert_eq!(prefix, None);
}

#[test]
fn test_get_schema_prefix_some() {
    let config = serde_json::json!({ "dialect": "postgresql", "schema": "my_schema" });
    let prefix = get_schema_prefix(&config, Dialect::PostgreSQL);
    assert_eq!(prefix, Some("my_schema".to_string()));
}

#[test]
fn test_get_schema_prefix_sqlite_ignored() {
    let config = serde_json::json!({ "dialect": "sqlite", "schema": "my_schema" });
    let prefix = get_schema_prefix(&config, Dialect::SQLite);
    assert_eq!(prefix, None);
}

#[test]
fn test_migrate_field_removed() {
    let schema = Schema {
        type_aliases: HashMap::new(),
        models: HashMap::new(),
    };

    use cdm_plugin_interface::TypeExpression;

    let deltas = vec![Delta::FieldRemoved {
        model: "User".to_string(),
        field: "email".to_string(),
        before: FieldDefinition {
            name: "email".to_string(),
            field_type: TypeExpression::Identifier { name: "string".to_string() },
            optional: false,
            entity_id: local_id(2),
            default: None,
            config: serde_json::json!({}),
        },
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Check that up migration drops the column
    assert!(files[0].content.contains("DROP COLUMN"));
    assert!(files[0].content.contains("\"email\""));

    // Check that down migration adds the column back
    assert!(files[1].content.contains("ADD COLUMN"));
    assert!(files[1].content.contains("\"email\""));
}

#[test]
fn test_migrate_field_default_changed_to_some() {
    use std::collections::HashMap;
    use cdm_plugin_interface::{ModelDefinition, TypeExpression};

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "status".to_string(),
                field_type: TypeExpression::Identifier { name: "string".to_string() },
                optional: false,
                entity_id: local_id(2),
                default: Some(cdm_plugin_interface::Value::String("active".to_string())),
                config: serde_json::json!({}),
            }],
            config: serde_json::json!({}),
        },
    );

    let schema = Schema {
        type_aliases: HashMap::new(),
        models,
    };

    let deltas = vec![Delta::FieldDefaultChanged {
        model: "User".to_string(),
        field: "status".to_string(),
        before: None,
        after: Some(cdm_plugin_interface::Value::String("active".to_string())),
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Check that up migration sets default
    assert!(files[0].content.contains("SET DEFAULT"));
    assert!(files[0].content.contains("active"));

    // Check that down migration drops default
    assert!(files[1].content.contains("DROP DEFAULT"));
}

#[test]
fn test_migrate_model_config_changed_no_indexes() {
    use std::collections::HashMap;
    use cdm_plugin_interface::ModelDefinition;

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![],
            config: serde_json::json!({ "table_name": "users_new" }),
        },
    );

    let schema = Schema {
        type_aliases: HashMap::new(),
        models,
    };

    let deltas = vec![Delta::ModelConfigChanged {
        model: "User".to_string(),
        before: serde_json::json!({ "table_name": "users_old" }),
        after: serde_json::json!({ "table_name": "users_new" }),
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);

    // Should have files but no index changes since no indexes are defined
    assert_eq!(files.len(), 2);
}

#[test]
fn test_migrate_empty_schema_with_model_added() {
    let schema = Schema {
        type_aliases: HashMap::new(),
        models: HashMap::new(),
    };

    let deltas = vec![Delta::ModelAdded {
        name: "EmptyModel".to_string(),
        after: ModelDefinition {
            name: "EmptyModel".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![],
            config: serde_json::json!({}),
        },
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Up migration should create empty table
    assert!(files[0].content.contains("CREATE TABLE"));
    // Table name might be pluralized or modified based on config
    assert!(files[0].content.contains("empty") || files[0].content.contains("Empty"));

    // Down migration should drop table
    assert!(files[1].content.contains("DROP TABLE"));
}

#[test]
fn test_generate_column_definition_with_all_properties() {
    use cdm_plugin_interface::{TypeAliasDefinition, TypeExpression};

    let field_def = FieldDefinition {
        name: "email".to_string(),
        field_type: TypeExpression::Identifier { name: "string".to_string() },
        optional: false,
        entity_id: local_id(2),
        default: Some(cdm_plugin_interface::Value::String("user@example.com".to_string())),
        config: serde_json::json!({ "column_name": "user_email", "max_length": 255 }),
    };

    let config = serde_json::json!({ "dialect": "postgresql" });
    let type_aliases: HashMap<String, TypeAliasDefinition> = HashMap::new();
    let type_mapper = TypeMapper::new(&config, &type_aliases);

    let column_def = generate_column_definition(&field_def, "user_email", &config, &type_mapper);

    // Verify basic properties are present
    assert!(column_def.contains("user_email"));
    assert!(column_def.contains("VARCHAR(255)"));
    assert!(column_def.contains("NOT NULL"));
    // Default value handling depends on the implementation
}

#[test]
fn test_migrate_complex_multiple_deltas_mixed() {
    use std::collections::HashMap;
    use cdm_plugin_interface::{ModelDefinition, TypeExpression};

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "name".to_string(),
                    field_type: TypeExpression::Identifier { name: "string".to_string() },
                    optional: false,
                    entity_id: local_id(2),
                    default: None,
                    config: serde_json::json!({}),
                },
                FieldDefinition {
                    name: "email".to_string(),
                    field_type: TypeExpression::Identifier { name: "string".to_string() },
                    optional: false,
                    entity_id: local_id(3),
                    default: None,
                    config: serde_json::json!({}),
                },
            ],
            config: serde_json::json!({}),
        },
    );

    let schema = Schema {
        type_aliases: HashMap::new(),
        models,
    };

    // Mix of different delta types
    let deltas = vec![
        Delta::ModelAdded {
            name: "Product".to_string(),
            after: ModelDefinition {
                name: "Product".to_string(),
                entity_id: local_id(10),
                parents: vec![],
                fields: vec![],
                config: serde_json::json!({}),
            },
        },
        Delta::FieldAdded {
            model: "User".to_string(),
            field: "age".to_string(),
            after: FieldDefinition {
                name: "age".to_string(),
                field_type: TypeExpression::Identifier { name: "number".to_string() },
                optional: true,
                entity_id: local_id(4),
                default: None,
                config: serde_json::json!({}),
            },
        },
        Delta::FieldRenamed {
            model: "User".to_string(),
            old_name: "name".to_string(),
            new_name: "full_name".to_string(),
            id: local_id(2),
            before: FieldDefinition {
                name: "name".to_string(),
                field_type: TypeExpression::Identifier { name: "string".to_string() },
                optional: false,
                entity_id: local_id(2),
                default: None,
                config: serde_json::json!({}),
            },
            after: FieldDefinition {
                name: "full_name".to_string(),
                field_type: TypeExpression::Identifier { name: "string".to_string() },
                optional: false,
                entity_id: local_id(2),
                default: None,
                config: serde_json::json!({}),
            },
        },
    ];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Verify up migration contains all operations
    assert!(files[0].content.contains("CREATE TABLE"));
    assert!(files[0].content.contains("\"product\""));
    assert!(files[0].content.contains("ADD COLUMN"));
    assert!(files[0].content.contains("\"age\""));
    assert!(files[0].content.contains("RENAME COLUMN"));
    assert!(files[0].content.contains("\"name\""));
    assert!(files[0].content.contains("\"full_name\""));

    // Verify down migration reverses all operations
    assert!(files[1].content.contains("DROP TABLE"));
    assert!(files[1].content.contains("DROP COLUMN"));
    assert!(files[1].content.contains("RENAME COLUMN"));
}

#[test]
fn test_migrate_with_type_alias_sql_type_override() {
    // This test verifies that a type alias with @sql { type: "INTEGER" } annotation
    // is correctly used in migrations when a field references that type alias.
    // Bug: numeric type aliases with explicit SQL type were incorrectly built as JSONB.

    use cdm_plugin_interface::{Delta, TypeAliasDefinition, TypeExpression};

    let mut type_aliases = HashMap::new();
    type_aliases.insert(
        "ID".to_string(),
        TypeAliasDefinition {
            name: "ID".to_string(),
            alias_type: TypeExpression::Identifier {
                name: "number".to_string(),
            },
            config: serde_json::json!({
                "type": "INTEGER"
            }),
            entity_id: None,
        },
    );

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "id".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "ID".to_string(),
                },
                optional: false,
                default: None,
                entity_id: None,
                config: serde_json::json!({}),
            }],
            entity_id: None,
            config: serde_json::json!({}),
        },
    );

    let schema = Schema {
        type_aliases,
        models: models.clone(),
    };

    // Simulate adding the User model
    let deltas = vec![Delta::ModelAdded {
        name: "User".to_string(),
        after: models.get("User").unwrap().clone(),
    }];

    let config = serde_json::json!({
        "dialect": "postgresql",
        "pluralize_table_names": false
    });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    let up_sql = &files[0].content;

    // The field should use INTEGER (from type alias config), not JSONB
    assert!(
        up_sql.contains("\"id\" INTEGER NOT NULL"),
        "Expected 'id' column to be INTEGER in migration, but got:\n{}",
        up_sql
    );
    assert!(
        !up_sql.contains("JSONB"),
        "Should not contain JSONB in migration, but got:\n{}",
        up_sql
    );
}

// ============================================================================
// indexes_equal tests
// ============================================================================

#[test]
fn test_indexes_equal_same_index() {
    use crate::utils::IndexInfo;

    let a = IndexInfo {
        name: "idx_a".to_string(),
        fields: vec!["email".to_string()],
        is_unique: false,
        is_primary: false,
        method: None,
        where_clause: None,
    };

    let b = IndexInfo {
        name: "idx_b".to_string(), // Different name should not matter
        fields: vec!["email".to_string()],
        is_unique: false,
        is_primary: false,
        method: None,
        where_clause: None,
    };

    assert!(super::indexes_equal(&a, &b));
}

#[test]
fn test_indexes_equal_different_fields() {
    use crate::utils::IndexInfo;

    let a = IndexInfo {
        name: "idx_a".to_string(),
        fields: vec!["email".to_string()],
        is_unique: false,
        is_primary: false,
        method: None,
        where_clause: None,
    };

    let b = IndexInfo {
        name: "idx_a".to_string(),
        fields: vec!["name".to_string()],
        is_unique: false,
        is_primary: false,
        method: None,
        where_clause: None,
    };

    assert!(!super::indexes_equal(&a, &b));
}

#[test]
fn test_indexes_equal_different_field_order() {
    use crate::utils::IndexInfo;

    let a = IndexInfo {
        name: "idx_a".to_string(),
        fields: vec!["email".to_string(), "name".to_string()],
        is_unique: false,
        is_primary: false,
        method: None,
        where_clause: None,
    };

    let b = IndexInfo {
        name: "idx_a".to_string(),
        fields: vec!["name".to_string(), "email".to_string()],
        is_unique: false,
        is_primary: false,
        method: None,
        where_clause: None,
    };

    assert!(!super::indexes_equal(&a, &b));
}

#[test]
fn test_indexes_equal_different_unique() {
    use crate::utils::IndexInfo;

    let a = IndexInfo {
        name: "idx_a".to_string(),
        fields: vec!["email".to_string()],
        is_unique: true,
        is_primary: false,
        method: None,
        where_clause: None,
    };

    let b = IndexInfo {
        name: "idx_a".to_string(),
        fields: vec!["email".to_string()],
        is_unique: false,
        is_primary: false,
        method: None,
        where_clause: None,
    };

    assert!(!super::indexes_equal(&a, &b));
}

#[test]
fn test_indexes_equal_different_method() {
    use crate::utils::IndexInfo;

    let a = IndexInfo {
        name: "idx_a".to_string(),
        fields: vec!["email".to_string()],
        is_unique: false,
        is_primary: false,
        method: Some("btree".to_string()),
        where_clause: None,
    };

    let b = IndexInfo {
        name: "idx_a".to_string(),
        fields: vec!["email".to_string()],
        is_unique: false,
        is_primary: false,
        method: Some("hash".to_string()),
        where_clause: None,
    };

    assert!(!super::indexes_equal(&a, &b));
}

// ============================================================================
// ModelConfigChanged index removal tests
// ============================================================================

#[test]
fn test_migrate_model_config_changed_index_removed() {
    use cdm_plugin_interface::TypeExpression;

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "email".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({}),
                entity_id: local_id(2),
            }],
            config: serde_json::json!({}), // Current state has no indexes
        },
    );

    let schema = Schema {
        type_aliases: HashMap::new(),
        models,
    };

    // Simulate removing an index
    let deltas = vec![Delta::ModelConfigChanged {
        model: "User".to_string(),
        before: serde_json::json!({
            "indexes": [
                { "fields": ["email"] }
            ]
        }),
        after: serde_json::json!({}),
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Up migration should DROP INDEX
    let up_content = &files[0].content;
    assert!(up_content.contains("DROP INDEX"), "Expected DROP INDEX in up migration, got: {}", up_content);

    // Down migration should CREATE INDEX
    let down_content = &files[1].content;
    assert!(down_content.contains("CREATE INDEX"), "Expected CREATE INDEX in down migration, got: {}", down_content);
}

#[test]
fn test_migrate_model_config_changed_primary_key_added() {
    use cdm_plugin_interface::TypeExpression;

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "id".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "number".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({}),
                entity_id: local_id(2),
            }],
            config: serde_json::json!({
                "indexes": [
                    { "fields": ["id"], "primary": true }
                ]
            }),
        },
    );

    let schema = Schema {
        type_aliases: HashMap::new(),
        models,
    };

    // Simulate adding a primary key
    let deltas = vec![Delta::ModelConfigChanged {
        model: "User".to_string(),
        before: serde_json::json!({}),
        after: serde_json::json!({
            "indexes": [
                { "fields": ["id"], "primary": true }
            ]
        }),
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    // Primary key changes should show manual migration comment
    let up_content = &files[0].content;
    assert!(up_content.contains("Primary key change requires manual migration"),
        "Expected manual migration comment, got: {}", up_content);
}

#[test]
fn test_migrate_model_config_changed_multiple_index_changes() {
    use cdm_plugin_interface::TypeExpression;

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "email".to_string(),
                    field_type: TypeExpression::Identifier { name: "string".to_string() },
                    optional: false,
                    default: None,
                    config: serde_json::json!({}),
                    entity_id: local_id(2),
                },
                FieldDefinition {
                    name: "name".to_string(),
                    field_type: TypeExpression::Identifier { name: "string".to_string() },
                    optional: false,
                    default: None,
                    config: serde_json::json!({}),
                    entity_id: local_id(3),
                },
            ],
            config: serde_json::json!({
                "indexes": [
                    { "fields": ["name"] }
                ]
            }),
        },
    );

    let schema = Schema {
        type_aliases: HashMap::new(),
        models,
    };

    // Simulate: remove email index, keep name index (which exists in both)
    // before: email index only
    // after: name index only
    let deltas = vec![Delta::ModelConfigChanged {
        model: "User".to_string(),
        before: serde_json::json!({
            "indexes": [
                { "fields": ["email"] }
            ]
        }),
        after: serde_json::json!({
            "indexes": [
                { "fields": ["name"] }
            ]
        }),
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);
    assert_eq!(files.len(), 2);

    let up_content = &files[0].content;
    // Should drop the email index
    assert!(up_content.contains("DROP INDEX"), "Expected DROP INDEX in up migration");
    // Should create the name index
    assert!(up_content.contains("CREATE INDEX"), "Expected CREATE INDEX in up migration");
}

#[test]
fn test_migrate_model_config_changed_index_with_schema_prefix() {
    use cdm_plugin_interface::TypeExpression;

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "email".to_string(),
                field_type: TypeExpression::Identifier { name: "string".to_string() },
                optional: false,
                default: None,
                config: serde_json::json!({}),
                entity_id: local_id(2),
            }],
            config: serde_json::json!({
                "indexes": [
                    { "fields": ["email"] }
                ]
            }),
        },
    );

    let schema = Schema {
        type_aliases: HashMap::new(),
        models,
    };

    let deltas = vec![Delta::ModelConfigChanged {
        model: "User".to_string(),
        before: serde_json::json!({}),
        after: serde_json::json!({
            "indexes": [
                { "fields": ["email"] }
            ]
        }),
    }];

    let config = serde_json::json!({
        "dialect": "postgresql",
        "schema": "public",
        "pluralize_table_names": false
    });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);

    let up_content = &files[0].content;
    assert!(up_content.contains("\"public\".\"user\""),
        "Expected schema prefix in CREATE INDEX, got: {}", up_content);

    let down_content = &files[1].content;
    assert!(down_content.contains("\"public\"."),
        "Expected schema prefix in DROP INDEX, got: {}", down_content);
}

#[test]
fn test_migrate_model_config_changed_sqlite_index() {
    use cdm_plugin_interface::TypeExpression;

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "email".to_string(),
                field_type: TypeExpression::Identifier { name: "string".to_string() },
                optional: false,
                default: None,
                config: serde_json::json!({}),
                entity_id: local_id(2),
            }],
            config: serde_json::json!({
                "indexes": [
                    { "fields": ["email"] }
                ]
            }),
        },
    );

    let schema = Schema {
        type_aliases: HashMap::new(),
        models,
    };

    let deltas = vec![Delta::ModelConfigChanged {
        model: "User".to_string(),
        before: serde_json::json!({}),
        after: serde_json::json!({
            "indexes": [
                { "fields": ["email"] }
            ]
        }),
    }];

    let config = serde_json::json!({ "dialect": "sqlite", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);

    let up_content = &files[0].content;
    assert!(up_content.contains("CREATE INDEX"),
        "Expected CREATE INDEX for SQLite, got: {}", up_content);

    let down_content = &files[1].content;
    assert!(down_content.contains("DROP INDEX"),
        "Expected DROP INDEX for SQLite, got: {}", down_content);
}

#[test]
fn test_migrate_field_added_with_skip_true_should_not_generate_sql() {
    use cdm_plugin_interface::TypeExpression;

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "id".to_string(),
                    field_type: TypeExpression::Identifier { name: "number".to_string() },
                    optional: false,
                    default: None,
                    config: serde_json::json!({}),
                    entity_id: local_id(2),
                },
                // Field with skip: true - should NOT appear in migrations
                FieldDefinition {
                    name: "posts".to_string(),
                    field_type: TypeExpression::Array {
                        element_type: Box::new(TypeExpression::Identifier { name: "Post".to_string() }),
                    },
                    optional: true,
                    default: None,
                    config: serde_json::json!({ "skip": true }),
                    entity_id: local_id(3),
                },
            ],
            config: serde_json::json!({}),
        },
    );

    let schema = Schema {
        type_aliases: HashMap::new(),
        models: models.clone(),
    };

    // Simulate adding a field with skip: true
    let deltas = vec![Delta::FieldAdded {
        model: "User".to_string(),
        field: "posts".to_string(),
        after: FieldDefinition {
            name: "posts".to_string(),
            field_type: TypeExpression::Array {
                element_type: Box::new(TypeExpression::Identifier { name: "Post".to_string() }),
            },
            optional: true,
            default: None,
            config: serde_json::json!({ "skip": true }),
            entity_id: local_id(3),
        },
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);

    // Should generate files but they should NOT contain ALTER TABLE ADD COLUMN for posts
    if files.is_empty() {
        // This is also acceptable - no files needed if nothing to migrate
        return;
    }

    let up_content = &files[0].content;
    assert!(
        !up_content.contains("ADD COLUMN") || !up_content.contains("posts"),
        "Fields with skip: true should NOT generate ALTER TABLE ADD COLUMN. Got: {}",
        up_content
    );
}

#[test]
fn test_migrate_field_removed_with_skip_true_should_not_generate_sql() {
    use cdm_plugin_interface::TypeExpression;

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "id".to_string(),
                    field_type: TypeExpression::Identifier { name: "number".to_string() },
                    optional: false,
                    default: None,
                    config: serde_json::json!({}),
                    entity_id: local_id(2),
                },
            ],
            config: serde_json::json!({}),
        },
    );

    let schema = Schema {
        type_aliases: HashMap::new(),
        models,
    };

    // Simulate removing a field with skip: true
    let deltas = vec![Delta::FieldRemoved {
        model: "User".to_string(),
        field: "posts".to_string(),
        before: FieldDefinition {
            name: "posts".to_string(),
            field_type: TypeExpression::Array {
                element_type: Box::new(TypeExpression::Identifier { name: "Post".to_string() }),
            },
            optional: true,
            default: None,
            config: serde_json::json!({ "skip": true }),
            entity_id: local_id(3),
        },
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);

    if files.is_empty() {
        return;
    }

    let up_content = &files[0].content;
    assert!(
        !up_content.contains("DROP COLUMN") || !up_content.contains("posts"),
        "Fields with skip: true should NOT generate ALTER TABLE DROP COLUMN. Got: {}",
        up_content
    );
}

#[test]
fn test_migrate_field_renamed_with_skip_true_should_not_generate_sql() {
    use cdm_plugin_interface::TypeExpression;

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "id".to_string(),
                    field_type: TypeExpression::Identifier { name: "number".to_string() },
                    optional: false,
                    default: None,
                    config: serde_json::json!({}),
                    entity_id: local_id(2),
                },
                FieldDefinition {
                    name: "comments".to_string(),
                    field_type: TypeExpression::Array {
                        element_type: Box::new(TypeExpression::Identifier { name: "Comment".to_string() }),
                    },
                    optional: true,
                    default: None,
                    config: serde_json::json!({ "skip": true }),
                    entity_id: local_id(3),
                },
            ],
            config: serde_json::json!({}),
        },
    );

    let schema = Schema {
        type_aliases: HashMap::new(),
        models,
    };

    // Simulate renaming a field with skip: true
    let deltas = vec![Delta::FieldRenamed {
        model: "User".to_string(),
        old_name: "posts".to_string(),
        new_name: "comments".to_string(),
        id: local_id(3),
        before: FieldDefinition {
            name: "posts".to_string(),
            field_type: TypeExpression::Array {
                element_type: Box::new(TypeExpression::Identifier { name: "Post".to_string() }),
            },
            optional: true,
            default: None,
            config: serde_json::json!({ "skip": true }),
            entity_id: local_id(3),
        },
        after: FieldDefinition {
            name: "comments".to_string(),
            field_type: TypeExpression::Array {
                element_type: Box::new(TypeExpression::Identifier { name: "Comment".to_string() }),
            },
            optional: true,
            default: None,
            config: serde_json::json!({ "skip": true }),
            entity_id: local_id(3),
        },
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);

    if files.is_empty() {
        return;
    }

    let up_content = &files[0].content;
    assert!(
        !up_content.contains("RENAME COLUMN"),
        "Fields with skip: true should NOT generate ALTER TABLE RENAME COLUMN. Got: {}",
        up_content
    );
}

#[test]
fn test_migrate_field_added_with_nested_sql_skip_true_should_not_generate_sql() {
    use cdm_plugin_interface::TypeExpression;

    // This test matches the real-world usage pattern where skip is nested inside sql config:
    // @sql { skip: true }
    // Which produces: { "sql": { "skip": true } }

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "id".to_string(),
                    field_type: TypeExpression::Identifier { name: "number".to_string() },
                    optional: false,
                    default: None,
                    config: serde_json::json!({}),
                    entity_id: local_id(2),
                },
                // Field with @sql { skip: true } - should NOT appear in migrations
                FieldDefinition {
                    name: "identities".to_string(),
                    field_type: TypeExpression::Array {
                        element_type: Box::new(TypeExpression::Identifier { name: "Identity".to_string() }),
                    },
                    optional: true,
                    default: None,
                    config: serde_json::json!({
                        "sql": { "skip": true },
                        "typeorm": {
                            "relation": {
                                "type": "one_to_many",
                                "inverse_side": "user"
                            }
                        }
                    }),
                    entity_id: local_id(3),
                },
            ],
            config: serde_json::json!({}),
        },
    );

    let schema = Schema {
        type_aliases: HashMap::new(),
        models: models.clone(),
    };

    // Simulate adding a relation field with nested sql.skip: true
    let deltas = vec![Delta::FieldAdded {
        model: "User".to_string(),
        field: "identities".to_string(),
        after: FieldDefinition {
            name: "identities".to_string(),
            field_type: TypeExpression::Array {
                element_type: Box::new(TypeExpression::Identifier { name: "Identity".to_string() }),
            },
            optional: true,
            default: None,
            config: serde_json::json!({
                "sql": { "skip": true },
                "typeorm": {
                    "relation": {
                        "type": "one_to_many",
                        "inverse_side": "user"
                    }
                }
            }),
            entity_id: local_id(3),
        },
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);

    // The migration should NOT contain ALTER TABLE ADD COLUMN for the skipped field
    for file in &files {
        // Check that we don't have ALTER TABLE ... ADD COLUMN ... "identities"
        assert!(
            !file.content.contains("\"identities\""),
            "Fields with sql.skip: true should NOT appear in migrations. Got:\n{}",
            file.content
        );
    }
}

#[test]
fn test_migrate_field_added_with_unwrapped_skip_config() {
    use cdm_plugin_interface::TypeExpression;

    // This test uses the UNWRAPPED config format that the plugin actually receives
    // after transform_deltas_for_plugin() extracts the plugin-specific config.
    //
    // In the CDM flow:
    // 1. Raw config: { "sql": { "skip": true }, "typeorm": { ... } }
    // 2. After unwrap for sql plugin: { "skip": true }
    //
    // This is the critical test case for the bug fix.

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            entity_id: local_id(1),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "id".to_string(),
                    field_type: TypeExpression::Identifier { name: "number".to_string() },
                    optional: false,
                    default: None,
                    config: serde_json::json!({}),
                    entity_id: local_id(2),
                },
                FieldDefinition {
                    name: "projects".to_string(),
                    field_type: TypeExpression::Array {
                        element_type: Box::new(TypeExpression::Identifier { name: "Project".to_string() }),
                    },
                    optional: true,
                    default: None,
                    // UNWRAPPED config - this is what the plugin receives
                    config: serde_json::json!({ "skip": true }),
                    entity_id: local_id(3),
                },
            ],
            config: serde_json::json!({}),
        },
    );

    let schema = Schema {
        type_aliases: HashMap::new(),
        models: models.clone(),
    };

    // Delta with UNWRAPPED config (what the plugin receives)
    let deltas = vec![Delta::FieldAdded {
        model: "User".to_string(),
        field: "projects".to_string(),
        after: FieldDefinition {
            name: "projects".to_string(),
            field_type: TypeExpression::Array {
                element_type: Box::new(TypeExpression::Identifier { name: "Project".to_string() }),
            },
            optional: true,
            default: None,
            // UNWRAPPED config - skip is at top level
            config: serde_json::json!({ "skip": true }),
            entity_id: local_id(3),
        },
    }];

    let config = serde_json::json!({ "dialect": "postgresql", "pluralize_table_names": false });
    let utils = Utils;

    let files = migrate(schema, deltas, config, &utils);

    // The migration should NOT contain ALTER TABLE ADD COLUMN for the skipped field
    for file in &files {
        assert!(
            !file.content.contains("\"projects\""),
            "Fields with skip: true should NOT appear in migrations (unwrapped config test). Got:\n{}",
            file.content
        );
    }
}
