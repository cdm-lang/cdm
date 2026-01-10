use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

mod document;
mod position;
mod diagnostics;
mod navigation;
mod completion;
mod formatting;
mod workspace;
mod symbols;
mod rename;
mod code_actions;
mod folding;
mod semantic_tokens;
mod plugin_schema_cache;

pub use plugin_schema_cache::{PluginSchemaCache, PluginSettingsSchema, SettingsField};

use document::DocumentStore;
use workspace::Workspace;

/// The CDM Language Server
#[derive(Clone)]
pub struct CdmLanguageServer {
    client: Client,
    documents: DocumentStore,
    workspace: Workspace,
    plugin_schema_cache: PluginSchemaCache,
    assign_ids_on_save: std::sync::Arc<std::sync::RwLock<bool>>,
}

impl CdmLanguageServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: DocumentStore::new(),
            workspace: Workspace::new(),
            plugin_schema_cache: PluginSchemaCache::new(),
            assign_ids_on_save: std::sync::Arc::new(std::sync::RwLock::new(false)),
        }
    }

    /// Publish diagnostics for a document
    async fn publish_diagnostics(&self, uri: &Url) {
        if let Some(text) = self.documents.get(uri) {
            // Run compute_diagnostics in a blocking context because plugin validation
            // may trigger blocking HTTP requests (reqwest::blocking) to download plugins.
            // Running blocking code directly in an async context causes a panic when
            // the blocking runtime is dropped inside the async runtime.
            let text_clone = text.clone();
            let uri_clone = uri.clone();
            let diagnostics = tokio::task::spawn_blocking(move || {
                diagnostics::compute_diagnostics(&text_clone, &uri_clone)
            })
            .await
            .unwrap_or_default();
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

    /// Re-validate all open documents
    async fn revalidate_all_documents(&self) {
        let uris: Vec<Url> = self.documents.all_uris();
        for uri in uris {
            self.publish_diagnostics(&uri).await;
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for CdmLanguageServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        eprintln!("Initializing CDM Language Server v{}", env!("CARGO_PKG_VERSION"));
        eprintln!("  Root URI: {:?}", params.root_uri);
        eprintln!("  Client: {:?}", params.client_info);

        // Set workspace root if available
        if let Some(root_uri) = params.root_uri.clone() {
            self.workspace.set_root(root_uri);
        }

        // Read initialization options
        if let Some(init_options) = params.initialization_options {
            if let Some(assign_ids_on_save) = init_options.get("assignIdsOnSave").and_then(|v| v.as_bool()) {
                if let Ok(mut setting) = self.assign_ids_on_save.write() {
                    *setting = assign_ids_on_save;
                    eprintln!("  Assign IDs on save: {}", assign_ids_on_save);
                }
            }
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
                    trigger_characters: Some(vec![
                        ":".to_string(),
                        " ".to_string(),
                        "{".to_string(),  // Trigger after opening plugin config brace
                        ",".to_string(),  // Trigger after comma in plugin config
                    ]),
                    ..Default::default()
                }),
                // Formatting
                document_formatting_provider: Some(OneOf::Left(true)),
                // Document symbols
                document_symbol_provider: Some(OneOf::Left(true)),
                // Rename
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                })),
                // Code actions
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                // Execute command (for plugin refresh)
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["cdm.refreshPlugins".to_string()],
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                }),
                // Folding ranges
                folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
                // Semantic tokens
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: SemanticTokensLegend {
                                token_types: semantic_tokens::LEGEND_TYPE.to_vec(),
                                token_modifiers: semantic_tokens::LEGEND_MODIFIER.to_vec(),
                            },
                            range: Some(false),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            work_done_progress_options: WorkDoneProgressOptions::default(),
                        },
                    ),
                ),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "cdm".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        let version_msg = format!("CDM Language Server v{} initialized", env!("CARGO_PKG_VERSION"));
        eprintln!("{}", version_msg);
        self.client
            .log_message(MessageType::INFO, version_msg)
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
        if crate::is_builtin_type(&symbol) {
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

        // Clone data for spawn_blocking (plugin schema cache loading may trigger blocking HTTP)
        let text_clone = text.clone();
        let uri_clone = uri.clone();
        let cache_clone = self.plugin_schema_cache.clone();

        // Compute completions in blocking context because plugin schema loading
        // may trigger blocking HTTP requests
        let completions = tokio::task::spawn_blocking(move || {
            completion::compute_completions(
                &text_clone,
                position,
                Some(&cache_clone),
                Some(&uri_clone),
            )
        })
        .await
        .ok()
        .flatten();

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

        // Get the assign_ids_on_save setting
        let assign_ids = self.assign_ids_on_save.read()
            .map(|setting| *setting)
            .unwrap_or(false);

        // Format the document
        let edits = formatting::format_document(&text, uri, assign_ids);

        Ok(edits)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = &params.text_document.uri;

        eprintln!("Document symbol request for {}", uri);

        // Get the document text
        let text = match self.documents.get(uri) {
            Some(t) => t,
            None => return Ok(None),
        };

        // Compute document symbols
        let symbols = symbols::compute_document_symbols(&text);

        Ok(symbols.map(DocumentSymbolResponse::Nested))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = &params.text_document.uri;
        let position = params.position;

        eprintln!("Prepare rename request at {:?} in {}", position, uri);

        // Get the document text
        let text = match self.documents.get(uri) {
            Some(t) => t,
            None => return Ok(None),
        };

        // Prepare the rename
        let response = rename::prepare_rename(&text, position);

        Ok(response)
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let new_name = &params.new_name;

        eprintln!("Rename request at {:?} in {} to {}", position, uri, new_name);

        // Get the document text
        let text = match self.documents.get(uri) {
            Some(t) => t,
            None => return Ok(None),
        };

        // Perform the rename
        let edit = rename::rename_symbol(&text, position, new_name, uri);

        Ok(edit)
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = &params.text_document.uri;
        let range = params.range;

        eprintln!("Code action request for range {:?} in {}", range, uri);

        // Get the document text
        let text = match self.documents.get(uri) {
            Some(t) => t,
            None => return Ok(None),
        };

        // Get diagnostics from the params (provided by client) - these are at cursor position
        let cursor_diagnostics: Vec<Diagnostic> = params
            .context
            .diagnostics
            .iter()
            .cloned()
            .collect();

        // Also compute all diagnostics for the document to check for other missing plugins
        let text_clone = text.clone();
        let uri_clone = uri.clone();
        let all_diagnostics = tokio::task::spawn_blocking(move || {
            diagnostics::compute_diagnostics(&text_clone, &uri_clone)
        })
        .await
        .unwrap_or_default();

        // Compute code actions
        let actions = code_actions::compute_code_actions(&text, range, &cursor_diagnostics, &all_diagnostics, uri);

        Ok(actions)
    }

    async fn folding_range(&self, params: FoldingRangeParams) -> Result<Option<Vec<FoldingRange>>> {
        let uri = &params.text_document.uri;

        eprintln!("Folding range request for {}", uri);

        // Get the document text
        let text = match self.documents.get(uri) {
            Some(t) => t,
            None => return Ok(None),
        };

        // Compute folding ranges
        let ranges = folding::compute_folding_ranges(&text);

        Ok(ranges)
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = &params.text_document.uri;

        eprintln!("Semantic tokens request for {}", uri);

        // Get the document text
        let text = match self.documents.get(uri) {
            Some(t) => t,
            None => return Ok(None),
        };

        // Compute semantic tokens
        let tokens = semantic_tokens::compute_semantic_tokens(&text);

        Ok(tokens.map(|data| {
            SemanticTokensResult::Tokens(SemanticTokens {
                result_id: None,
                data,
            })
        }))
    }

    async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<serde_json::Value>> {
        eprintln!("Execute command: {}", params.command);

        match params.command.as_str() {
            "cdm.refreshPlugins" => {
                eprintln!("Refreshing plugins: clearing schema cache and revalidating documents");

                // Clear the in-memory plugin schema cache
                self.plugin_schema_cache.clear();

                // Re-validate all open documents to clear stale diagnostics
                self.revalidate_all_documents().await;

                self.client
                    .log_message(MessageType::INFO, "Plugin cache refreshed")
                    .await;

                Ok(None)
            }
            _ => {
                eprintln!("Unknown command: {}", params.command);
                Ok(None)
            }
        }
    }
}

/// Run the CDM Language Server
pub async fn run() {
    // Set up logging to stderr (LSP uses stdout for JSON-RPC)
    eprintln!("Starting CDM Language Server v{}...", env!("CARGO_PKG_VERSION"));

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(CdmLanguageServer::new);

    Server::new(stdin, stdout, socket).serve(service).await;
}

#[cfg(test)]
#[path = "lsp_tests.rs"]
mod lsp_tests;
