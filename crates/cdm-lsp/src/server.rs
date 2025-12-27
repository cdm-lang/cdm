use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

mod document;
mod position;
mod diagnostics;
mod navigation;
mod completion;
mod formatting;
mod workspace;

use document::DocumentStore;
use workspace::Workspace;

/// The CDM Language Server
#[derive(Clone)]
pub struct CdmLanguageServer {
    client: Client,
    documents: DocumentStore,
    workspace: Workspace,
}

impl CdmLanguageServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: DocumentStore::new(),
            workspace: Workspace::new(),
        }
    }

    /// Publish diagnostics for a document
    async fn publish_diagnostics(&self, uri: &Url) {
        if let Some(text) = self.documents.get(uri) {
            let diagnostics = diagnostics::compute_diagnostics(&text, uri);
            self.client.publish_diagnostics(uri.clone(), diagnostics, None).await;
        }
    }

    /// Re-validate all files that depend on the given file
    async fn revalidate_dependents(&self, uri: &Url) {
        let dependents = self.workspace.get_all_dependents(uri);

        for dependent_uri in dependents {
            self.publish_diagnostics(&dependent_uri).await;
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for CdmLanguageServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        eprintln!("Initializing CDM Language Server");
        eprintln!("  Root URI: {:?}", params.root_uri);
        eprintln!("  Client: {:?}", params.client_info);

        // Set workspace root if available
        if let Some(root_uri) = params.root_uri.clone() {
            self.workspace.set_root(root_uri);
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                            include_text: Some(false),
                        })),
                        ..Default::default()
                    },
                )),
                // Navigation features
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                // Completion
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![":".to_string(), " ".to_string()]),
                    ..Default::default()
                }),
                // Formatting
                document_formatting_provider: Some(OneOf::Left(true)),
                // Future features to be implemented:
                // - document_symbol_provider
                // - rename_provider
                // - code_action_provider
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "cdm-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        eprintln!("CDM Language Server initialized");
        self.client
            .log_message(MessageType::INFO, "CDM Language Server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        eprintln!("Shutting down CDM Language Server");
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        eprintln!("Document opened: {}", params.text_document.uri);

        let uri = params.text_document.uri;
        let text = params.text_document.text;

        self.documents.insert(uri.clone(), text.clone());
        self.workspace.update_document(uri.clone(), text);

        // Publish diagnostics for this file
        self.publish_diagnostics(&uri).await;

        // Re-validate dependent files
        self.revalidate_dependents(&uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        eprintln!("Document changed: {}", params.text_document.uri);

        let uri = params.text_document.uri;

        // We use FULL sync, so there's only one change with the full text
        if let Some(change) = params.content_changes.into_iter().next() {
            self.documents.insert(uri.clone(), change.text.clone());
            self.workspace.update_document(uri.clone(), change.text);

            // Publish diagnostics for this file
            self.publish_diagnostics(&uri).await;

            // Re-validate dependent files
            self.revalidate_dependents(&uri).await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        eprintln!("Document saved: {}", params.text_document.uri);

        // Re-validate on save
        self.publish_diagnostics(&params.text_document.uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        eprintln!("Document closed: {}", params.text_document.uri);

        let uri = params.text_document.uri;
        self.documents.remove(&uri);
        self.workspace.remove_document(&uri);

        // Clear diagnostics
        self.client.publish_diagnostics(uri, vec![], None).await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        eprintln!("Hover request at {:?} in {}", position, uri);

        // Get the document text
        let text = match self.documents.get(uri) {
            Some(t) => t,
            None => return Ok(None),
        };

        // Find the symbol at the cursor position
        let (symbol, _range) = match navigation::find_symbol_at_position(&text, position) {
            Some(s) => s,
            None => return Ok(None),
        };

        // Get all definitions in the document
        let definitions = navigation::extract_definitions(&text);

        // Find the definition for this symbol
        if let Some((_, def_info)) = definitions.iter().find(|(name, _)| name == &symbol) {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: def_info.hover_text.clone(),
                }),
                range: None,
            }));
        }

        // Check if it's a built-in type
        if cdm::is_builtin_type(&symbol) {
            let hover_text = format!("```cdm\n{}\n```\n\nBuilt-in type", symbol);
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: hover_text,
                }),
                range: None,
            }));
        }

        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        eprintln!("Completion request at {:?} in {}", position, uri);

        // Get the document text
        let text = match self.documents.get(uri) {
            Some(t) => t,
            None => return Ok(None),
        };

        // Compute completions
        let completions = completion::compute_completions(&text, position);

        Ok(completions.map(CompletionResponse::Array))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        eprintln!("Go-to-definition request at {:?} in {}", position, uri);

        // Get the document text
        let text = match self.documents.get(uri) {
            Some(t) => t,
            None => return Ok(None),
        };

        // Find the symbol at the cursor position
        let (symbol, _range) = match navigation::find_symbol_at_position(&text, position) {
            Some(s) => s,
            None => return Ok(None),
        };

        // Get all definitions in the document
        let definitions = navigation::extract_definitions(&text);

        // Find the definition for this symbol
        if let Some((_, def_info)) = definitions.iter().find(|(name, _)| name == &symbol) {
            return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                uri: uri.clone(),
                range: def_info.range,
            })));
        }

        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let include_declaration = params.context.include_declaration;

        eprintln!("References request at {:?} in {} (include_declaration: {})",
                  position, uri, include_declaration);

        // Get the document text
        let text = match self.documents.get(uri) {
            Some(t) => t,
            None => return Ok(None),
        };

        // Find the symbol at the cursor position
        let (symbol, _range) = match navigation::find_symbol_at_position(&text, position) {
            Some(s) => s,
            None => return Ok(None),
        };

        // Find all references to this symbol
        let ranges = navigation::find_all_references(&text, &symbol);

        // Convert ranges to locations
        let locations: Vec<Location> = ranges
            .into_iter()
            .map(|range| Location {
                uri: uri.clone(),
                range,
            })
            .collect();

        if locations.is_empty() {
            Ok(None)
        } else {
            Ok(Some(locations))
        }
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = &params.text_document.uri;

        eprintln!("Formatting request for {}", uri);

        // Get the document text
        let text = match self.documents.get(uri) {
            Some(t) => t,
            None => return Ok(None),
        };

        // Format the document
        let edits = formatting::format_document(&text, uri);

        Ok(edits)
    }
}

#[cfg(test)]
mod tests {
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
}
