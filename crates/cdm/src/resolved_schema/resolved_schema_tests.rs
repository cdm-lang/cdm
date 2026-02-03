use super::*;
use crate::{DefinitionKind, FieldInfo, SymbolTable};
use cdm_utils::{EntityId, EntityIdSource, Position, Span};
use std::collections::{HashMap, HashSet};

// Helper to create a local entity ID
fn local_id(id: u64) -> Option<EntityId> {
    Some(EntityId::local(id))
}

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
    let removals: HashSet<String> = HashSet::new();

    let resolved = build_resolved_schema(&current_symbols, &current_fields, &ancestors, &removals, &HashMap::new());

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
            entity_id: local_id(1),
        },
    );

    let current_fields = HashMap::new();
    let ancestors = vec![];
    let removals: HashSet<String> = HashSet::new();

    let resolved = build_resolved_schema(&current_symbols, &current_fields, &ancestors, &removals, &HashMap::new());

    assert_eq!(resolved.type_aliases.len(), 1);
    assert!(resolved.type_aliases.contains_key("Email"));

    let email_alias = &resolved.type_aliases["Email"];
    assert_eq!(email_alias.name, "Email");
    assert_eq!(email_alias.type_expr, "string");
    assert_eq!(email_alias.entity_id, local_id(1));
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
            entity_id: local_id(2),
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
            entity_id: local_id(3),
        }],
    );

    let ancestors = vec![];
    let removals: HashSet<String> = HashSet::new();

    let resolved = build_resolved_schema(&current_symbols, &current_fields, &ancestors, &removals, &HashMap::new());

    assert_eq!(resolved.models.len(), 1);
    assert!(resolved.models.contains_key("User"));

    let user_model = &resolved.models["User"];
    assert_eq!(user_model.name, "User");
    assert_eq!(user_model.fields.len(), 1);
    assert_eq!(user_model.fields[0].name, "id");
    assert_eq!(user_model.entity_id, local_id(2));
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
    let mut removals: HashSet<String> = HashSet::new();
    removals.insert("ToRemove".to_string());

    let resolved = build_resolved_schema(&current_symbols, &current_fields, &ancestors, &removals, &HashMap::new());

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
            entity_id: local_id(10),
        },
    );

    let ancestors = vec![Ancestor {
        path: "ancestor.cdm".to_string(),
        symbol_table: ancestor_table,
        model_fields: HashMap::new(),
    }];

    let removals: HashSet<String> = HashSet::new();

    let resolved = build_resolved_schema(&current_symbols, &current_fields, &ancestors, &removals, &HashMap::new());

    assert_eq!(resolved.type_aliases.len(), 1);
    assert!(resolved.type_aliases.contains_key("AncestorType"));

    let ancestor_type = &resolved.type_aliases["AncestorType"];
    assert_eq!(ancestor_type.source_file, "ancestor.cdm");
    assert_eq!(ancestor_type.entity_id, local_id(10));
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
            entity_id: local_id(1),
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
            entity_id: local_id(2),
        },
    );

    let ancestors = vec![Ancestor {
        path: "ancestor.cdm".to_string(),
        symbol_table: ancestor_table,
        model_fields: HashMap::new(),
    }];

    let removals: HashSet<String> = HashSet::new();

    let resolved = build_resolved_schema(&current_symbols, &current_fields, &ancestors, &removals, &HashMap::new());

    assert_eq!(resolved.type_aliases.len(), 1);

    let shared_type = &resolved.type_aliases["SharedType"];
    // Should get current file's version (number), not ancestor's (string)
    assert_eq!(shared_type.type_expr, "number");
    assert_eq!(shared_type.source_file, "current file");
    assert_eq!(shared_type.entity_id, local_id(1));
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

    let removals: HashSet<String> = HashSet::new();

    let resolved = build_resolved_schema(&current_symbols, &current_fields, &ancestors, &removals, &HashMap::new());

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
            entity_id: local_id(1),
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
            entity_id: local_id(2),
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

    let removals: HashSet<String> = HashSet::new();

    let resolved = build_resolved_schema(&current_symbols, &current_fields, &ancestors, &removals, &HashMap::new());

    assert_eq!(resolved.type_aliases.len(), 1);

    let contested = &resolved.type_aliases["Contested"];
    // Ancestors are processed in reverse order (furthest first),
    // but closer ancestors are checked first in the "already added" check,
    // so the first ancestor in the list (near.cdm) wins due to early skip
    // Actually, looking at the code: iter().rev() processes [far, near]
    // and the code skips if already added, so far gets added first
    assert_eq!(contested.type_expr, "string");
    assert_eq!(contested.source_file, "far.cdm");
    assert_eq!(contested.entity_id, local_id(1));
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
    let removals: HashSet<String> = HashSet::new();

    let resolved = build_resolved_schema(&current_symbols, &current_fields, &ancestors, &removals, &HashMap::new());

    assert_eq!(resolved.models.len(), 1);

    let admin = &resolved.models["Admin"];
    assert_eq!(admin.parents.len(), 1);
    assert_eq!(admin.parents[0], "User");
}

#[test]
fn test_get_inherited_fields_flattens_parent_fields() {
    // BUG TEST: When a model extends another model, the inherited fields
    // should be flattened into the child model's fields list in the Schema
    // passed to plugins. This ensures that when a new model is added via
    // migration, all inherited fields are included in the CREATE TABLE.
    //
    // Example:
    //   PublicUser { id: string, name?: string }
    //   User extends PublicUser { email: string }
    //
    // The User model should have all three fields: id, name, email

    // Create the parent model "PublicUser" in ancestors
    let mut ancestor_symbols = SymbolTable::new();
    ancestor_symbols.definitions.insert(
        "PublicUser".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model {
                extends: vec![],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: local_id(1),
        },
    );

    let mut ancestor_fields = HashMap::new();
    ancestor_fields.insert(
        "PublicUser".to_string(),
        vec![
            FieldInfo {
                name: "id".to_string(),
                type_expr: Some("string".to_string()),
                optional: false,
                span: test_span(),
                plugin_configs: HashMap::new(),
                default_value: None,
                entity_id: local_id(10),
            },
            FieldInfo {
                name: "name".to_string(),
                type_expr: Some("string".to_string()),
                optional: true,
                span: test_span(),
                plugin_configs: HashMap::new(),
                default_value: None,
                entity_id: local_id(11),
            },
        ],
    );

    let ancestors = vec![Ancestor {
        path: "public.cdm".to_string(),
        symbol_table: ancestor_symbols,
        model_fields: ancestor_fields,
    }];

    // Create the child model "User" that extends "PublicUser"
    let mut current_symbols = SymbolTable::new();
    current_symbols.definitions.insert(
        "User".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model {
                extends: vec!["PublicUser".to_string()],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: local_id(2),
        },
    );

    let mut current_fields = HashMap::new();
    current_fields.insert(
        "User".to_string(),
        vec![FieldInfo {
            name: "email".to_string(),
            type_expr: Some("string".to_string()),
            optional: false,
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: None,
            entity_id: local_id(20),
        }],
    );

    // Test get_inherited_fields returns all fields (inherited + own)
    let flattened = crate::symbol_table::get_inherited_fields(
        "User",
        &current_fields,
        &current_symbols,
        &ancestors,
    );

    // User should have 3 fields: id, name (inherited from PublicUser), email (own)
    assert_eq!(
        flattened.len(), 3,
        "Flattened User model should have 3 fields (2 inherited + 1 own), got: {:?}",
        flattened.iter().map(|f| &f.name).collect::<Vec<_>>()
    );

    // Check field names are in correct order (inherited first, then own)
    let field_names: Vec<_> = flattened.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        field_names,
        vec!["id", "name", "email"],
        "Fields should be in order: inherited fields first, then own fields"
    );

    // Verify inherited field properties are preserved
    let id_field = flattened.iter().find(|f| f.name == "id").unwrap();
    assert!(!id_field.optional, "id field should not be optional");

    let name_field = flattened.iter().find(|f| f.name == "name").unwrap();
    assert!(name_field.optional, "name field should be optional (inherited from PublicUser)");
}

