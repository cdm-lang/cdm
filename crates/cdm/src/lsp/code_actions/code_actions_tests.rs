use super::*;

#[test]
fn test_calculate_next_entity_id() {
    let text = r#"Email: string #5

User {
  name: string #1
} #10"#;

    let mut parser = Parser::new();
    parser.set_language(&grammar::LANGUAGE.into()).unwrap();
    let tree = parser.parse(text, None).unwrap();
    let root = tree.root_node();

    let next_id = calculate_next_entity_id(&root, text);
    assert_eq!(next_id, 11); // Max is 10, so next is 11
}

#[test]
fn test_calculate_next_field_id() {
    let text = r#"User {
  name: string #1
  email: string #3
  age: number #2
} #10"#;

    let mut parser = Parser::new();
    parser.set_language(&grammar::LANGUAGE.into()).unwrap();
    let tree = parser.parse(text, None).unwrap();
    let root = tree.root_node();

    // Find the model node
    let model_node = root.child(0).unwrap();
    let next_id = calculate_next_field_id(&model_node, text);
    assert_eq!(next_id, 4); // Max is 3, so next is 4
}

#[test]
fn test_ranges_overlap() {
    let range1 = Range::new(Position::new(0, 0), Position::new(0, 10));
    let range2 = Range::new(Position::new(0, 5), Position::new(0, 15));
    assert!(ranges_overlap(&range1, &range2));

    let range3 = Range::new(Position::new(0, 0), Position::new(0, 5));
    let range4 = Range::new(Position::new(0, 10), Position::new(0, 15));
    assert!(!ranges_overlap(&range3, &range4));
}

#[test]
fn test_ranges_overlap_exact_match() {
    let range1 = Range::new(Position::new(0, 0), Position::new(0, 10));
    let range2 = Range::new(Position::new(0, 0), Position::new(0, 10));
    assert!(ranges_overlap(&range1, &range2));
}

#[test]
fn test_ranges_overlap_touching_boundaries() {
    let range1 = Range::new(Position::new(0, 0), Position::new(0, 5));
    let range2 = Range::new(Position::new(0, 5), Position::new(0, 10));
    assert!(ranges_overlap(&range1, &range2));
}

#[test]
fn test_ranges_overlap_multiline() {
    let range1 = Range::new(Position::new(0, 0), Position::new(2, 5));
    let range2 = Range::new(Position::new(1, 0), Position::new(3, 0));
    assert!(ranges_overlap(&range1, &range2));
}

#[test]
fn test_ranges_no_overlap_different_lines() {
    let range1 = Range::new(Position::new(0, 0), Position::new(0, 10));
    let range2 = Range::new(Position::new(2, 0), Position::new(2, 10));
    assert!(!ranges_overlap(&range1, &range2));
}

#[test]
fn test_byte_offset_to_position_simple() {
    let text = "Hello\nWorld\n";
    let pos = byte_offset_to_position(text, 6);
    assert_eq!(pos.line, 1);
    assert_eq!(pos.character, 0);
}

#[test]
fn test_byte_offset_to_position_start() {
    let text = "Hello\nWorld\n";
    let pos = byte_offset_to_position(text, 0);
    assert_eq!(pos.line, 0);
    assert_eq!(pos.character, 0);
}

#[test]
fn test_byte_offset_to_position_mid_line() {
    let text = "Hello\nWorld\n";
    let pos = byte_offset_to_position(text, 3);
    assert_eq!(pos.line, 0);
    assert_eq!(pos.character, 3);
}

#[test]
fn test_extract_text_at_range_single_line() {
    let text = "Hello World\nNext line";
    let range = Range::new(Position::new(0, 0), Position::new(0, 5));
    let result = extract_text_at_range(text, &range);
    assert_eq!(result, Some("Hello".to_string()));
}

#[test]
fn test_extract_text_at_range_mid_word() {
    let text = "Hello World\nNext line";
    let range = Range::new(Position::new(0, 6), Position::new(0, 11));
    let result = extract_text_at_range(text, &range);
    assert_eq!(result, Some("World".to_string()));
}

