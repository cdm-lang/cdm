use super::*;
use std::collections::HashMap;
use cdm_plugin_interface::{TypeExpression, Value, FieldDefinition, ModelDefinition, TypeAliasDefinition};
use serde_json::json;

// Helper to create a simple identifier type
fn ident_type(name: &str) -> TypeExpression {
    TypeExpression::Identifier { name: name.to_string() }
}

// Helper to create an array type
fn array_type(element: TypeExpression) -> TypeExpression {
    TypeExpression::Array { element_type: Box::new(element) }
}

// Helper to create a union type
fn union_type(types: Vec<TypeExpression>) -> TypeExpression {
    TypeExpression::Union { types }
}

// Helper to create a string literal type
fn string_literal(value: &str) -> TypeExpression {
    TypeExpression::StringLiteral { value: value.to_string() }
}

// Helper for test spans
fn test_span() -> cdm_utils::Span {
    cdm_utils::Span {
        start: cdm_utils::Position { line: 0, column: 0 },
        end: cdm_utils::Position { line: 0, column: 0 },
    }
}

#[test]
#[serial_test::serial]
fn test_resolve_plugin_path_registry_plugin() {
    // This test verifies that a plugin can be resolved from the registry in migrate
    // It uses the real typescript plugin from the registry
    let source_file = std::path::PathBuf::from("test.cdm");

    let import = crate::PluginImport {
        name: "typescript".to_string(),
        source: None, // No source = try local, then registry
        global_config: Some(json!({
            "version": "0.1.0"
        })),
        source_file: source_file.clone(),
        span: test_span(),
    };

    let result = crate::plugin_resolver::resolve_plugin_path(&import);

    // Should succeed - will download from registry if not cached
    assert!(
        result.is_ok(),
        "Registry plugin resolution should succeed: {:?}",
        result.err()
    );

    let wasm_path = result.unwrap();
    assert!(
        wasm_path.exists(),
        "Resolved WASM file should exist: {}",
        wasm_path.display()
    );

    // Verify it's in the cache directory (platform-specific location with "plugins/typescript")
    let path_str = wasm_path.to_string_lossy();
    assert!(
        path_str.contains("plugins/typescript"),
        "Plugin should be cached in plugins/typescript directory, got: {}",
        path_str
    );
}

#[test]
#[serial_test::serial]
fn test_resolve_plugin_path_registry_plugin_cached() {
    // This test verifies that cached plugins are reused in migrate
    // First resolution will download (if needed), second should use cache
    let source_file = std::path::PathBuf::from("test.cdm");

    let import = crate::PluginImport {
        name: "typescript".to_string(),
        source: None,
        global_config: Some(json!({
            "version": "0.1.0"
        })),
        source_file: source_file.clone(),
        span: test_span(),
    };

    // First resolution
    let result1 = crate::plugin_resolver::resolve_plugin_path(&import);
    assert!(result1.is_ok(), "First resolution should succeed");
    let path1 = result1.unwrap();

    // Second resolution should return the same cached path
    let result2 = crate::plugin_resolver::resolve_plugin_path(&import);
    assert!(result2.is_ok(), "Second resolution should succeed");
    let path2 = result2.unwrap();

    assert_eq!(path1, path2, "Cached plugin should return same path");
    assert!(path1.exists(), "Cached plugin file should exist");
}

#[test]
fn test_types_equal_identifiers() {
    assert!(types_equal(&ident_type("string"), &ident_type("string")));
    assert!(!types_equal(&ident_type("string"), &ident_type("number")));
}

#[test]
fn test_types_equal_arrays() {
    assert!(types_equal(
        &array_type(ident_type("string")),
        &array_type(ident_type("string"))
    ));
    assert!(!types_equal(
        &array_type(ident_type("string")),
        &array_type(ident_type("number"))
    ));
}

#[test]
fn test_types_equal_unions_order_independent() {
    let union1 = union_type(vec![ident_type("string"), ident_type("number")]);
    let union2 = union_type(vec![ident_type("number"), ident_type("string")]);
    assert!(types_equal(&union1, &union2));
}

#[test]
fn test_types_equal_unions_different_length() {
    let union1 = union_type(vec![ident_type("string"), ident_type("number")]);
    let union2 = union_type(vec![ident_type("string")]);
    assert!(!types_equal(&union1, &union2));
}

#[test]
fn test_types_equal_string_literals() {
    assert!(types_equal(&string_literal("active"), &string_literal("active")));
    assert!(!types_equal(&string_literal("active"), &string_literal("pending")));
}

#[test]
fn test_types_equal_mixed_types() {
    assert!(!types_equal(&ident_type("string"), &array_type(ident_type("string"))));
    assert!(!types_equal(&ident_type("string"), &string_literal("string")));
}

#[test]
fn test_values_equal_none() {
    assert!(values_equal(&None, &None));
}

#[test]
fn test_values_equal_some_vs_none() {
    assert!(!values_equal(&Some(Value::String("test".to_string())), &None));
    assert!(!values_equal(&None, &Some(Value::String("test".to_string()))));
}

#[test]
fn test_values_equal_strings() {
    assert!(values_equal(
        &Some(Value::String("test".to_string())),
        &Some(Value::String("test".to_string()))
    ));
    assert!(!values_equal(
        &Some(Value::String("test".to_string())),
        &Some(Value::String("other".to_string()))
    ));
}

