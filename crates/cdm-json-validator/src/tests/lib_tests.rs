use super::*;
use cdm_utils::{ResolvedField, ResolvedModel, Position, Span};
use std::collections::HashMap;

/// Helper to create a test schema with models and type aliases
fn create_test_schema() -> ResolvedSchema {
    let mut models = HashMap::new();
    let mut type_aliases = HashMap::new();

    let span = Span {
        start: Position { line: 0, column: 0 },
        end: Position { line: 0, column: 0 }
    };

    // Create User model with fields
    let user_fields = vec![
        ResolvedField::new("id".to_string(), Some("number".to_string()), false, "test.cdm".to_string(), span),
        ResolvedField::new("name".to_string(), Some("string".to_string()), false, "test.cdm".to_string(), span),
        ResolvedField::new("email".to_string(), Some("string".to_string()), true, "test.cdm".to_string(), span),
        ResolvedField::new("role".to_string(), Some("\"admin\" | \"user\"".to_string()), false, "test.cdm".to_string(), span),
    ];

    models.insert(
        "User".to_string(),
        ResolvedModel {
            name: "User".to_string(),
            fields: user_fields,
            parents: vec![],
            plugin_configs: std::collections::HashMap::new(),
            source_file: "test.cdm".to_string(),
            source_span: span,
            entity_id: None,
        },
    );

    // Create Post model with reference to User
    let post_fields = vec![
        ResolvedField::new("title".to_string(), Some("string".to_string()), false, "test.cdm".to_string(), span),
        ResolvedField::new("author".to_string(), Some("User".to_string()), false, "test.cdm".to_string(), span),
        ResolvedField::new("tags".to_string(), Some("string[]".to_string()), false, "test.cdm".to_string(), span),
    ];

    models.insert(
        "Post".to_string(),
        ResolvedModel {
            name: "Post".to_string(),
            fields: post_fields,
            parents: vec![],
            plugin_configs: std::collections::HashMap::new(),
            source_file: "test.cdm".to_string(),
            source_span: span,
            entity_id: None,
        },
    );

    // Create type alias
    let id_alias = cdm_utils::ResolvedTypeAlias::new(
        "ID".to_string(),
        "number".to_string(),
        vec![],
        "test.cdm".to_string(),
        span,
    );

    type_aliases.insert("ID".to_string(), id_alias);

    ResolvedSchema {
        models: models.clone(),
        type_aliases,
        all_models_for_inheritance: models,
    }
}

#[test]
fn test_validate_primitive_string() {
    let schema = create_test_schema();
    let json = serde_json::json!("hello");
    let errors = validate_value(&schema, &json, &ParsedType::Primitive(PrimitiveType::String), &[]);
    assert_eq!(errors.len(), 0);
}

#[test]
fn test_validate_primitive_string_wrong_type() {
    let schema = create_test_schema();
    let json = serde_json::json!(123);
    let errors = validate_value(&schema, &json, &ParsedType::Primitive(PrimitiveType::String), &[]);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].message, "Expected string, got number");
}

#[test]
fn test_validate_primitive_number() {
    let schema = create_test_schema();
    let json = serde_json::json!(42);
    let errors = validate_value(&schema, &json, &ParsedType::Primitive(PrimitiveType::Number), &[]);
    assert_eq!(errors.len(), 0);
}

#[test]
fn test_validate_primitive_boolean() {
    let schema = create_test_schema();
    let json = serde_json::json!(true);
    let errors = validate_value(&schema, &json, &ParsedType::Primitive(PrimitiveType::Boolean), &[]);
    assert_eq!(errors.len(), 0);
}

#[test]
fn test_validate_literal_match() {
    let schema = create_test_schema();
    let json = serde_json::json!("admin");
    let errors = validate_value(&schema, &json, &ParsedType::Literal("admin".to_string()), &[]);
    assert_eq!(errors.len(), 0);
}

#[test]
fn test_validate_literal_mismatch() {
    let schema = create_test_schema();
    let json = serde_json::json!("guest");
    let errors = validate_value(&schema, &json, &ParsedType::Literal("admin".to_string()), &[]);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].message, "Expected literal 'admin', got 'guest'");
}

#[test]
fn test_validate_null() {
    let schema = create_test_schema();
    let json = serde_json::json!(null);
    let errors = validate_value(&schema, &json, &ParsedType::Null, &[]);
    assert_eq!(errors.len(), 0);
}

#[test]
fn test_validate_null_wrong_type() {
    let schema = create_test_schema();
    let json = serde_json::json!("not null");
    let errors = validate_value(&schema, &json, &ParsedType::Null, &[]);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].message, "Expected null, got string");
}

