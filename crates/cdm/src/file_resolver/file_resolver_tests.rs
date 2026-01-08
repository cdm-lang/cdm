use super::*;

fn fixtures_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_fixtures")
        .join("file_resolver")
}

#[test]
fn test_load_single_file() {
    let file_path = fixtures_path().join("single_file/simple.cdm");
    let result = FileResolver::load(&file_path);

    assert!(result.is_ok(), "Failed to load: {:?}", result.err());

    let tree = result.unwrap();
    let source = tree.main.source().expect("Failed to read source");
    assert!(source.contains("User"));
    assert_eq!(tree.ancestors.len(), 0);
}

#[test]
fn test_load_with_single_extends() {
    let file_path = fixtures_path().join("single_extends/child.cdm");
    let result = FileResolver::load(&file_path);

    assert!(result.is_ok(), "Failed to load: {:?}", result.err());

    let tree = result.unwrap();
    let source = tree.main.source().expect("Failed to read source");
    assert!(source.contains("PublicUser"));
    assert_eq!(tree.ancestors.len(), 1);

    let ancestor_source = tree.ancestors[0].source().expect("Failed to read ancestor source");
    assert!(ancestor_source.contains("User"));
}

#[test]
fn test_load_with_multiple_extends() {
    let file_path = fixtures_path().join("multiple_extends/child.cdm");
    let result = FileResolver::load(&file_path);

    assert!(result.is_ok(), "Failed to load: {:?}", result.err());

    let tree = result.unwrap();
    let source = tree.main.source().expect("Failed to read source");
    assert!(source.contains("User"));
    assert_eq!(tree.ancestors.len(), 2); // types.cdm and mixins.cdm
}

#[test]
fn test_load_nested_chain() {
    let file_path = fixtures_path().join("nested_chain/mobile.cdm");
    let result = FileResolver::load(&file_path);

    assert!(result.is_ok(), "Failed to load: {:?}", result.err());

    let tree = result.unwrap();
    let source = tree.main.source().expect("Failed to read source");
    assert!(source.contains("MobileUser"));
    // Should have client.cdm and base.cdm (client's ancestor)
    assert_eq!(tree.ancestors.len(), 2);
}

#[test]
fn test_load_circular_detected() {
    let file_path = fixtures_path().join("circular/a.cdm");
    let result = FileResolver::load(&file_path);

    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors[0].message.contains("Circular extends detected"));
}

#[test]
fn test_load_file_not_found() {
    let file_path = fixtures_path().join("invalid/missing_extends.cdm");
    let result = FileResolver::load(&file_path);

    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors[0].message.contains("Failed to read file"));
}

#[test]
fn test_loaded_file_lazy_loading() {
    let file_path = fixtures_path().join("single_file/simple.cdm");
    let loaded_file = LoadedFile::new_for_test(file_path);

    // Source should not be cached initially
    assert!(loaded_file.cached_source.borrow().is_none());

    // First access should read and cache
    let source1 = loaded_file.source().expect("Failed to read source");
    assert!(loaded_file.cached_source.borrow().is_some());

    // Second access should use cache (no re-read)
    let source2 = loaded_file.source().expect("Failed to read cached source");
    assert_eq!(source1, source2);
}

#[test]
fn test_file_resolver_default() {
    let resolver1 = FileResolver::default();
    let resolver2 = FileResolver::new();

    // Both should create empty resolvers
    assert_eq!(resolver1.loaded_files.len(), 0);
    assert_eq!(resolver2.loaded_files.len(), 0);
}

#[test]
fn test_loaded_file_debug() {
    let file_path = PathBuf::from("test.cdm");
    let loaded_file = LoadedFile::new_for_test(file_path.clone());

    let debug_output = format!("{:?}", loaded_file);
    assert!(debug_output.contains("LoadedFile"));
    assert!(debug_output.contains("test.cdm"));
}

#[test]
fn test_loaded_file_tree_debug() {
    let main = LoadedFile::new_for_test(PathBuf::from("main.cdm"));
    let ancestor = LoadedFile::new_for_test(PathBuf::from("base.cdm"));

    let tree = LoadedFileTree {
        main,
        ancestors: vec![ancestor],
    };

    let debug_output = format!("{:?}", tree);
    assert!(debug_output.contains("LoadedFileTree"));
    assert!(debug_output.contains("main.cdm"));
    assert!(debug_output.contains("base.cdm"));
}

#[test]
fn test_resolve_path_same_directory() {
    let resolver = FileResolver::new();
    let current_file = Path::new("/path/to/schema.cdm");
    let extends_path = "./types.cdm";

    let resolved = resolver.resolve_path(current_file, extends_path);
    assert_eq!(resolved, PathBuf::from("/path/to/types.cdm"));
}

