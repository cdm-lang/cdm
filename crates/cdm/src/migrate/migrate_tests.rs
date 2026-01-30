use super::*;
use std::collections::HashMap;
use cdm_plugin_interface::{TypeExpression, Value, FieldDefinition, ModelDefinition, TypeAliasDefinition, EntityId};
use serde_json::json;

// Helper to create a local entity ID
fn local_id(id: u64) -> Option<EntityId> {
    Some(EntityId::local(id))
}

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
        name_span: test_span(),
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
        name_span: test_span(),
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
            entity_id: local_id(1),
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
            entity_id: local_id(1),
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
            entity_id: local_id(1),
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
            entity_id: local_id(1), // Same ID, different name = rename
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
            assert_eq!(id.as_ref().map(|e| e.local_id), Some(1));
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
            entity_id: local_id(1),
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
            entity_id: local_id(1),
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
            entity_id: local_id(1),
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
            entity_id: local_id(1),
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
            entity_id: local_id(1),
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
            entity_id: local_id(1), // Same ID, different name = rename
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
            assert_eq!(id.as_ref().map(|e| e.local_id), Some(1));
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
            entity_id: local_id(1),
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
            entity_id: local_id(1),
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
            entity_id: local_id(1),
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
            entity_id: local_id(1),
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
            entity_id: local_id(1),
        },
    ];
    let curr_fields = vec![
        FieldDefinition {
            name: "emailAddress".to_string(),
            field_type: ident_type("string"),
            optional: false,
            default: None,
            config: json!({}),
            entity_id: local_id(1), // Same ID, different name = rename
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
            assert_eq!(id.as_ref().map(|e| e.local_id), Some(1));
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
            entity_id: local_id(1),
        },
    ];
    let curr_fields = vec![
        FieldDefinition {
            name: "email".to_string(),
            field_type: array_type(ident_type("string")), // Changed type
            optional: false,
            default: None,
            config: json!({}),
            entity_id: local_id(1),
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
            entity_id: local_id(1),
        },
    ];
    let curr_fields = vec![
        FieldDefinition {
            name: "count".to_string(),
            field_type: ident_type("number"), // Now explicitly typed as number
            optional: false,
            default: None,
            config: json!({}),
            entity_id: local_id(1),
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
            entity_id: local_id(1),
        },
    ];
    let curr_fields = vec![
        FieldDefinition {
            name: "email".to_string(),
            field_type: ident_type("string"),
            optional: true, // Changed optionality
            default: None,
            config: json!({}),
            entity_id: local_id(1),
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
            entity_id: local_id(1),
        },
    ];
    let curr_fields = vec![
        FieldDefinition {
            name: "status".to_string(),
            field_type: ident_type("string"),
            optional: false,
            default: Some(Value::String("pending".to_string())), // Changed default
            config: json!({}),
            entity_id: local_id(1),
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
            entity_id: local_id(1),
        },
    ];
    let curr_fields = vec![
        FieldDefinition {
            name: "email".to_string(),
            field_type: ident_type("string"),
            optional: false,
            default: None,
            config: json!({"unique": false}), // Changed config
            entity_id: local_id(1),
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
                    entity_id: local_id(1),
                },
                FieldDefinition {
                    name: "name".to_string(),
                    field_type: ident_type("string"),
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(2),
                },
            ],
            config: json!({}),
            entity_id: local_id(10),
        },
    );

    let mut prev_aliases = HashMap::new();
    prev_aliases.insert(
        "Email".to_string(),
        TypeAliasDefinition {
            name: "Email".to_string(),
            alias_type: ident_type("string"),
            config: json!({}),
            entity_id: local_id(20),
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
                    entity_id: local_id(1),
                },
                FieldDefinition {
                    name: "fullName".to_string(), // Renamed from "name"
                    field_type: ident_type("string"),
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(2),
                },
                FieldDefinition {
                    name: "email".to_string(), // Added field
                    field_type: ident_type("string"),
                    optional: true,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(3),
                },
            ],
            config: json!({}),
            entity_id: local_id(10),
        },
    );

    let mut curr_aliases = HashMap::new();
    curr_aliases.insert(
        "EmailAddress".to_string(), // Renamed from "Email"
        TypeAliasDefinition {
            name: "EmailAddress".to_string(),
            alias_type: ident_type("string"),
            config: json!({}),
            entity_id: local_id(20),
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
                    entity_id: local_id(1),
                },
                FieldDefinition {
                    name: "email".to_string(),
                    field_type: ident_type("string"),
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(2),
                },
            ],
            config: json!({}),
            entity_id: local_id(10),
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
                    entity_id: local_id(3),
                },
            ],
            config: json!({}),
            entity_id: local_id(20),
        },
    );

    let mut curr_aliases = HashMap::new();
    curr_aliases.insert(
        "Email".to_string(),
        TypeAliasDefinition {
            name: "Email".to_string(),
            alias_type: ident_type("string"),
            config: json!({}),
            entity_id: local_id(30),
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

#[test]
fn test_compute_deltas_model_with_inherited_fields_from_skipped_parent() {
    // BUG TEST: When a model extends another model with skip: true,
    // the inherited fields should still be included in the ModelAdded delta.
    //
    // Scenario:
    //   PublicUser { id: string, name?: string } with @sql { skip: true }
    //   User extends PublicUser { email: string }
    //
    // When computing deltas for first migration:
    // - PublicUser should NOT generate a ModelAdded (it has skip: true)
    // - User SHOULD generate a ModelAdded with ALL fields: id, name, email
    //
    // Note: This test verifies the schema structure. The actual field flattening
    // happens in build_cdm_schema_for_plugin, which is tested separately.

    let previous = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };

    // Create a schema where User has inherited fields from PublicUser
    // (simulating what build_cdm_schema_for_plugin should produce)
    let mut curr_models = HashMap::new();

    // PublicUser with skip: true - this model won't generate a table
    curr_models.insert(
        "PublicUser".to_string(),
        ModelDefinition {
            name: "PublicUser".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "id".to_string(),
                    field_type: ident_type("string"),
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(1),
                },
                FieldDefinition {
                    name: "name".to_string(),
                    field_type: ident_type("string"),
                    optional: true,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(2),
                },
            ],
            config: json!({ "skip": true }), // SQL plugin will skip this model
            entity_id: local_id(10),
        },
    );

    // User extends PublicUser - should have all 3 fields (flattened)
    curr_models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec!["PublicUser".to_string()],
            // Fields should be flattened: inherited (id, name) + own (email)
            fields: vec![
                FieldDefinition {
                    name: "id".to_string(),
                    field_type: ident_type("string"),
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(1), // Same as parent's field
                },
                FieldDefinition {
                    name: "name".to_string(),
                    field_type: ident_type("string"),
                    optional: true,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(2), // Same as parent's field
                },
                FieldDefinition {
                    name: "email".to_string(),
                    field_type: ident_type("string"),
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(20), // User's own field
                },
            ],
            config: json!({}),
            entity_id: local_id(20),
        },
    );

    let current = Schema {
        models: curr_models,
        type_aliases: HashMap::new(),
    };

    let deltas = compute_deltas(&previous, &current).unwrap();

    // Find the User model delta
    let user_delta = deltas.iter().find(|d| {
        matches!(d, Delta::ModelAdded { name, .. } if name == "User")
    });

    assert!(
        user_delta.is_some(),
        "Expected ModelAdded delta for User model"
    );

    if let Some(Delta::ModelAdded { name, after }) = user_delta {
        assert_eq!(name, "User");

        // User should have 3 fields: id, name (inherited), email (own)
        assert_eq!(
            after.fields.len(), 3,
            "User model should have 3 fields (2 inherited + 1 own), got: {:?}",
            after.fields.iter().map(|f| &f.name).collect::<Vec<_>>()
        );

        // Verify field names
        let field_names: Vec<_> = after.fields.iter().map(|f| f.name.as_str()).collect();
        assert!(
            field_names.contains(&"id"),
            "User should have inherited 'id' field"
        );
        assert!(
            field_names.contains(&"name"),
            "User should have inherited 'name' field"
        );
        assert!(
            field_names.contains(&"email"),
            "User should have own 'email' field"
        );

        // Verify inherited field optionality is preserved
        let name_field = after.fields.iter().find(|f| f.name == "name").unwrap();
        assert!(
            name_field.optional,
            "Inherited 'name' field should remain optional"
        );
    }
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
            entity_id: local_id(100),
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
            entity_id: local_id(2),
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
            entity_id: local_id(100),
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
            entity_id: local_id(100),
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
                    entity_id: local_id(1),
                },
                FieldDefinition {
                    name: "email".to_string(),
                    field_type: ident_type("string"),
                    optional: false,
                    default: None,
                    config: json!({
                        "sql": { "type": "VARCHAR(320)" }
                    }),
                    entity_id: local_id(2),
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

// ============================================================================
// Context isolation tests
// ============================================================================

#[test]
fn test_context_isolation_different_contexts_have_separate_schema_files() {
    // BUG TEST: Different CDM contexts (e.g., base.cdm vs client.cdm) should have
    // separate migration history. When we save a schema for "base" context, it should
    // NOT be loaded when we later migrate "client" context.
    //
    // The current bug is that all contexts share a single "previous_schema.json" file,
    // which causes incorrect delta computation when switching between contexts.

    use tempfile::TempDir;

    // Create a temporary directory to simulate the .cdm directory
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let cdm_dir = temp_dir.path();

    // Create a schema for the "base" context
    let mut base_models = HashMap::new();
    base_models.insert(
        "BaseUser".to_string(),
        ModelDefinition {
            name: "BaseUser".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "id".to_string(),
                    field_type: ident_type("number"),
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(1),
                },
                FieldDefinition {
                    name: "password_hash".to_string(),
                    field_type: ident_type("string"),
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(2),
                },
            ],
            config: json!({}),
            entity_id: local_id(100),
        },
    );

    let base_schema = Schema {
        models: base_models,
        type_aliases: HashMap::new(),
    };

    // Save schema for "base" context
    save_current_schema(&base_schema, cdm_dir, "base").expect("Failed to save base schema");

    // Now try to load schema for "client" context - should be None since we never saved it
    let client_schema = load_previous_schema(cdm_dir, "client").expect("Failed to load client schema");

    assert!(
        client_schema.is_none(),
        "Loading 'client' context should return None when only 'base' context was saved. \
         This fails if contexts are not properly isolated."
    );

    // Verify that loading "base" context still works
    let base_loaded = load_previous_schema(cdm_dir, "base").expect("Failed to load base schema");
    assert!(
        base_loaded.is_some(),
        "Loading 'base' context should return the saved schema"
    );

    // Verify the loaded base schema matches what we saved
    let base_loaded = base_loaded.unwrap();
    assert!(
        base_loaded.models.contains_key("BaseUser"),
        "Loaded base schema should contain BaseUser model"
    );
}