#[test]
fn test_validate_array_empty() {
    let schema = create_test_schema();
    let json = serde_json::json!([]);
    let array_type = ParsedType::Array(Box::new(ParsedType::Primitive(PrimitiveType::String)));
    let errors = validate_value(&schema, &json, &array_type, &[]);
    assert_eq!(errors.len(), 0);
}

#[test]
fn test_validate_array_valid_elements() {
    let schema = create_test_schema();
    let json = serde_json::json!(["a", "b", "c"]);
    let array_type = ParsedType::Array(Box::new(ParsedType::Primitive(PrimitiveType::String)));
    let errors = validate_value(&schema, &json, &array_type, &[]);
    assert_eq!(errors.len(), 0);
}

#[test]
fn test_validate_array_invalid_element() {
    let schema = create_test_schema();
    let json = serde_json::json!(["a", 123, "c"]);
    let array_type = ParsedType::Array(Box::new(ParsedType::Primitive(PrimitiveType::String)));
    let errors = validate_value(&schema, &json, &array_type, &[]);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].path[0].name, "1");
    assert_eq!(errors[0].message, "Expected string, got number");
}

#[test]
fn test_validate_array_not_array() {
    let schema = create_test_schema();
    let json = serde_json::json!("not an array");
    let array_type = ParsedType::Array(Box::new(ParsedType::Primitive(PrimitiveType::String)));
    let errors = validate_value(&schema, &json, &array_type, &[]);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].message, "Expected array, got string");
}

#[test]
fn test_validate_union_first_type_matches() {
    let schema = create_test_schema();
    let json = serde_json::json!("hello");
    let union_type = ParsedType::Union(vec![
        ParsedType::Primitive(PrimitiveType::String),
        ParsedType::Primitive(PrimitiveType::Number),
    ]);
    let errors = validate_value(&schema, &json, &union_type, &[]);
    assert_eq!(errors.len(), 0);
}

#[test]
fn test_validate_union_second_type_matches() {
    let schema = create_test_schema();
    let json = serde_json::json!(42);
    let union_type = ParsedType::Union(vec![
        ParsedType::Primitive(PrimitiveType::String),
        ParsedType::Primitive(PrimitiveType::Number),
    ]);
    let errors = validate_value(&schema, &json, &union_type, &[]);
    assert_eq!(errors.len(), 0);
}

#[test]
fn test_validate_union_no_match() {
    let schema = create_test_schema();
    let json = serde_json::json!(true);
    let union_type = ParsedType::Union(vec![
        ParsedType::Primitive(PrimitiveType::String),
        ParsedType::Primitive(PrimitiveType::Number),
    ]);
    let errors = validate_value(&schema, &json, &union_type, &[]);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("Value does not match any type in union"));
}

#[test]
fn test_validate_json_valid_user() {
    let schema = create_test_schema();
    let json = serde_json::json!({
        "id": 1,
        "name": "Alice",
        "role": "admin"
    });
    let errors = validate_json(&schema, &json, "User");
    assert_eq!(errors.len(), 0);
}

#[test]
fn test_validate_json_valid_user_with_optional_field() {
    let schema = create_test_schema();
    let json = serde_json::json!({
        "id": 1,
        "name": "Alice",
        "email": "alice@example.com",
        "role": "user"
    });
    let errors = validate_json(&schema, &json, "User");
    assert_eq!(errors.len(), 0);
}

#[test]
fn test_validate_json_missing_required_field() {
    let schema = create_test_schema();
    let json = serde_json::json!({
        "id": 1,
        "role": "admin"
    });
    let errors = validate_json(&schema, &json, "User");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].path[0].name, "name");
    assert_eq!(errors[0].message, "Required field 'name' is missing");
}

#[test]
fn test_validate_json_unknown_field() {
    let schema = create_test_schema();
    let json = serde_json::json!({
        "id": 1,
        "name": "Alice",
        "role": "admin",
        "extra": "unexpected"
    });
    let errors = validate_json(&schema, &json, "User");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].path[0].name, "extra");
    assert_eq!(errors[0].message, "Unknown field 'extra'");
}

#[test]
fn test_validate_json_wrong_field_type() {
    let schema = create_test_schema();
    let json = serde_json::json!({
        "id": "not a number",
        "name": "Alice",
        "role": "admin"
    });
    let errors = validate_json(&schema, &json, "User");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].path[0].name, "id");
    assert_eq!(errors[0].message, "Expected number, got string");
}

#[test]
fn test_validate_json_model_not_found() {
    let schema = create_test_schema();
    let json = serde_json::json!({});
    let errors = validate_json(&schema, &json, "NonExistent");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].message, "'NonExistent' not found in schema");
}