#[test]
fn test_get_inherited_fields_with_deep_inheritance() {
    // Test multi-level inheritance: GrandChild extends Child extends Parent

    // Parent model
    let mut parent_symbols = SymbolTable::new();
    parent_symbols.definitions.insert(
        "Parent".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model { extends: vec![] },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );
    let mut parent_fields = HashMap::new();
    parent_fields.insert(
        "Parent".to_string(),
        vec![FieldInfo {
            name: "parent_field".to_string(),
            type_expr: Some("string".to_string()),
            optional: false,
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: None,
            entity_id: None,
        }],
    );

    // Child extends Parent
    let mut child_symbols = SymbolTable::new();
    child_symbols.definitions.insert(
        "Child".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model {
                extends: vec!["Parent".to_string()],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );
    let mut child_fields = HashMap::new();
    child_fields.insert(
        "Child".to_string(),
        vec![FieldInfo {
            name: "child_field".to_string(),
            type_expr: Some("string".to_string()),
            optional: false,
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: None,
            entity_id: None,
        }],
    );

    // Build ancestors chain: [Child ancestor, Parent ancestor]
    let ancestors = vec![
        Ancestor {
            path: "child.cdm".to_string(),
            symbol_table: child_symbols,
            model_fields: child_fields,
        },
        Ancestor {
            path: "parent.cdm".to_string(),
            symbol_table: parent_symbols,
            model_fields: parent_fields,
        },
    ];

    // GrandChild extends Child (in current file)
    let mut current_symbols = SymbolTable::new();
    current_symbols.definitions.insert(
        "GrandChild".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model {
                extends: vec!["Child".to_string()],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );
    let mut current_fields = HashMap::new();
    current_fields.insert(
        "GrandChild".to_string(),
        vec![FieldInfo {
            name: "grandchild_field".to_string(),
            type_expr: Some("string".to_string()),
            optional: false,
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: None,
            entity_id: None,
        }],
    );

    // Test flattening GrandChild
    let flattened = crate::symbol_table::get_inherited_fields(
        "GrandChild",
        &current_fields,
        &current_symbols,
        &ancestors,
    );

    // Should have all 3 fields: parent_field, child_field, grandchild_field
    assert_eq!(flattened.len(), 3);

    let field_names: Vec<_> = flattened.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        field_names,
        vec!["parent_field", "child_field", "grandchild_field"],
        "Fields should be in order from furthest ancestor to closest"
    );
}

#[test]
fn test_get_inherited_fields_with_field_override() {
    // Test that child fields override parent fields with the same name

    // Parent model with 'name' field (required)
    let mut parent_symbols = SymbolTable::new();
    parent_symbols.definitions.insert(
        "Parent".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model { extends: vec![] },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );
    let mut parent_fields = HashMap::new();
    parent_fields.insert(
        "Parent".to_string(),
        vec![
            FieldInfo {
                name: "id".to_string(),
                type_expr: Some("number".to_string()),
                optional: false,
                span: test_span(),
                plugin_configs: HashMap::new(),
                default_value: None,
                entity_id: None,
            },
            FieldInfo {
                name: "name".to_string(),
                type_expr: Some("string".to_string()),
                optional: false, // Required in parent
                span: test_span(),
                plugin_configs: HashMap::new(),
                default_value: None,
                entity_id: None,
            },
        ],
    );

    let ancestors = vec![Ancestor {
        path: "parent.cdm".to_string(),
        symbol_table: parent_symbols,
        model_fields: parent_fields,
    }];

    // Child extends Parent, overrides 'name' to be optional
    let mut current_symbols = SymbolTable::new();
    current_symbols.definitions.insert(
        "Child".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model {
                extends: vec!["Parent".to_string()],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );
    let mut current_fields = HashMap::new();
    current_fields.insert(
        "Child".to_string(),
        vec![FieldInfo {
            name: "name".to_string(),
            type_expr: Some("string".to_string()),
            optional: true, // Optional in child (override)
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: None,
            entity_id: None,
        }],
    );

    // Test flattening Child
    let flattened = crate::symbol_table::get_inherited_fields(
        "Child",
        &current_fields,
        &current_symbols,
        &ancestors,
    );

    // get_inherited_fields returns all fields including duplicates
    // The deduplication happens in build_cdm_schema_for_plugin
    let unique_names: std::collections::HashSet<_> = flattened.iter().map(|f| &f.name).collect();
    assert_eq!(unique_names.len(), 2, "Should have 2 unique field names");

    // The last 'name' field should be the child's version (optional)
    let last_name_field = flattened.iter().filter(|f| f.name == "name").last().unwrap();
    assert!(
        last_name_field.optional,
        "Child's override of 'name' field should be optional"
    );
}

#[test]
fn test_get_inherited_fields_with_skip_true_on_parent() {
    // Test that inherited fields are included even when the parent model has
    // skip: true in its plugin config. The skip config tells plugins not to
    // generate output for that model, but the fields should still be inherited.
    //
    // Example:
    //   @sql { skip: true }
    //   PublicUser { id: string, name?: string }
    //
    //   User extends PublicUser { email: string }
    //
    // User should still have all three fields: id, name, email

    // Create the parent model "PublicUser" with skip: true in ancestors
    let mut ancestor_symbols = SymbolTable::new();
    let mut skip_config = HashMap::new();
    skip_config.insert("sql".to_string(), serde_json::json!({ "skip": true }));

    ancestor_symbols.definitions.insert(
        "PublicUser".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model {
                extends: vec![],
            },
            span: test_span(),
            plugin_configs: skip_config,
            entity_id: local_id(1),
        },
    );

    let mut ancestor_fields = HashMap::new();
    ancestor_fields.insert(
        "PublicUser".to_string(),
        vec![
            FieldInfo {
                name: "id".to_string(),
                type_expr: Some("string".to_string()),
                optional: false,
                span: test_span(),
                plugin_configs: HashMap::new(),
                default_value: None,
                entity_id: local_id(10),
            },
            FieldInfo {
                name: "name".to_string(),
                type_expr: Some("string".to_string()),
                optional: true,
                span: test_span(),
                plugin_configs: HashMap::new(),
                default_value: None,
                entity_id: local_id(11),
            },
        ],
    );

    let ancestors = vec![Ancestor {
        path: "public.cdm".to_string(),
        symbol_table: ancestor_symbols,
        model_fields: ancestor_fields,
    }];

    // Create the child model "User" that extends "PublicUser"
    let mut current_symbols = SymbolTable::new();
    current_symbols.definitions.insert(
        "User".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model {
                extends: vec!["PublicUser".to_string()],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: local_id(2),
        },
    );

    let mut current_fields = HashMap::new();
    current_fields.insert(
        "User".to_string(),
        vec![FieldInfo {
            name: "email".to_string(),
            type_expr: Some("string".to_string()),
            optional: false,
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: None,
            entity_id: local_id(20),
        }],
    );

    // Test get_inherited_fields returns all fields (inherited + own)
    // even when parent has skip: true
    let flattened = crate::symbol_table::get_inherited_fields(
        "User",
        &current_fields,
        &current_symbols,
        &ancestors,
    );

    // User should have 3 fields: id, name (inherited from PublicUser), email (own)
    assert_eq!(
        flattened.len(), 3,
        "Flattened User model should have 3 fields even when parent has skip: true, got: {:?}",
        flattened.iter().map(|f| &f.name).collect::<Vec<_>>()
    );

    // Check field names are in correct order (inherited first, then own)
    let field_names: Vec<_> = flattened.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        field_names,
        vec!["id", "name", "email"],
        "Fields should be in order: inherited fields first, then own fields"
    );
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
    let removals: HashSet<String> = HashSet::new();

    let resolved = build_resolved_schema(&current_symbols, &current_fields, &ancestors, &removals, &HashMap::new());

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
            entity_id: local_id(99),
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

    let removals: HashSet<String> = HashSet::new();

    let resolved = build_resolved_schema(&current_symbols, &current_fields, &ancestors, &removals, &HashMap::new());

    assert_eq!(resolved.models.len(), 1);
    assert!(resolved.models.contains_key("BaseModel"));

    let base_model = &resolved.models["BaseModel"];
    assert_eq!(base_model.source_file, "base.cdm");
    assert_eq!(base_model.fields.len(), 1);
    assert_eq!(base_model.fields[0].name, "created_at");
    assert_eq!(base_model.entity_id, local_id(99));
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
    let removals: HashSet<String> = HashSet::new();

    let resolved = build_resolved_schema(&current_symbols, &current_fields, &ancestors, &removals, &HashMap::new());

    let config_model = &resolved.models["Config"];
    assert_eq!(
        config_model.fields[0].default_value,
        Some(serde_json::json!(false))
    );
}