#[test]
fn test_values_equal_numbers() {
    assert!(values_equal(
        &Some(Value::Number(42.0)),
        &Some(Value::Number(42.0))
    ));
    assert!(!values_equal(
        &Some(Value::Number(42.0)),
        &Some(Value::Number(43.0))
    ));
}

#[test]
fn test_values_equal_booleans() {
    assert!(values_equal(
        &Some(Value::Boolean(true)),
        &Some(Value::Boolean(true))
    ));
    assert!(!values_equal(
        &Some(Value::Boolean(true)),
        &Some(Value::Boolean(false))
    ));
}

#[test]
fn test_values_equal_different_types() {
    assert!(!values_equal(
        &Some(Value::String("42".to_string())),
        &Some(Value::Number(42.0))
    ));
}

#[test]
fn test_configs_equal_same() {
    assert!(configs_equal(&json!({"key": "value"}), &json!({"key": "value"})));
}

#[test]
fn test_configs_equal_different() {
    assert!(!configs_equal(&json!({"key": "value"}), &json!({"key": "other"})));
}

#[test]
fn test_configs_equal_nested() {
    assert!(configs_equal(
        &json!({"outer": {"inner": "value"}}),
        &json!({"outer": {"inner": "value"}})
    ));
    assert!(!configs_equal(
        &json!({"outer": {"inner": "value"}}),
        &json!({"outer": {"inner": "other"}})
    ));
}

#[test]
fn test_compute_type_alias_deltas_addition() {
    let previous = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };

    let mut current_aliases = HashMap::new();
    current_aliases.insert(
        "Email".to_string(),
        TypeAliasDefinition {
            name: "Email".to_string(),
            alias_type: ident_type("string"),
            config: json!({}),
            entity_id: Some(1),
        },
    );

    let current = Schema {
        models: HashMap::new(),
        type_aliases: current_aliases,
    };

    let mut deltas = Vec::new();
    compute_type_alias_deltas(&previous, &current, &mut deltas).unwrap();

    assert_eq!(deltas.len(), 1);
    match &deltas[0] {
        Delta::TypeAliasAdded { name, .. } => {
            assert_eq!(name, "Email");
        }
        _ => panic!("Expected TypeAliasAdded delta"),
    }
}

#[test]
fn test_compute_type_alias_deltas_removal() {
    let mut previous_aliases = HashMap::new();
    previous_aliases.insert(
        "Email".to_string(),
        TypeAliasDefinition {
            name: "Email".to_string(),
            alias_type: ident_type("string"),
            config: json!({}),
            entity_id: Some(1),
        },
    );

    let previous = Schema {
        models: HashMap::new(),
        type_aliases: previous_aliases,
    };

    let current = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };

    let mut deltas = Vec::new();
    compute_type_alias_deltas(&previous, &current, &mut deltas).unwrap();

    assert_eq!(deltas.len(), 1);
    match &deltas[0] {
        Delta::TypeAliasRemoved { name, .. } => {
            assert_eq!(name, "Email");
        }
        _ => panic!("Expected TypeAliasRemoved delta"),
    }
}

#[test]
fn test_compute_type_alias_deltas_rename() {
    let mut previous_aliases = HashMap::new();
    previous_aliases.insert(
        "Email".to_string(),
        TypeAliasDefinition {
            name: "Email".to_string(),
            alias_type: ident_type("string"),
            config: json!({}),
            entity_id: Some(1),
        },
    );

    let previous = Schema {
        models: HashMap::new(),
        type_aliases: previous_aliases,
    };

    let mut current_aliases = HashMap::new();
    current_aliases.insert(
        "EmailAddress".to_string(),
        TypeAliasDefinition {
            name: "EmailAddress".to_string(),
            alias_type: ident_type("string"),
            config: json!({}),
            entity_id: Some(1), // Same ID, different name = rename
        },
    );

    let current = Schema {
        models: HashMap::new(),
        type_aliases: current_aliases,
    };

    let mut deltas = Vec::new();
    compute_type_alias_deltas(&previous, &current, &mut deltas).unwrap();

    assert_eq!(deltas.len(), 1);
    match &deltas[0] {
        Delta::TypeAliasRenamed { old_name, new_name, id, .. } => {
            assert_eq!(old_name, "Email");
            assert_eq!(new_name, "EmailAddress");
            assert_eq!(*id, Some(1));
        }
        _ => panic!("Expected TypeAliasRenamed delta"),
    }
}

#[test]
fn test_compute_type_alias_deltas_type_changed() {
    let mut previous_aliases = HashMap::new();
    previous_aliases.insert(
        "Email".to_string(),
        TypeAliasDefinition {
            name: "Email".to_string(),
            alias_type: ident_type("string"),
            config: json!({}),
            entity_id: Some(1),
        },
    );

    let previous = Schema {
        models: HashMap::new(),
        type_aliases: previous_aliases,
    };

    let mut current_aliases = HashMap::new();
    current_aliases.insert(
        "Email".to_string(),
        TypeAliasDefinition {
            name: "Email".to_string(),
            alias_type: array_type(ident_type("string")), // Changed type
            config: json!({}),
            entity_id: Some(1),
        },
    );

    let current = Schema {
        models: HashMap::new(),
        type_aliases: current_aliases,
    };

    let mut deltas = Vec::new();
    compute_type_alias_deltas(&previous, &current, &mut deltas).unwrap();

    assert_eq!(deltas.len(), 1);
    match &deltas[0] {
        Delta::TypeAliasTypeChanged { name, before, after } => {
            assert_eq!(name, "Email");
            assert!(types_equal(before, &ident_type("string")));
            assert!(types_equal(after, &array_type(ident_type("string"))));
        }
        _ => panic!("Expected TypeAliasTypeChanged delta"),
    }
}

