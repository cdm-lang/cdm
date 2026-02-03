use super::*;
use serde_json::json;

#[test]
fn test_apply_name_format() {
    assert_eq!(apply_name_format("UserProfile", "snake_case"), "user_profile");
    assert_eq!(apply_name_format("UserProfile", "preserve"), "UserProfile");
    assert_eq!(apply_name_format("user_profile", "pascal_case"), "UserProfile");
    assert_eq!(apply_name_format("user_profile", "camel_case"), "userProfile");
}

#[test]
fn test_get_table_name_with_override() {
    let model_config = json!({ "table_name": "custom_users" });
    let global_config = json!({});

    let name = get_table_name("User", &model_config, &global_config);
    assert_eq!(name, "custom_users");
}

#[test]
fn test_get_table_name_with_formatting() {
    let model_config = json!({});
    let global_config = json!({
        "table_name_format": "snake_case",
        "pluralize_table_names": false  // Disable for this test
    });

    let name = get_table_name("UserProfile", &model_config, &global_config);
    assert_eq!(name, "user_profile");
}

#[test]
fn test_get_table_name_with_pluralization() {
    let model_config = json!({});
    let global_config = json!({
        "table_name_format": "snake_case",
        "pluralize_table_names": true
    });

    let name = get_table_name("User", &model_config, &global_config);
    assert_eq!(name, "users");
}

#[test]
fn test_get_table_name_with_pluralization_default() {
    let model_config = json!({});
    let global_config = json!({
        "table_name_format": "snake_case"
        // pluralize_table_names not specified, should default to true
    });

    let name = get_table_name("Category", &model_config, &global_config);
    assert_eq!(name, "categories");
}

#[test]
fn test_get_table_name_pluralization_disabled() {
    let model_config = json!({});
    let global_config = json!({
        "table_name_format": "snake_case",
        "pluralize_table_names": false
    });

    let name = get_table_name("User", &model_config, &global_config);
    assert_eq!(name, "user");
}

#[test]
fn test_get_table_name_pluralization_irregular() {
    let model_config = json!({});
    let global_config = json!({
        "table_name_format": "snake_case",
        "pluralize_table_names": true
    });

    let name = get_table_name("Person", &model_config, &global_config);
    assert_eq!(name, "people");
}

#[test]
fn test_get_table_name_override_ignores_pluralization() {
    let model_config = json!({ "table_name": "my_custom_table" });
    let global_config = json!({
        "table_name_format": "snake_case",
        "pluralize_table_names": true
    });

    let name = get_table_name("User", &model_config, &global_config);
    assert_eq!(name, "my_custom_table");
}

#[test]
fn test_get_column_name_with_override() {
    let field_config = json!({ "column_name": "custom_id" });
    let global_config = json!({});

    let name = get_column_name("userId", &field_config, &global_config);
    assert_eq!(name, "custom_id");
}

#[test]
fn test_get_column_name_with_formatting() {
    let field_config = json!({});
    let global_config = json!({ "column_name_format": "snake_case" });

    let name = get_column_name("firstName", &field_config, &global_config);
    assert_eq!(name, "first_name");
}

#[test]
fn test_quote_identifier() {
    assert_eq!(quote_identifier("user", Dialect::PostgreSQL), "\"user\"");
    assert_eq!(quote_identifier("user", Dialect::SQLite), "\"user\"");
    assert_eq!(quote_identifier("first_name", Dialect::PostgreSQL), "\"first_name\"");
}

#[test]
fn test_format_default_value() {
    use cdm_plugin_interface::Value;

    assert_eq!(format_default_value(&Value::String("hello".to_string())), "'hello'");
    assert_eq!(format_default_value(&Value::String("it's".to_string())), "'it''s'");
    assert_eq!(format_default_value(&Value::Number(42.0)), "42");
    assert_eq!(format_default_value(&Value::Boolean(true)), "TRUE");
    assert_eq!(format_default_value(&Value::Boolean(false)), "FALSE");
    assert_eq!(format_default_value(&Value::Null), "NULL");
}

// ============================================================================
// extract_indexes tests
// ============================================================================

#[test]
fn test_extract_indexes_empty_config() {
    let config = json!({});
    let indexes = extract_indexes(&config, "users");
    assert!(indexes.is_empty());
}