#[test]
fn test_context_isolation_saves_to_context_specific_files() {
    // Test that saving schemas for different contexts creates separate files
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let cdm_dir = temp_dir.path();

    // Create and save schema for "base" context
    let base_schema = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };
    save_current_schema(&base_schema, cdm_dir, "base").expect("Failed to save base schema");

    // Create and save schema for "client" context
    let mut client_models = HashMap::new();
    client_models.insert(
        "ClientUser".to_string(),
        ModelDefinition {
            name: "ClientUser".to_string(),
            parents: vec![],
            fields: vec![],
            config: json!({}),
            entity_id: local_id(200),
        },
    );
    let client_schema = Schema {
        models: client_models,
        type_aliases: HashMap::new(),
    };
    save_current_schema(&client_schema, cdm_dir, "client").expect("Failed to save client schema");

    // Verify both files exist separately
    let base_file = cdm_dir.join("previous_schema_base.json");
    let client_file = cdm_dir.join("previous_schema_client.json");

    assert!(
        base_file.exists(),
        "Expected context-specific file 'previous_schema_base.json' to exist"
    );
    assert!(
        client_file.exists(),
        "Expected context-specific file 'previous_schema_client.json' to exist"
    );

    // Verify loading each context returns the correct schema
    let base_loaded = load_previous_schema(cdm_dir, "base")
        .expect("Failed to load base")
        .expect("Base schema should exist");
    let client_loaded = load_previous_schema(cdm_dir, "client")
        .expect("Failed to load client")
        .expect("Client schema should exist");

    assert!(
        base_loaded.models.is_empty(),
        "Base schema should have no models"
    );
    assert!(
        client_loaded.models.contains_key("ClientUser"),
        "Client schema should have ClientUser model"
    );
}