#[test]
fn test_validate_json_not_object() {
    let schema = create_test_schema();
    let json = serde_json::json!("not an object");
    let errors = validate_json(&schema, &json, "User");
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("Expected object for model 'User'"));
}

#[test]
fn test_validate_reference_to_model() {
    let schema = create_test_schema();
    let json = serde_json::json!({
        "title": "My Post",
        "author": {
            "id": 1,
            "name": "Alice",
            "role": "admin"
        },
        "tags": ["rust", "cdm"]
    });
    let errors = validate_json(&schema, &json, "Post");
    assert_eq!(errors.len(), 0);
}

#[test]
fn test_validate_reference_to_type_alias() {
    let schema = create_test_schema();
    let json = serde_json::json!(42);
    let errors = validate_value(&schema, &json, &ParsedType::Reference("ID".to_string()), &[]);
    assert_eq!(errors.len(), 0);
}

#[test]
fn test_validate_reference_not_found() {
    let schema = create_test_schema();
    let json = serde_json::json!(42);
    let errors = validate_value(&schema, &json, &ParsedType::Reference("Unknown".to_string()), &[]);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].message, "Type 'Unknown' not found in schema");
}

#[test]
fn test_validate_nested_model_with_invalid_field() {
    let schema = create_test_schema();
    let json = serde_json::json!({
        "title": "My Post",
        "author": {
            "id": 1,
            "name": "Alice",
            "role": "invalid_role"
        },
        "tags": ["rust"]
    });
    let errors = validate_json(&schema, &json, "Post");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].path.len(), 2);
    assert_eq!(errors[0].path[0].name, "author");
    assert_eq!(errors[0].path[1].name, "role");
}

#[test]
fn test_validate_array_of_numbers() {
    let schema = create_test_schema();
    let json = serde_json::json!([1, 2, 3, 4, 5]);
    let array_type = ParsedType::Array(Box::new(ParsedType::Primitive(PrimitiveType::Number)));
    let errors = validate_value(&schema, &json, &array_type, &[]);
    assert_eq!(errors.len(), 0);
}

#[test]
fn test_validate_complex_nested_structure() {
    let schema = create_test_schema();
    let json = serde_json::json!({
        "title": "Nested Post",
        "author": {
            "id": 1,
            "name": "Bob",
            "email": "bob@example.com",
            "role": "user"
        },
        "tags": ["nested", "complex", "test"]
    });
    let errors = validate_json(&schema, &json, "Post");
    assert_eq!(errors.len(), 0);
}

#[test]
fn test_validate_literal_union() {
    let schema = create_test_schema();
    let json = serde_json::json!("admin");
    let union_type = ParsedType::Union(vec![
        ParsedType::Literal("admin".to_string()),
        ParsedType::Literal("user".to_string()),
    ]);
    let errors = validate_value(&schema, &json, &union_type, &[]);
    assert_eq!(errors.len(), 0);
}

// Tests for builtin JSON type

#[test]
fn test_validate_json_builtin_accepts_string() {
    let schema = create_test_schema();
    let json = serde_json::json!("hello");
    let errors = validate_value(&schema, &json, &ParsedType::Reference("JSON".to_string()), &[]);
    assert_eq!(errors.len(), 0);
}

#[test]
fn test_validate_json_builtin_accepts_number() {
    let schema = create_test_schema();
    let json = serde_json::json!(42);
    let errors = validate_value(&schema, &json, &ParsedType::Reference("JSON".to_string()), &[]);
    assert_eq!(errors.len(), 0);
}

#[test]
fn test_validate_json_builtin_accepts_boolean() {
    let schema = create_test_schema();
    let json = serde_json::json!(true);
    let errors = validate_value(&schema, &json, &ParsedType::Reference("JSON".to_string()), &[]);
    assert_eq!(errors.len(), 0);
}

#[test]
fn test_validate_json_builtin_accepts_null() {
    let schema = create_test_schema();
    let json = serde_json::json!(null);
    let errors = validate_value(&schema, &json, &ParsedType::Reference("JSON".to_string()), &[]);
    assert_eq!(errors.len(), 0);
}

#[test]
fn test_validate_json_builtin_accepts_array() {
    let schema = create_test_schema();
    let json = serde_json::json!([1, "two", true, null]);
    let errors = validate_value(&schema, &json, &ParsedType::Reference("JSON".to_string()), &[]);
    assert_eq!(errors.len(), 0);
}

#[test]
fn test_validate_json_builtin_accepts_object() {
    let schema = create_test_schema();
    let json = serde_json::json!({
        "key": "value",
        "nested": {
            "array": [1, 2, 3]
        }
    });
    let errors = validate_value(&schema, &json, &ParsedType::Reference("JSON".to_string()), &[]);
    assert_eq!(errors.len(), 0);
}

