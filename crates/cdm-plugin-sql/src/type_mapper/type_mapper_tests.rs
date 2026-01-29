use super::*;
use cdm_plugin_interface::TypeAliasDefinition;
use serde_json::json;
use std::collections::HashMap;

fn empty_type_aliases() -> HashMap<String, TypeAliasDefinition> {
    HashMap::new()
}

#[test]
fn test_dialect_from_config() {
    let config = json!({ "dialect": "postgresql" });
    assert_eq!(Dialect::from_config(&config), Dialect::PostgreSQL);

    let config = json!({ "dialect": "sqlite" });
    assert_eq!(Dialect::from_config(&config), Dialect::SQLite);

    let config = json!({});
    assert_eq!(Dialect::from_config(&config), Dialect::PostgreSQL);
}

#[test]
fn test_type_mapper_postgresql_string() {
    let type_aliases = empty_type_aliases();
    let config = json!({ "dialect": "postgresql" });
    let mapper = TypeMapper::new(&config, &type_aliases);

    let sql_type = mapper.map_type(&TypeExpression::Identifier { name: "string".to_string() }, false);
    assert_eq!(sql_type, "VARCHAR(255)");

    let config = json!({
        "dialect": "postgresql",
        "default_string_length": 500
    });
    let mapper = TypeMapper::new(&config, &type_aliases);
    let sql_type = mapper.map_type(&TypeExpression::Identifier { name: "string".to_string() }, false);
    assert_eq!(sql_type, "VARCHAR(500)");
}

#[test]
fn test_type_mapper_sqlite_string() {
    let type_aliases = empty_type_aliases();
    let config = json!({ "dialect": "sqlite" });
    let mapper = TypeMapper::new(&config, &type_aliases);

    let sql_type = mapper.map_type(&TypeExpression::Identifier { name: "string".to_string() }, false);
    assert_eq!(sql_type, "TEXT");
}

#[test]
fn test_type_mapper_postgresql_number() {
    let type_aliases = empty_type_aliases();
    let config = json!({ "dialect": "postgresql" });
    let mapper = TypeMapper::new(&config, &type_aliases);
    let sql_type = mapper.map_type(&TypeExpression::Identifier { name: "number".to_string() }, false);
    assert_eq!(sql_type, "DOUBLE PRECISION");

    let config = json!({
        "dialect": "postgresql",
        "number_type": "real"
    });
    let mapper = TypeMapper::new(&config, &type_aliases);
    let sql_type = mapper.map_type(&TypeExpression::Identifier { name: "number".to_string() }, false);
    assert_eq!(sql_type, "REAL");

    let config = json!({
        "dialect": "postgresql",
        "number_type": "numeric"
    });
    let mapper = TypeMapper::new(&config, &type_aliases);
    let sql_type = mapper.map_type(&TypeExpression::Identifier { name: "number".to_string() }, false);
    assert_eq!(sql_type, "NUMERIC");
}

#[test]
fn test_type_mapper_sqlite_number() {
    let type_aliases = empty_type_aliases();
    let config = json!({ "dialect": "sqlite" });
    let mapper = TypeMapper::new(&config, &type_aliases);
    let sql_type = mapper.map_type(&TypeExpression::Identifier { name: "number".to_string() }, false);
    assert_eq!(sql_type, "REAL");

    let config = json!({
        "dialect": "sqlite",
        "number_type": "numeric"
    });
    let mapper = TypeMapper::new(&config, &type_aliases);
    let sql_type = mapper.map_type(&TypeExpression::Identifier { name: "number".to_string() }, false);
    assert_eq!(sql_type, "NUMERIC");
}

#[test]
fn test_type_mapper_boolean() {
    let type_aliases = empty_type_aliases();
    let config = json!({ "dialect": "postgresql" });
    let mapper = TypeMapper::new(&config, &type_aliases);
    let sql_type = mapper.map_type(&TypeExpression::Identifier { name: "boolean".to_string() }, false);
    assert_eq!(sql_type, "BOOLEAN");

    let config = json!({ "dialect": "sqlite" });
    let mapper = TypeMapper::new(&config, &type_aliases);
    let sql_type = mapper.map_type(&TypeExpression::Identifier { name: "boolean".to_string() }, false);
    assert_eq!(sql_type, "INTEGER");
}