#[test]
fn test_context_isolation_prevents_cross_context_delta_pollution() {
    // BUG TEST: This test demonstrates the real-world impact of the context isolation bug.
    //
    // Scenario:
    // 1. base.cdm defines BaseModel with fields: id, secret_field
    // 2. client.cdm extends base.cdm but removes secret_field (not exposed to clients)
    // 3. First, we migrate base.cdm - saves schema with secret_field
    // 4. Then, we migrate client.cdm - should compute deltas from empty (first migration for client)
    //
    // BUG: Without context isolation, client.cdm will load base.cdm's schema,
    // and incorrectly compute a FieldRemoved delta for secret_field.

    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let cdm_dir = temp_dir.path();

    // Step 1: Schema for base.cdm (has secret_field)
    let mut base_models = HashMap::new();
    base_models.insert(
        "BaseModel".to_string(),
        ModelDefinition {
            name: "BaseModel".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "id".to_string(),
                    field_type: ident_type("number"),
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(1),
                },
                FieldDefinition {
                    name: "secret_field".to_string(),
                    field_type: ident_type("string"),
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(2),
                },
            ],
            config: json!({}),
            entity_id: local_id(100),
        },
    );
    let base_schema = Schema {
        models: base_models,
        type_aliases: HashMap::new(),
    };

    // Step 2: Save base.cdm schema (simulating running `cdm migrate base.cdm`)
    save_current_schema(&base_schema, cdm_dir, "base").expect("Failed to save base schema");

    // Step 3: Schema for client.cdm (does NOT have secret_field)
    let mut client_models = HashMap::new();
    client_models.insert(
        "BaseModel".to_string(),
        ModelDefinition {
            name: "BaseModel".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "id".to_string(),
                    field_type: ident_type("number"),
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(1),
                },
                // secret_field is intentionally NOT here - it's not exposed in client context
            ],
            config: json!({}),
            entity_id: local_id(100),
        },
    );
    let client_schema = Schema {
        models: client_models,
        type_aliases: HashMap::new(),
    };

    // Step 4: Load previous schema for client.cdm context
    let client_previous = load_previous_schema(cdm_dir, "client")
        .expect("Failed to load client previous schema");

    // This is the first migration for client.cdm, so there should be NO previous schema
    assert!(
        client_previous.is_none(),
        "First migration for 'client' context should have no previous schema. \
         If this fails, it means the context isolation is broken and 'client' is \
         incorrectly loading 'base' context's schema."
    );

    // Step 5: Compute deltas for client.cdm's first migration
    let empty_schema = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };
    let previous = client_previous.as_ref().unwrap_or(&empty_schema);
    let deltas = compute_deltas(previous, &client_schema).expect("Failed to compute deltas");

    // For first migration, we should see ModelAdded (not FieldRemoved!)
    let has_model_added = deltas.iter().any(|d| {
        matches!(d, Delta::ModelAdded { name, .. } if name == "BaseModel")
    });
    let has_field_removed = deltas.iter().any(|d| {
        matches!(d, Delta::FieldRemoved { field, .. } if field == "secret_field")
    });

    assert!(
        has_model_added,
        "First migration for client context should have ModelAdded delta for BaseModel"
    );
    assert!(
        !has_field_removed,
        "First migration for client context should NOT have FieldRemoved delta for secret_field. \
         This indicates context isolation is broken - client is loading base's schema."
    );
}

