use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tower_lsp::lsp_types::Url;

/// Thread-safe document storage
#[derive(Clone)]
pub struct DocumentStore {
    documents: Arc<RwLock<HashMap<Url, String>>>,
}

impl DocumentStore {
    pub fn new() -> Self {
        Self {
            documents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Insert or update a document
    pub fn insert(&self, uri: Url, text: String) {
        let mut docs = self.documents.write().unwrap();
        docs.insert(uri, text);
    }

    /// Get a document's text
    pub fn get(&self, uri: &Url) -> Option<String> {
        let docs = self.documents.read().unwrap();
        docs.get(uri).cloned()
    }

    /// Remove a document
    pub fn remove(&self, uri: &Url) {
        let mut docs = self.documents.write().unwrap();
        docs.remove(uri);
    }

    /// Check if a document exists
    #[allow(dead_code)]
    pub fn contains(&self, uri: &Url) -> bool {
        let docs = self.documents.read().unwrap();
        docs.contains_key(uri)
    }

    /// Get all document URIs
    pub fn all_uris(&self) -> Vec<Url> {
        let docs = self.documents.read().unwrap();
        docs.keys().cloned().collect()
    }
}


#[cfg(test)]
#[path = "document/document_tests.rs"]
mod document_tests;