#[test]
fn test_type_mapper_json() {
    let type_aliases = empty_type_aliases();
    let config = json!({ "dialect": "postgresql" });
    let mapper = TypeMapper::new(&config, &type_aliases);
    let sql_type = mapper.map_type(&TypeExpression::Identifier { name: "JSON".to_string() }, false);
    assert_eq!(sql_type, "JSONB");

    let config = json!({ "dialect": "sqlite" });
    let mapper = TypeMapper::new(&config, &type_aliases);
    let sql_type = mapper.map_type(&TypeExpression::Identifier { name: "JSON".to_string() }, false);
    assert_eq!(sql_type, "TEXT");
}

#[test]
fn test_type_mapper_array_postgresql() {
    let type_aliases = empty_type_aliases();
    let config = json!({ "dialect": "postgresql" });
    let mapper = TypeMapper::new(&config, &type_aliases);

    let sql_type = mapper.map_type(
        &TypeExpression::Array {
            element_type: Box::new(TypeExpression::Identifier { name: "string".to_string() })
        },
        false,
    );
    assert_eq!(sql_type, "VARCHAR(255)[]");

    let sql_type = mapper.map_type(
        &TypeExpression::Array {
            element_type: Box::new(TypeExpression::Identifier { name: "number".to_string() })
        },
        false,
    );
    assert_eq!(sql_type, "DOUBLE PRECISION[]");
}

#[test]
fn test_type_mapper_array_sqlite() {
    let type_aliases = empty_type_aliases();
    let config = json!({ "dialect": "sqlite" });
    let mapper = TypeMapper::new(&config, &type_aliases);

    let sql_type = mapper.map_type(
        &TypeExpression::Array {
            element_type: Box::new(TypeExpression::Identifier { name: "string".to_string() })
        },
        false,
    );
    assert_eq!(sql_type, "TEXT");
}

#[test]
fn test_type_mapper_model_reference() {
    let type_aliases = empty_type_aliases();
    let config = json!({ "dialect": "postgresql" });
    let mapper = TypeMapper::new(&config, &type_aliases);

    let sql_type = mapper.map_type(
        &TypeExpression::Identifier { name: "User".to_string() },
        false,
    );
    assert_eq!(sql_type, "JSONB");

    let config = json!({ "dialect": "sqlite" });
    let mapper = TypeMapper::new(&config, &type_aliases);
    let sql_type = mapper.map_type(
        &TypeExpression::Identifier { name: "User".to_string() },
        false,
    );
    assert_eq!(sql_type, "TEXT");
}

#[test]
fn test_type_mapper_union() {
    let type_aliases = empty_type_aliases();
    let config = json!({ "dialect": "postgresql" });
    let mapper = TypeMapper::new(&config, &type_aliases);

    let sql_type = mapper.map_type(
        &TypeExpression::Union {
            types: vec![
                TypeExpression::StringLiteral { value: "active".to_string() },
                TypeExpression::StringLiteral { value: "inactive".to_string() },
            ]
        },
        false,
    );
    assert_eq!(sql_type, "VARCHAR(255)");
}

#[test]
fn test_type_mapper_type_alias_with_sql_type_override() {
    // Test that a type alias with @sql { type: "INTEGER" } uses the override
    let mut type_aliases = HashMap::new();
    type_aliases.insert(
        "ID".to_string(),
        TypeAliasDefinition {
            name: "ID".to_string(),
            alias_type: TypeExpression::Identifier { name: "number".to_string() },
            config: json!({ "type": "INTEGER" }),
            entity_id: None,
        },
    );

    let config = json!({ "dialect": "postgresql" });
    let mapper = TypeMapper::new(&config, &type_aliases);

    let sql_type = mapper.map_type(
        &TypeExpression::Identifier { name: "ID".to_string() },
        false,
    );
    assert_eq!(sql_type, "INTEGER");
}

#[test]
fn test_type_mapper_type_alias_without_sql_type_override() {
    // Test that a type alias without @sql { type } falls back to underlying type
    let mut type_aliases = HashMap::new();
    type_aliases.insert(
        "UserName".to_string(),
        TypeAliasDefinition {
            name: "UserName".to_string(),
            alias_type: TypeExpression::Identifier { name: "string".to_string() },
            config: json!({}),
            entity_id: None,
        },
    );

    let config = json!({ "dialect": "postgresql" });
    let mapper = TypeMapper::new(&config, &type_aliases);

    let sql_type = mapper.map_type(
        &TypeExpression::Identifier { name: "UserName".to_string() },
        false,
    );
    assert_eq!(sql_type, "VARCHAR(255)");
}