#[test]
fn test_extract_text_at_range_invalid_line() {
    let text = "Hello World";
    let range = Range::new(Position::new(10, 0), Position::new(10, 5));
    let result = extract_text_at_range(text, &range);
    assert_eq!(result, None);
}

#[test]
fn test_extract_text_at_range_out_of_bounds() {
    let text = "Hello";
    let range = Range::new(Position::new(0, 0), Position::new(0, 100));
    let result = extract_text_at_range(text, &range);
    assert_eq!(result, None);
}

#[test]
fn test_find_type_alias_insert_position_after_directives() {
    let text = r#"extends "./base.cdm"
@plugin "test-plugin"

User { name: string } #1"#;

    let mut parser = Parser::new();
    parser.set_language(&grammar::LANGUAGE.into()).unwrap();
    let tree = parser.parse(text, None).unwrap();
    let root = tree.root_node();

    let pos = find_type_alias_insert_position(&root, text);
    assert!(pos.is_some());
}

#[test]
fn test_find_type_alias_insert_position_no_directives() {
    let text = r#"User { name: string } #1"#;

    let mut parser = Parser::new();
    parser.set_language(&grammar::LANGUAGE.into()).unwrap();
    let tree = parser.parse(text, None).unwrap();
    let root = tree.root_node();

    let pos = find_type_alias_insert_position(&root, text);
    assert!(pos.is_some());
}

#[test]
fn test_calculate_next_entity_id_empty_document() {
    let text = "";

    let mut parser = Parser::new();
    parser.set_language(&grammar::LANGUAGE.into()).unwrap();
    let tree = parser.parse(text, None).unwrap();
    let root = tree.root_node();

    let next_id = calculate_next_entity_id(&root, text);
    assert_eq!(next_id, 1);
}

#[test]
fn test_calculate_next_field_id_no_fields() {
    let text = r#"User {} #10"#;

    let mut parser = Parser::new();
    parser.set_language(&grammar::LANGUAGE.into()).unwrap();
    let tree = parser.parse(text, None).unwrap();
    let root = tree.root_node();

    let model_node = root.child(0).unwrap();
    let next_id = calculate_next_field_id(&model_node, text);
    assert_eq!(next_id, 1);
}

#[test]
fn test_compute_code_actions_no_diagnostics() {
    let text = r#"User { name: string #1 } #2"#;
    let range = Range::new(Position::new(0, 0), Position::new(0, 20));
    let diagnostics = vec![];
    let uri = Url::parse("file:///test.cdm").unwrap();

    let result = compute_code_actions(text, range, &diagnostics, &diagnostics, &uri);
    assert_eq!(result, None);
}

#[test]
fn test_compute_code_actions_w005_missing_entity_id() {
    let text = r#"User { name: string #1 }"#;
    let range = Range::new(Position::new(0, 0), Position::new(0, 24));
    let diagnostics = vec![Diagnostic {
        range,
        severity: Some(tower_lsp::lsp_types::DiagnosticSeverity::WARNING),
        message: "W005: Model is missing entity ID".to_string(),
        ..Default::default()
    }];
    let uri = Url::parse("file:///test.cdm").unwrap();

    let result = compute_code_actions(text, range, &diagnostics, &diagnostics, &uri);
    assert!(result.is_some());
    let actions = result.unwrap();
    assert_eq!(actions.len(), 1);

    if let CodeActionOrCommand::CodeAction(action) = &actions[0] {
        assert!(action.title.contains("Add entity ID"));
    }
}

