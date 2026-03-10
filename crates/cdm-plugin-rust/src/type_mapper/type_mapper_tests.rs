use super::*;

#[test]
fn test_builtin_types() {
    let config = TypeMapperConfig::default();
    assert_eq!(map_builtin_type("string", &config), "String");
    assert_eq!(map_builtin_type("number", &config), "f64");
    assert_eq!(map_builtin_type("boolean", &config), "bool");
}

#[test]
fn test_json_type() {
    let config = TypeMapperConfig::default();
    assert_eq!(map_builtin_type("JSON", &config), "serde_json::Value");
}

#[test]
fn test_number_type_override() {
    let config = TypeMapperConfig {
        number_type: "i64".to_string(),
        ..Default::default()
    };
    assert_eq!(map_builtin_type("number", &config), "i64");
}

#[test]
fn test_user_defined_type() {
    let config = TypeMapperConfig::default();
    assert_eq!(map_builtin_type("User", &config), "User");
    assert_eq!(map_builtin_type("CustomType", &config), "CustomType");
}

#[test]
fn test_array_type() {
    let config = TypeMapperConfig::default();
    let type_expr = TypeExpression::Array {
        element_type: Box::new(TypeExpression::Identifier {
            name: "string".to_string(),
        }),
    };
    assert_eq!(map_type_to_rust(&type_expr, &config), "Vec<String>");
}

#[test]
fn test_nested_array() {
    let config = TypeMapperConfig::default();
    let type_expr = TypeExpression::Array {
        element_type: Box::new(TypeExpression::Array {
            element_type: Box::new(TypeExpression::Identifier {
                name: "number".to_string(),
            }),
        }),
    };
    assert_eq!(map_type_to_rust(&type_expr, &config), "Vec<Vec<f64>>");
}

#[test]
fn test_map_type_hashmap() {
    let config = TypeMapperConfig::default();
    let type_expr = TypeExpression::Map {
        key_type: Box::new(TypeExpression::Identifier {
            name: "string".to_string(),
        }),
        value_type: Box::new(TypeExpression::Identifier {
            name: "number".to_string(),
        }),
    };
    assert_eq!(
        map_type_to_rust(&type_expr, &config),
        "HashMap<String, f64>"
    );
}

#[test]
fn test_map_type_btreemap() {
    let config = TypeMapperConfig {
        map_type: "BTreeMap".to_string(),
        ..Default::default()
    };
    let type_expr = TypeExpression::Map {
        key_type: Box::new(TypeExpression::Identifier {
            name: "string".to_string(),
        }),
        value_type: Box::new(TypeExpression::Identifier {
            name: "number".to_string(),
        }),
    };
    assert_eq!(
        map_type_to_rust(&type_expr, &config),
        "BTreeMap<String, f64>"
    );
}

#[test]
fn test_string_literal() {
    let config = TypeMapperConfig::default();
    let type_expr = TypeExpression::StringLiteral {
        value: "active".to_string(),
    };
    assert_eq!(map_type_to_rust(&type_expr, &config), "\"active\"");
}

#[test]
fn test_number_literal_integer() {
    let config = TypeMapperConfig::default();
    let type_expr = TypeExpression::NumberLiteral { value: 42.0 };
    assert_eq!(map_type_to_rust(&type_expr, &config), "42");
}

#[test]
fn test_number_literal_float() {
    let config = TypeMapperConfig::default();
    let type_expr = TypeExpression::NumberLiteral { value: 3.14 };
    assert_eq!(map_type_to_rust(&type_expr, &config), "3.14");
}

#[test]
fn test_union_returns_empty() {
    let config = TypeMapperConfig::default();
    let type_expr = TypeExpression::Union {
        types: vec![
            TypeExpression::StringLiteral {
                value: "a".to_string(),
            },
            TypeExpression::StringLiteral {
                value: "b".to_string(),
            },
        ],
    };
    assert_eq!(map_type_to_rust(&type_expr, &config), "");
}

#[test]
fn test_is_string_literal_union() {
    let union_expr = TypeExpression::Union {
        types: vec![
            TypeExpression::StringLiteral {
                value: "active".to_string(),
            },
            TypeExpression::StringLiteral {
                value: "inactive".to_string(),
            },
        ],
    };
    assert!(is_string_literal_union(&union_expr));

    let mixed_expr = TypeExpression::Union {
        types: vec![
            TypeExpression::StringLiteral {
                value: "active".to_string(),
            },
            TypeExpression::Identifier {
                name: "User".to_string(),
            },
        ],
    };
    assert!(!is_string_literal_union(&mixed_expr));

    let non_union = TypeExpression::Identifier {
        name: "string".to_string(),
    };
    assert!(!is_string_literal_union(&non_union));
}

#[test]
fn test_is_type_reference_union() {
    let ref_union = TypeExpression::Union {
        types: vec![
            TypeExpression::Identifier {
                name: "TextBlock".to_string(),
            },
            TypeExpression::Identifier {
                name: "ImageBlock".to_string(),
            },
        ],
    };
    assert!(is_type_reference_union(&ref_union));

    let string_union = TypeExpression::Union {
        types: vec![
            TypeExpression::StringLiteral {
                value: "a".to_string(),
            },
            TypeExpression::StringLiteral {
                value: "b".to_string(),
            },
        ],
    };
    assert!(!is_type_reference_union(&string_union));
}

#[test]
fn test_escape_string() {
    assert_eq!(escape_string("hello"), "hello");
    assert_eq!(escape_string("hello\"world"), "hello\\\"world");
    assert_eq!(escape_string("line1\nline2"), "line1\\nline2");
    assert_eq!(escape_string("tab\there"), "tab\\there");
}

#[test]
fn test_array_of_user_type() {
    let config = TypeMapperConfig::default();
    let type_expr = TypeExpression::Array {
        element_type: Box::new(TypeExpression::Identifier {
            name: "Post".to_string(),
        }),
    };
    assert_eq!(map_type_to_rust(&type_expr, &config), "Vec<Post>");
}

#[test]
fn test_identifier_type() {
    let config = TypeMapperConfig::default();
    let type_expr = TypeExpression::Identifier {
        name: "string".to_string(),
    };
    assert_eq!(map_type_to_rust(&type_expr, &config), "String");
}