// ============================================================================
// Migration file overwrite prevention tests
// ============================================================================

#[test]
fn test_write_migration_files_prevents_overwrite() {
    // Test that write_migration_files returns an error when files already exist
    use tempfile::TempDir;
    use cdm_plugin_interface::OutputFile;

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let base_dir = temp_dir.path();

    // Create an existing migration file
    let existing_file = base_dir.join("001_migration.up.postgres.sql");
    std::fs::write(&existing_file, "-- existing content").expect("Failed to create existing file");

    // Try to write a migration with the same name
    let files = vec![
        OutputFile {
            path: "001_migration.up.postgres.sql".to_string(),
            content: "-- new content".to_string(),
        },
        OutputFile {
            path: "001_migration.down.postgres.sql".to_string(),
            content: "-- new down content".to_string(),
        },
    ];

    let result = write_migration_files(&files, base_dir);

    assert!(
        result.is_err(),
        "write_migration_files should return error when files already exist"
    );

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("already exist"),
        "Error message should mention files already exist: {}",
        error_msg
    );
    assert!(
        error_msg.contains("001_migration.up.postgres.sql"),
        "Error message should mention the conflicting file: {}",
        error_msg
    );

    // Verify the existing file was not overwritten
    let existing_content = std::fs::read_to_string(&existing_file).expect("Failed to read file");
    assert_eq!(
        existing_content, "-- existing content",
        "Existing file should not be modified when overwrite is prevented"
    );
}

