use super::*;
use tower_lsp::LspService;

// Helper function to create a mock server with client
fn create_test_server() -> CdmLanguageServer {
    let (service, _socket) = LspService::new(|client| CdmLanguageServer::new(client));
    service.inner().clone()
}

#[tokio::test]
async fn test_server_initialization() {
    
    let server = create_test_server();

    let params = InitializeParams {
        process_id: Some(1234),
        root_uri: Some(Url::parse("file:///test/workspace").unwrap()),
        client_info: Some(ClientInfo {
            name: "test-client".to_string(),
            version: Some("1.0.0".to_string()),
        }),
        capabilities: ClientCapabilities::default(),
        ..Default::default()
    };

    let result = server.initialize(params).await.unwrap();

    // Verify server capabilities
    assert!(result.capabilities.text_document_sync.is_some());
    assert!(result.capabilities.hover_provider.is_some());
    assert!(result.capabilities.definition_provider.is_some());
    assert!(result.capabilities.references_provider.is_some());
    assert!(result.capabilities.completion_provider.is_some());
    assert!(result.capabilities.document_formatting_provider.is_some());

    // Verify server info
    assert!(result.server_info.is_some());
    let server_info = result.server_info.unwrap();
    assert_eq!(server_info.name, "cdm-lsp");
    assert!(server_info.version.is_some());
}

#[tokio::test]
async fn test_server_shutdown() {
    
    let server = create_test_server();

    let result = server.shutdown().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_did_open_document() {
    
    let server = create_test_server();

    let uri = Url::parse("file:///test.cdm").unwrap();
    let text = r#"
User {
  name: string #1
} #10
"#.to_string();

    let params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "cdm".to_string(),
            version: 1,
            text: text.clone(),
        },
    };

    server.did_open(params).await;

    // Verify document is stored
    let stored_text = server.documents.get(&uri);
    assert_eq!(stored_text, Some(text));
}

#[tokio::test]
async fn test_did_change_document() {
    
    let server = create_test_server();

    let uri = Url::parse("file:///test.cdm").unwrap();

    // First open the document
    let open_params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "cdm".to_string(),
            version: 1,
            text: "initial text".to_string(),
        },
    };
    server.did_open(open_params).await;

    // Now change it
    let updated_text = "updated text".to_string();
    let change_params = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: updated_text.clone(),
        }],
    };

    server.did_change(change_params).await;

    // Verify document is updated
    let stored_text = server.documents.get(&uri);
    assert_eq!(stored_text, Some(updated_text));
}

#[tokio::test]
async fn test_did_close_document() {
    
    let server = create_test_server();

    let uri = Url::parse("file:///test.cdm").unwrap();

    // First open the document
    let open_params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "cdm".to_string(),
            version: 1,
            text: "test content".to_string(),
        },
    };
    server.did_open(open_params).await;

    // Verify it's stored
    assert!(server.documents.get(&uri).is_some());

    // Now close it
    let close_params = DidCloseTextDocumentParams {
        text_document: TextDocumentIdentifier {
            uri: uri.clone(),
        },
    };
    server.did_close(close_params).await;

    // Verify it's removed
    assert!(server.documents.get(&uri).is_none());
}

#[tokio::test]
async fn test_hover_on_type_alias() {
    
    let server = create_test_server();

    let uri = Url::parse("file:///test.cdm").unwrap();
    let text = r#"Email: string #1

User {
  email_addr: Email #2
} #10
"#.to_string();

    // Open the document
    let open_params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "cdm".to_string(),
            version: 1,
            text,
        },
    };
    server.did_open(open_params).await;

    // Request hover on "Email" usage (line 3, character 14)
    let hover_params = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: uri.clone(),
            },
            position: Position {
                line: 3,
                character: 14,
            },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
    };

    let result = server.hover(hover_params).await.unwrap();
    assert!(result.is_some());

    let hover = result.unwrap();
    if let HoverContents::Markup(content) = hover.contents {
        assert!(content.value.contains("Email"));
        assert!(content.value.contains("string"));
    } else {
        panic!("Expected markup content");
    }
}

