use super::*;

#[test]
fn test_map_primitive_types() {
    let mut mapper = TypeMapper::new("enum".to_string());

    let string_type = mapper.map_type(&TypeExpression::Identifier { name: "string".to_string() }, None, &json!({}));
    assert_eq!(string_type, json!({ "type": "string" }));

    let number_type = mapper.map_type(&TypeExpression::Identifier { name: "number".to_string() }, None, &json!({}));
    assert_eq!(number_type, json!({ "type": "number" }));

    let boolean_type = mapper.map_type(&TypeExpression::Identifier { name: "boolean".to_string() }, None, &json!({}));
    assert_eq!(boolean_type, json!({ "type": "boolean" }));
}

#[test]
fn test_map_array_type() {
    let mut mapper = TypeMapper::new("enum".to_string());

    let array_type = mapper.map_type(
        &TypeExpression::Array {
            element_type: Box::new(TypeExpression::Identifier { name: "string".to_string() }),
        },
        None,
        &json!({}),
    );

    assert_eq!(
        array_type,
        json!({
            "type": "array",
            "items": { "type": "string" }
        })
    );
}

#[test]
fn test_map_union_enum_mode() {
    let mut mapper = TypeMapper::new("enum".to_string());

    let union_type = mapper.map_type(
        &TypeExpression::Union {
            types: vec![
                TypeExpression::StringLiteral { value: "active".to_string() },
                TypeExpression::StringLiteral { value: "pending".to_string() },
            ],
        },
        None,
        &json!({}),
    );

    assert_eq!(
        union_type,
        json!({
            "type": "string",
            "enum": ["active", "pending"]
        })
    );
}

#[test]
fn test_map_union_oneof_mode() {
    let mut mapper = TypeMapper::new("oneOf".to_string());

    let union_type = mapper.map_type(
        &TypeExpression::Union {
            types: vec![
                TypeExpression::StringLiteral { value: "active".to_string() },
                TypeExpression::StringLiteral { value: "pending".to_string() },
            ],
        },
        None,
        &json!({}),
    );

    assert_eq!(
        union_type,
        json!({
            "oneOf": [
                { "const": "active" },
                { "const": "pending" }
            ]
        })
    );
}

#[test]
fn test_apply_field_constraints() {
    let base_schema = serde_json::Map::new();
    let config = json!({
        "pattern": "^[a-z]+$",
        "min_length": 1,
        "max_length": 100,
        "description": "A lowercase string"
    });

    let result = apply_field_constraints(base_schema, &config);

    assert_eq!(result.get("pattern"), Some(&json!("^[a-z]+$")));
    assert_eq!(result.get("minLength"), Some(&json!(1)));
    assert_eq!(result.get("maxLength"), Some(&json!(100)));
    assert_eq!(result.get("description"), Some(&json!("A lowercase string")));
}