#[test]
fn test_resolve_path_parent_directory() {
    let resolver = FileResolver::new();
    let current_file = Path::new("/path/to/schema.cdm");
    let extends_path = "../shared/base.cdm";

    let resolved = resolver.resolve_path(current_file, extends_path);
    // resolve_path doesn't normalize paths, it just joins them
    assert_eq!(resolved, PathBuf::from("/path/to/../shared/base.cdm"));
}

#[test]
fn test_resolve_path_multiple_levels_up() {
    let resolver = FileResolver::new();
    let current_file = Path::new("/path/to/deep/schema.cdm");
    let extends_path = "../../common/types.cdm";

    let resolved = resolver.resolve_path(current_file, extends_path);
    // resolve_path doesn't normalize paths, it just joins them
    assert_eq!(resolved, PathBuf::from("/path/to/deep/../../common/types.cdm"));
}

#[test]
fn test_resolve_path_subdirectory() {
    let resolver = FileResolver::new();
    let current_file = Path::new("/path/to/schema.cdm");
    let extends_path = "./models/user.cdm";

    let resolved = resolver.resolve_path(current_file, extends_path);
    assert_eq!(resolved, PathBuf::from("/path/to/models/user.cdm"));
}

#[test]
fn test_to_absolute_path_nonexistent() {
    let result = FileResolver::to_absolute_path(Path::new("/nonexistent/path/file.cdm"));

    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors[0].message.contains("Failed to resolve path"));
}

#[test]
fn test_circular_dependency_error_details() {
    let file_path = fixtures_path().join("circular/a.cdm");
    let result = FileResolver::load(&file_path);

    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].severity, Severity::Error);
    assert!(errors[0].message.contains("Circular"));
    assert!(errors[0].message.contains("extends"));
}

#[test]
fn test_missing_file_error_details() {
    let file_path = fixtures_path().join("invalid/missing_extends.cdm");
    let result = FileResolver::load(&file_path);

    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].severity, Severity::Error);
    assert!(errors[0].message.contains("Failed to read file"));
    assert!(errors[0].message.contains("No such file or directory"));
}

#[test]
fn test_loaded_file_source_io_error() {
    // Create a LoadedFile pointing to a nonexistent file
    let loaded_file = LoadedFile::new_for_test(PathBuf::from("/nonexistent/file.cdm"));

    let result = loaded_file.source();
    assert!(result.is_err());
}

#[test]
fn test_ancestor_order_depth_first() {
    // In a chain A -> B -> C, ancestors should be [C, B] (depth-first)
    let file_path = fixtures_path().join("nested_chain/mobile.cdm");
    let result = FileResolver::load(&file_path);

    assert!(result.is_ok());
    let tree = result.unwrap();

    // Verify we have the right number of ancestors
    assert_eq!(tree.ancestors.len(), 2);

    // First ancestor should be the deepest one (base.cdm contains "User")
    let first_source = tree.ancestors[0].source().expect("Failed to read first ancestor");
    assert!(first_source.contains("User") && !first_source.contains("ClientUser"));

    // Second ancestor should be the immediate parent (client.cdm contains "ClientUser")
    let second_source = tree.ancestors[1].source().expect("Failed to read second ancestor");
    assert!(second_source.contains("ClientUser"));
}

#[test]
fn test_multiple_extends_all_loaded() {
    let file_path = fixtures_path().join("multiple_extends/child.cdm");
    let result = FileResolver::load(&file_path);

    assert!(result.is_ok());
    let tree = result.unwrap();

    // Should have 2 ancestors (types.cdm and mixins.cdm)
    assert_eq!(tree.ancestors.len(), 2);

    // Verify both ancestors are loaded
    for ancestor in &tree.ancestors {
        let source = ancestor.source().expect("Failed to read ancestor");
        assert!(!source.is_empty());
    }
}

#[test]
fn test_loaded_file_path_preserved() {
    let original_path = PathBuf::from("/some/path/test.cdm");
    let loaded_file = LoadedFile::new_for_test(original_path.clone());

    assert_eq!(loaded_file.path, original_path);
}

#[test]
fn test_file_resolver_tracks_loaded_files() {
    let mut resolver = FileResolver::new();
    let file_path = fixtures_path().join("single_file/simple.cdm");

    // Initially empty
    assert_eq!(resolver.loaded_files.len(), 0);

    // After loading, should contain the file
    let _ = resolver.load_single_file(&file_path);
    assert!(resolver.loaded_files.contains(&file_path));
}