#[tokio::test]
async fn test_hover_on_builtin_type() {
    
    let server = create_test_server();

    let uri = Url::parse("file:///test.cdm").unwrap();
    let text = r#"User {
  name: string #1
} #10
"#.to_string();

    server.did_open(DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "cdm".to_string(),
            version: 1,
            text,
        },
    }).await;

    // Hover on "string" (line 1, character 10)
    let hover_params = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri },
            position: Position { line: 1, character: 10 },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
    };

    let result = server.hover(hover_params).await.unwrap();
    assert!(result.is_some());

    let hover = result.unwrap();
    if let HoverContents::Markup(content) = hover.contents {
        assert!(content.value.contains("string"));
        assert!(content.value.contains("Built-in type"));
    } else {
        panic!("Expected markup content");
    }
}

#[tokio::test]
async fn test_hover_on_unopened_document() {
    
    let server = create_test_server();

    let uri = Url::parse("file:///nonexistent.cdm").unwrap();

    let hover_params = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri },
            position: Position { line: 0, character: 0 },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
    };

    let result = server.hover(hover_params).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_goto_definition() {
    
    let server = create_test_server();

    let uri = Url::parse("file:///test.cdm").unwrap();
    let text = r#"Email: string #1

User {
  email_addr: Email #2
} #10
"#.to_string();

    server.did_open(DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "cdm".to_string(),
            version: 1,
            text,
        },
    }).await;

    // Go to definition of "Email" usage
    let params = GotoDefinitionParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position { line: 3, character: 14 },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };

    let result = server.goto_definition(params).await.unwrap();
    assert!(result.is_some());

    if let Some(GotoDefinitionResponse::Scalar(location)) = result {
        assert_eq!(location.uri, uri);
        // Should point to line 0 where Email is defined
        assert_eq!(location.range.start.line, 0);
    } else {
        panic!("Expected scalar location response");
    }
}

#[tokio::test]
async fn test_goto_definition_on_unopened_document() {
    
    let server = create_test_server();

    let uri = Url::parse("file:///nonexistent.cdm").unwrap();

    let params = GotoDefinitionParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri },
            position: Position { line: 0, character: 0 },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };

    let result = server.goto_definition(params).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_find_references() {
    
    let server = create_test_server();

    let uri = Url::parse("file:///test.cdm").unwrap();
    let text = r#"Email: string #1

User {
  email: Email #2
  backup_email: Email #3
} #10
"#.to_string();

    server.did_open(DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "cdm".to_string(),
            version: 1,
            text,
        },
    }).await;

    // Find references to "Email"
    let params = ReferenceParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position { line: 3, character: 9 },
        },
        context: ReferenceContext {
            include_declaration: true,
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };

    let result = server.references(params).await.unwrap();
    assert!(result.is_some());

    let locations = result.unwrap();
    // Should find 3 references: definition + 2 usages
    assert_eq!(locations.len(), 3);

    // All should be in the same file
    for location in &locations {
        assert_eq!(location.uri, uri);
    }
}

#[tokio::test]
async fn test_find_references_on_unopened_document() {
    
    let server = create_test_server();

    let uri = Url::parse("file:///nonexistent.cdm").unwrap();

    let params = ReferenceParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri },
            position: Position { line: 0, character: 0 },
        },
        context: ReferenceContext {
            include_declaration: true,
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
    };

    let result = server.references(params).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_completion_after_colon() {
    let server = create_test_server();

    let uri = Url::parse("file:///test.cdm").unwrap();
    let text = r#"Email: string #1

User {
  name:
} #10
"#.to_string();

    server.did_open(DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "cdm".to_string(),
            version: 1,
            text,
        },
    }).await;

    // Request completion after "name: "
    let params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri },
            position: Position { line: 3, character: 8 },
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
        partial_result_params: PartialResultParams::default(),
        context: None,
    };

    let result = server.completion(params).await.unwrap();
    assert!(result.is_some());

    if let Some(CompletionResponse::Array(items)) = result {
        // Should have built-in types + Email type alias
        assert!(items.len() >= 4);
        assert!(items.iter().any(|i| i.label == "string"));
        assert!(items.iter().any(|i| i.label == "Email"));
    } else {
        panic!("Expected array of completion items");
    }
}