#[test]
fn test_compute_code_actions_w006_missing_field_id() {
    let text = r#"User { name: string } #1"#;
    let range = Range::new(Position::new(0, 7), Position::new(0, 19));
    let diagnostics = vec![Diagnostic {
        range,
        severity: Some(tower_lsp::lsp_types::DiagnosticSeverity::WARNING),
        message: "W006: Field is missing field ID".to_string(),
        ..Default::default()
    }];
    let uri = Url::parse("file:///test.cdm").unwrap();

    let result = compute_code_actions(text, range, &diagnostics, &diagnostics, &uri);
    assert!(result.is_some());
    let actions = result.unwrap();
    assert_eq!(actions.len(), 1);

    if let CodeActionOrCommand::CodeAction(action) = &actions[0] {
        assert!(action.title.contains("Add field ID"));
    }
}

#[test]
fn test_compute_code_actions_undefined_type() {
    let text = r#"User { email: Email } #1"#;
    let range = Range::new(Position::new(0, 14), Position::new(0, 19));
    let diagnostics = vec![Diagnostic {
        range,
        severity: Some(tower_lsp::lsp_types::DiagnosticSeverity::ERROR),
        message: "Undefined type: Email".to_string(),
        ..Default::default()
    }];
    let uri = Url::parse("file:///test.cdm").unwrap();

    let result = compute_code_actions(text, range, &diagnostics, &diagnostics, &uri);
    assert!(result.is_some());
    let actions = result.unwrap();
    assert_eq!(actions.len(), 1);

    if let CodeActionOrCommand::CodeAction(action) = &actions[0] {
        assert!(action.title.contains("Create type alias"));
        assert!(action.title.contains("Email"));
    }
}

#[test]
fn test_compute_code_actions_non_overlapping_diagnostic() {
    let text = r#"User { name: string #1 } #2"#;
    let requested_range = Range::new(Position::new(0, 0), Position::new(0, 5));
    let diagnostic_range = Range::new(Position::new(1, 0), Position::new(1, 10));
    let diagnostics = vec![Diagnostic {
        range: diagnostic_range,
        severity: Some(tower_lsp::lsp_types::DiagnosticSeverity::WARNING),
        message: "W005: Model is missing entity ID".to_string(),
        ..Default::default()
    }];
    let uri = Url::parse("file:///test.cdm").unwrap();

    let result = compute_code_actions(text, requested_range, &diagnostics, &diagnostics, &uri);
    assert_eq!(result, None);
}

#[test]
fn test_find_model_at_range_not_found() {
    let text = r#"Email: string #1"#;
    let mut parser = Parser::new();
    parser.set_language(&grammar::LANGUAGE.into()).unwrap();
    let tree = parser.parse(text, None).unwrap();
    let root = tree.root_node();

    let range = Range::new(Position::new(0, 0), Position::new(0, 10));
    let result = find_model_at_range(&root, text, &range);
    assert!(result.is_none());
}

#[test]
fn test_find_field_at_range_not_found() {
    let text = r#"Email: string #1"#;
    let mut parser = Parser::new();
    parser.set_language(&grammar::LANGUAGE.into()).unwrap();
    let tree = parser.parse(text, None).unwrap();
    let root = tree.root_node();

    let range = Range::new(Position::new(0, 0), Position::new(0, 10));
    let result = find_field_at_range(&root, text, &range);
    assert!(result.is_none());
}

#[test]
fn test_extract_plugin_name_from_cache_message() {
    let msg = "E401: Plugin not found: 'typescript' - Plugin 'typescript' not found in cache. Run 'cdm build' to download it.";
    let result = extract_plugin_name(msg);
    assert_eq!(result, Some("typescript".to_string()));
}

#[test]
fn test_extract_plugin_name_from_not_found_message() {
    let msg = "Plugin not found: 'sql'";
    let result = extract_plugin_name(msg);
    assert_eq!(result, Some("sql".to_string()));
}

#[test]
fn test_extract_plugin_name_no_match() {
    let msg = "W005: Model is missing entity ID";
    let result = extract_plugin_name(msg);
    assert_eq!(result, None);
}