#[test]
fn test_compute_model_deltas_addition() {
    let previous = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };

    let mut current_models = HashMap::new();
    current_models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![],
            config: json!({}),
            entity_id: Some(1),
        },
    );

    let current = Schema {
        models: current_models,
        type_aliases: HashMap::new(),
    };

    let mut deltas = Vec::new();
    compute_model_deltas(&previous, &current, &mut deltas).unwrap();

    assert_eq!(deltas.len(), 1);
    match &deltas[0] {
        Delta::ModelAdded { name, .. } => {
            assert_eq!(name, "User");
        }
        _ => panic!("Expected ModelAdded delta"),
    }
}

#[test]
fn test_compute_model_deltas_removal() {
    let mut previous_models = HashMap::new();
    previous_models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![],
            config: json!({}),
            entity_id: Some(1),
        },
    );

    let previous = Schema {
        models: previous_models,
        type_aliases: HashMap::new(),
    };

    let current = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };

    let mut deltas = Vec::new();
    compute_model_deltas(&previous, &current, &mut deltas).unwrap();

    assert_eq!(deltas.len(), 1);
    match &deltas[0] {
        Delta::ModelRemoved { name, .. } => {
            assert_eq!(name, "User");
        }
        _ => panic!("Expected ModelRemoved delta"),
    }
}

#[test]
fn test_compute_model_deltas_rename() {
    let mut previous_models = HashMap::new();
    previous_models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![],
            config: json!({}),
            entity_id: Some(1),
        },
    );

    let previous = Schema {
        models: previous_models,
        type_aliases: HashMap::new(),
    };

    let mut current_models = HashMap::new();
    current_models.insert(
        "Account".to_string(),
        ModelDefinition {
            name: "Account".to_string(),
            parents: vec![],
            fields: vec![],
            config: json!({}),
            entity_id: Some(1), // Same ID, different name = rename
        },
    );

    let current = Schema {
        models: current_models,
        type_aliases: HashMap::new(),
    };

    let mut deltas = Vec::new();
    compute_model_deltas(&previous, &current, &mut deltas).unwrap();

    assert_eq!(deltas.len(), 1);
    match &deltas[0] {
        Delta::ModelRenamed { old_name, new_name, id, .. } => {
            assert_eq!(old_name, "User");
            assert_eq!(new_name, "Account");
            assert_eq!(*id, Some(1));
        }
        _ => panic!("Expected ModelRenamed delta"),
    }
}

#[test]
fn test_compute_model_deltas_config_changed() {
    let mut previous_models = HashMap::new();
    previous_models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![],
            config: json!({"table": "users"}),
            entity_id: Some(1),
        },
    );

    let previous = Schema {
        models: previous_models,
        type_aliases: HashMap::new(),
    };

    let mut current_models = HashMap::new();
    current_models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![],
            config: json!({"table": "accounts"}), // Changed config
            entity_id: Some(1),
        },
    );

    let current = Schema {
        models: current_models,
        type_aliases: HashMap::new(),
    };

    let mut deltas = Vec::new();
    compute_model_deltas(&previous, &current, &mut deltas).unwrap();

    assert_eq!(deltas.len(), 1);
    match &deltas[0] {
        Delta::ModelConfigChanged { model, before, after } => {
            assert_eq!(model, "User");
            assert_eq!(before, &json!({"table": "users"}));
            assert_eq!(after, &json!({"table": "accounts"}));
        }
        _ => panic!("Expected ModelConfigChanged delta"),
    }
}

#[test]
fn test_compute_field_deltas_addition() {
    let prev_fields = vec![];
    let curr_fields = vec![
        FieldDefinition {
            name: "email".to_string(),
            field_type: ident_type("string"),
            optional: false,
            default: None,
            config: json!({}),
            entity_id: Some(1),
        },
    ];

    let mut deltas = Vec::new();
    compute_field_deltas("User", &prev_fields, &curr_fields, &mut deltas).unwrap();

    assert_eq!(deltas.len(), 1);
    match &deltas[0] {
        Delta::FieldAdded { model, field, .. } => {
            assert_eq!(model, "User");
            assert_eq!(field, "email");
        }
        _ => panic!("Expected FieldAdded delta"),
    }
}

#[test]
fn test_compute_field_deltas_removal() {
    let prev_fields = vec![
        FieldDefinition {
            name: "email".to_string(),
            field_type: ident_type("string"),
            optional: false,
            default: None,
            config: json!({}),
            entity_id: Some(1),
        },
    ];
    let curr_fields = vec![];

    let mut deltas = Vec::new();
    compute_field_deltas("User", &prev_fields, &curr_fields, &mut deltas).unwrap();

    assert_eq!(deltas.len(), 1);
    match &deltas[0] {
        Delta::FieldRemoved { model, field, .. } => {
            assert_eq!(model, "User");
            assert_eq!(field, "email");
        }
        _ => panic!("Expected FieldRemoved delta"),
    }
}

