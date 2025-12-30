use super::*;

#[test]
fn test_lsp_service_creation() {
    // Test that we can create the LSP service without panicking
    let (service, _socket) = LspService::new(|client| server::CdmLanguageServer::new(client));

    // The service should be created successfully
    drop(service);
}

#[test]
fn test_server_creation_via_lsp_service() {
    // Test server creation indirectly through LspService
    let (_service, _socket) = LspService::new(|client| {
        // Create the server
        let server = server::CdmLanguageServer::new(client);
        // Verify it was created
        server
    });
}