#[test]
fn test_write_migration_files_succeeds_when_no_conflicts() {
    // Test that write_migration_files succeeds when files don't exist
    use tempfile::TempDir;
    use cdm_plugin_interface::OutputFile;

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let base_dir = temp_dir.path();

    let files = vec![
        OutputFile {
            path: "002_add_users.up.postgres.sql".to_string(),
            content: "-- new content".to_string(),
        },
        OutputFile {
            path: "002_add_users.down.postgres.sql".to_string(),
            content: "-- new down content".to_string(),
        },
    ];

    let result = write_migration_files(&files, base_dir);

    assert!(
        result.is_ok(),
        "write_migration_files should succeed when no conflicts: {:?}",
        result.err()
    );

    // Verify files were created
    let up_file = base_dir.join("002_add_users.up.postgres.sql");
    let down_file = base_dir.join("002_add_users.down.postgres.sql");

    assert!(up_file.exists(), "Up migration file should be created");
    assert!(down_file.exists(), "Down migration file should be created");

    let up_content = std::fs::read_to_string(&up_file).expect("Failed to read up file");
    assert_eq!(up_content, "-- new content");
}

#[test]
fn test_write_migration_files_lists_all_conflicting_files() {
    // Test that when multiple files conflict, all are listed in the error
    use tempfile::TempDir;
    use cdm_plugin_interface::OutputFile;

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let base_dir = temp_dir.path();

    // Create both existing migration files
    let up_file = base_dir.join("001_migration.up.postgres.sql");
    let down_file = base_dir.join("001_migration.down.postgres.sql");
    std::fs::write(&up_file, "-- existing up").expect("Failed to create up file");
    std::fs::write(&down_file, "-- existing down").expect("Failed to create down file");

    let files = vec![
        OutputFile {
            path: "001_migration.up.postgres.sql".to_string(),
            content: "-- new up".to_string(),
        },
        OutputFile {
            path: "001_migration.down.postgres.sql".to_string(),
            content: "-- new down".to_string(),
        },
    ];

    let result = write_migration_files(&files, base_dir);

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();

    // Both files should be mentioned in the error
    assert!(
        error_msg.contains("001_migration.up.postgres.sql"),
        "Error should mention up file: {}",
        error_msg
    );
    assert!(
        error_msg.contains("001_migration.down.postgres.sql"),
        "Error should mention down file: {}",
        error_msg
    );
}

// =============================================================================
// MODEL MODIFICATION INTEGRATION TESTS
// =============================================================================