#[test]
fn test_extract_indexes_single_regular_index() {
    let config = json!({
        "indexes": {
            "email_idx": { "fields": ["email"] }
        }
    });
    let indexes = extract_indexes(&config, "users");

    assert_eq!(indexes.len(), 1);
    assert_eq!(indexes[0].name, "email_idx");
    assert_eq!(indexes[0].fields, vec!["email"]);
    assert!(!indexes[0].is_unique);
    assert!(!indexes[0].is_primary);
}

#[test]
fn test_extract_indexes_unique_index() {
    let config = json!({
        "indexes": {
            "email_unique": { "fields": ["email"], "unique": true }
        }
    });
    let indexes = extract_indexes(&config, "users");

    assert_eq!(indexes.len(), 1);
    assert!(indexes[0].is_unique);
    assert!(!indexes[0].is_primary);
}

#[test]
fn test_extract_indexes_primary_key() {
    let config = json!({
        "indexes": {
            "primary": { "fields": ["id"], "primary": true }
        }
    });
    let indexes = extract_indexes(&config, "users");

    assert_eq!(indexes.len(), 1);
    assert!(indexes[0].is_primary);
    assert!(!indexes[0].is_unique);
}

#[test]
fn test_extract_indexes_composite_index() {
    let config = json!({
        "indexes": {
            "name_idx": { "fields": ["first_name", "last_name"] }
        }
    });
    let indexes = extract_indexes(&config, "users");

    assert_eq!(indexes.len(), 1);
    assert_eq!(indexes[0].fields, vec!["first_name", "last_name"]);
}

#[test]
fn test_extract_indexes_with_custom_name() {
    // In keyed object format, the key IS the name
    let config = json!({
        "indexes": {
            "custom_email_idx": { "fields": ["email"] }
        }
    });
    let indexes = extract_indexes(&config, "users");

    assert_eq!(indexes[0].name, "custom_email_idx");
}

#[test]
fn test_extract_indexes_with_method_and_where() {
    let config = json!({
        "indexes": {
            "email_partial": {
                "fields": ["email"],
                "method": "btree",
                "where": "deleted_at IS NULL"
            }
        }
    });
    let indexes = extract_indexes(&config, "users");

    assert_eq!(indexes[0].method, Some("btree".to_string()));
    assert_eq!(indexes[0].where_clause, Some("deleted_at IS NULL".to_string()));
}

#[test]
fn test_extract_indexes_multiple() {
    let config = json!({
        "indexes": {
            "primary": { "fields": ["id"], "primary": true },
            "email_unique": { "fields": ["email"], "unique": true },
            "created_at_idx": { "fields": ["created_at"] }
        }
    });
    let indexes = extract_indexes(&config, "users");

    assert_eq!(indexes.len(), 3);
    // Note: HashMap iteration order is not guaranteed, so we check by finding each index
    let primary_idx = indexes.iter().find(|i| i.is_primary).expect("Should have primary");
    let unique_idx = indexes.iter().find(|i| i.is_unique).expect("Should have unique");
    let regular_idx = indexes.iter().find(|i| !i.is_primary && !i.is_unique).expect("Should have regular");

    assert_eq!(primary_idx.fields, vec!["id"]);
    assert_eq!(unique_idx.fields, vec!["email"]);
    assert_eq!(regular_idx.fields, vec!["created_at"]);
}

#[test]
fn test_extract_indexes_skips_empty_fields() {
    let config = json!({
        "indexes": {
            "empty_idx": { "fields": [] },
            "email_idx": { "fields": ["email"] }
        }
    });
    let indexes = extract_indexes(&config, "users");

    assert_eq!(indexes.len(), 1);
    assert_eq!(indexes[0].fields, vec!["email"]);
}

// ============================================================================
// generate_create_index_sql tests
// ============================================================================

#[test]
fn test_generate_create_index_sql_regular_postgres() {
    let index = IndexInfo {
        name: "idx_users_0".to_string(),
        fields: vec!["email".to_string()],
        is_unique: false,
        is_primary: false,
        method: None,
        where_clause: None,
    };

    let sql = generate_create_index_sql(&index, "users", "", Dialect::PostgreSQL);
    assert_eq!(sql, "CREATE INDEX \"idx_users_0\" ON \"users\" (\"email\");\n");
}