#[test]
fn test_compute_code_actions_e401_plugin_not_found() {
    let text = r#"@typescript {}"#;
    let range = Range::new(Position::new(0, 0), Position::new(0, 14));
    let diagnostics = vec![Diagnostic {
        range,
        severity: Some(tower_lsp::lsp_types::DiagnosticSeverity::ERROR),
        message: "E401: Plugin not found: 'typescript' - Plugin 'typescript' not found in cache".to_string(),
        ..Default::default()
    }];
    let uri = Url::parse("file:///test.cdm").unwrap();

    let result = compute_code_actions(text, range, &diagnostics, &diagnostics, &uri);
    assert!(result.is_some());
    let actions = result.unwrap();
    assert_eq!(actions.len(), 1);

    if let CodeActionOrCommand::CodeAction(action) = &actions[0] {
        assert_eq!(action.title, "Download plugin 'typescript'");
        assert!(action.command.is_some());
        let cmd = action.command.as_ref().unwrap();
        assert_eq!(cmd.command, "cdm.downloadPlugin");
        assert!(cmd.arguments.is_some());
        let args = cmd.arguments.as_ref().unwrap();
        assert_eq!(args.len(), 1);
        assert_eq!(args[0], serde_json::Value::String("typescript".to_string()));
    } else {
        panic!("Expected CodeAction");
    }
}

#[test]
fn test_compute_code_actions_multiple_missing_plugins() {
    let text = r#"@typescript {}
@sql {}"#;
    let range = Range::new(Position::new(0, 0), Position::new(1, 7));
    let diagnostics = vec![
        Diagnostic {
            range: Range::new(Position::new(0, 0), Position::new(0, 14)),
            severity: Some(tower_lsp::lsp_types::DiagnosticSeverity::ERROR),
            message: "E401: Plugin not found: 'typescript' - Plugin 'typescript' not found in cache".to_string(),
            ..Default::default()
        },
        Diagnostic {
            range: Range::new(Position::new(1, 0), Position::new(1, 7)),
            severity: Some(tower_lsp::lsp_types::DiagnosticSeverity::ERROR),
            message: "E401: Plugin not found: 'sql' - Plugin 'sql' not found in cache".to_string(),
            ..Default::default()
        },
    ];
    let uri = Url::parse("file:///test.cdm").unwrap();

    let result = compute_code_actions(text, range, &diagnostics, &diagnostics, &uri);
    assert!(result.is_some());
    let actions = result.unwrap();
    // Should have 2 individual download actions only (no "download all" since cursor is on both)
    // The "download all" only appears when there are OTHER missing plugins not at cursor
    assert_eq!(actions.len(), 2);

    // Check that we have download actions for both plugins
    let titles: Vec<String> = actions.iter().filter_map(|a| {
        if let CodeActionOrCommand::CodeAction(action) = a {
            Some(action.title.clone())
        } else {
            None
        }
    }).collect();

    assert!(titles.iter().any(|t| t == "Download plugin 'typescript'"));
    assert!(titles.iter().any(|t| t == "Download plugin 'sql'"));
}