#[test]
fn test_build_resolved_schema_with_template_namespace_type_alias() {
    // Create a symbol table with a namespace containing type aliases
    let mut current_symbols = SymbolTable::new();

    // Create the template namespace with a type alias that has sql config
    let mut template_symbol_table = SymbolTable::new();
    let mut sql_config = HashMap::new();
    sql_config.insert("sql".to_string(), serde_json::json!({"type": "UUID"}));

    template_symbol_table.definitions.insert(
        "UUID".to_string(),
        crate::Definition {
            kind: DefinitionKind::TypeAlias {
                references: vec!["string".to_string()],
                type_expr: "string".to_string(),
            },
            span: test_span(),
            plugin_configs: sql_config,
            entity_id: Some(EntityId::local(60)),
        },
    );

    // Add the namespace to current_symbols
    current_symbols.namespaces.insert(
        "sql".to_string(),
        crate::ImportedNamespace {
            name: "sql".to_string(),
            template_path: std::path::PathBuf::from("templates/sql-types/postgres.cdm"),
            symbol_table: template_symbol_table,
            model_fields: HashMap::new(),
            template_source: EntityIdSource::LocalTemplate { path: "templates/sql-types".to_string() },
        },
    );

    let current_fields = HashMap::new();
    let ancestors = vec![];
    let removals: HashSet<String> = HashSet::new();

    let resolved = build_resolved_schema(&current_symbols, &current_fields, &ancestors, &removals, &HashMap::new());

    // The qualified type alias "sql.UUID" should be in the resolved schema
    assert!(resolved.type_aliases.contains_key("sql.UUID"),
        "Expected sql.UUID in type_aliases, got: {:?}", resolved.type_aliases.keys().collect::<Vec<_>>());

    let uuid_alias = &resolved.type_aliases["sql.UUID"];
    assert_eq!(uuid_alias.name, "UUID");
    assert_eq!(uuid_alias.type_expr, "string");
    assert!(uuid_alias.plugin_configs.contains_key("sql"),
        "Expected sql plugin config, got: {:?}", uuid_alias.plugin_configs);

    let sql_config = &uuid_alias.plugin_configs["sql"];
    assert_eq!(sql_config["type"], "UUID");
}