#[test]
fn test_generate_create_index_sql_unique_postgres() {
    let index = IndexInfo {
        name: "idx_users_email".to_string(),
        fields: vec!["email".to_string()],
        is_unique: true,
        is_primary: false,
        method: None,
        where_clause: None,
    };

    let sql = generate_create_index_sql(&index, "users", "", Dialect::PostgreSQL);
    assert_eq!(sql, "CREATE UNIQUE INDEX \"idx_users_email\" ON \"users\" (\"email\");\n");
}

#[test]
fn test_generate_create_index_sql_with_schema_prefix() {
    let index = IndexInfo {
        name: "idx_users_0".to_string(),
        fields: vec!["email".to_string()],
        is_unique: false,
        is_primary: false,
        method: None,
        where_clause: None,
    };

    let sql = generate_create_index_sql(&index, "users", "\"public\".", Dialect::PostgreSQL);
    assert!(sql.contains("\"public\".\"users\""));
}

#[test]
fn test_generate_create_index_sql_composite() {
    let index = IndexInfo {
        name: "idx_users_name".to_string(),
        fields: vec!["first_name".to_string(), "last_name".to_string()],
        is_unique: false,
        is_primary: false,
        method: None,
        where_clause: None,
    };

    let sql = generate_create_index_sql(&index, "users", "", Dialect::PostgreSQL);
    assert!(sql.contains("(\"first_name\", \"last_name\")"));
}

#[test]
fn test_generate_create_index_sql_with_method() {
    let index = IndexInfo {
        name: "idx_users_0".to_string(),
        fields: vec!["email".to_string()],
        is_unique: false,
        is_primary: false,
        method: Some("btree".to_string()),
        where_clause: None,
    };

    let sql = generate_create_index_sql(&index, "users", "", Dialect::PostgreSQL);
    assert!(sql.contains("USING BTREE"));
}

#[test]
fn test_generate_create_index_sql_with_where_clause() {
    let index = IndexInfo {
        name: "idx_users_active".to_string(),
        fields: vec!["email".to_string()],
        is_unique: false,
        is_primary: false,
        method: None,
        where_clause: Some("deleted_at IS NULL".to_string()),
    };

    let sql = generate_create_index_sql(&index, "users", "", Dialect::PostgreSQL);
    assert!(sql.contains("WHERE deleted_at IS NULL"));
}

#[test]
fn test_generate_create_index_sql_sqlite() {
    let index = IndexInfo {
        name: "idx_users_0".to_string(),
        fields: vec!["email".to_string()],
        is_unique: false,
        is_primary: false,
        method: Some("btree".to_string()), // Should be ignored for SQLite
        where_clause: Some("active = 1".to_string()), // Should be ignored for SQLite
    };

    let sql = generate_create_index_sql(&index, "users", "", Dialect::SQLite);
    assert!(!sql.contains("USING"));
    assert!(!sql.contains("WHERE"));
    assert_eq!(sql, "CREATE INDEX \"idx_users_0\" ON \"users\" (\"email\");\n");
}

// ============================================================================
// generate_drop_index_sql tests
// ============================================================================

#[test]
fn test_generate_drop_index_sql_postgres() {
    let index = IndexInfo {
        name: "idx_users_0".to_string(),
        fields: vec!["email".to_string()],
        is_unique: false,
        is_primary: false,
        method: None,
        where_clause: None,
    };

    let sql = generate_drop_index_sql(&index, "", Dialect::PostgreSQL);
    assert_eq!(sql, "DROP INDEX \"idx_users_0\";\n");
}

#[test]
fn test_generate_drop_index_sql_postgres_with_schema() {
    let index = IndexInfo {
        name: "idx_users_0".to_string(),
        fields: vec!["email".to_string()],
        is_unique: false,
        is_primary: false,
        method: None,
        where_clause: None,
    };

    let sql = generate_drop_index_sql(&index, "\"public\".", Dialect::PostgreSQL);
    assert_eq!(sql, "DROP INDEX \"public\".\"idx_users_0\";\n");
}

#[test]
fn test_generate_drop_index_sql_sqlite() {
    let index = IndexInfo {
        name: "idx_users_0".to_string(),
        fields: vec!["email".to_string()],
        is_unique: false,
        is_primary: false,
        method: None,
        where_clause: None,
    };

    let sql = generate_drop_index_sql(&index, "", Dialect::SQLite);
    assert_eq!(sql, "DROP INDEX \"idx_users_0\";\n");
}

