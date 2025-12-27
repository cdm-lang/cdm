use tower_lsp::{LspService, Server};

mod server;

#[tokio::main]
async fn main() {
    // Set up logging to stderr (LSP uses stdout for JSON-RPC)
    eprintln!("Starting CDM Language Server...");

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| server::CdmLanguageServer::new(client));

    Server::new(stdin, stdout, socket).serve(service).await;

    eprintln!("CDM Language Server stopped.");
}