#[test]
fn test_compute_field_deltas_rename() {
    let prev_fields = vec![
        FieldDefinition {
            name: "email".to_string(),
            field_type: ident_type("string"),
            optional: false,
            default: None,
            config: json!({}),
            entity_id: Some(1),
        },
    ];
    let curr_fields = vec![
        FieldDefinition {
            name: "emailAddress".to_string(),
            field_type: ident_type("string"),
            optional: false,
            default: None,
            config: json!({}),
            entity_id: Some(1), // Same ID, different name = rename
        },
    ];

    let mut deltas = Vec::new();
    compute_field_deltas("User", &prev_fields, &curr_fields, &mut deltas).unwrap();

    assert_eq!(deltas.len(), 1);
    match &deltas[0] {
        Delta::FieldRenamed { model, old_name, new_name, id, .. } => {
            assert_eq!(model, "User");
            assert_eq!(old_name, "email");
            assert_eq!(new_name, "emailAddress");
            assert_eq!(*id, Some(1));
        }
        _ => panic!("Expected FieldRenamed delta"),
    }
}

#[test]
fn test_compute_field_deltas_type_changed() {
    let prev_fields = vec![
        FieldDefinition {
            name: "email".to_string(),
            field_type: ident_type("string"),
            optional: false,
            default: None,
            config: json!({}),
            entity_id: Some(1),
        },
    ];
    let curr_fields = vec![
        FieldDefinition {
            name: "email".to_string(),
            field_type: array_type(ident_type("string")), // Changed type
            optional: false,
            default: None,
            config: json!({}),
            entity_id: Some(1),
        },
    ];

    let mut deltas = Vec::new();
    compute_field_deltas("User", &prev_fields, &curr_fields, &mut deltas).unwrap();

    assert_eq!(deltas.len(), 1);
    match &deltas[0] {
        Delta::FieldTypeChanged { model, field, before, after } => {
            assert_eq!(model, "User");
            assert_eq!(field, "email");
            assert!(types_equal(before, &ident_type("string")));
            assert!(types_equal(after, &array_type(ident_type("string"))));
        }
        _ => panic!("Expected FieldTypeChanged delta"),
    }
}

#[test]
fn test_compute_field_deltas_type_changed_from_implicit_string() {
    // This test simulates the bug where a field with no type specified (defaults to string)
    // is changed to an explicit non-string type. The previous schema will have "string"
    // (from the default), and the current schema should have the new type.
    let prev_fields = vec![
        FieldDefinition {
            name: "count".to_string(),
            field_type: ident_type("string"), // Implicit string (no type specified in CDM)
            optional: false,
            default: None,
            config: json!({}),
            entity_id: Some(1),
        },
    ];
    let curr_fields = vec![
        FieldDefinition {
            name: "count".to_string(),
            field_type: ident_type("number"), // Now explicitly typed as number
            optional: false,
            default: None,
            config: json!({}),
            entity_id: Some(1),
        },
    ];

    let mut deltas = Vec::new();
    compute_field_deltas("User", &prev_fields, &curr_fields, &mut deltas).unwrap();

    assert_eq!(deltas.len(), 1, "Expected exactly one delta for type change from implicit string to explicit number");
    match &deltas[0] {
        Delta::FieldTypeChanged { model, field, before, after } => {
            assert_eq!(model, "User");
            assert_eq!(field, "count");
            assert!(types_equal(before, &ident_type("string")), "Expected before type to be string");
            assert!(types_equal(after, &ident_type("number")), "Expected after type to be number");
        }
        _ => panic!("Expected FieldTypeChanged delta, got: {:?}", deltas[0]),
    }
}

#[test]
fn test_compute_field_deltas_type_changed_without_entity_id() {
    // BUG: When fields don't have entity IDs, type changes are not detected
    // This happens when the previous schema was saved before entity IDs were added,
    // or when fields are defined without explicit IDs.
    let prev_fields = vec![
        FieldDefinition {
            name: "count".to_string(),
            field_type: ident_type("string"), // Was implicitly string
            optional: false,
            default: None,
            config: json!({}),
            entity_id: None, // No entity ID
        },
    ];
    let curr_fields = vec![
        FieldDefinition {
            name: "count".to_string(),
            field_type: ident_type("number"), // Now explicitly number
            optional: false,
            default: None,
            config: json!({}),
            entity_id: None, // Still no entity ID
        },
    ];

    let mut deltas = Vec::new();
    compute_field_deltas("User", &prev_fields, &curr_fields, &mut deltas).unwrap();

    assert_eq!(deltas.len(), 1, "Expected exactly one delta for type change");
    match &deltas[0] {
        Delta::FieldTypeChanged { model, field, before, after } => {
            assert_eq!(model, "User");
            assert_eq!(field, "count");
            assert!(types_equal(before, &ident_type("string")), "Expected before type to be string");
            assert!(types_equal(after, &ident_type("number")), "Expected after type to be number");
        }
        _ => panic!("Expected FieldTypeChanged delta, got: {:?}", deltas[0]),
    }
}