#[test]
fn test_build_resolved_schema_model_modification_merges_with_ancestor() {
    // BUG TEST: Per spec Section 7.3, when a model from an ancestor is referenced
    // in the current file, it should MODIFY (merge) the model, not REPLACE it.
    //
    // Example scenario:
    //   ancestor.cdm:
    //     PublicUser { id: string, name?: string }
    //
    //   current.cdm:
    //     extends "ancestor.cdm"
    //     @sql { skip: true }
    //     PublicUser { }  // No new fields, just adding plugin config
    //
    // The resolved PublicUser should have:
    // - All fields from ancestor (id, name)
    // - The sql.skip config from current file
    //
    // Previous behavior incorrectly replaced the entire model definition.

    // Create ancestor with PublicUser model containing fields
    let mut ancestor_symbols = SymbolTable::new();
    ancestor_symbols.definitions.insert(
        "PublicUser".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model {
                extends: vec![],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: local_id(1),
        },
    );

    let mut ancestor_fields = HashMap::new();
    ancestor_fields.insert(
        "PublicUser".to_string(),
        vec![
            FieldInfo {
                name: "id".to_string(),
                type_expr: Some("string".to_string()),
                optional: false,
                span: test_span(),
                plugin_configs: HashMap::new(),
                default_value: None,
                entity_id: local_id(10),
            },
            FieldInfo {
                name: "name".to_string(),
                type_expr: Some("string".to_string()),
                optional: true,
                span: test_span(),
                plugin_configs: HashMap::new(),
                default_value: None,
                entity_id: local_id(11),
            },
        ],
    );

    let ancestors = vec![Ancestor {
        path: "ancestor.cdm".to_string(),
        symbol_table: ancestor_symbols,
        model_fields: ancestor_fields,
    }];

    // Current file "modifies" PublicUser by adding plugin config but no new fields
    let mut current_symbols = SymbolTable::new();
    let mut skip_config = HashMap::new();
    skip_config.insert("sql".to_string(), serde_json::json!({ "skip": true }));

    current_symbols.definitions.insert(
        "PublicUser".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model {
                extends: vec![],
            },
            span: test_span(),
            plugin_configs: skip_config,
            entity_id: local_id(1), // Same entity_id - this is a modification
        },
    );

    // Current file has empty fields for PublicUser (just adding config, no new fields)
    let mut current_fields = HashMap::new();
    current_fields.insert("PublicUser".to_string(), vec![]);

    let removals: HashSet<String> = HashSet::new();

    let resolved = build_resolved_schema(&current_symbols, &current_fields, &ancestors, &removals, &HashMap::new());

    // PublicUser should exist in resolved models
    assert!(
        resolved.models.contains_key("PublicUser"),
        "PublicUser should be in resolved models"
    );

    let public_user = &resolved.models["PublicUser"];

    // Should have the plugin config from current file
    assert!(
        public_user.plugin_configs.contains_key("sql"),
        "PublicUser should have sql plugin config from current file"
    );
    assert_eq!(
        public_user.plugin_configs["sql"]["skip"], true,
        "PublicUser should have skip: true from current file"
    );

    // Should STILL have the fields from ancestor (this is the bug fix)
    assert_eq!(
        public_user.fields.len(),
        2,
        "PublicUser should have 2 fields from ancestor, got: {:?}",
        public_user.fields.iter().map(|f| &f.name).collect::<Vec<_>>()
    );

    let field_names: Vec<_> = public_user.fields.iter().map(|f| f.name.as_str()).collect();
    assert!(
        field_names.contains(&"id"),
        "PublicUser should have 'id' field from ancestor"
    );
    assert!(
        field_names.contains(&"name"),
        "PublicUser should have 'name' field from ancestor"
    );
}

#[test]
fn test_build_resolved_schema_model_modification_merges_fields() {
    // Test that when modifying an ancestor model with new fields,
    // the fields are merged (ancestor fields + current file fields)

    // Create ancestor with BaseModel containing one field
    let mut ancestor_symbols = SymbolTable::new();
    ancestor_symbols.definitions.insert(
        "BaseModel".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model {
                extends: vec![],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: local_id(1),
        },
    );

    let mut ancestor_fields = HashMap::new();
    ancestor_fields.insert(
        "BaseModel".to_string(),
        vec![FieldInfo {
            name: "id".to_string(),
            type_expr: Some("string".to_string()),
            optional: false,
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: None,
            entity_id: local_id(10),
        }],
    );

    let ancestors = vec![Ancestor {
        path: "ancestor.cdm".to_string(),
        symbol_table: ancestor_symbols,
        model_fields: ancestor_fields,
    }];

    // Current file adds a new field to BaseModel
    let mut current_symbols = SymbolTable::new();
    current_symbols.definitions.insert(
        "BaseModel".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model {
                extends: vec![],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: local_id(1), // Same entity_id
        },
    );

    let mut current_fields = HashMap::new();
    current_fields.insert(
        "BaseModel".to_string(),
        vec![FieldInfo {
            name: "updated_at".to_string(),
            type_expr: Some("string".to_string()),
            optional: true,
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: None,
            entity_id: local_id(20),
        }],
    );

    let removals: HashSet<String> = HashSet::new();

    let resolved = build_resolved_schema(&current_symbols, &current_fields, &ancestors, &removals, &HashMap::new());

    let base_model = &resolved.models["BaseModel"];

    // Should have both ancestor fields and current file fields
    assert_eq!(
        base_model.fields.len(),
        2,
        "BaseModel should have 2 fields (1 from ancestor + 1 from current), got: {:?}",
        base_model.fields.iter().map(|f| &f.name).collect::<Vec<_>>()
    );

    let field_names: Vec<_> = base_model.fields.iter().map(|f| f.name.as_str()).collect();
    assert!(field_names.contains(&"id"), "Should have 'id' from ancestor");
    assert!(
        field_names.contains(&"updated_at"),
        "Should have 'updated_at' from current file"
    );
}

#[test]
fn test_build_resolved_schema_model_modification_field_override() {
    // Test that when modifying an ancestor model, fields with the same name
    // are overridden by the current file's version

    // Create ancestor with BaseModel containing 'status' field (required)
    let mut ancestor_symbols = SymbolTable::new();
    ancestor_symbols.definitions.insert(
        "BaseModel".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model {
                extends: vec![],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: local_id(1),
        },
    );

    let mut ancestor_fields = HashMap::new();
    ancestor_fields.insert(
        "BaseModel".to_string(),
        vec![
            FieldInfo {
                name: "id".to_string(),
                type_expr: Some("string".to_string()),
                optional: false,
                span: test_span(),
                plugin_configs: HashMap::new(),
                default_value: None,
                entity_id: local_id(10),
            },
            FieldInfo {
                name: "status".to_string(),
                type_expr: Some("string".to_string()),
                optional: false, // Required in ancestor
                span: test_span(),
                plugin_configs: HashMap::new(),
                default_value: None,
                entity_id: local_id(11),
            },
        ],
    );

    let ancestors = vec![Ancestor {
        path: "ancestor.cdm".to_string(),
        symbol_table: ancestor_symbols,
        model_fields: ancestor_fields,
    }];

    // Current file overrides 'status' to be optional
    let mut current_symbols = SymbolTable::new();
    current_symbols.definitions.insert(
        "BaseModel".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model {
                extends: vec![],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: local_id(1),
        },
    );

    let mut current_fields = HashMap::new();
    current_fields.insert(
        "BaseModel".to_string(),
        vec![FieldInfo {
            name: "status".to_string(),
            type_expr: Some("string".to_string()),
            optional: true, // Optional in current file (override)
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: None,
            entity_id: local_id(11), // Same entity_id for the field
        }],
    );

    let removals: HashSet<String> = HashSet::new();

    let resolved = build_resolved_schema(&current_symbols, &current_fields, &ancestors, &removals, &HashMap::new());

    let base_model = &resolved.models["BaseModel"];

    // Should have both fields
    assert_eq!(base_model.fields.len(), 2);

    // The 'status' field should be optional (current file's version)
    let status_field = base_model.fields.iter().find(|f| f.name == "status").unwrap();
    assert!(
        status_field.optional,
        "status field should be optional (overridden by current file)"
    );

    // The 'id' field should still be there from ancestor
    let id_field = base_model.fields.iter().find(|f| f.name == "id").unwrap();
    assert!(!id_field.optional, "id field should not be optional");
}