#[test]
fn test_compute_code_actions_download_all_with_other_missing_plugins() {
    // Test that "download all" appears when there are missing plugins at cursor
    // AND other missing plugins elsewhere in the document
    let text = r#"@typescript {}
@sql {}"#;
    // Cursor is only on the first line (typescript)
    let cursor_range = Range::new(Position::new(0, 0), Position::new(0, 14));

    // Diagnostic at cursor (typescript)
    let cursor_diagnostics = vec![
        Diagnostic {
            range: Range::new(Position::new(0, 0), Position::new(0, 14)),
            severity: Some(tower_lsp::lsp_types::DiagnosticSeverity::ERROR),
            message: "E401: Plugin not found: 'typescript' - Plugin 'typescript' not found in cache".to_string(),
            ..Default::default()
        },
    ];

    // All diagnostics in the document (both typescript and sql)
    let all_diagnostics = vec![
        Diagnostic {
            range: Range::new(Position::new(0, 0), Position::new(0, 14)),
            severity: Some(tower_lsp::lsp_types::DiagnosticSeverity::ERROR),
            message: "E401: Plugin not found: 'typescript' - Plugin 'typescript' not found in cache".to_string(),
            ..Default::default()
        },
        Diagnostic {
            range: Range::new(Position::new(1, 0), Position::new(1, 7)),
            severity: Some(tower_lsp::lsp_types::DiagnosticSeverity::ERROR),
            message: "E401: Plugin not found: 'sql' - Plugin 'sql' not found in cache".to_string(),
            ..Default::default()
        },
    ];
    let uri = Url::parse("file:///test.cdm").unwrap();

    let result = compute_code_actions(text, cursor_range, &cursor_diagnostics, &all_diagnostics, &uri);
    assert!(result.is_some());
    let actions = result.unwrap();

    // Should have 1 download action for typescript + 1 "download all" action
    // (because sql is missing elsewhere in the document)
    assert_eq!(actions.len(), 2, "Expected 2 actions, got: {:?}",
        actions.iter().filter_map(|a| {
            if let CodeActionOrCommand::CodeAction(action) = a {
                Some(action.title.clone())
            } else {
                None
            }
        }).collect::<Vec<_>>()
    );

    let titles: Vec<String> = actions.iter().filter_map(|a| {
        if let CodeActionOrCommand::CodeAction(action) = a {
            Some(action.title.clone())
        } else {
            None
        }
    }).collect();

    assert!(titles.iter().any(|t| t == "Download plugin 'typescript'"), "Missing typescript action");
    assert!(titles.iter().any(|t| t == "Download all missing plugins"), "Missing download all action");
}

// =========================================================================
// TEMPLATE CODE ACTION TESTS
// =========================================================================

#[test]
fn test_extract_template_name_from_e601_message() {
    let msg = "E601: Template not found: 'sql-types' - Template 'sql-types' not found in registry. Run 'cdm template cache sql-types' to download it.";
    let result = extract_template_name(msg);
    assert_eq!(result, Some("sql-types".to_string()));
}

#[test]
fn test_extract_template_name_from_failed_to_load_message() {
    let msg = "Failed to load template 'sql-types': Template 'sql-types' not found in registry";
    let result = extract_template_name(msg);
    assert_eq!(result, Some("sql-types".to_string()));
}

#[test]
fn test_extract_template_name_no_match() {
    let msg = "E401: Plugin not found: 'typescript'";
    let result = extract_template_name(msg);
    assert_eq!(result, None);
}

#[test]
fn test_compute_code_actions_e601_template_not_found() {
    let text = r#"import pg from "sql-types/postgres""#;
    let range = Range::new(Position::new(0, 0), Position::new(0, 35));
    let diagnostics = vec![Diagnostic {
        range,
        severity: Some(tower_lsp::lsp_types::DiagnosticSeverity::ERROR),
        message: "E601: Template not found: 'sql-types' - Template 'sql-types' not found in registry".to_string(),
        ..Default::default()
    }];
    let uri = Url::parse("file:///test.cdm").unwrap();

    let result = compute_code_actions(text, range, &diagnostics, &diagnostics, &uri);
    assert!(result.is_some());
    let actions = result.unwrap();
    assert_eq!(actions.len(), 1);

    if let CodeActionOrCommand::CodeAction(action) = &actions[0] {
        assert_eq!(action.title, "Download template 'sql-types'");
        assert!(action.command.is_some());
        let cmd = action.command.as_ref().unwrap();
        assert_eq!(cmd.command, "cdm.downloadTemplate");
        assert!(cmd.arguments.is_some());
        let args = cmd.arguments.as_ref().unwrap();
        assert_eq!(args.len(), 1);
        assert_eq!(args[0], serde_json::Value::String("sql-types".to_string()));
    } else {
        panic!("Expected CodeAction");
    }
}

