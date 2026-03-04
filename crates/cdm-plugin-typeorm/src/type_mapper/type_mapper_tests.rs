use super::*;
use cdm_plugin_interface::TypeExpression;
use std::collections::{BTreeSet, HashMap};

fn create_test_mapper() -> TypeMapper<'static> {
    static TYPE_ALIASES: std::sync::OnceLock<HashMap<String, cdm_plugin_interface::TypeAliasDefinition>> = std::sync::OnceLock::new();
    let type_aliases = TYPE_ALIASES.get_or_init(HashMap::new);
    let config = serde_json::json!({});
    let model_names = vec!["User".to_string(), "Post".to_string()];
    TypeMapper::new(&config, type_aliases, model_names)
}

#[test]
fn test_map_string_to_column_type() {
    let mapper = create_test_mapper();
    let type_expr = TypeExpression::Identifier {
        name: "string".to_string(),
    };
    assert_eq!(mapper.map_to_column_type(&type_expr), "varchar(255)");
}

#[test]
fn test_map_number_to_column_type() {
    let mapper = create_test_mapper();
    let type_expr = TypeExpression::Identifier {
        name: "number".to_string(),
    };
    assert_eq!(mapper.map_to_column_type(&type_expr), "double precision");
}

#[test]
fn test_map_boolean_to_column_type() {
    let mapper = create_test_mapper();
    let type_expr = TypeExpression::Identifier {
        name: "boolean".to_string(),
    };
    assert_eq!(mapper.map_to_column_type(&type_expr), "boolean");
}

#[test]
fn test_map_json_to_column_type() {
    let mapper = create_test_mapper();
    let type_expr = TypeExpression::Identifier {
        name: "JSON".to_string(),
    };
    assert_eq!(mapper.map_to_column_type(&type_expr), "jsonb");
}

#[test]
fn test_map_string_to_typescript_type() {
    let mapper = create_test_mapper();
    let type_expr = TypeExpression::Identifier {
        name: "string".to_string(),
    };
    assert_eq!(mapper.map_to_typescript_type(&type_expr), "string");
}

#[test]
fn test_map_array_to_typescript_type() {
    let mapper = create_test_mapper();
    let type_expr = TypeExpression::Array {
        element_type: Box::new(TypeExpression::Identifier {
            name: "string".to_string(),
        }),
    };
    assert_eq!(mapper.map_to_typescript_type(&type_expr), "string[]");
}

#[test]
fn test_is_model_reference() {
    let mapper = create_test_mapper();

    let user_ref = TypeExpression::Identifier {
        name: "User".to_string(),
    };
    assert_eq!(mapper.is_model_reference(&user_ref), Some("User".to_string()));

    let string_type = TypeExpression::Identifier {
        name: "string".to_string(),
    };
    assert_eq!(mapper.is_model_reference(&string_type), None);
}

#[test]
fn test_is_model_array() {
    let mapper = create_test_mapper();

    let posts_array = TypeExpression::Array {
        element_type: Box::new(TypeExpression::Identifier {
            name: "Post".to_string(),
        }),
    };
    assert!(mapper.is_model_array(&posts_array));

    let string_array = TypeExpression::Array {
        element_type: Box::new(TypeExpression::Identifier {
            name: "string".to_string(),
        }),
    };
    assert!(!mapper.is_model_array(&string_array));
}

#[test]
fn test_collect_model_references_array_of_model() {
    let mapper = create_test_mapper();
    let type_expr = TypeExpression::Array {
        element_type: Box::new(TypeExpression::Identifier {
            name: "Post".to_string(),
        }),
    };
    let mut refs = BTreeSet::new();
    mapper.collect_model_references(&type_expr, &mut refs);
    assert_eq!(refs.len(), 1);
    assert!(refs.contains("Post"));
}

#[test]
fn test_collect_model_references_skips_primitives() {
    let mapper = create_test_mapper();
    let type_expr = TypeExpression::Identifier {
        name: "string".to_string(),
    };
    let mut refs = BTreeSet::new();
    mapper.collect_model_references(&type_expr, &mut refs);
    assert!(refs.is_empty());
}

#[test]
fn test_collect_model_references_resolves_type_alias() {
    static TYPE_ALIASES: std::sync::OnceLock<HashMap<String, cdm_plugin_interface::TypeAliasDefinition>> = std::sync::OnceLock::new();
    let type_aliases = TYPE_ALIASES.get_or_init(|| {
        let mut map = HashMap::new();
        map.insert(
            "PortConfig".to_string(),
            cdm_plugin_interface::TypeAliasDefinition {
                name: "PortConfig".to_string(),
                alias_type: TypeExpression::Identifier {
                    name: "User".to_string(),
                },
                config: serde_json::json!({}),
                entity_id: None,
            },
        );
        map
    });
    let config = serde_json::json!({});
    let model_names = vec!["User".to_string(), "Post".to_string()];
    let mapper = TypeMapper::new(&config, type_aliases, model_names);

    let type_expr = TypeExpression::Identifier {
        name: "PortConfig".to_string(),
    };
    let mut refs = BTreeSet::new();
    mapper.collect_model_references(&type_expr, &mut refs);
    assert_eq!(refs.len(), 1);
    assert!(refs.contains("User"));
}

#[test]
fn test_collect_model_references_identifier_model() {
    let mapper = create_test_mapper();
    let type_expr = TypeExpression::Identifier {
        name: "User".to_string(),
    };
    let mut refs = BTreeSet::new();
    mapper.collect_model_references(&type_expr, &mut refs);
    assert_eq!(refs.len(), 1);
    assert!(refs.contains("User"));
}