#[test]
fn test_generate_drop_index_sql_unique() {
    let index = IndexInfo {
        name: "idx_users_email".to_string(),
        fields: vec!["email".to_string()],
        is_unique: true,
        is_primary: false,
        method: None,
        where_clause: None,
    };

    let sql = generate_drop_index_sql(&index, "", Dialect::PostgreSQL);
    assert_eq!(sql, "DROP INDEX \"idx_users_email\";\n");
}

#[test]
fn test_should_skip_field_top_level() {
    // Test top-level skip: true
    let config = json!({ "skip": true });
    assert!(should_skip_field(&config), "should_skip_field should return true for top-level skip: true");

    let config = json!({ "skip": false });
    assert!(!should_skip_field(&config), "should_skip_field should return false for top-level skip: false");
}

#[test]
fn test_should_skip_field_nested_in_sql() {
    // Test nested sql.skip: true (this is the real-world format from @sql { skip: true })
    let config = json!({
        "sql": { "skip": true },
        "typeorm": { "type": "varchar" }
    });
    assert!(should_skip_field(&config), "should_skip_field should return true for nested sql.skip: true");

    let config = json!({
        "sql": { "skip": false },
        "typeorm": { "type": "varchar" }
    });
    assert!(!should_skip_field(&config), "should_skip_field should return false for nested sql.skip: false");

    let config = json!({
        "sql": { "type": "VARCHAR" },
        "typeorm": { "type": "varchar" }
    });
    assert!(!should_skip_field(&config), "should_skip_field should return false when sql.skip is not present");
}

#[test]
fn test_should_skip_field_empty_config() {
    let config = json!({});
    assert!(!should_skip_field(&config), "should_skip_field should return false for empty config");
}

#[test]
fn test_should_skip_field_real_world_relation_config() {
    // This matches the exact config structure from the bug report:
    // user?: User {
    //   @sql { skip: true }
    //   @typeorm { relation: { type: "many_to_one", ... } }
    // }
    let config = json!({
        "sql": {
            "skip": true
        },
        "typeorm": {
            "join_column": {
                "name": "user_id"
            },
            "relation": {
                "inverse_side": "identities",
                "on_delete": "CASCADE",
                "type": "many_to_one"
            }
        }
    });
    assert!(should_skip_field(&config), "should_skip_field should return true for real-world relation config with sql.skip: true");
}

// ============================================================================
// Foreign key references tests
// ============================================================================

#[test]
fn test_generate_create_table_with_foreign_key_reference() {
    // Test that a field with @sql { references: { table: "users", column: "id" } }
    // generates a REFERENCES clause in the CREATE TABLE statement
    use cdm_plugin_interface::{FieldDefinition, TypeExpression};

    let model = cdm_plugin_interface::ModelDefinition {
        name: "Project".to_string(),
        parents: vec![],
        fields: vec![
            FieldDefinition {
                name: "id".to_string(),
                field_type: TypeExpression::Identifier { name: "string".to_string() },
                optional: false,
                default: None,
                config: json!({ "type": "UUID" }),
                entity_id: None,
            },
            FieldDefinition {
                name: "owner_id".to_string(),
                field_type: TypeExpression::Identifier { name: "string".to_string() },
                optional: false,
                default: None,
                config: json!({
                    "type": "UUID",
                    "references": {
                        "table": "users",
                        "column": "id",
                        "on_delete": "cascade"
                    }
                }),
                entity_id: None,
            },
        ],
        config: json!({}),
        entity_id: None,
    };

    let global_config = json!({
        "dialect": "postgresql",
        "pluralize_table_names": true
    });
    let type_aliases = std::collections::HashMap::new();
    let type_mapper = TypeMapper::new(&global_config, &type_aliases);

    let sql = generate_create_table("Project", &model, &global_config, &type_mapper);

    // Should include the REFERENCES clause for owner_id
    assert!(
        sql.contains("REFERENCES") && sql.contains("\"users\""),
        "CREATE TABLE should include REFERENCES clause for foreign key. Got:\n{}",
        sql
    );
    assert!(
        sql.contains("ON DELETE CASCADE"),
        "CREATE TABLE should include ON DELETE CASCADE. Got:\n{}",
        sql
    );
}
