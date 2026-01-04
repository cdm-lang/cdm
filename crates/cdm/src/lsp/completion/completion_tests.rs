use super::*;

#[test]
fn test_builtin_type_completions() {
    let items = builtin_type_completions();

    assert_eq!(items.len(), 4);
    assert!(items.iter().any(|i| i.label == "string"));
    assert!(items.iter().any(|i| i.label == "number"));
    assert!(items.iter().any(|i| i.label == "boolean"));
    assert!(items.iter().any(|i| i.label == "JSON"));
}

#[test]
fn test_user_defined_type_completions() {
    let text = r#"
Email: string #1
Status: "active" | "inactive" #2

User {
  name: string #1
} #10
"#;
    let mut parser = Parser::new();
    parser.set_language(&grammar::LANGUAGE.into()).unwrap();
    let tree = parser.parse(text, None).unwrap();

    let items = user_defined_type_completions(tree.root_node(), text);

    // Should have 3 items: Email, Status, User
    assert_eq!(items.len(), 3);

    let email = items.iter().find(|i| i.label == "Email");
    assert!(email.is_some());
    assert_eq!(email.unwrap().kind, Some(CompletionItemKind::TYPE_PARAMETER));

    let user = items.iter().find(|i| i.label == "User");
    assert!(user.is_some());
    assert_eq!(user.unwrap().kind, Some(CompletionItemKind::CLASS));
}

#[test]
fn test_model_name_completions() {
    let text = r#"
Email: string #1

User {
  name: string #1
} #10

Post {
  title: string #1
} #20
"#;
    let mut parser = Parser::new();
    parser.set_language(&grammar::LANGUAGE.into()).unwrap();
    let tree = parser.parse(text, None).unwrap();

    let items = model_name_completions(tree.root_node(), text);

    // Should have 2 models: User, Post (not Email)
    assert_eq!(items.len(), 2);
    assert!(items.iter().any(|i| i.label == "User"));
    assert!(items.iter().any(|i| i.label == "Post"));
    assert!(!items.iter().any(|i| i.label == "Email"));
}

#[test]
fn test_snippet_completions() {
    let items = snippet_completions();

    assert!(items.len() >= 3);
    assert!(items.iter().any(|i| i.label == "model"));
    assert!(items.iter().any(|i| i.label == "type"));
    assert!(items.iter().any(|i| i.label == "extends"));
}

#[test]
fn test_is_after_colon() {
    let text = "User {\n  name: ";
    assert!(is_after_colon(text, text.len()));

    let text2 = "User {\n  name";
    assert!(!is_after_colon(text2, text2.len()));

    let text3 = "User {\n  name:  ";
    assert!(is_after_colon(text3, text3.len()));
}

#[test]
fn test_compute_completions_after_colon() {
    let text = r#"
Email: string #1

User {
  name:
} #10
"#;
    // Position after "name: " (line 4, after the space)
    let position = Position { line: 4, character: 8 };

    let completions = compute_completions(text, position, None, None);
    assert!(completions.is_some());

    let items = completions.unwrap();
    // Should have built-in types + Email type alias + User model
    assert!(items.len() >= 4);
    assert!(items.iter().any(|i| i.label == "string"));
    assert!(items.iter().any(|i| i.label == "Email"));
}

#[test]
fn test_compute_completions_after_colon_and_space() {
    let text = "Email: string #1

User {
  name:
} #10
";
    // Position after "name: " with a trailing space (line 3, character 9)
    let position = Position { line: 3, character: 9 };

    let completions = compute_completions(text, position, None, None);
    assert!(completions.is_some());

    let items = completions.unwrap();
    // Should have built-in types + Email type alias, NOT snippets
    assert!(items.len() >= 4, "Expected at least 4 items, got {}", items.len());
    assert!(items.iter().any(|i| i.label == "string"), "Should suggest 'string' type");
    assert!(items.iter().any(|i| i.label == "Email"), "Should suggest 'Email' type alias");

    // Should NOT suggest snippets in a type position
    assert!(!items.iter().any(|i| i.label == "model"), "Should NOT suggest 'model' snippet in type position");
    assert!(!items.iter().any(|i| i.label == "type"), "Should NOT suggest 'type' snippet in type position");
    assert!(!items.iter().any(|i| i.label == "extends"), "Should NOT suggest 'extends' snippet in type position");
}

