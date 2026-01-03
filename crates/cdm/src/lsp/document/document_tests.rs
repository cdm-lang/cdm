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