#[test]
fn test_compute_field_deltas_optionality_changed_without_entity_id() {
    // Test that optionality changes are detected for fields without entity IDs
    let prev_fields = vec![
        FieldDefinition {
            name: "bio".to_string(),
            field_type: ident_type("string"),
            optional: false,
            default: None,
            config: json!({}),
            entity_id: None,
        },
    ];
    let curr_fields = vec![
        FieldDefinition {
            name: "bio".to_string(),
            field_type: ident_type("string"),
            optional: true, // Changed to optional
            default: None,
            config: json!({}),
            entity_id: None,
        },
    ];

    let mut deltas = Vec::new();
    compute_field_deltas("User", &prev_fields, &curr_fields, &mut deltas).unwrap();

    assert_eq!(deltas.len(), 1, "Expected exactly one delta for optionality change");
    match &deltas[0] {
        Delta::FieldOptionalityChanged { model, field, before, after } => {
            assert_eq!(model, "User");
            assert_eq!(field, "bio");
            assert_eq!(*before, false);
            assert_eq!(*after, true);
        }
        _ => panic!("Expected FieldOptionalityChanged delta, got: {:?}", deltas[0]),
    }
}

#[test]
fn test_compute_field_deltas_default_changed_without_entity_id() {
    // Test that default value changes are detected for fields without entity IDs
    let prev_fields = vec![
        FieldDefinition {
            name: "status".to_string(),
            field_type: ident_type("string"),
            optional: false,
            default: Some(Value::String("draft".to_string())),
            config: json!({}),
            entity_id: None,
        },
    ];
    let curr_fields = vec![
        FieldDefinition {
            name: "status".to_string(),
            field_type: ident_type("string"),
            optional: false,
            default: Some(Value::String("published".to_string())), // Changed default
            config: json!({}),
            entity_id: None,
        },
    ];

    let mut deltas = Vec::new();
    compute_field_deltas("User", &prev_fields, &curr_fields, &mut deltas).unwrap();

    assert_eq!(deltas.len(), 1, "Expected exactly one delta for default change");
    match &deltas[0] {
        Delta::FieldDefaultChanged { model, field, before, after } => {
            assert_eq!(model, "User");
            assert_eq!(field, "status");
            match (before, after) {
                (Some(Value::String(b)), Some(Value::String(a))) => {
                    assert_eq!(b, "draft");
                    assert_eq!(a, "published");
                }
                _ => panic!("Expected string values"),
            }
        }
        _ => panic!("Expected FieldDefaultChanged delta, got: {:?}", deltas[0]),
    }
}

#[test]
fn test_compute_field_deltas_multiple_changes_without_entity_id() {
    // Test that multiple changes are detected for a field without entity ID
    let prev_fields = vec![
        FieldDefinition {
            name: "score".to_string(),
            field_type: ident_type("string"), // Was string
            optional: false,
            default: None,
            config: json!({}),
            entity_id: None,
        },
    ];
    let curr_fields = vec![
        FieldDefinition {
            name: "score".to_string(),
            field_type: ident_type("number"), // Now number
            optional: true, // Now optional
            default: Some(Value::Number(0.0)), // Added default
            config: json!({"indexed": true}), // Changed config
            entity_id: None,
        },
    ];

    let mut deltas = Vec::new();
    compute_field_deltas("User", &prev_fields, &curr_fields, &mut deltas).unwrap();

    assert_eq!(deltas.len(), 4, "Expected four deltas for type, optionality, default, and config changes");

    // Check that all expected deltas are present
    let has_type_change = deltas.iter().any(|d| matches!(d, Delta::FieldTypeChanged { .. }));
    let has_optionality_change = deltas.iter().any(|d| matches!(d, Delta::FieldOptionalityChanged { .. }));
    let has_default_change = deltas.iter().any(|d| matches!(d, Delta::FieldDefaultChanged { .. }));
    let has_config_change = deltas.iter().any(|d| matches!(d, Delta::FieldConfigChanged { .. }));

    assert!(has_type_change, "Expected FieldTypeChanged delta");
    assert!(has_optionality_change, "Expected FieldOptionalityChanged delta");
    assert!(has_default_change, "Expected FieldDefaultChanged delta");
    assert!(has_config_change, "Expected FieldConfigChanged delta");
}

#[test]
fn test_compute_field_deltas_optionality_changed() {
    let prev_fields = vec![
        FieldDefinition {
            name: "email".to_string(),
            field_type: ident_type("string"),
            optional: false,
            default: None,
            config: json!({}),
            entity_id: Some(1),
        },
    ];
    let curr_fields = vec![
        FieldDefinition {
            name: "email".to_string(),
            field_type: ident_type("string"),
            optional: true, // Changed optionality
            default: None,
            config: json!({}),
            entity_id: Some(1),
        },
    ];

    let mut deltas = Vec::new();
    compute_field_deltas("User", &prev_fields, &curr_fields, &mut deltas).unwrap();

    assert_eq!(deltas.len(), 1);
    match &deltas[0] {
        Delta::FieldOptionalityChanged { model, field, before, after } => {
            assert_eq!(model, "User");
            assert_eq!(field, "email");
            assert_eq!(*before, false);
            assert_eq!(*after, true);
        }
        _ => panic!("Expected FieldOptionalityChanged delta"),
    }
}

