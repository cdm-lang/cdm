use super::*;
use cdm_plugin_interface::TypeExpression;

#[test]
fn test_map_string_type() {
    let expr = TypeExpression::Identifier {
        name: "string".to_string(),
    };
    assert_eq!(map_type_to_zod(&expr, true), "z.string()");
}

#[test]
fn test_map_number_type() {
    let expr = TypeExpression::Identifier {
        name: "number".to_string(),
    };
    assert_eq!(map_type_to_zod(&expr, true), "z.number()");
}

#[test]
fn test_map_boolean_type() {
    let expr = TypeExpression::Identifier {
        name: "boolean".to_string(),
    };
    assert_eq!(map_type_to_zod(&expr, true), "z.boolean()");
}

#[test]
fn test_map_json_type_strict() {
    let expr = TypeExpression::Identifier {
        name: "JSON".to_string(),
    };
    assert_eq!(
        map_type_to_zod(&expr, true),
        "z.record(z.string(), z.unknown()).or(z.array(z.unknown()))"
    );
}

#[test]
fn test_map_json_type_non_strict() {
    let expr = TypeExpression::Identifier {
        name: "JSON".to_string(),
    };
    assert_eq!(map_type_to_zod(&expr, false), "z.any()");
}

#[test]
fn test_map_array_type() {
    let expr = TypeExpression::Array {
        element_type: Box::new(TypeExpression::Identifier {
            name: "string".to_string(),
        }),
    };
    assert_eq!(map_type_to_zod(&expr, true), "z.array(z.string())");
}

#[test]
fn test_map_nested_array_type() {
    let expr = TypeExpression::Array {
        element_type: Box::new(TypeExpression::Array {
            element_type: Box::new(TypeExpression::Identifier {
                name: "number".to_string(),
            }),
        }),
    };
    assert_eq!(
        map_type_to_zod(&expr, true),
        "z.array(z.array(z.number()))"
    );
}

#[test]
fn test_map_union_type() {
    let expr = TypeExpression::Union {
        types: vec![
            TypeExpression::Identifier {
                name: "string".to_string(),
            },
            TypeExpression::Identifier {
                name: "number".to_string(),
            },
        ],
    };
    assert_eq!(
        map_type_to_zod(&expr, true),
        "z.union([z.string(), z.number()])"
    );
}

#[test]
fn test_map_single_union_type() {
    let expr = TypeExpression::Union {
        types: vec![TypeExpression::Identifier {
            name: "string".to_string(),
        }],
    };
    assert_eq!(map_type_to_zod(&expr, true), "z.string()");
}

#[test]
fn test_map_string_literal_type() {
    let expr = TypeExpression::StringLiteral {
        value: "active".to_string(),
    };
    assert_eq!(map_type_to_zod(&expr, true), "z.literal(\"active\")");
}

#[test]
fn test_map_string_literal_union() {
    let expr = TypeExpression::Union {
        types: vec![
            TypeExpression::StringLiteral {
                value: "active".to_string(),
            },
            TypeExpression::StringLiteral {
                value: "inactive".to_string(),
            },
        ],
    };
    assert_eq!(
        map_type_to_zod(&expr, true),
        "z.union([z.literal(\"active\"), z.literal(\"inactive\")])"
    );
}

#[test]
fn test_map_user_defined_type() {
    let expr = TypeExpression::Identifier {
        name: "User".to_string(),
    };
    assert_eq!(map_type_to_zod(&expr, true), "UserSchema");
}

#[test]
fn test_map_array_of_user_defined_type() {
    let expr = TypeExpression::Array {
        element_type: Box::new(TypeExpression::Identifier {
            name: "Post".to_string(),
        }),
    };
    assert_eq!(map_type_to_zod(&expr, true), "z.array(PostSchema)");
}

#[test]
fn test_escape_string_literal() {
    let expr = TypeExpression::StringLiteral {
        value: "hello\"world".to_string(),
    };
    assert_eq!(map_type_to_zod(&expr, true), "z.literal(\"hello\\\"world\")");
}