#[test]
fn test_compute_completions_top_level() {
    let text = r#"
Email: string #1


"#;
    // Position at the blank line (line 3)
    let position = Position { line: 3, character: 0 };

    let completions = compute_completions(text, position, None, None);
    assert!(completions.is_some());

    let items = completions.unwrap();
    // Should have snippets
    assert!(items.iter().any(|i| i.label == "model"));
    assert!(items.iter().any(|i| i.label == "type"));
}

// Tests for plugin config context detection

#[test]
fn test_plugin_config_level_detection() {
    // Test GlobalSettings context detection
    let text = r#"
@sql {

}
"#;
    let mut parser = Parser::new();
    parser.set_language(&grammar::LANGUAGE.into()).unwrap();
    let tree = parser.parse(text, None).unwrap();
    let root = tree.root_node();

    // Position inside the @sql { } block (line 2, after spaces)
    let position = Position { line: 2, character: 2 };
    let byte_offset = super::super::position::lsp_position_to_byte_offset(text, position);

    if let Some(node) = find_node_at_offset(root, byte_offset) {
        let context = detect_plugin_config_context(node, text, byte_offset);
        if let Some(CompletionContext::PluginConfigField { plugin_name, config_level }) = context {
            assert_eq!(plugin_name, "sql");
            assert!(matches!(config_level, PluginConfigLevel::Global));
        } else {
            panic!("Expected PluginConfigField context for global plugin config");
        }
    }
}

#[test]
fn test_plugin_field_completions_with_mock_schema() {
    use super::super::plugin_schema_cache::{PluginSettingsSchema, SettingsField};

    let schema = PluginSettingsSchema {
        global_settings: vec![
            SettingsField {
                name: "dialect".to_string(),
                type_expr: Some("\"postgresql\" | \"sqlite\"".to_string()),
                optional: false,
                default_value: Some(serde_json::json!("postgresql")),
                literal_values: vec!["postgresql".to_string(), "sqlite".to_string()],
                is_boolean: false,
            },
            SettingsField {
                name: "schema".to_string(),
                type_expr: Some("string".to_string()),
                optional: true,
                default_value: None,
                literal_values: vec![],
                is_boolean: false,
            },
            SettingsField {
                name: "infer_not_null".to_string(),
                type_expr: Some("boolean".to_string()),
                optional: false,
                default_value: Some(serde_json::json!(true)),
                literal_values: vec![],
                is_boolean: true,
            },
        ],
        type_alias_settings: vec![],
        model_settings: vec![],
        field_settings: vec![],
    };

    let already_defined = std::collections::HashSet::new();
    let items = plugin_field_completions(&schema, &PluginConfigLevel::Global, &already_defined);

    assert_eq!(items.len(), 3);
    assert!(items.iter().any(|i| i.label == "dialect"));
    assert!(items.iter().any(|i| i.label == "schema"));
    assert!(items.iter().any(|i| i.label == "infer_not_null"));

    // Test filtering already defined fields
    let mut already_defined = std::collections::HashSet::new();
    already_defined.insert("dialect".to_string());
    let items = plugin_field_completions(&schema, &PluginConfigLevel::Global, &already_defined);
    assert_eq!(items.len(), 2);
    assert!(!items.iter().any(|i| i.label == "dialect"));
}

#[test]
fn test_plugin_value_completions_with_mock_schema() {
    use super::super::plugin_schema_cache::{PluginSettingsSchema, SettingsField};

    let schema = PluginSettingsSchema {
        global_settings: vec![
            SettingsField {
                name: "dialect".to_string(),
                type_expr: Some("\"postgresql\" | \"sqlite\"".to_string()),
                optional: false,
                default_value: Some(serde_json::json!("postgresql")),
                literal_values: vec!["postgresql".to_string(), "sqlite".to_string()],
                is_boolean: false,
            },
            SettingsField {
                name: "infer_not_null".to_string(),
                type_expr: Some("boolean".to_string()),
                optional: false,
                default_value: Some(serde_json::json!(true)),
                literal_values: vec![],
                is_boolean: true,
            },
        ],
        type_alias_settings: vec![],
        model_settings: vec![],
        field_settings: vec![],
    };

    // Test enum value completions
    let items = plugin_value_completions(&schema, &PluginConfigLevel::Global, "dialect");
    assert_eq!(items.len(), 2);
    assert!(items.iter().any(|i| i.label == "\"postgresql\""));
    assert!(items.iter().any(|i| i.label == "\"sqlite\""));

    // Test boolean value completions
    let items = plugin_value_completions(&schema, &PluginConfigLevel::Global, "infer_not_null");
    assert_eq!(items.len(), 2);
    assert!(items.iter().any(|i| i.label == "true"));
    assert!(items.iter().any(|i| i.label == "false"));
}