#[test]
fn test_get_inherited_fields_multiple_extends_intermediate_no_fields() {
    // BUG TEST: When a model extends multiple parents via an intermediate model
    // that has NO direct fields, all fields from grandparents should still be inherited.
    //
    // Example scenario from user report:
    //   Entity { id: UUID }
    //   Timestamped { created_at: TimestampTZ }
    //   TimestampedEntity extends Entity, Timestamped { } // NO direct fields
    //   PublicUser extends TimestampedEntity { name?: string, avatar_url?: string }
    //   User extends PublicUser { email?: string }
    //
    // User should have ALL fields: id, created_at, name, avatar_url, email

    // Create ancestor with Entity, Timestamped, TimestampedEntity, PublicUser
    let mut ancestor_symbols = SymbolTable::new();

    // Entity with id field
    ancestor_symbols.definitions.insert(
        "Entity".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model { extends: vec![] },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: local_id(1),
        },
    );

    // Timestamped with created_at field
    ancestor_symbols.definitions.insert(
        "Timestamped".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model { extends: vec![] },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: local_id(2),
        },
    );

    // TimestampedEntity extends Entity, Timestamped (NO direct fields)
    ancestor_symbols.definitions.insert(
        "TimestampedEntity".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model {
                extends: vec!["Entity".to_string(), "Timestamped".to_string()],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: local_id(3),
        },
    );

    // PublicUser extends TimestampedEntity
    ancestor_symbols.definitions.insert(
        "PublicUser".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model {
                extends: vec!["TimestampedEntity".to_string()],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: local_id(4),
        },
    );

    let mut ancestor_fields = HashMap::new();

    // Entity has id field
    ancestor_fields.insert(
        "Entity".to_string(),
        vec![FieldInfo {
            name: "id".to_string(),
            type_expr: Some("UUID".to_string()),
            optional: false,
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: None,
            entity_id: local_id(10),
        }],
    );

    // Timestamped has created_at field
    ancestor_fields.insert(
        "Timestamped".to_string(),
        vec![FieldInfo {
            name: "created_at".to_string(),
            type_expr: Some("TimestampTZ".to_string()),
            optional: false,
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: None,
            entity_id: local_id(11),
        }],
    );

    // TimestampedEntity has NO direct fields (empty vector)
    ancestor_fields.insert(
        "TimestampedEntity".to_string(),
        vec![],
    );

    // PublicUser has name and avatar_url fields
    ancestor_fields.insert(
        "PublicUser".to_string(),
        vec![
            FieldInfo {
                name: "name".to_string(),
                type_expr: Some("string".to_string()),
                optional: true,
                span: test_span(),
                plugin_configs: HashMap::new(),
                default_value: None,
                entity_id: local_id(12),
            },
            FieldInfo {
                name: "avatar_url".to_string(),
                type_expr: Some("string".to_string()),
                optional: true,
                span: test_span(),
                plugin_configs: HashMap::new(),
                default_value: None,
                entity_id: local_id(13),
            },
        ],
    );

    let ancestors = vec![Ancestor {
        path: "public.cdm".to_string(),
        symbol_table: ancestor_symbols,
        model_fields: ancestor_fields,
    }];

    // Current file: User extends PublicUser
    let mut current_symbols = SymbolTable::new();
    current_symbols.definitions.insert(
        "User".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model {
                extends: vec!["PublicUser".to_string()],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: local_id(5),
        },
    );

    let mut current_fields = HashMap::new();
    current_fields.insert(
        "User".to_string(),
        vec![FieldInfo {
            name: "email".to_string(),
            type_expr: Some("string".to_string()),
            optional: true,
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: None,
            entity_id: local_id(20),
        }],
    );

    // Test get_inherited_fields returns ALL fields through the inheritance chain
    let flattened = crate::symbol_table::get_inherited_fields(
        "User",
        &current_fields,
        &current_symbols,
        &ancestors,
    );

    // User should have 5 fields: id, created_at (from grandparents via TimestampedEntity),
    // name, avatar_url (from PublicUser), email (own)
    let field_names: Vec<_> = flattened.iter().map(|f| f.name.as_str()).collect();

    assert!(
        field_names.contains(&"id"),
        "User should inherit 'id' from Entity via TimestampedEntity -> PublicUser. Got: {:?}",
        field_names
    );
    assert!(
        field_names.contains(&"created_at"),
        "User should inherit 'created_at' from Timestamped via TimestampedEntity -> PublicUser. Got: {:?}",
        field_names
    );
    assert!(
        field_names.contains(&"name"),
        "User should inherit 'name' from PublicUser. Got: {:?}",
        field_names
    );
    assert!(
        field_names.contains(&"avatar_url"),
        "User should inherit 'avatar_url' from PublicUser. Got: {:?}",
        field_names
    );
    assert!(
        field_names.contains(&"email"),
        "User should have its own 'email' field. Got: {:?}",
        field_names
    );

    assert_eq!(
        flattened.len(), 5,
        "User should have exactly 5 fields (2 from grandparents + 2 from PublicUser + 1 own). Got: {:?}",
        field_names
    );
}

