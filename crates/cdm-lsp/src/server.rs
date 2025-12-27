use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

mod document;
mod position;
mod diagnostics;

use document::DocumentStore;

/// The CDM Language Server
pub struct CdmLanguageServer {
    client: Client,
    documents: DocumentStore,
}

impl CdmLanguageServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: DocumentStore::new(),
        }
    }

    /// Publish diagnostics for a document
    async fn publish_diagnostics(&self, uri: &Url) {
        if let Some(text) = self.documents.get(uri) {
            let diagnostics = diagnostics::compute_diagnostics(&text, uri);
            self.client.publish_diagnostics(uri.clone(), diagnostics, None).await;
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for CdmLanguageServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        eprintln!("Initializing CDM Language Server");
        eprintln!("  Root URI: {:?}", params.root_uri);
        eprintln!("  Client: {:?}", params.client_info);

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
                // Only advertise capabilities that are actually implemented
                // Future features will be enabled as they're implemented:
                // - hover_provider
                // - completion_provider
                // - definition_provider
                // - references_provider
                // - document_formatting_provider
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

        self.documents.insert(uri.clone(), text);
        self.publish_diagnostics(&uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        eprintln!("Document changed: {}", params.text_document.uri);

        let uri = params.text_document.uri;

        // We use FULL sync, so there's only one change with the full text
        if let Some(change) = params.content_changes.into_iter().next() {
            self.documents.insert(uri.clone(), change.text);
            self.publish_diagnostics(&uri).await;
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

        // Clear diagnostics
        self.client.publish_diagnostics(uri, vec![], None).await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        eprintln!("Hover request at {:?}", params.text_document_position_params.position);

        // TODO: Implement hover provider
        // For now, return None
        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        eprintln!("Completion request at {:?}", params.text_document_position.position);

        // TODO: Implement completion provider
        // For now, return None
        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        eprintln!("Go-to-definition request at {:?}", params.text_document_position_params.position);

        // TODO: Implement go-to-definition
        // For now, return None
        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        eprintln!("References request at {:?}", params.text_document_position.position);

        // TODO: Implement find references
        // For now, return None
        Ok(None)
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        eprintln!("Formatting request for {}", params.text_document.uri);

        // TODO: Implement document formatting
        // For now, return None
        Ok(None)
    }
}