#[tokio::test]
async fn test_formatting_with_bad_whitespace() {
    use std::io::Write;

    let server = create_test_server();

    // Create a real temporary file
    let mut temp_file = tempfile::Builder::new()
        .suffix(".cdm")
        .tempfile()
        .unwrap();

    let text = r#"Email:string#1

User{
id:string#1
}#10
"#;
    write!(temp_file, "{}", text).unwrap();
    temp_file.flush().unwrap();

    let uri = Url::from_file_path(temp_file.path()).unwrap();

    server.did_open(DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "cdm".to_string(),
            version: 1,
            text: text.to_string(),
        },
    }).await;

    let params = DocumentFormattingParams {
        text_document: TextDocumentIdentifier { uri },
        options: FormattingOptions {
            tab_size: 2,
            insert_spaces: true,
            ..Default::default()
        },
        work_done_progress_params: WorkDoneProgressParams::default(),
    };

    let result = server.formatting(params).await.unwrap();
    assert!(result.is_some());

    let edits = result.unwrap();
    assert_eq!(edits.len(), 1);

    // Should have proper spacing
    let formatted = &edits[0].new_text;
    assert!(formatted.contains("Email: string #1"));
    assert!(formatted.contains("User {"));
    assert!(formatted.contains("  id: string #1"));
}

#[tokio::test]
async fn test_prepare_rename_type_alias() {
    let server = create_test_server();

    let uri = Url::parse("file:///test.cdm").unwrap();
    let text = r#"Email: string #1

User {
  email: Email #1
} #10"#;

    server.did_open(DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "cdm".to_string(),
            version: 1,
            text: text.to_string(),
        },
    }).await;

    // Prepare rename on "Email" in definition
    let params = TextDocumentPositionParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        position: Position { line: 0, character: 2 },
    };

    let result = server.prepare_rename(params).await.unwrap();
    assert!(result.is_some(), "Should be able to prepare rename for type alias");
}

#[tokio::test]
async fn test_rename_type_alias_server() {
    let server = create_test_server();

    let uri = Url::parse("file:///test.cdm").unwrap();
    let text = r#"Email: string #1

User {
  email: Email #1
} #10"#;

    server.did_open(DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "cdm".to_string(),
            version: 1,
            text: text.to_string(),
        },
    }).await;

    // Rename "Email" to "EmailAddress"
    let params = RenameParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position { line: 0, character: 2 },
        },
        new_name: "EmailAddress".to_string(),
        work_done_progress_params: WorkDoneProgressParams::default(),
    };

    let result = server.rename(params).await.unwrap();
    assert!(result.is_some(), "Rename should return WorkspaceEdit");

    let edit = result.unwrap();
    assert!(edit.changes.is_some(), "WorkspaceEdit should have changes");

    let changes = edit.changes.unwrap();
    let text_edits = changes.get(&uri).unwrap();

    // Should find 2 occurrences
    assert_eq!(text_edits.len(), 2, "Should find 2 occurrences of Email");

    // Check that all edits have the new name
    for text_edit in text_edits {
        assert_eq!(text_edit.new_text, "EmailAddress");
    }
}

#[tokio::test]
async fn test_document_lifecycle_full_flow() {
    
    let server = create_test_server();

    let uri = Url::parse("file:///test.cdm").unwrap();

    // 1. Open document
    let initial_text = "User { name: string #1 } #10".to_string();
    server.did_open(DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "cdm".to_string(),
            version: 1,
            text: initial_text.clone(),
        },
    }).await;
    assert_eq!(server.documents.get(&uri), Some(initial_text));

    // 2. Change document
    let updated_text = "User { name: string #1, email: string #2 } #10".to_string();
    server.did_change(DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: updated_text.clone(),
        }],
    }).await;
    assert_eq!(server.documents.get(&uri), Some(updated_text.clone()));

    // 3. Save document
    server.did_save(DidSaveTextDocumentParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        text: None,
    }).await;
    assert_eq!(server.documents.get(&uri), Some(updated_text));

    // 4. Close document
    server.did_close(DidCloseTextDocumentParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
    }).await;
    assert!(server.documents.get(&uri).is_none());
}