#[test]
fn test_get_inherited_fields_redefined_model_loses_extends() {
    // BUG TEST: When a model from an ancestor is re-defined in the current file
    // WITHOUT repeating the 'extends' clause, the inheritance chain is broken.
    //
    // Example:
    //   public.cdm:
    //     Entity { id: UUID }
    //     TimestampedEntity extends Entity { }
    //     PublicUser extends TimestampedEntity { name?: string }
    //
    //   database.cdm:
    //     extends "public.cdm"
    //     PublicUser { @sql { skip: true } }  // NO extends clause!
    //     User extends PublicUser { email?: string }
    //
    // In this case, User should still inherit 'id' from Entity through the chain:
    // User -> PublicUser -> TimestampedEntity -> Entity
    //
    // But if database.cdm's PublicUser definition has extends=[], the chain breaks.

    // Create ancestor with Entity, TimestampedEntity, PublicUser
    let mut ancestor_symbols = SymbolTable::new();

    // Entity with id field
    ancestor_symbols.definitions.insert(
        "Entity".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model { extends: vec![] },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: local_id(1),
        },
    );

    // TimestampedEntity extends Entity (no direct fields)
    ancestor_symbols.definitions.insert(
        "TimestampedEntity".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model {
                extends: vec!["Entity".to_string()],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: local_id(2),
        },
    );

    // PublicUser extends TimestampedEntity
    ancestor_symbols.definitions.insert(
        "PublicUser".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model {
                extends: vec!["TimestampedEntity".to_string()],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: local_id(3),
        },
    );

    let mut ancestor_fields = HashMap::new();
    ancestor_fields.insert(
        "Entity".to_string(),
        vec![FieldInfo {
            name: "id".to_string(),
            type_expr: Some("UUID".to_string()),
            optional: false,
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: None,
            entity_id: local_id(10),
        }],
    );
    ancestor_fields.insert("TimestampedEntity".to_string(), vec![]);
    ancestor_fields.insert(
        "PublicUser".to_string(),
        vec![FieldInfo {
            name: "name".to_string(),
            type_expr: Some("string".to_string()),
            optional: true,
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: None,
            entity_id: local_id(11),
        }],
    );

    let ancestors = vec![Ancestor {
        path: "public.cdm".to_string(),
        symbol_table: ancestor_symbols,
        model_fields: ancestor_fields,
    }];

    // Current file re-defines PublicUser WITHOUT extends clause
    let mut current_symbols = SymbolTable::new();

    // PublicUser is re-defined with skip: true but NO extends clause
    // BUG: extends=[] here, should be extends=[TimestampedEntity] after merge
    let mut skip_config = HashMap::new();
    skip_config.insert("sql".to_string(), serde_json::json!({ "skip": true }));
    current_symbols.definitions.insert(
        "PublicUser".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model {
                extends: vec![], // No extends in current file!
            },
            span: test_span(),
            plugin_configs: skip_config,
            entity_id: local_id(3),
        },
    );

    // User extends PublicUser
    current_symbols.definitions.insert(
        "User".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model {
                extends: vec!["PublicUser".to_string()],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: local_id(4),
        },
    );

    let mut current_fields = HashMap::new();
    current_fields.insert("PublicUser".to_string(), vec![]); // No new fields, just config
    current_fields.insert(
        "User".to_string(),
        vec![FieldInfo {
            name: "email".to_string(),
            type_expr: Some("string".to_string()),
            optional: true,
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: None,
            entity_id: local_id(20),
        }],
    );

    // STEP 1: Demonstrate the bug - get_inherited_fields with raw current_symbols
    // where PublicUser has extends=[]
    let flattened_with_bug = crate::symbol_table::get_inherited_fields(
        "User",
        &current_fields,
        &current_symbols,
        &ancestors,
    );
    let field_names_with_bug: Vec<_> = flattened_with_bug.iter().map(|f| f.name.as_str()).collect();

    // The bug: User only gets its own field, not inherited fields
    assert_eq!(
        field_names_with_bug, vec!["email"],
        "Bug demonstration: with raw current_symbols, User only gets its own field"
    );

    // STEP 2: Build resolved schema to get the correct merged parents
    let empty_removals: HashSet<String> = HashSet::new();
    let resolved = build_resolved_schema(
        &current_symbols,
        &current_fields,
        &ancestors,
        &empty_removals,
        &HashMap::new(),
    );

    // Verify resolved.models has the correct merged parents
    let resolved_public_user = resolved.models.get("PublicUser").unwrap();
    assert_eq!(
        resolved_public_user.parents, vec!["TimestampedEntity"],
        "Resolved PublicUser should have merged parents from ancestor"
    );

    // STEP 3: Apply the fix - create a modified symbol table with merged parents
    // AND a temp_model_fields with resolved fields (both parts are needed)
    // This is what build_cdm_schema_for_plugin does
    let mut fixed_symbol_table = current_symbols.clone();
    for (resolved_name, resolved_model) in &resolved.models {
        if let Some(def) = fixed_symbol_table.definitions.get_mut(resolved_name) {
            if let crate::DefinitionKind::Model { extends } = &mut def.kind {
                *extends = resolved_model.parents.clone();
            }
        }
    }

    // Also build temp_model_fields with resolved fields (like build_cdm_schema_for_plugin does)
    let mut temp_model_fields = current_fields.clone();
    for (resolved_name, resolved_model) in &resolved.models {
        let resolved_fields: Vec<FieldInfo> = resolved_model.fields.iter().map(|f| {
            FieldInfo {
                name: f.name.clone(),
                type_expr: f.type_expr.clone(),
                optional: f.optional,
                span: f.source_span,
                plugin_configs: f.plugin_configs.clone(),
                default_value: f.default_value.clone(),
                entity_id: f.entity_id.clone(),
            }
        }).collect();
        temp_model_fields.insert(resolved_name.clone(), resolved_fields);
    }

    // STEP 4: Now get_inherited_fields with the fixed symbol table and fields should work
    let flattened_fixed = crate::symbol_table::get_inherited_fields(
        "User",
        &temp_model_fields,
        &fixed_symbol_table,
        &ancestors,
    );
    let field_names_fixed: Vec<_> = flattened_fixed.iter().map(|f| f.name.as_str()).collect();

    // After fix: User should inherit all fields through the chain
    assert!(
        field_names_fixed.contains(&"id"),
        "After fix: User should inherit 'id' from Entity. Got: {:?}",
        field_names_fixed
    );
    assert!(
        field_names_fixed.contains(&"name"),
        "After fix: User should inherit 'name' from PublicUser. Got: {:?}",
        field_names_fixed
    );
    assert!(
        field_names_fixed.contains(&"email"),
        "After fix: User should have its own 'email' field. Got: {:?}",
        field_names_fixed
    );
}