#[test]
fn test_model_modification_preserves_inherited_fields_in_schema() {
    // INTEGRATION TEST: This test verifies the full pipeline from file loading
    // through schema building correctly preserves inherited fields when a model
    // from an ancestor is modified (e.g., adding @sql { skip: true }).
    //
    // Scenario:
    //   base.cdm:
    //     PublicUser { id, name?, avatar_url? }
    //     User extends PublicUser { email, created_at }
    //
    //   child.cdm:
    //     extends "./base.cdm"
    //     @sql { skip: true }
    //     PublicUser { }  // Modify to skip table, but keep fields
    //
    // Expected:
    //   - PublicUser should have skip: true AND all 3 fields from base.cdm
    //   - User should have all 5 fields (3 inherited + 2 own)

    use std::path::PathBuf;
    use crate::file_resolver::FileResolver;
    use crate::validate::validate_tree;
    use crate::build_cdm_schema_for_plugin;

    let fixtures_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_fixtures")
        .join("model_modification")
        .join("child.cdm");

    // Load and validate the child file (which extends base.cdm)
    let tree = FileResolver::load(&fixtures_path)
        .expect("Failed to load child.cdm");

    let validation_result = validate_tree(tree)
        .expect("Validation failed");

    assert!(
        !validation_result.has_errors(),
        "Validation should succeed, got errors: {:?}",
        validation_result.diagnostics
    );

    // Build schema for plugin (this is what gets passed to SQL plugin for migrations)
    let ancestor_paths: Vec<PathBuf> = vec![
        fixtures_path.parent().unwrap().join("base.cdm")
    ];

    let schema = build_cdm_schema_for_plugin(&validation_result, &ancestor_paths, "sql")
        .expect("Failed to build schema for plugin");

    // Verify PublicUser model
    let public_user = schema.models.get("PublicUser")
        .expect("PublicUser should exist in schema");

    // Should have skip: true from child.cdm
    assert_eq!(
        public_user.config.get("skip"),
        Some(&serde_json::json!(true)),
        "PublicUser should have skip: true from child.cdm modification"
    );

    // Should have all 3 fields from base.cdm (this is the bug fix!)
    let public_user_field_names: Vec<&str> = public_user.fields.iter()
        .map(|f| f.name.as_str())
        .collect();

    assert!(
        public_user_field_names.contains(&"id"),
        "PublicUser should have 'id' field from base.cdm. Got: {:?}",
        public_user_field_names
    );
    assert!(
        public_user_field_names.contains(&"name"),
        "PublicUser should have 'name' field from base.cdm. Got: {:?}",
        public_user_field_names
    );
    assert!(
        public_user_field_names.contains(&"avatar_url"),
        "PublicUser should have 'avatar_url' field from base.cdm. Got: {:?}",
        public_user_field_names
    );

    assert_eq!(
        public_user.fields.len(),
        3,
        "PublicUser should have exactly 3 fields. Got: {:?}",
        public_user_field_names
    );

    // Verify User model has all inherited fields flattened
    let user = schema.models.get("User")
        .expect("User should exist in schema");

    let user_field_names: Vec<&str> = user.fields.iter()
        .map(|f| f.name.as_str())
        .collect();

    // User should have inherited fields (id, name, avatar_url) + own fields (email, created_at)
    assert!(
        user_field_names.contains(&"id"),
        "User should have inherited 'id' field. Got: {:?}",
        user_field_names
    );
    assert!(
        user_field_names.contains(&"name"),
        "User should have inherited 'name' field. Got: {:?}",
        user_field_names
    );
    assert!(
        user_field_names.contains(&"avatar_url"),
        "User should have inherited 'avatar_url' field. Got: {:?}",
        user_field_names
    );
    assert!(
        user_field_names.contains(&"email"),
        "User should have own 'email' field. Got: {:?}",
        user_field_names
    );
    assert!(
        user_field_names.contains(&"created_at"),
        "User should have own 'created_at' field. Got: {:?}",
        user_field_names
    );

    assert_eq!(
        user.fields.len(),
        5,
        "User should have exactly 5 fields (3 inherited + 2 own). Got: {:?}",
        user_field_names
    );
}
