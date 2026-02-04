use super::*;

#[test]
fn test_parse_primitives() {
    assert_eq!(
        parse_type_string("string"),
        Ok(ParsedType::Primitive(PrimitiveType::String))
    );
    assert_eq!(
        parse_type_string("number"),
        Ok(ParsedType::Primitive(PrimitiveType::Number))
    );
    assert_eq!(
        parse_type_string("boolean"),
        Ok(ParsedType::Primitive(PrimitiveType::Boolean))
    );
    assert_eq!(parse_type_string("null"), Ok(ParsedType::Null));
}

#[test]
fn test_parse_primitives_with_whitespace() {
    assert_eq!(
        parse_type_string("  string  "),
        Ok(ParsedType::Primitive(PrimitiveType::String))
    );
    assert_eq!(
        parse_type_string(" number\t"),
        Ok(ParsedType::Primitive(PrimitiveType::Number))
    );
}

#[test]
fn test_parse_references() {
    assert_eq!(
        parse_type_string("User"),
        Ok(ParsedType::Reference("User".to_string()))
    );
    assert_eq!(
        parse_type_string("EmailAddress"),
        Ok(ParsedType::Reference("EmailAddress".to_string()))
    );
    assert_eq!(
        parse_type_string("_internal"),
        Ok(ParsedType::Reference("_internal".to_string()))
    );
}