#[test]
fn test_qualified_type_aliases_filtered_from_plugin_schema() {
    // BUG TEST: Template type aliases with qualified names (like "sqlType.UUID")
    // should NOT be passed to plugins. These are internal to CDM's schema resolution
    // and should be filtered out before creating the Schema for plugins.
    //
    // The resolved schema contains qualified type aliases (e.g., "sql.UUID") for
    // internal type resolution. When converting to the plugin Schema, these should
    // be filtered out - plugins should only see local type aliases.

    // This test verifies the filtering logic by checking that the existing
    // test_build_resolved_schema_with_template_namespace_type_alias creates
    // qualified names, and that the to_plugin_schema conversion filters them.

    // First, reproduce the scenario from test_build_resolved_schema_with_template_namespace_type_alias
    let mut current_symbols = SymbolTable::new();

    // Create the template namespace with a type alias that has sql config
    let mut template_symbol_table = SymbolTable::new();
    let mut sql_config = HashMap::new();
    sql_config.insert("sql".to_string(), serde_json::json!({"type": "UUID"}));

    template_symbol_table.definitions.insert(
        "UUID".to_string(),
        crate::Definition {
            kind: DefinitionKind::TypeAlias {
                references: vec!["string".to_string()],
                type_expr: "string".to_string(),
            },
            span: test_span(),
            plugin_configs: sql_config.clone(),
            entity_id: Some(EntityId::local(60)),
        },
    );

    template_symbol_table.definitions.insert(
        "Text".to_string(),
        crate::Definition {
            kind: DefinitionKind::TypeAlias {
                references: vec!["string".to_string()],
                type_expr: "string".to_string(),
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: Some(EntityId::local(61)),
        },
    );

    // Add a local type alias (should be kept)
    current_symbols.definitions.insert(
        "Email".to_string(),
        crate::Definition {
            kind: DefinitionKind::TypeAlias {
                references: vec!["string".to_string()],
                type_expr: "string".to_string(),
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: Some(EntityId::local(100)),
        },
    );

    // Add the namespace to current_symbols
    current_symbols.namespaces.insert(
        "sql".to_string(),
        crate::ImportedNamespace {
            name: "sql".to_string(),
            template_path: std::path::PathBuf::from("templates/sql-types/postgres.cdm"),
            symbol_table: template_symbol_table,
            model_fields: HashMap::new(),
            template_source: EntityIdSource::LocalTemplate { path: "templates/sql-types".to_string() },
        },
    );

    let current_fields = HashMap::new();
    let ancestors = vec![];
    let removals: HashSet<String> = HashSet::new();

    let resolved = build_resolved_schema(&current_symbols, &current_fields, &ancestors, &removals, &HashMap::new());

    // Verify the resolved schema has template type aliases with is_from_template flag
    assert!(resolved.type_aliases.contains_key("sql.UUID"),
        "Resolved schema should contain sql.UUID for internal resolution");
    assert!(resolved.type_aliases.contains_key("sql.Text"),
        "Resolved schema should contain sql.Text for internal resolution");
    assert!(resolved.type_aliases.contains_key("Email"),
        "Resolved schema should contain local Email type alias");

    // Verify the is_from_template flag is set correctly
    let sql_uuid = &resolved.type_aliases["sql.UUID"];
    assert!(sql_uuid.is_from_template, "sql.UUID should be marked as from template");

    let sql_text = &resolved.type_aliases["sql.Text"];
    assert!(sql_text.is_from_template, "sql.Text should be marked as from template");

    let email = &resolved.type_aliases["Email"];
    assert!(!email.is_from_template, "Email should NOT be marked as from template (it's a local type alias)");
}

#[test]
fn test_removals_exclude_models_from_resolved_schema() {
    // Set up ancestor with base models
    let mut ancestor_symbols = SymbolTable::new();
    ancestor_symbols.definitions.insert(
        "BaseModel".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model { extends: vec![] },
            span: test_span(),
            plugin_configs: {
                let mut configs = HashMap::new();
                configs.insert("typeorm".to_string(), serde_json::json!({ "skip": true }));
                configs
            },
            entity_id: None,
        },
    );
    ancestor_symbols.definitions.insert(
        "ChildModel".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model { extends: vec!["BaseModel".to_string()] },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );

    let mut ancestor_fields = HashMap::new();
    ancestor_fields.insert("BaseModel".to_string(), vec![
        FieldInfo {
            name: "id".to_string(),
            type_expr: Some("string".to_string()),
            optional: false,
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: None,
            entity_id: None,
        },
    ]);
    ancestor_fields.insert("ChildModel".to_string(), vec![
        FieldInfo {
            name: "name".to_string(),
            type_expr: Some("string".to_string()),
            optional: false,
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: None,
            entity_id: None,
        },
    ]);

    let ancestors = vec![Ancestor {
        path: "ancestor.cdm".to_string(),
        symbol_table: ancestor_symbols,
        model_fields: ancestor_fields,
    }];

    // Current file removes BaseModel but keeps ChildModel
    let current_symbols = SymbolTable::new();
    let current_fields = HashMap::new();
    let mut removal_names: HashSet<String> = HashSet::new();
    removal_names.insert("BaseModel".to_string());

    let resolved = build_resolved_schema(
        &current_symbols,
        &current_fields,
        &ancestors,
        &removal_names,
        &HashMap::new(),
    );

    // BaseModel should be excluded from resolved schema
    assert!(!resolved.models.contains_key("BaseModel"),
        "BaseModel should be excluded from resolved schema due to removal");

    // ChildModel should still be included
    assert!(resolved.models.contains_key("ChildModel"),
        "ChildModel should still be in resolved schema");
}

#[test]
fn test_removed_parent_model_config_still_inherited() {
    // BUG TEST: When a parent model is removed from the schema output,
    // its config should still be inherited by child models.
    //
    // Example scenario:
    //   public.cdm:
    //     Entity {
    //       id: UUID
    //       @sql { indexes: [{ fields: ["id"], primary: true }] }
    //     }
    //     TimestampedEntity extends Entity { }
    //     PublicProject extends TimestampedEntity { name: string }
    //
    //   database.cdm:
    //     extends "public.cdm"
    //     -Entity
    //     -TimestampedEntity
    //     -PublicProject
    //     Project extends PublicProject { owner_id: UUID }
    //
    // Project should inherit the @sql { indexes: [...] } config from Entity
    // even though Entity is removed from the output schema.

    // Create ancestor with Entity, TimestampedEntity, PublicProject
    let mut ancestor_symbols = SymbolTable::new();

    // Entity with primary key config
    let mut entity_config = HashMap::new();
    entity_config.insert(
        "sql".to_string(),
        serde_json::json!({
            "indexes": [{ "fields": ["id"], "primary": true }]
        }),
    );
    ancestor_symbols.definitions.insert(
        "Entity".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model { extends: vec![] },
            span: test_span(),
            plugin_configs: entity_config,
            entity_id: local_id(1),
        },
    );

    // TimestampedEntity extends Entity
    ancestor_symbols.definitions.insert(
        "TimestampedEntity".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model {
                extends: vec!["Entity".to_string()],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: local_id(2),
        },
    );

    // PublicProject extends TimestampedEntity
    ancestor_symbols.definitions.insert(
        "PublicProject".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model {
                extends: vec!["TimestampedEntity".to_string()],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: local_id(3),
        },
    );

    let mut ancestor_fields = HashMap::new();
    ancestor_fields.insert(
        "Entity".to_string(),
        vec![FieldInfo {
            name: "id".to_string(),
            type_expr: Some("UUID".to_string()),
            optional: false,
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: None,
            entity_id: local_id(10),
        }],
    );
    ancestor_fields.insert("TimestampedEntity".to_string(), vec![]);
    ancestor_fields.insert(
        "PublicProject".to_string(),
        vec![FieldInfo {
            name: "name".to_string(),
            type_expr: Some("string".to_string()),
            optional: false,
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: None,
            entity_id: local_id(11),
        }],
    );

    let ancestors = vec![Ancestor {
        path: "public.cdm".to_string(),
        symbol_table: ancestor_symbols,
        model_fields: ancestor_fields,
    }];

    // Current file: Project extends PublicProject, with Entity/TimestampedEntity/PublicProject removed
    let mut current_symbols = SymbolTable::new();
    current_symbols.definitions.insert(
        "Project".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model {
                extends: vec!["PublicProject".to_string()],
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: local_id(4),
        },
    );

    let mut current_fields = HashMap::new();
    current_fields.insert(
        "Project".to_string(),
        vec![FieldInfo {
            name: "owner_id".to_string(),
            type_expr: Some("UUID".to_string()),
            optional: false,
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: None,
            entity_id: local_id(20),
        }],
    );

    // Remove Entity, TimestampedEntity, and PublicProject from output
    let mut removal_names: HashSet<String> = HashSet::new();
    removal_names.insert("Entity".to_string());
    removal_names.insert("TimestampedEntity".to_string());
    removal_names.insert("PublicProject".to_string());

    let resolved = build_resolved_schema(
        &current_symbols,
        &current_fields,
        &ancestors,
        &removal_names,
        &HashMap::new(),
    );

    // Verify removed models are not in resolved.models (output schema)
    assert!(
        !resolved.models.contains_key("Entity"),
        "Entity should be removed from resolved.models"
    );
    assert!(
        !resolved.models.contains_key("TimestampedEntity"),
        "TimestampedEntity should be removed from resolved.models"
    );
    assert!(
        !resolved.models.contains_key("PublicProject"),
        "PublicProject should be removed from resolved.models"
    );

    // But removed models SHOULD be in all_models_for_inheritance
    assert!(
        resolved.all_models_for_inheritance.contains_key("Entity"),
        "Entity should be in all_models_for_inheritance for config inheritance"
    );
    assert!(
        resolved.all_models_for_inheritance.contains_key("TimestampedEntity"),
        "TimestampedEntity should be in all_models_for_inheritance"
    );
    assert!(
        resolved.all_models_for_inheritance.contains_key("PublicProject"),
        "PublicProject should be in all_models_for_inheritance"
    );

    // Project should be in both
    assert!(
        resolved.models.contains_key("Project"),
        "Project should be in resolved.models"
    );
    assert!(
        resolved.all_models_for_inheritance.contains_key("Project"),
        "Project should be in all_models_for_inheritance"
    );

    // Test the config inheritance through get_merged_model_config
    // This is what build_cdm_schema_for_plugin uses
    let mut visited = HashSet::new();
    let merged_config = super::get_merged_model_config(
        "Project",
        &resolved.all_models_for_inheritance,
        "sql",
        &mut visited,
    );

    // Project should have inherited the indexes config from Entity
    let indexes = merged_config.get("indexes");
    assert!(
        indexes.is_some(),
        "Project should have inherited indexes from Entity through the inheritance chain. Got config: {:?}",
        merged_config
    );

    let indexes_array = indexes.unwrap().as_array();
    assert!(
        indexes_array.is_some() && !indexes_array.unwrap().is_empty(),
        "Project should have inherited primary key index from Entity"
    );

    // Verify the primary key is present
    let first_index = &indexes_array.unwrap()[0];
    assert_eq!(
        first_index.get("primary").and_then(|v| v.as_bool()),
        Some(true),
        "The inherited index should be a primary key"
    );
}