#[test]
fn test_compute_field_deltas_default_changed() {
    let prev_fields = vec![
        FieldDefinition {
            name: "status".to_string(),
            field_type: ident_type("string"),
            optional: false,
            default: Some(Value::String("active".to_string())),
            config: json!({}),
            entity_id: Some(1),
        },
    ];
    let curr_fields = vec![
        FieldDefinition {
            name: "status".to_string(),
            field_type: ident_type("string"),
            optional: false,
            default: Some(Value::String("pending".to_string())), // Changed default
            config: json!({}),
            entity_id: Some(1),
        },
    ];

    let mut deltas = Vec::new();
    compute_field_deltas("User", &prev_fields, &curr_fields, &mut deltas).unwrap();

    assert_eq!(deltas.len(), 1);
    match &deltas[0] {
        Delta::FieldDefaultChanged { model, field, before, after } => {
            assert_eq!(model, "User");
            assert_eq!(field, "status");
            // Check values using pattern matching since Value doesn't implement PartialEq
            match (before, after) {
                (Some(Value::String(b)), Some(Value::String(a))) => {
                    assert_eq!(b, "active");
                    assert_eq!(a, "pending");
                }
                _ => panic!("Expected string values"),
            }
        }
        _ => panic!("Expected FieldDefaultChanged delta"),
    }
}

#[test]
fn test_compute_field_deltas_config_changed() {
    let prev_fields = vec![
        FieldDefinition {
            name: "email".to_string(),
            field_type: ident_type("string"),
            optional: false,
            default: None,
            config: json!({"unique": true}),
            entity_id: Some(1),
        },
    ];
    let curr_fields = vec![
        FieldDefinition {
            name: "email".to_string(),
            field_type: ident_type("string"),
            optional: false,
            default: None,
            config: json!({"unique": false}), // Changed config
            entity_id: Some(1),
        },
    ];

    let mut deltas = Vec::new();
    compute_field_deltas("User", &prev_fields, &curr_fields, &mut deltas).unwrap();

    assert_eq!(deltas.len(), 1);
    match &deltas[0] {
        Delta::FieldConfigChanged { model, field, before, after } => {
            assert_eq!(model, "User");
            assert_eq!(field, "email");
            assert_eq!(before, &json!({"unique": true}));
            assert_eq!(after, &json!({"unique": false}));
        }
        _ => panic!("Expected FieldConfigChanged delta"),
    }
}

#[test]
fn test_compute_inheritance_deltas_added() {
    let prev_parents = vec![];
    let curr_parents = vec!["Base".to_string()];

    let mut deltas = Vec::new();
    compute_inheritance_deltas("User", &prev_parents, &curr_parents, &mut deltas);

    assert_eq!(deltas.len(), 1);
    match &deltas[0] {
        Delta::InheritanceAdded { model, parent } => {
            assert_eq!(model, "User");
            assert_eq!(parent, "Base");
        }
        _ => panic!("Expected InheritanceAdded delta"),
    }
}

#[test]
fn test_compute_inheritance_deltas_removed() {
    let prev_parents = vec!["Base".to_string()];
    let curr_parents = vec![];

    let mut deltas = Vec::new();
    compute_inheritance_deltas("User", &prev_parents, &curr_parents, &mut deltas);

    assert_eq!(deltas.len(), 1);
    match &deltas[0] {
        Delta::InheritanceRemoved { model, parent } => {
            assert_eq!(model, "User");
            assert_eq!(parent, "Base");
        }
        _ => panic!("Expected InheritanceRemoved delta"),
    }
}

#[test]
fn test_compute_field_deltas_without_entity_ids() {
    // Test that fields without entity IDs are treated as remove+add, not renames
    let prev_fields = vec![
        FieldDefinition {
            name: "email".to_string(),
            field_type: ident_type("string"),
            optional: false,
            default: None,
            config: json!({}),
            entity_id: None, // No entity ID
        },
    ];
    let curr_fields = vec![
        FieldDefinition {
            name: "emailAddress".to_string(),
            field_type: ident_type("string"),
            optional: false,
            default: None,
            config: json!({}),
            entity_id: None, // No entity ID
        },
    ];

    let mut deltas = Vec::new();
    compute_field_deltas("User", &prev_fields, &curr_fields, &mut deltas).unwrap();

    // Should be 2 deltas: removal and addition (not a rename)
    assert_eq!(deltas.len(), 2);

    let has_removal = deltas.iter().any(|d| matches!(d, Delta::FieldRemoved { field, .. } if field == "email"));
    let has_addition = deltas.iter().any(|d| matches!(d, Delta::FieldAdded { field, .. } if field == "emailAddress"));

    assert!(has_removal, "Expected FieldRemoved delta for 'email'");
    assert!(has_addition, "Expected FieldAdded delta for 'emailAddress'");
}