#[test]
fn test_parse_string_literals() {
    assert_eq!(
        parse_type_string(r#""active""#),
        Ok(ParsedType::Literal("active".to_string()))
    );
    assert_eq!(
        parse_type_string(r#""pending""#),
        Ok(ParsedType::Literal("pending".to_string()))
    );
    assert_eq!(
        parse_type_string(r#"'completed'"#),
        Ok(ParsedType::Literal("completed".to_string()))
    );
}

#[test]
fn test_parse_arrays() {
    assert_eq!(
        parse_type_string("string[]"),
        Ok(ParsedType::Array(Box::new(ParsedType::Primitive(
            PrimitiveType::String
        ))))
    );
    assert_eq!(
        parse_type_string("User[]"),
        Ok(ParsedType::Array(Box::new(ParsedType::Reference(
            "User".to_string()
        ))))
    );
    // Nested arrays
    assert_eq!(
        parse_type_string("string[][]"),
        Ok(ParsedType::Array(Box::new(ParsedType::Array(Box::new(
            ParsedType::Primitive(PrimitiveType::String)
        )))))
    );
}

#[test]
fn test_parse_unions() {
    // Simple union
    let result = parse_type_string("string | number").unwrap();
    match result {
        ParsedType::Union(parts) => {
            assert_eq!(parts.len(), 2);
            assert_eq!(parts[0], ParsedType::Primitive(PrimitiveType::String));
            assert_eq!(parts[1], ParsedType::Primitive(PrimitiveType::Number));
        }
        _ => panic!("Expected Union type"),
    }

    // Union with null
    let result = parse_type_string("User | null").unwrap();
    match result {
        ParsedType::Union(parts) => {
            assert_eq!(parts.len(), 2);
            assert_eq!(parts[0], ParsedType::Reference("User".to_string()));
            assert_eq!(parts[1], ParsedType::Null);
        }
        _ => panic!("Expected Union type"),
    }

    // Three-way union
    let result = parse_type_string("string | number | boolean").unwrap();
    match result {
        ParsedType::Union(parts) => {
            assert_eq!(parts.len(), 3);
            assert_eq!(parts[0], ParsedType::Primitive(PrimitiveType::String));
            assert_eq!(parts[1], ParsedType::Primitive(PrimitiveType::Number));
            assert_eq!(parts[2], ParsedType::Primitive(PrimitiveType::Boolean));
        }
        _ => panic!("Expected Union type"),
    }
}

#[test]
fn test_parse_union_with_literals() {
    let result = parse_type_string(r#""active" | "pending" | "completed""#).unwrap();
    match result {
        ParsedType::Union(parts) => {
            assert_eq!(parts.len(), 3);
            assert_eq!(parts[0], ParsedType::Literal("active".to_string()));
            assert_eq!(parts[1], ParsedType::Literal("pending".to_string()));
            assert_eq!(parts[2], ParsedType::Literal("completed".to_string()));
        }
        _ => panic!("Expected Union type"),
    }
}

#[test]
fn test_parse_complex_types() {
    // Array union
    let result = parse_type_string("string[] | number[]").unwrap();
    match result {
        ParsedType::Union(parts) => {
            assert_eq!(parts.len(), 2);
            assert!(matches!(parts[0], ParsedType::Array(_)));
            assert!(matches!(parts[1], ParsedType::Array(_)));
        }
        _ => panic!("Expected Union type"),
    }

    // Union of references
    let result = parse_type_string("User | Admin | Guest").unwrap();
    match result {
        ParsedType::Union(parts) => {
            assert_eq!(parts.len(), 3);
            assert_eq!(parts[0], ParsedType::Reference("User".to_string()));
            assert_eq!(parts[1], ParsedType::Reference("Admin".to_string()));
            assert_eq!(parts[2], ParsedType::Reference("Guest".to_string()));
        }
        _ => panic!("Expected Union type"),
    }
}

#[test]
fn test_parse_errors() {
    // Empty string
    assert!(parse_type_string("").is_err());

    // Invalid identifier (starts with number)
    assert!(parse_type_string("9User").is_err());

    // Invalid identifier (special characters)
    assert!(parse_type_string("User-Name").is_err());
}

#[test]
fn test_is_valid_identifier() {
    // Valid simple identifiers
    assert!(is_valid_identifier("User"));
    assert!(is_valid_identifier("_private"));
    assert!(is_valid_identifier("User123"));
    assert!(is_valid_identifier("snake_case"));
    assert!(is_valid_identifier("PascalCase"));
    assert!(is_valid_identifier("camelCase"));

    // Valid qualified identifiers (for template types like sql.UUID)
    assert!(is_valid_identifier("sql.UUID"));
    assert!(is_valid_identifier("auth.types.Email"));
    assert!(is_valid_identifier("namespace.Type"));

    // Invalid identifiers
    assert!(!is_valid_identifier(""));
    assert!(!is_valid_identifier("123abc"));
    assert!(!is_valid_identifier("user-name"));
    assert!(!is_valid_identifier("user name"));
    assert!(!is_valid_identifier(".name")); // Can't start with dot
    assert!(!is_valid_identifier("name.")); // Can't end with dot
    assert!(!is_valid_identifier("foo..bar")); // No empty parts
}

#[test]
fn test_resolved_field_parsed_type_caching() {
    let field = ResolvedField {
        name: "test".to_string(),
        type_expr: Some("string | number".to_string()),
        optional: false,
        default_value: None,
        plugin_configs: std::collections::HashMap::new(),
        source_file: "test.cdm".to_string(),
        source_span: Span {
            start: Position { line: 0, column: 0 },
            end: Position {
                line: 0,
                column: 10,
            },
        },
        cached_parsed_type: std::cell::RefCell::new(None),
        entity_id: None,
    };

    // First call should parse
    let result1 = field.parsed_type().unwrap();
    assert!(matches!(result1, ParsedType::Union(_)));

    // Second call should return cached result
    let result2 = field.parsed_type().unwrap();
    assert_eq!(result1, result2);

    // Verify cache is populated
    assert!(field.cached_parsed_type.borrow().is_some());
}

#[test]
fn test_resolved_field_default_type() {
    let field = ResolvedField {
        name: "test".to_string(),
        type_expr: None, // No type specified
        optional: false,
        default_value: None,
        plugin_configs: std::collections::HashMap::new(),
        source_file: "test.cdm".to_string(),
        source_span: Span {
            start: Position { line: 0, column: 0 },
            end: Position {
                line: 0,
                column: 10,
            },
        },
        cached_parsed_type: std::cell::RefCell::new(None),
        entity_id: None,
    };

    // Should default to string
    let result = field.parsed_type().unwrap();
    assert_eq!(result, ParsedType::Primitive(PrimitiveType::String));
}

#[test]
fn test_resolved_type_alias_parsed_type() {
    let alias = ResolvedTypeAlias {
        name: "Status".to_string(),
        type_expr: r#""active" | "pending""#.to_string(),
        references: vec![],
        plugin_configs: std::collections::HashMap::new(),
        source_file: "test.cdm".to_string(),
        source_span: Span {
            start: Position { line: 0, column: 0 },
            end: Position {
                line: 0,
                column: 10,
            },
        },
        cached_parsed_type: std::cell::RefCell::new(None),
        entity_id: None,
        is_from_template: false,
    };

    let result = alias.parsed_type().unwrap();
    match result {
        ParsedType::Union(parts) => {
            assert_eq!(parts.len(), 2);
            assert_eq!(parts[0], ParsedType::Literal("active".to_string()));
            assert_eq!(parts[1], ParsedType::Literal("pending".to_string()));
        }
        _ => panic!("Expected Union type"),
    }
}

#[test]
fn test_resolved_field_clone_preserves_new_fields() {
    let mut original = ResolvedField::new(
        "name".to_string(),
        Some("string".to_string()),
        false,
        "test.cdm".to_string(),
        Span {
            start: Position { line: 0, column: 0 },
            end: Position {
                line: 0,
                column: 10,
            },
        },
    );

    original.default_value = Some(serde_json::json!("default"));
    original
        .plugin_configs
        .insert("test".to_string(), serde_json::json!({"key": "value"}));

    let cloned = original.clone();

    assert_eq!(cloned.default_value, original.default_value);
    assert_eq!(cloned.plugin_configs.len(), 1);
    assert_eq!(
        cloned.plugin_configs.get("test").unwrap(),
        &serde_json::json!({"key": "value"})
    );
}

#[test]
fn test_resolved_model_clone_preserves_new_fields() {
    let span = Span {
        start: Position { line: 0, column: 0 },
        end: Position {
            line: 0,
            column: 10,
        },
    };

    let mut plugin_configs = std::collections::HashMap::new();
    plugin_configs.insert("test".to_string(), serde_json::json!({"key": "value"}));

    let original = ResolvedModel {
        name: "User".to_string(),
        fields: vec![],
        parents: vec!["Base".to_string()],
        plugin_configs,
        source_file: "test.cdm".to_string(),
        source_span: span,
        entity_id: None,
    };

    let cloned = original.clone();

    assert_eq!(cloned.parents, original.parents);
    assert_eq!(cloned.plugin_configs.len(), 1);
    assert_eq!(
        cloned.plugin_configs.get("test").unwrap(),
        &serde_json::json!({"key": "value"})
    );
}

#[test]
fn test_resolved_type_alias_clone_preserves_new_fields() {
    let mut plugin_configs = std::collections::HashMap::new();
    plugin_configs.insert("test".to_string(), serde_json::json!({"key": "value"}));

    let original = ResolvedTypeAlias {
        name: "Status".to_string(),
        type_expr: "string".to_string(),
        references: vec![],
        plugin_configs,
        source_file: "test.cdm".to_string(),
        source_span: Span {
            start: Position { line: 0, column: 0 },
            end: Position {
                line: 0,
                column: 10,
            },
        },
        cached_parsed_type: std::cell::RefCell::new(None),
        entity_id: None,
        is_from_template: false,
    };

    let cloned = original.clone();

    assert_eq!(cloned.plugin_configs.len(), 1);
    assert_eq!(
        cloned.plugin_configs.get("test").unwrap(),
        &serde_json::json!({"key": "value"})
    );
    assert!(!cloned.is_from_template, "is_from_template should be cloned");
}

#[test]
fn test_parse_number_literal() {
    assert_eq!(parse_type_string("42"), Ok(ParsedType::NumberLiteral(42.0)));
    assert_eq!(
        parse_type_string("3.14"),
        Ok(ParsedType::NumberLiteral(3.14))
    );
    assert_eq!(
        parse_type_string("-10"),
        Ok(ParsedType::NumberLiteral(-10.0))
    );
}

#[test]
fn test_parse_number_literal_union() {
    let result = parse_type_string("1 | 2 | 3").unwrap();
    match result {
        ParsedType::Union(parts) => {
            assert_eq!(parts.len(), 3);
            assert_eq!(parts[0], ParsedType::NumberLiteral(1.0));
            assert_eq!(parts[1], ParsedType::NumberLiteral(2.0));
            assert_eq!(parts[2], ParsedType::NumberLiteral(3.0));
        }
        _ => panic!("Expected Union type"),
    }
}

#[test]
fn test_parse_map_type_basic() {
    let result = parse_type_string("User[string]").unwrap();
    match result {
        ParsedType::Map {
            value_type,
            key_type,
        } => {
            assert_eq!(*value_type, ParsedType::Reference("User".to_string()));
            assert_eq!(*key_type, ParsedType::Primitive(PrimitiveType::String));
        }
        _ => panic!("Expected Map type"),
    }
}

#[test]
fn test_parse_map_type_with_number_key() {
    let result = parse_type_string("User[number]").unwrap();
    match result {
        ParsedType::Map {
            value_type,
            key_type,
        } => {
            assert_eq!(*value_type, ParsedType::Reference("User".to_string()));
            assert_eq!(*key_type, ParsedType::Primitive(PrimitiveType::Number));
        }
        _ => panic!("Expected Map type"),
    }
}

#[test]
fn test_parse_map_type_nested() {
    // string[string][Locale] -> Map { value: Map { value: string, key: string }, key: Locale }
    let result = parse_type_string("string[string][Locale]").unwrap();
    match result {
        ParsedType::Map {
            value_type,
            key_type,
        } => {
            assert_eq!(*key_type, ParsedType::Reference("Locale".to_string()));
            match *value_type {
                ParsedType::Map {
                    value_type: inner_value,
                    key_type: inner_key,
                } => {
                    assert_eq!(
                        *inner_value,
                        ParsedType::Primitive(PrimitiveType::String)
                    );
                    assert_eq!(*inner_key, ParsedType::Primitive(PrimitiveType::String));
                }
                _ => panic!("Expected nested Map type"),
            }
        }
        _ => panic!("Expected Map type"),
    }
}

#[test]
fn test_parse_map_type_with_literal_union_key() {
    let result = parse_type_string(r#"Prize["gold" | "silver" | "bronze"]"#).unwrap();
    match result {
        ParsedType::Map {
            value_type,
            key_type,
        } => {
            assert_eq!(*value_type, ParsedType::Reference("Prize".to_string()));
            match *key_type {
                ParsedType::Union(parts) => {
                    assert_eq!(parts.len(), 3);
                    assert_eq!(parts[0], ParsedType::Literal("gold".to_string()));
                    assert_eq!(parts[1], ParsedType::Literal("silver".to_string()));
                    assert_eq!(parts[2], ParsedType::Literal("bronze".to_string()));
                }
                _ => panic!("Expected Union key type"),
            }
        }
        _ => panic!("Expected Map type"),
    }
}

#[test]
fn test_parse_map_type_with_number_literal_union_key() {
    let result = parse_type_string("Prize[1 | 2 | 3]").unwrap();
    match result {
        ParsedType::Map {
            value_type,
            key_type,
        } => {
            assert_eq!(*value_type, ParsedType::Reference("Prize".to_string()));
            match *key_type {
                ParsedType::Union(parts) => {
                    assert_eq!(parts.len(), 3);
                    assert_eq!(parts[0], ParsedType::NumberLiteral(1.0));
                    assert_eq!(parts[1], ParsedType::NumberLiteral(2.0));
                    assert_eq!(parts[2], ParsedType::NumberLiteral(3.0));
                }
                _ => panic!("Expected Union key type"),
            }
        }
        _ => panic!("Expected Map type"),
    }
}

#[test]
fn test_parse_map_type_in_union() {
    let result = parse_type_string("string | User[number]").unwrap();
    match result {
        ParsedType::Union(parts) => {
            assert_eq!(parts.len(), 2);
            assert_eq!(parts[0], ParsedType::Primitive(PrimitiveType::String));
            match &parts[1] {
                ParsedType::Map {
                    value_type,
                    key_type,
                } => {
                    assert_eq!(**value_type, ParsedType::Reference("User".to_string()));
                    assert_eq!(**key_type, ParsedType::Primitive(PrimitiveType::Number));
                }
                _ => panic!("Expected Map type in union"),
            }
        }
        _ => panic!("Expected Union type"),
    }
}

#[test]
fn test_array_still_works_after_map_support() {
    // Make sure array parsing still works correctly
    assert_eq!(
        parse_type_string("User[]"),
        Ok(ParsedType::Array(Box::new(ParsedType::Reference(
            "User".to_string()
        ))))
    );
    assert_eq!(
        parse_type_string("string[]"),
        Ok(ParsedType::Array(Box::new(ParsedType::Primitive(
            PrimitiveType::String
        ))))
    );
}

#[test]
fn test_parse_model_ref_type() {
    // Model is a special built-in type that references CDM models
    assert_eq!(parse_type_string("Model"), Ok(ParsedType::ModelRef));
}

#[test]
fn test_parse_type_ref_type() {
    // Type is a special built-in type that references CDM type aliases
    assert_eq!(parse_type_string("Type"), Ok(ParsedType::TypeRef));
}

#[test]
fn test_parse_model_type_union() {
    // Model | Type union - accepts either a model or type alias reference
    let result = parse_type_string("Model | Type").unwrap();
    match result {
        ParsedType::Union(parts) => {
            assert_eq!(parts.len(), 2);
            assert_eq!(parts[0], ParsedType::ModelRef);
            assert_eq!(parts[1], ParsedType::TypeRef);
        }
        _ => panic!("Expected Union type"),
    }
}

#[test]
fn test_parse_optional_model_type() {
    // Model | null union - optional model reference
    let result = parse_type_string("Model | null").unwrap();
    match result {
        ParsedType::Union(parts) => {
            assert_eq!(parts.len(), 2);
            assert_eq!(parts[0], ParsedType::ModelRef);
            assert_eq!(parts[1], ParsedType::Null);
        }
        _ => panic!("Expected Union type"),
    }
}