#[test]
fn test_compute_code_actions_multiple_missing_templates() {
    let text = r#"import pg from "sql-types/postgres"
import auth from "auth-types/base""#;
    let range = Range::new(Position::new(0, 0), Position::new(1, 35));
    let diagnostics = vec![
        Diagnostic {
            range: Range::new(Position::new(0, 0), Position::new(0, 35)),
            severity: Some(tower_lsp::lsp_types::DiagnosticSeverity::ERROR),
            message: "E601: Template not found: 'sql-types' - Template 'sql-types' not found in registry".to_string(),
            ..Default::default()
        },
        Diagnostic {
            range: Range::new(Position::new(1, 0), Position::new(1, 35)),
            severity: Some(tower_lsp::lsp_types::DiagnosticSeverity::ERROR),
            message: "E601: Template not found: 'auth-types' - Template 'auth-types' not found in registry".to_string(),
            ..Default::default()
        },
    ];
    let uri = Url::parse("file:///test.cdm").unwrap();

    let result = compute_code_actions(text, range, &diagnostics, &diagnostics, &uri);
    assert!(result.is_some());
    let actions = result.unwrap();
    // Should have 2 individual download actions only (no "download all" since cursor is on both)
    assert_eq!(actions.len(), 2);

    let titles: Vec<String> = actions.iter().filter_map(|a| {
        if let CodeActionOrCommand::CodeAction(action) = a {
            Some(action.title.clone())
        } else {
            None
        }
    }).collect();

    assert!(titles.iter().any(|t| t == "Download template 'sql-types'"));
    assert!(titles.iter().any(|t| t == "Download template 'auth-types'"));
}

#[test]
fn test_compute_code_actions_download_all_templates_with_other_missing() {
    // Test that "download all templates" appears when there are missing templates at cursor
    // AND other missing templates elsewhere in the document
    let text = r#"import pg from "sql-types/postgres"
import auth from "auth-types/base""#;
    // Cursor is only on the first line (sql-types)
    let cursor_range = Range::new(Position::new(0, 0), Position::new(0, 35));

    // Diagnostic at cursor (sql-types)
    let cursor_diagnostics = vec![
        Diagnostic {
            range: Range::new(Position::new(0, 0), Position::new(0, 35)),
            severity: Some(tower_lsp::lsp_types::DiagnosticSeverity::ERROR),
            message: "E601: Template not found: 'sql-types' - Template 'sql-types' not found in registry".to_string(),
            ..Default::default()
        },
    ];

    // All diagnostics in the document (both sql-types and auth-types)
    let all_diagnostics = vec![
        Diagnostic {
            range: Range::new(Position::new(0, 0), Position::new(0, 35)),
            severity: Some(tower_lsp::lsp_types::DiagnosticSeverity::ERROR),
            message: "E601: Template not found: 'sql-types' - Template 'sql-types' not found in registry".to_string(),
            ..Default::default()
        },
        Diagnostic {
            range: Range::new(Position::new(1, 0), Position::new(1, 35)),
            severity: Some(tower_lsp::lsp_types::DiagnosticSeverity::ERROR),
            message: "E601: Template not found: 'auth-types' - Template 'auth-types' not found in registry".to_string(),
            ..Default::default()
        },
    ];
    let uri = Url::parse("file:///test.cdm").unwrap();

    let result = compute_code_actions(text, cursor_range, &cursor_diagnostics, &all_diagnostics, &uri);
    assert!(result.is_some());
    let actions = result.unwrap();

    // Should have 1 download action for sql-types + 1 "download all" action
    assert_eq!(actions.len(), 2, "Expected 2 actions, got: {:?}",
        actions.iter().filter_map(|a| {
            if let CodeActionOrCommand::CodeAction(action) = a {
                Some(action.title.clone())
            } else {
                None
            }
        }).collect::<Vec<_>>()
    );

    let titles: Vec<String> = actions.iter().filter_map(|a| {
        if let CodeActionOrCommand::CodeAction(action) = a {
            Some(action.title.clone())
        } else {
            None
        }
    }).collect();

    assert!(titles.iter().any(|t| t == "Download template 'sql-types'"), "Missing sql-types action");
    assert!(titles.iter().any(|t| t == "Download all missing templates"), "Missing download all action");
}
