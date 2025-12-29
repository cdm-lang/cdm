use super::*;
use cdm_utils::{Position, Span};

// Helper to create a test span
fn test_span() -> Span {
    Span {
        start: Position { line: 0, column: 0 },
        end: Position { line: 0, column: 10 },
    }
}

#[test]
fn test_symbol_table_new() {
    let table = SymbolTable::new();
    assert_eq!(table.definitions.len(), 0);
}

#[test]
fn test_symbol_table_default() {
    let table = SymbolTable::default();
    assert_eq!(table.definitions.len(), 0);
}

#[test]
fn test_is_builtin_type() {
    assert!(is_builtin_type("string"));
    assert!(is_builtin_type("number"));
    assert!(is_builtin_type("boolean"));
    assert!(is_builtin_type("JSON"));
    assert!(!is_builtin_type("String"));
    assert!(!is_builtin_type("Number"));
    assert!(!is_builtin_type("custom"));
    assert!(!is_builtin_type("User"));
}

#[test]
fn test_symbol_table_is_defined_builtin() {
    let table = SymbolTable::new();
    assert!(table.is_defined("string"));
    assert!(table.is_defined("number"));
    assert!(table.is_defined("boolean"));
    assert!(table.is_defined("JSON"));
}

#[test]
fn test_symbol_table_is_defined_user_type() {
    let mut table = SymbolTable::new();
    table.definitions.insert(
        "Email".to_string(),
        Definition {
            kind: DefinitionKind::TypeAlias {
                references: vec!["string".to_string()],
                type_expr: "string".to_string(),
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );

    assert!(table.is_defined("Email"));
    assert!(!table.is_defined("User"));
}

#[test]
fn test_symbol_table_get() {
    let mut table = SymbolTable::new();
    table.definitions.insert(
        "User".to_string(),
        Definition {
            kind: DefinitionKind::Model {
                extends: vec![],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );

    assert!(table.get("User").is_some());
    assert!(table.get("NonExistent").is_none());
}

#[test]
fn test_is_type_defined_in_local() {
    let mut local = SymbolTable::new();
    local.definitions.insert(
        "LocalType".to_string(),
        Definition {
            kind: DefinitionKind::TypeAlias {
                references: vec![],
                type_expr: "string".to_string(),
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );

    let ancestors = vec![];
    assert!(is_type_defined("LocalType", &local, &ancestors));
    assert!(is_type_defined("string", &local, &ancestors));
    assert!(!is_type_defined("NonExistent", &local, &ancestors));
}

#[test]
fn test_is_type_defined_in_ancestor() {
    let local = SymbolTable::new();

    let mut ancestor_table = SymbolTable::new();
    ancestor_table.definitions.insert(
        "AncestorType".to_string(),
        Definition {
            kind: DefinitionKind::TypeAlias {
                references: vec![],
                type_expr: "number".to_string(),
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );

    let ancestors = vec![Ancestor {
        path: "ancestor.cdm".to_string(),
        symbol_table: ancestor_table,
        model_fields: HashMap::new(),
    }];

    assert!(is_type_defined("AncestorType", &local, &ancestors));
    assert!(!is_type_defined("NonExistent", &local, &ancestors));
}

#[test]
fn test_resolve_definition_from_local() {
    let mut local = SymbolTable::new();
    local.definitions.insert(
        "LocalDef".to_string(),
        Definition {
            kind: DefinitionKind::Model {
                extends: vec![],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );

    let ancestors = vec![];
    let result = resolve_definition("LocalDef", &local, &ancestors);
    assert!(result.is_some());

    let (def, source) = result.unwrap();
    assert!(matches!(def.kind, DefinitionKind::Model { .. }));
    assert!(source.is_none()); // From local, not ancestor
}

#[test]
fn test_resolve_definition_from_ancestor() {
    let local = SymbolTable::new();

    let mut ancestor_table = SymbolTable::new();
    ancestor_table.definitions.insert(
        "AncestorDef".to_string(),
        Definition {
            kind: DefinitionKind::TypeAlias {
                references: vec![],
                type_expr: "boolean".to_string(),
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );

    let ancestors = vec![Ancestor {
        path: "ancestor.cdm".to_string(),
        symbol_table: ancestor_table,
        model_fields: HashMap::new(),
    }];

    let result = resolve_definition("AncestorDef", &local, &ancestors);
    assert!(result.is_some());

    let (def, source) = result.unwrap();
    assert!(matches!(def.kind, DefinitionKind::TypeAlias { .. }));
    assert_eq!(source, Some("ancestor.cdm"));
}

#[test]
fn test_resolve_definition_not_found() {
    let local = SymbolTable::new();
    let ancestors = vec![];

    let result = resolve_definition("NonExistent", &local, &ancestors);
    assert!(result.is_none());
}

#[test]
fn test_resolve_definition_local_overrides_ancestor() {
    let mut local = SymbolTable::new();
    local.definitions.insert(
        "SharedDef".to_string(),
        Definition {
            kind: DefinitionKind::Model {
                extends: vec![],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: Some(1),
        },
    );

    let mut ancestor_table = SymbolTable::new();
    ancestor_table.definitions.insert(
        "SharedDef".to_string(),
        Definition {
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

    let result = resolve_definition("SharedDef", &local, &ancestors);
    assert!(result.is_some());

    let (def, source) = result.unwrap();
    // Should get local definition (Model), not ancestor (TypeAlias)
    assert!(matches!(def.kind, DefinitionKind::Model { .. }));
    assert!(source.is_none());
    assert_eq!(def.entity_id, Some(1));
}

#[test]
fn test_get_inherited_fields_no_parents() {
    let mut local_fields = HashMap::new();
    local_fields.insert(
        "User".to_string(),
        vec![FieldInfo {
            name: "id".to_string(),
            type_expr: Some("number".to_string()),
            optional: false,
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: None,
            entity_id: None,
        }],
    );

    let mut local_table = SymbolTable::new();
    local_table.definitions.insert(
        "User".to_string(),
        Definition {
            kind: DefinitionKind::Model {
                extends: vec![],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );

    let ancestors = vec![];
    let fields = get_inherited_fields("User", &local_fields, &local_table, &ancestors);

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].name, "id");
}

#[test]
fn test_get_inherited_fields_with_parent() {
    let mut local_fields = HashMap::new();
    local_fields.insert(
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
    local_fields.insert(
        "User".to_string(),
        vec![FieldInfo {
            name: "id".to_string(),
            type_expr: Some("number".to_string()),
            optional: false,
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: None,
            entity_id: None,
        }],
    );

    let mut local_table = SymbolTable::new();
    local_table.definitions.insert(
        "User".to_string(),
        Definition {
            kind: DefinitionKind::Model {
                extends: vec![],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );
    local_table.definitions.insert(
        "Admin".to_string(),
        Definition {
            kind: DefinitionKind::Model {
                extends: vec!["User".to_string()],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );

    let ancestors = vec![];
    let fields = get_inherited_fields("Admin", &local_fields, &local_table, &ancestors);

    assert_eq!(fields.len(), 2);
    // Parent fields come first
    assert_eq!(fields[0].name, "id");
    assert_eq!(fields[1].name, "role");
}

#[test]
fn test_get_inherited_fields_from_ancestor() {
    let local_fields = HashMap::new();
    let local_table = SymbolTable::new();

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

    let mut ancestor_table = SymbolTable::new();
    ancestor_table.definitions.insert(
        "BaseModel".to_string(),
        Definition {
            kind: DefinitionKind::Model {
                extends: vec![],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );

    let ancestors = vec![Ancestor {
        path: "base.cdm".to_string(),
        symbol_table: ancestor_table,
        model_fields: ancestor_fields,
    }];

    let fields = get_inherited_fields("BaseModel", &local_fields, &local_table, &ancestors);

    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].name, "created_at");
}

#[test]
fn test_field_exists_in_parents_true() {
    let mut local_fields = HashMap::new();
    local_fields.insert(
        "User".to_string(),
        vec![FieldInfo {
            name: "id".to_string(),
            type_expr: Some("number".to_string()),
            optional: false,
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: None,
            entity_id: None,
        }],
    );
    local_fields.insert(
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

    let mut local_table = SymbolTable::new();
    local_table.definitions.insert(
        "User".to_string(),
        Definition {
            kind: DefinitionKind::Model {
                extends: vec![],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );
    local_table.definitions.insert(
        "Admin".to_string(),
        Definition {
            kind: DefinitionKind::Model {
                extends: vec!["User".to_string()],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );

    let ancestors = vec![];
    assert!(field_exists_in_parents(
        "Admin",
        "id",
        &local_fields,
        &local_table,
        &ancestors
    ));
}

#[test]
fn test_field_exists_in_parents_false() {
    let mut local_fields = HashMap::new();
    local_fields.insert(
        "User".to_string(),
        vec![FieldInfo {
            name: "id".to_string(),
            type_expr: Some("number".to_string()),
            optional: false,
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: None,
            entity_id: None,
        }],
    );

    let mut local_table = SymbolTable::new();
    local_table.definitions.insert(
        "User".to_string(),
        Definition {
            kind: DefinitionKind::Model {
                extends: vec![],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );

    let ancestors = vec![];
    assert!(!field_exists_in_parents(
        "User",
        "nonexistent",
        &local_fields,
        &local_table,
        &ancestors
    ));
}

#[test]
fn test_field_exists_in_parents_implicit_extension() {
    let local_fields = HashMap::new();

    let mut local_table = SymbolTable::new();
    local_table.definitions.insert(
        "User".to_string(),
        Definition {
            kind: DefinitionKind::Model {
                extends: vec![], // No explicit extends
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );

    let mut ancestor_fields = HashMap::new();
    ancestor_fields.insert(
        "User".to_string(),
        vec![FieldInfo {
            name: "id".to_string(),
            type_expr: Some("number".to_string()),
            optional: false,
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: None,
            entity_id: None,
        }],
    );

    let mut ancestor_table = SymbolTable::new();
    ancestor_table.definitions.insert(
        "User".to_string(),
        Definition {
            kind: DefinitionKind::Model {
                extends: vec![],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );

    let ancestors = vec![Ancestor {
        path: "base.cdm".to_string(),
        symbol_table: ancestor_table,
        model_fields: ancestor_fields,
    }];

    // Should find "id" field in ancestor's User model (implicit extension)
    assert!(field_exists_in_parents(
        "User",
        "id",
        &local_fields,
        &local_table,
        &ancestors
    ));
}

#[test]
fn test_symbol_table_display() {
    let mut table = SymbolTable::new();
    table.definitions.insert(
        "Email".to_string(),
        Definition {
            kind: DefinitionKind::TypeAlias {
                references: vec!["string".to_string()],
                type_expr: "string".to_string(),
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );
    table.definitions.insert(
        "User".to_string(),
        Definition {
            kind: DefinitionKind::Model {
                extends: vec![],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );

    let display = format!("{}", table);
    assert!(display.contains("Email"));
    assert!(display.contains("User"));
    assert!(display.contains("type alias"));
    assert!(display.contains("model"));
}

#[test]
fn test_definition_kind_type_alias() {
    let def = Definition {
        kind: DefinitionKind::TypeAlias {
            references: vec!["User".to_string(), "Admin".to_string()],
            type_expr: "User | Admin".to_string(),
        },
        span: test_span(),
        plugin_configs: HashMap::new(),
        entity_id: Some(42),
    };

    match &def.kind {
        DefinitionKind::TypeAlias { references, type_expr } => {
            assert_eq!(references.len(), 2);
            assert_eq!(type_expr, "User | Admin");
        }
        _ => panic!("Expected TypeAlias"),
    }
    assert_eq!(def.entity_id, Some(42));
}

#[test]
fn test_definition_kind_model() {
    let def = Definition {
        kind: DefinitionKind::Model {
            extends: vec!["BaseModel".to_string(), "Timestamped".to_string()],
        },
        span: test_span(),
        plugin_configs: HashMap::new(),
        entity_id: None,
    };

    match &def.kind {
        DefinitionKind::Model { extends } => {
            assert_eq!(extends.len(), 2);
            assert_eq!(extends[0], "BaseModel");
            assert_eq!(extends[1], "Timestamped");
        }
        _ => panic!("Expected Model"),
    }
}

#[test]
fn test_field_info_with_plugin_configs() {
    let mut configs = HashMap::new();
    configs.insert("sql".to_string(), serde_json::json!({"index": true}));

    let field = FieldInfo {
        name: "email".to_string(),
        type_expr: Some("string".to_string()),
        optional: false,
        span: test_span(),
        plugin_configs: configs,
        default_value: Some(serde_json::json!("test@example.com")),
        entity_id: Some(100),
    };

    assert_eq!(field.name, "email");
    assert_eq!(field.type_expr, Some("string".to_string()));
    assert!(!field.optional);
    assert_eq!(field.plugin_configs.len(), 1);
    assert_eq!(field.default_value, Some(serde_json::json!("test@example.com")));
    assert_eq!(field.entity_id, Some(100));
}

#[test]
fn test_ancestor_structure() {
    let mut symbol_table = SymbolTable::new();
    symbol_table.definitions.insert(
        "Base".to_string(),
        Definition {
            kind: DefinitionKind::Model {
                extends: vec![],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );

    let ancestor = Ancestor {
        path: "base.cdm".to_string(),
        symbol_table,
        model_fields: HashMap::new(),
    };

    assert_eq!(ancestor.path, "base.cdm");
    assert_eq!(ancestor.symbol_table.definitions.len(), 1);
}