#[test]
fn test_compute_deltas_comprehensive() {
    // Test a comprehensive scenario with multiple types of changes
    let mut prev_models = HashMap::new();
    prev_models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "id".to_string(),
                    field_type: ident_type("number"),
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: Some(1),
                },
                FieldDefinition {
                    name: "name".to_string(),
                    field_type: ident_type("string"),
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: Some(2),
                },
            ],
            config: json!({}),
            entity_id: Some(10),
        },
    );

    let mut prev_aliases = HashMap::new();
    prev_aliases.insert(
        "Email".to_string(),
        TypeAliasDefinition {
            name: "Email".to_string(),
            alias_type: ident_type("string"),
            config: json!({}),
            entity_id: Some(20),
        },
    );

    let previous = Schema {
        models: prev_models,
        type_aliases: prev_aliases,
    };

    let mut curr_models = HashMap::new();
    curr_models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec!["Base".to_string()], // Added inheritance
            fields: vec![
                FieldDefinition {
                    name: "id".to_string(),
                    field_type: ident_type("number"),
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: Some(1),
                },
                FieldDefinition {
                    name: "fullName".to_string(), // Renamed from "name"
                    field_type: ident_type("string"),
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: Some(2),
                },
                FieldDefinition {
                    name: "email".to_string(), // Added field
                    field_type: ident_type("string"),
                    optional: true,
                    default: None,
                    config: json!({}),
                    entity_id: Some(3),
                },
            ],
            config: json!({}),
            entity_id: Some(10),
        },
    );

    let mut curr_aliases = HashMap::new();
    curr_aliases.insert(
        "EmailAddress".to_string(), // Renamed from "Email"
        TypeAliasDefinition {
            name: "EmailAddress".to_string(),
            alias_type: ident_type("string"),
            config: json!({}),
            entity_id: Some(20),
        },
    );

    let current = Schema {
        models: curr_models,
        type_aliases: curr_aliases,
    };

    let deltas = compute_deltas(&previous, &current).unwrap();

    // Verify we have the expected deltas
    let has_type_alias_rename = deltas.iter().any(|d| {
        matches!(d, Delta::TypeAliasRenamed { old_name, new_name, .. }
            if old_name == "Email" && new_name == "EmailAddress")
    });

    let has_inheritance_added = deltas.iter().any(|d| {
        matches!(d, Delta::InheritanceAdded { model, parent }
            if model == "User" && parent == "Base")
    });

    let has_field_rename = deltas.iter().any(|d| {
        matches!(d, Delta::FieldRenamed { model, old_name, new_name, .. }
            if model == "User" && old_name == "name" && new_name == "fullName")
    });

    let has_field_added = deltas.iter().any(|d| {
        matches!(d, Delta::FieldAdded { model, field, .. }
            if model == "User" && field == "email")
    });

    assert!(has_type_alias_rename, "Expected TypeAliasRenamed delta");
    assert!(has_inheritance_added, "Expected InheritanceAdded delta");
    assert!(has_field_rename, "Expected FieldRenamed delta");
    assert!(has_field_added, "Expected FieldAdded delta");
}

#[test]
fn test_compute_deltas_first_migration_no_previous_schema() {
    // BUG TEST: When there's no previous schema (first migration),
    // compute_deltas should generate ModelAdded deltas for all models
    // in the current schema, not return an empty vector.

    // No previous schema
    let previous = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };

    // Current schema has models
    let mut curr_models = HashMap::new();
    curr_models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "id".to_string(),
                    field_type: ident_type("number"),
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: Some(1),
                },
                FieldDefinition {
                    name: "email".to_string(),
                    field_type: ident_type("string"),
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: Some(2),
                },
            ],
            config: json!({}),
            entity_id: Some(10),
        },
    );
    curr_models.insert(
        "Post".to_string(),
        ModelDefinition {
            name: "Post".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "title".to_string(),
                    field_type: ident_type("string"),
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: Some(3),
                },
            ],
            config: json!({}),
            entity_id: Some(20),
        },
    );

    let mut curr_aliases = HashMap::new();
    curr_aliases.insert(
        "Email".to_string(),
        TypeAliasDefinition {
            name: "Email".to_string(),
            alias_type: ident_type("string"),
            config: json!({}),
            entity_id: Some(30),
        },
    );

    let current = Schema {
        models: curr_models,
        type_aliases: curr_aliases,
    };

    let deltas = compute_deltas(&previous, &current).unwrap();

    // Should have deltas for adding all models and type aliases
    assert!(
        !deltas.is_empty(),
        "Expected deltas for first migration, but got empty vector"
    );

    // Check that we have ModelAdded for both models
    let user_added = deltas.iter().any(|d| {
        matches!(d, Delta::ModelAdded { name, .. } if name == "User")
    });
    let post_added = deltas.iter().any(|d| {
        matches!(d, Delta::ModelAdded { name, .. } if name == "Post")
    });
    let email_alias_added = deltas.iter().any(|d| {
        matches!(d, Delta::TypeAliasAdded { name, .. } if name == "Email")
    });

    assert!(user_added, "Expected ModelAdded delta for User model");
    assert!(post_added, "Expected ModelAdded delta for Post model");
    assert!(email_alias_added, "Expected TypeAliasAdded delta for Email type alias");
}

// ============================================================================
// Tests for transform_deltas_for_plugin
// ============================================================================

#[test]
fn test_transform_deltas_for_plugin_unwraps_model_config() {
    // When deltas are computed, configs are wrapped like: {"sql": {"indexes": [...]}}
    // But plugins expect unwrapped configs like: {"indexes": [...]}

    let wrapped_delta = Delta::ModelAdded {
        name: "User".to_string(),
        after: ModelDefinition {
            name: "User".to_string(),
            entity_id: Some(100),
            parents: vec![],
            fields: vec![],
            config: json!({
                "sql": {
                    "table_name": "users",
                    "indexes": [
                        { "fields": ["id"], "unique": true }
                    ]
                }
            }),
        },
    };

    let transformed = super::transform_deltas_for_plugin(&[wrapped_delta], "sql");

    assert_eq!(transformed.len(), 1);

    if let Delta::ModelAdded { after, .. } = &transformed[0] {
        // Config should be unwrapped - directly contain table_name and indexes
        assert_eq!(after.config.get("table_name").and_then(|v| v.as_str()), Some("users"));
        assert!(after.config.get("indexes").is_some());
        // Should NOT have "sql" wrapper
        assert!(after.config.get("sql").is_none());
    } else {
        panic!("Expected ModelAdded delta");
    }
}