#[test]
fn test_type_mapper_nested_type_alias() {
    // Test that nested type aliases are resolved correctly
    let mut type_aliases = HashMap::new();
    type_aliases.insert(
        "BaseID".to_string(),
        TypeAliasDefinition {
            name: "BaseID".to_string(),
            alias_type: TypeExpression::Identifier { name: "number".to_string() },
            config: json!({ "type": "BIGINT" }),
            entity_id: None,
        },
    );
    type_aliases.insert(
        "UserID".to_string(),
        TypeAliasDefinition {
            name: "UserID".to_string(),
            alias_type: TypeExpression::Identifier { name: "BaseID".to_string() },
            config: json!({}),  // No override, should inherit from BaseID
            entity_id: None,
        },
    );

    let config = json!({ "dialect": "postgresql" });
    let mapper = TypeMapper::new(&config, &type_aliases);

    // BaseID should use its explicit type
    let sql_type = mapper.map_type(
        &TypeExpression::Identifier { name: "BaseID".to_string() },
        false,
    );
    assert_eq!(sql_type, "BIGINT");

    // UserID should resolve through BaseID
    let sql_type = mapper.map_type(
        &TypeExpression::Identifier { name: "UserID".to_string() },
        false,
    );
    assert_eq!(sql_type, "BIGINT");
}

#[test]
fn test_type_mapper_template_types_with_sql_config() {
    // Simulate template types like sql.UUID, sql.TimestampTZ, etc. that should
    // map to specific SQL types rather than falling back to VARCHAR
    let mut type_aliases = HashMap::new();

    // UUID: string with @sql { type: "UUID" }
    type_aliases.insert(
        "UUID".to_string(),
        TypeAliasDefinition {
            name: "UUID".to_string(),
            alias_type: TypeExpression::Identifier { name: "string".to_string() },
            config: json!({ "type": "UUID" }),
            entity_id: None,
        },
    );

    // TimestampTZ: string with @sql { type: "TIMESTAMPTZ" }
    type_aliases.insert(
        "TimestampTZ".to_string(),
        TypeAliasDefinition {
            name: "TimestampTZ".to_string(),
            alias_type: TypeExpression::Identifier { name: "string".to_string() },
            config: json!({ "type": "TIMESTAMPTZ" }),
            entity_id: None,
        },
    );

    // JSONB: string with @sql { type: "JSONB" }
    type_aliases.insert(
        "JSONB".to_string(),
        TypeAliasDefinition {
            name: "JSONB".to_string(),
            alias_type: TypeExpression::Identifier { name: "string".to_string() },
            config: json!({ "type": "JSONB" }),
            entity_id: None,
        },
    );

    // Varchar: string with @sql { type: "VARCHAR" }
    type_aliases.insert(
        "Varchar".to_string(),
        TypeAliasDefinition {
            name: "Varchar".to_string(),
            alias_type: TypeExpression::Identifier { name: "string".to_string() },
            config: json!({ "type": "VARCHAR" }),
            entity_id: None,
        },
    );

    let config = json!({ "dialect": "postgresql" });
    let mapper = TypeMapper::new(&config, &type_aliases);

    // Test each template type maps to correct SQL type
    let uuid_type = mapper.map_type(
        &TypeExpression::Identifier { name: "UUID".to_string() },
        false,
    );
    assert_eq!(uuid_type, "UUID");

    let timestamp_type = mapper.map_type(
        &TypeExpression::Identifier { name: "TimestampTZ".to_string() },
        false,
    );
    assert_eq!(timestamp_type, "TIMESTAMPTZ");

    let jsonb_type = mapper.map_type(
        &TypeExpression::Identifier { name: "JSONB".to_string() },
        false,
    );
    assert_eq!(jsonb_type, "JSONB");

    let varchar_type = mapper.map_type(
        &TypeExpression::Identifier { name: "Varchar".to_string() },
        false,
    );
    assert_eq!(varchar_type, "VARCHAR");
}