// Tests for Model and Type reference types

#[test]
fn test_validate_model_ref_valid() {
    let schema = create_test_schema();
    // "User" is a model in the schema
    let json = serde_json::json!("User");
    let errors = validate_value(&schema, &json, &ParsedType::ModelRef, &[]);
    assert_eq!(errors.len(), 0);
}

#[test]
fn test_validate_model_ref_not_a_model() {
    let schema = create_test_schema();
    // "ID" is a type alias, not a model
    let json = serde_json::json!("ID");
    let errors = validate_value(&schema, &json, &ParsedType::ModelRef, &[]);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("is a type alias, not a model"));
}

#[test]
fn test_validate_model_ref_not_found() {
    let schema = create_test_schema();
    let json = serde_json::json!("NonExistent");
    let errors = validate_value(&schema, &json, &ParsedType::ModelRef, &[]);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("Model 'NonExistent' not found"));
}

#[test]
fn test_validate_model_ref_wrong_type() {
    let schema = create_test_schema();
    let json = serde_json::json!(123);
    let errors = validate_value(&schema, &json, &ParsedType::ModelRef, &[]);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("Expected model name (string)"));
}

#[test]
fn test_validate_type_ref_valid() {
    let schema = create_test_schema();
    // "ID" is a type alias in the schema
    let json = serde_json::json!("ID");
    let errors = validate_value(&schema, &json, &ParsedType::TypeRef, &[]);
    assert_eq!(errors.len(), 0);
}

#[test]
fn test_validate_type_ref_not_a_type_alias() {
    let schema = create_test_schema();
    // "User" is a model, not a type alias
    let json = serde_json::json!("User");
    let errors = validate_value(&schema, &json, &ParsedType::TypeRef, &[]);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("is a model, not a type alias"));
}

#[test]
fn test_validate_type_ref_not_found() {
    let schema = create_test_schema();
    let json = serde_json::json!("NonExistent");
    let errors = validate_value(&schema, &json, &ParsedType::TypeRef, &[]);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("Type alias 'NonExistent' not found"));
}

#[test]
fn test_validate_type_ref_wrong_type() {
    let schema = create_test_schema();
    let json = serde_json::json!(123);
    let errors = validate_value(&schema, &json, &ParsedType::TypeRef, &[]);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("Expected type alias name (string)"));
}

#[test]
fn test_validate_model_or_type_union() {
    let schema = create_test_schema();
    // Model | Type union - should accept either a model name or type alias name
    let union_type = ParsedType::Union(vec![ParsedType::ModelRef, ParsedType::TypeRef]);

    // Should accept model name
    let json = serde_json::json!("User");
    let errors = validate_value(&schema, &json, &union_type, &[]);
    assert_eq!(errors.len(), 0);

    // Should accept type alias name
    let json = serde_json::json!("ID");
    let errors = validate_value(&schema, &json, &union_type, &[]);
    assert_eq!(errors.len(), 0);

    // Should reject unknown name
    let json = serde_json::json!("Unknown");
    let errors = validate_value(&schema, &json, &union_type, &[]);
    assert_eq!(errors.len(), 1);
}

#[test]
fn test_validate_json_with_user_schema() {
    // Create a plugin schema with a Model field
    let mut plugin_models = HashMap::new();
    let span = Span {
        start: Position { line: 0, column: 0 },
        end: Position { line: 0, column: 0 }
    };

    let config_fields = vec![
        ResolvedField::new("model_ref".to_string(), Some("Model".to_string()), false, "plugin.cdm".to_string(), span),
    ];

    plugin_models.insert(
        "Config".to_string(),
        ResolvedModel {
            name: "Config".to_string(),
            fields: config_fields,
            parents: vec![],
            plugin_configs: std::collections::HashMap::new(),
            source_file: "plugin.cdm".to_string(),
            source_span: span,
            entity_id: None,
        },
    );

    let plugin_schema = ResolvedSchema {
        models: plugin_models.clone(),
        type_aliases: HashMap::new(),
        all_models_for_inheritance: plugin_models,
    };

    // Create a user schema with a User model
    let user_schema = create_test_schema();

    // Validate config that references a model from user schema
    let json = serde_json::json!({
        "model_ref": "User"
    });

    // Using validate_json_with_user_schema should allow validation against user schema
    let errors = validate_json_with_user_schema(&plugin_schema, &json, "Config", Some(&user_schema));
    assert_eq!(errors.len(), 0);

    // If we try to reference a non-existent model, it should fail
    let json_invalid = serde_json::json!({
        "model_ref": "NonExistent"
    });

    let errors = validate_json_with_user_schema(&plugin_schema, &json_invalid, "Config", Some(&user_schema));
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("Model 'NonExistent' not found"));
}