#[test]
fn test_transform_deltas_for_plugin_unwraps_field_config() {
    let wrapped_delta = Delta::FieldAdded {
        model: "User".to_string(),
        field: "email".to_string(),
        after: FieldDefinition {
            name: "email".to_string(),
            field_type: ident_type("string"),
            optional: false,
            default: None,
            config: json!({
                "sql": {
                    "column_name": "user_email",
                    "type": "VARCHAR(320)"
                }
            }),
            entity_id: Some(2),
        },
    };

    let transformed = super::transform_deltas_for_plugin(&[wrapped_delta], "sql");

    assert_eq!(transformed.len(), 1);

    if let Delta::FieldAdded { after, .. } = &transformed[0] {
        // Config should be unwrapped
        assert_eq!(after.config.get("column_name").and_then(|v| v.as_str()), Some("user_email"));
        assert_eq!(after.config.get("type").and_then(|v| v.as_str()), Some("VARCHAR(320)"));
        // Should NOT have "sql" wrapper
        assert!(after.config.get("sql").is_none());
    } else {
        panic!("Expected FieldAdded delta");
    }
}

#[test]
fn test_transform_deltas_for_plugin_handles_missing_plugin_config() {
    // When a model has no config for the requested plugin, should return empty object
    let wrapped_delta = Delta::ModelAdded {
        name: "User".to_string(),
        after: ModelDefinition {
            name: "User".to_string(),
            entity_id: Some(100),
            parents: vec![],
            fields: vec![],
            config: json!({
                "typescript": {
                    "interface_name": "IUser"
                }
            }),
        },
    };

    let transformed = super::transform_deltas_for_plugin(&[wrapped_delta], "sql");

    assert_eq!(transformed.len(), 1);

    if let Delta::ModelAdded { after, .. } = &transformed[0] {
        // Config should be empty object since there's no "sql" config
        assert!(after.config.is_object());
        assert!(after.config.as_object().unwrap().is_empty());
    } else {
        panic!("Expected ModelAdded delta");
    }
}

#[test]
fn test_transform_deltas_for_plugin_handles_model_config_changed() {
    let wrapped_delta = Delta::ModelConfigChanged {
        model: "User".to_string(),
        before: json!({
            "sql": { "table_name": "users" }
        }),
        after: json!({
            "sql": {
                "table_name": "users",
                "indexes": [{ "fields": ["email"] }]
            }
        }),
    };

    let transformed = super::transform_deltas_for_plugin(&[wrapped_delta], "sql");

    assert_eq!(transformed.len(), 1);

    if let Delta::ModelConfigChanged { before, after, .. } = &transformed[0] {
        // Before should be unwrapped
        assert_eq!(before.get("table_name").and_then(|v| v.as_str()), Some("users"));
        assert!(before.get("sql").is_none());

        // After should be unwrapped
        assert_eq!(after.get("table_name").and_then(|v| v.as_str()), Some("users"));
        assert!(after.get("indexes").is_some());
        assert!(after.get("sql").is_none());
    } else {
        panic!("Expected ModelConfigChanged delta");
    }
}

#[test]
fn test_transform_deltas_for_plugin_transforms_nested_fields() {
    // When a model is added, its fields should also have their configs unwrapped
    let wrapped_delta = Delta::ModelAdded {
        name: "User".to_string(),
        after: ModelDefinition {
            name: "User".to_string(),
            entity_id: Some(100),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "id".to_string(),
                    field_type: ident_type("number"),
                    optional: false,
                    default: None,
                    config: json!({
                        "sql": { "type": "INTEGER" }
                    }),
                    entity_id: Some(1),
                },
                FieldDefinition {
                    name: "email".to_string(),
                    field_type: ident_type("string"),
                    optional: false,
                    default: None,
                    config: json!({
                        "sql": { "type": "VARCHAR(320)" }
                    }),
                    entity_id: Some(2),
                },
            ],
            config: json!({
                "sql": {
                    "table_name": "users",
                    "indexes": [{ "fields": ["id"], "primary": true }]
                }
            }),
        },
    };

    let transformed = super::transform_deltas_for_plugin(&[wrapped_delta], "sql");

    assert_eq!(transformed.len(), 1);

    if let Delta::ModelAdded { after, .. } = &transformed[0] {
        // Model config should be unwrapped
        assert_eq!(after.config.get("table_name").and_then(|v| v.as_str()), Some("users"));

        // Field configs should also be unwrapped
        assert_eq!(after.fields.len(), 2);
        assert_eq!(after.fields[0].config.get("type").and_then(|v| v.as_str()), Some("INTEGER"));
        assert_eq!(after.fields[1].config.get("type").and_then(|v| v.as_str()), Some("VARCHAR(320)"));
    } else {
        panic!("Expected ModelAdded delta");
    }
}
