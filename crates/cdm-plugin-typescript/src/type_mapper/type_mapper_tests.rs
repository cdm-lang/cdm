use super::*;

#[test]
fn test_builtin_types() {
    assert_eq!(map_builtin_type("string", true), "string");
    assert_eq!(map_builtin_type("number", true), "number");
    assert_eq!(map_builtin_type("boolean", true), "boolean");
}

#[test]
fn test_json_type_strict() {
    assert_eq!(
        map_builtin_type("JSON", true),
        "Record<string, unknown> | unknown[]"
    );
}

#[test]
fn test_json_type_permissive() {
    assert_eq!(map_builtin_type("JSON", false), "any");
}

#[test]
fn test_user_defined_type() {
    assert_eq!(map_builtin_type("User", true), "User");
    assert_eq!(map_builtin_type("CustomType", true), "CustomType");
}

#[test]
fn test_array_type() {
    let type_expr = TypeExpression::Array {
        element_type: Box::new(TypeExpression::Identifier {
            name: "string".to_string(),
        }),
    };
    assert_eq!(map_type_to_typescript(&type_expr, true), "string[]");
}

#[test]
fn test_nested_array() {
    let type_expr = TypeExpression::Array {
        element_type: Box::new(TypeExpression::Array {
            element_type: Box::new(TypeExpression::Identifier {
                name: "number".to_string(),
            }),
        }),
    };
    assert_eq!(map_type_to_typescript(&type_expr, true), "number[][]");
}

#[test]
fn test_string_literal() {
    let type_expr = TypeExpression::StringLiteral {
        value: "active".to_string(),
    };
    assert_eq!(map_type_to_typescript(&type_expr, true), "\"active\"");
}

#[test]
fn test_union_type() {
    let type_expr = TypeExpression::Union {
        types: vec![
            TypeExpression::StringLiteral {
                value: "active".to_string(),
            },
            TypeExpression::StringLiteral {
                value: "pending".to_string(),
            },
            TypeExpression::StringLiteral {
                value: "deleted".to_string(),
            },
        ],
    };
    assert_eq!(
        map_type_to_typescript(&type_expr, true),
        "\"active\" | \"pending\" | \"deleted\""
    );
}

#[test]
fn test_model_reference_union() {
    let type_expr = TypeExpression::Union {
        types: vec![
            TypeExpression::Identifier {
                name: "TextBlock".to_string(),
            },
            TypeExpression::Identifier {
                name: "ImageBlock".to_string(),
            },
        ],
    };
    assert_eq!(
        map_type_to_typescript(&type_expr, true),
        "TextBlock | ImageBlock"
    );
}

#[test]
fn test_escape_string() {
    assert_eq!(escape_string("hello"), "hello");
    assert_eq!(escape_string("hello\"world"), "hello\\\"world");
    assert_eq!(escape_string("line1\nline2"), "line1\\nline2");
    assert_eq!(escape_string("tab\there"), "tab\\there");
}
