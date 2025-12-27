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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_store() {
        let store = DocumentStore::new();
        let uri = Url::parse("file:///test.cdm").unwrap();

        // Insert document
        store.insert(uri.clone(), "Test content".to_string());
        assert!(store.contains(&uri));
        assert_eq!(store.get(&uri), Some("Test content".to_string()));

        // Update document
        store.insert(uri.clone(), "Updated content".to_string());
        assert_eq!(store.get(&uri), Some("Updated content".to_string()));

        // Remove document
        store.remove(&uri);
        assert!(!store.contains(&uri));
        assert_eq!(store.get(&uri), None);
    }
}