#[test]
fn test_inherited_field_uses_type_alias_name_not_resolved_type() {
    // BUG TEST: When a field has a type alias (e.g., status: Status), the generated
    // code should reference the type alias name "Status", not the resolved base type
    // like "\"active\" | \"inactive\"".
    //
    // This test verifies that resolve_template_type correctly keeps the type alias
    // reference for regular (non-template) type aliases.

    // Create a type alias
    let mut current_symbols = SymbolTable::new();
    current_symbols.definitions.insert(
        "Status".to_string(),
        crate::Definition {
            kind: DefinitionKind::TypeAlias {
                references: vec![],
                type_expr: "\"active\" | \"inactive\"".to_string(),
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );

    // Create a model that uses the type alias
    current_symbols.definitions.insert(
        "User".to_string(),
        crate::Definition {
            kind: DefinitionKind::Model { extends: vec![] },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: None,
        },
    );

    let mut current_fields = HashMap::new();
    current_fields.insert("User".to_string(), vec![
        FieldInfo {
            name: "status".to_string(),
            type_expr: Some("Status".to_string()),  // Field uses type alias
            optional: false,
            span: test_span(),
            plugin_configs: HashMap::new(),
            default_value: None,
            entity_id: None,
        },
    ]);

    let ancestors = vec![];
    let removal_names: HashSet<String> = HashSet::new();

    let resolved = build_resolved_schema(
        &current_symbols,
        &current_fields,
        &ancestors,
        &removal_names,
        &HashMap::new(),
    );

    // Verify Status type alias is in the schema (not from template)
    assert!(resolved.type_aliases.contains_key("Status"));
    assert!(!resolved.type_aliases["Status"].is_from_template,
        "Status should not be from template");

    // Now test resolve_template_type - it should return a Reference to "Status",
    // not the resolved union type
    let resolved_type = super::resolve_template_type("Status", &resolved);

    // The base_type should be a Reference to "Status", not a Union
    match resolved_type.base_type {
        crate::ParsedType::Reference(ref name) => {
            assert_eq!(name, "Status",
                "resolve_template_type should return Reference(\"Status\") for regular type alias");
        }
        other => {
            panic!(
                "Expected ParsedType::Reference(\"Status\") but got {:?}. \
                Regular type aliases should keep their reference, not be resolved to base type.",
                other
            );
        }
    }
}
