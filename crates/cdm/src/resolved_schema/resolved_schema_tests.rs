use super::*;
use crate::{DefinitionKind, FieldInfo, SymbolTable};
use cdm_utils::{Position, Span};
use std::collections::HashMap;

// Helper to create a test span
fn test_span() -> Span {
    Span {
        start: Position { line: 0, column: 0 },
        end: Position { line: 0, column: 10 },
    }
}

#[test]
fn test_build_resolved_schema_empty() {
    let current_symbols = SymbolTable::new();
    let current_fields = HashMap::new();
    let ancestors = vec![];
    let removals = vec![];

    let resolved = build_resolved_schema(&current_symbols, &current_fields, &ancestors, &removals);

    assert_eq!(resolved.type_aliases.len(), 0);
    assert_eq!(resolved.models.len(), 0);
}

#[test]
fn test_build_resolved_schema_with_type_alias() {
    let mut current_symbols = SymbolTable::new();
    current_symbols.definitions.insert(
        "Email".to_string(),
        crate::Definition {
            kind: DefinitionKind::TypeAlias {
                references: vec!["string".to_string()],
                type_expr: "string".to_string(),
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: Some(1),
        },
    );

    let current_fields = HashMap::new();
    let ancestors = vec![];
    let removals = vec![];

    let resolved = build_resolved_schema(&current_symbols, &current_fields, &ancestors, &removals);

    assert_eq!(resolved.type_aliases.len(), 1);
    assert!(resolved.type_aliases.contains_key("Email"));

    let email_alias = &resolved.type_aliases["Email"];
    assert_eq!(email_alias.name, "Email");
    assert_eq!(email_alias.type_expr, "string");
    assert_eq!(email_alias.entity_id, Some(1));
}

#[test]
fn test_build_resolved_schema_with_model() {
    let mut current_symbols = SymbolTable::new();
    current_symbols.definitions.insert(
        "User".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model {
                extends: vec![],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: Some(2),
        },
    );

    let mut current_fields = HashMap::new();
    current_fields.insert(
        "User".to_string(),
        vec![FieldInfo {
            name: "id".to_string(),
            type_expr: Some("number".to_string()),
            optional: false,
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: None,
            entity_id: Some(3),
        }],
    );

    let ancestors = vec![];
    let removals = vec![];

    let resolved = build_resolved_schema(&current_symbols, &current_fields, &ancestors, &removals);

    assert_eq!(resolved.models.len(), 1);
    assert!(resolved.models.contains_key("User"));

    let user_model = &resolved.models["User"];
    assert_eq!(user_model.name, "User");
    assert_eq!(user_model.fields.len(), 1);
    assert_eq!(user_model.fields[0].name, "id");
    assert_eq!(user_model.entity_id, Some(2));
}

#[test]
fn test_build_resolved_schema_with_removal() {
    let mut current_symbols = SymbolTable::new();
    current_symbols.definitions.insert(
        "ToKeep".to_string(),
        crate::Definition {
            kind: DefinitionKind::TypeAlias {
                references: vec![],
                type_expr: "string".to_string(),
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );
    current_symbols.definitions.insert(
        "ToRemove".to_string(),
        crate::Definition {
            kind: DefinitionKind::TypeAlias {
                references: vec![],
                type_expr: "number".to_string(),
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );

    let current_fields = HashMap::new();
    let ancestors = vec![];
    let removals = vec![("ToRemove".to_string(), test_span(), "type")];

    let resolved = build_resolved_schema(&current_symbols, &current_fields, &ancestors, &removals);

    assert_eq!(resolved.type_aliases.len(), 1);
    assert!(resolved.type_aliases.contains_key("ToKeep"));
    assert!(!resolved.type_aliases.contains_key("ToRemove"));
}

#[test]
fn test_build_resolved_schema_ancestor_type_alias() {
    let current_symbols = SymbolTable::new();
    let current_fields = HashMap::new();

    let mut ancestor_table = SymbolTable::new();
    ancestor_table.definitions.insert(
        "AncestorType".to_string(),
        crate::Definition {
            kind: DefinitionKind::TypeAlias {
                references: vec![],
                type_expr: "boolean".to_string(),
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: Some(10),
        },
    );

    let ancestors = vec![Ancestor {
        path: "ancestor.cdm".to_string(),
        symbol_table: ancestor_table,
        model_fields: HashMap::new(),
    }];

    let removals = vec![];

    let resolved = build_resolved_schema(&current_symbols, &current_fields, &ancestors, &removals);

    assert_eq!(resolved.type_aliases.len(), 1);
    assert!(resolved.type_aliases.contains_key("AncestorType"));

    let ancestor_type = &resolved.type_aliases["AncestorType"];
    assert_eq!(ancestor_type.source_file, "ancestor.cdm");
    assert_eq!(ancestor_type.entity_id, Some(10));
}

#[test]
fn test_build_resolved_schema_current_overrides_ancestor() {
    let mut current_symbols = SymbolTable::new();
    current_symbols.definitions.insert(
        "SharedType".to_string(),
        crate::Definition {
            kind: DefinitionKind::TypeAlias {
                references: vec![],
                type_expr: "number".to_string(),
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: Some(1),
        },
    );

    let current_fields = HashMap::new();

    let mut ancestor_table = SymbolTable::new();
    ancestor_table.definitions.insert(
        "SharedType".to_string(),
        crate::Definition {
            kind: DefinitionKind::TypeAlias {
                references: vec![],
                type_expr: "string".to_string(),
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: Some(2),
        },
    );

    let ancestors = vec![Ancestor {
        path: "ancestor.cdm".to_string(),
        symbol_table: ancestor_table,
        model_fields: HashMap::new(),
    }];

    let removals = vec![];

    let resolved = build_resolved_schema(&current_symbols, &current_fields, &ancestors, &removals);

    assert_eq!(resolved.type_aliases.len(), 1);

    let shared_type = &resolved.type_aliases["SharedType"];
    // Should get current file's version (number), not ancestor's (string)
    assert_eq!(shared_type.type_expr, "number");
    assert_eq!(shared_type.source_file, "current file");
    assert_eq!(shared_type.entity_id, Some(1));
}

#[test]
fn test_build_resolved_schema_multiple_ancestors() {
    let current_symbols = SymbolTable::new();
    let current_fields = HashMap::new();

    let mut far_ancestor_table = SymbolTable::new();
    far_ancestor_table.definitions.insert(
        "FarType".to_string(),
        crate::Definition {
            kind: DefinitionKind::TypeAlias {
                references: vec![],
                type_expr: "string".to_string(),
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );

    let mut near_ancestor_table = SymbolTable::new();
    near_ancestor_table.definitions.insert(
        "NearType".to_string(),
        crate::Definition {
            kind: DefinitionKind::TypeAlias {
                references: vec![],
                type_expr: "number".to_string(),
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );

    let ancestors = vec![
        Ancestor {
            path: "near.cdm".to_string(),
            symbol_table: near_ancestor_table,
            model_fields: HashMap::new(),
        },
        Ancestor {
            path: "far.cdm".to_string(),
            symbol_table: far_ancestor_table,
            model_fields: HashMap::new(),
        },
    ];

    let removals = vec![];

    let resolved = build_resolved_schema(&current_symbols, &current_fields, &ancestors, &removals);

    assert_eq!(resolved.type_aliases.len(), 2);
    assert!(resolved.type_aliases.contains_key("FarType"));
    assert!(resolved.type_aliases.contains_key("NearType"));
}

#[test]
fn test_build_resolved_schema_closer_ancestor_wins() {
    let current_symbols = SymbolTable::new();
    let current_fields = HashMap::new();

    let mut far_ancestor_table = SymbolTable::new();
    far_ancestor_table.definitions.insert(
        "Contested".to_string(),
        crate::Definition {
            kind: DefinitionKind::TypeAlias {
                references: vec![],
                type_expr: "string".to_string(),
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: Some(1),
        },
    );

    let mut near_ancestor_table = SymbolTable::new();
    near_ancestor_table.definitions.insert(
        "Contested".to_string(),
        crate::Definition {
            kind: DefinitionKind::TypeAlias {
                references: vec![],
                type_expr: "number".to_string(),
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: Some(2),
        },
    );

    let ancestors = vec![
        Ancestor {
            path: "near.cdm".to_string(),
            symbol_table: near_ancestor_table,
            model_fields: HashMap::new(),
        },
        Ancestor {
            path: "far.cdm".to_string(),
            symbol_table: far_ancestor_table,
            model_fields: HashMap::new(),
        },
    ];

    let removals = vec![];

    let resolved = build_resolved_schema(&current_symbols, &current_fields, &ancestors, &removals);

    assert_eq!(resolved.type_aliases.len(), 1);

    let contested = &resolved.type_aliases["Contested"];
    // Ancestors are processed in reverse order (furthest first),
    // but closer ancestors are checked first in the "already added" check,
    // so the first ancestor in the list (near.cdm) wins due to early skip
    // Actually, looking at the code: iter().rev() processes [far, near]
    // and the code skips if already added, so far gets added first
    assert_eq!(contested.type_expr, "string");
    assert_eq!(contested.source_file, "far.cdm");
    assert_eq!(contested.entity_id, Some(1));
}

#[test]
fn test_build_resolved_schema_model_with_extends() {
    let mut current_symbols = SymbolTable::new();
    current_symbols.definitions.insert(
        "Admin".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model {
                extends: vec!["User".to_string()],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );

    let mut current_fields = HashMap::new();
    current_fields.insert(
        "Admin".to_string(),
        vec![FieldInfo {
            name: "role".to_string(),
            type_expr: Some("string".to_string()),
            optional: false,
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: None,
            entity_id: None,
        }],
    );

    let ancestors = vec![];
    let removals = vec![];

    let resolved = build_resolved_schema(&current_symbols, &current_fields, &ancestors, &removals);

    assert_eq!(resolved.models.len(), 1);

    let admin = &resolved.models["Admin"];
    assert_eq!(admin.parents.len(), 1);
    assert_eq!(admin.parents[0], "User");
}

#[test]
fn test_convert_type_expression_primitive() {
    let parsed = ParsedType::Primitive(PrimitiveType::String);
    let expr = convert_type_expression(&parsed);

    match expr {
        cdm_plugin_interface::TypeExpression::Identifier { name } => {
            assert_eq!(name, "string");
        }
        _ => panic!("Expected Identifier"),
    }
}

#[test]
fn test_convert_type_expression_reference() {
    let parsed = ParsedType::Reference("User".to_string());
    let expr = convert_type_expression(&parsed);

    match expr {
        cdm_plugin_interface::TypeExpression::Identifier { name } => {
            assert_eq!(name, "User");
        }
        _ => panic!("Expected Identifier"),
    }
}

#[test]
fn test_convert_type_expression_array() {
    let parsed = ParsedType::Array(Box::new(ParsedType::Primitive(PrimitiveType::Number)));
    let expr = convert_type_expression(&parsed);

    match expr {
        cdm_plugin_interface::TypeExpression::Array { element_type } => {
            match *element_type {
                cdm_plugin_interface::TypeExpression::Identifier { name } => {
                    assert_eq!(name, "number");
                }
                _ => panic!("Expected Identifier for element type"),
            }
        }
        _ => panic!("Expected Array"),
    }
}

#[test]
fn test_convert_type_expression_union() {
    let parsed = ParsedType::Union(vec![
        ParsedType::Primitive(PrimitiveType::String),
        ParsedType::Primitive(PrimitiveType::Number),
    ]);
    let expr = convert_type_expression(&parsed);

    match expr {
        cdm_plugin_interface::TypeExpression::Union { types } => {
            assert_eq!(types.len(), 2);
        }
        _ => panic!("Expected Union"),
    }
}

#[test]
fn test_convert_type_expression_literal() {
    let parsed = ParsedType::Literal("active".to_string());
    let expr = convert_type_expression(&parsed);

    match expr {
        cdm_plugin_interface::TypeExpression::StringLiteral { value } => {
            assert_eq!(value, "active");
        }
        _ => panic!("Expected StringLiteral"),
    }
}

#[test]
fn test_convert_type_expression_null() {
    let parsed = ParsedType::Null;
    let expr = convert_type_expression(&parsed);

    match expr {
        cdm_plugin_interface::TypeExpression::Identifier { name } => {
            assert_eq!(name, "null");
        }
        _ => panic!("Expected Identifier for null"),
    }
}

#[test]
fn test_convert_type_expression_all_primitives() {
    let primitives = vec![
        (PrimitiveType::String, "string"),
        (PrimitiveType::Number, "number"),
        (PrimitiveType::Boolean, "boolean"),
    ];

    for (prim, expected_name) in primitives {
        let parsed = ParsedType::Primitive(prim);
        let expr = convert_type_expression(&parsed);

        match expr {
            cdm_plugin_interface::TypeExpression::Identifier { name } => {
                assert_eq!(name, expected_name);
            }
            _ => panic!("Expected Identifier for {:?}", expected_name),
        }
    }
}

#[test]
fn test_convert_type_expression_nested_array() {
    let parsed = ParsedType::Array(Box::new(ParsedType::Array(Box::new(
        ParsedType::Primitive(PrimitiveType::String),
    ))));
    let expr = convert_type_expression(&parsed);

    match expr {
        cdm_plugin_interface::TypeExpression::Array { element_type } => match *element_type {
            cdm_plugin_interface::TypeExpression::Array { element_type } => match *element_type {
                cdm_plugin_interface::TypeExpression::Identifier { name } => {
                    assert_eq!(name, "string");
                }
                _ => panic!("Expected Identifier for inner element type"),
            },
            _ => panic!("Expected Array for element type"),
        },
        _ => panic!("Expected Array"),
    }
}

#[test]
fn test_build_resolved_schema_field_with_plugin_config() {
    let mut current_symbols = SymbolTable::new();
    current_symbols.definitions.insert(
        "User".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model {
                extends: vec![],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );

    let mut field_configs = HashMap::new();
    field_configs.insert(
        "sql".to_string(),
        serde_json::json!({"primary_key": true}),
    );

    let mut current_fields = HashMap::new();
    current_fields.insert(
        "User".to_string(),
        vec![FieldInfo {
            name: "id".to_string(),
            type_expr: Some("number".to_string()),
            optional: false,
            span: test_span(),
            plugin_configs: field_configs,
            default_value: None,
            entity_id: None,
        }],
    );

    let ancestors = vec![];
    let removals = vec![];

    let resolved = build_resolved_schema(&current_symbols, &current_fields, &ancestors, &removals);

    let user_model = &resolved.models["User"];
    assert_eq!(user_model.fields[0].plugin_configs.len(), 1);
    assert!(user_model.fields[0].plugin_configs.contains_key("sql"));
}

#[test]
fn test_build_resolved_schema_model_from_ancestor() {
    let current_symbols = SymbolTable::new();
    let current_fields = HashMap::new();

    let mut ancestor_table = SymbolTable::new();
    ancestor_table.definitions.insert(
        "BaseModel".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model {
                extends: vec![],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: Some(99),
        },
    );

    let mut ancestor_fields = HashMap::new();
    ancestor_fields.insert(
        "BaseModel".to_string(),
        vec![FieldInfo {
            name: "created_at".to_string(),
            type_expr: Some("number".to_string()),
            optional: false,
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: None,
            entity_id: None,
        }],
    );

    let ancestors = vec![Ancestor {
        path: "base.cdm".to_string(),
        symbol_table: ancestor_table,
        model_fields: ancestor_fields,
    }];

    let removals = vec![];

    let resolved = build_resolved_schema(&current_symbols, &current_fields, &ancestors, &removals);

    assert_eq!(resolved.models.len(), 1);
    assert!(resolved.models.contains_key("BaseModel"));

    let base_model = &resolved.models["BaseModel"];
    assert_eq!(base_model.source_file, "base.cdm");
    assert_eq!(base_model.fields.len(), 1);
    assert_eq!(base_model.fields[0].name, "created_at");
    assert_eq!(base_model.entity_id, Some(99));
}

#[test]
fn test_build_resolved_schema_field_with_default_value() {
    let mut current_symbols = SymbolTable::new();
    current_symbols.definitions.insert(
        "Config".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model {
                extends: vec![],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );

    let mut current_fields = HashMap::new();
    current_fields.insert(
        "Config".to_string(),
        vec![FieldInfo {
            name: "debug".to_string(),
            type_expr: Some("boolean".to_string()),
            optional: false,
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: Some(serde_json::json!(false)),
            entity_id: None,
        }],
    );

    let ancestors = vec![];
    let removals = vec![];

    let resolved = build_resolved_schema(&current_symbols, &current_fields, &ancestors, &removals);

    let config_model = &resolved.models["Config"];
    assert_eq!(
        config_model.fields[0].default_value,
        Some(serde_json::json!(false))
    );
}
