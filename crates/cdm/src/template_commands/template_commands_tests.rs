use super::*;

#[test]
fn test_is_template_cached_returns_false_for_nonexistent() {
    use tempfile::tempdir;

    // Use an empty temp directory as the cache
    let temp_cache = tempdir().expect("Failed to create temp dir");

    // Non-existent template should return false
    let result = is_template_cached("nonexistent-template", "1.0.0", Some(temp_cache.path()));
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

#[test]
fn test_list_cached_templates_empty() {
    use tempfile::tempdir;

    // Use an empty temp directory as the cache
    let temp_cache = tempdir().expect("Failed to create temp dir");

    // When no templates are cached, should return empty list
    let result = list_cached_templates(Some(temp_cache.path()));
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[test]
fn test_cache_template_cmd_requires_name_or_all() {
    // Should error when neither name nor --all is provided
    let result = cache_template_cmd(None, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Must specify"));
}

#[test]
fn test_extract_base_template_name_simple() {
    assert_eq!(extract_base_template_name("sql-types"), "sql-types");
}

#[test]
fn test_extract_base_template_name_with_subpath() {
    assert_eq!(extract_base_template_name("sql-types/postgres"), "sql-types");
}

#[test]
fn test_extract_base_template_name_with_subpath_and_extension() {
    assert_eq!(extract_base_template_name("sql-types/postgres.cdm"), "sql-types");
}

#[test]
fn test_extract_base_template_name_with_nested_subpath() {
    assert_eq!(extract_base_template_name("sql-types/postgres/v2"), "sql-types");
}

#[test]
fn test_extract_base_template_name_scoped() {
    // Scoped names without subpath are kept as-is
    assert_eq!(extract_base_template_name("cdm/auth"), "cdm/auth");
}

#[test]
fn test_extract_base_template_name_scoped_with_subpath() {
    assert_eq!(extract_base_template_name("cdm/auth/types"), "cdm/auth");
}

#[test]
fn test_extract_base_template_name_scoped_with_nested_subpath() {
    assert_eq!(extract_base_template_name("cdm/auth/types/user"), "cdm/auth");
}

#[test]
fn test_list_cached_templates_finds_templates_without_metadata() {
    use tempfile::tempdir;

    // Create a temp directory for the cache
    let temp_cache = tempdir().expect("Failed to create temp dir");
    let cache_path = temp_cache.path();

    // Create a template directory structure without metadata
    // This simulates what happens when template_resolver.rs caches a template
    let templates_dir = cache_path.join("templates");
    let template_dir = templates_dir.join("test-template@1.0.0");
    std::fs::create_dir_all(&template_dir).expect("Failed to create template dir");

    // Create a cdm-template.json in the template directory
    let manifest = serde_json::json!({
        "name": "test-template",
        "version": "1.0.0",
        "files": []
    });
    std::fs::write(
        template_dir.join("cdm-template.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .expect("Failed to write manifest");

    // List cached templates - should find the template even without metadata
    let result = list_cached_templates(Some(cache_path));
    assert!(result.is_ok(), "list_cached_templates should succeed");

    let cached = result.unwrap();
    assert!(
        cached.iter().any(|(name, version, _)| name == "test-template" && version == "1.0.0"),
        "Should find test-template@1.0.0 in cached list, got: {:?}",
        cached
    );
}

#[test]
fn test_list_cached_templates_prefers_metadata_over_directory_scan() {
    use tempfile::tempdir;

    // Create a temp directory for the cache
    let temp_cache = tempdir().expect("Failed to create temp dir");
    let cache_path = temp_cache.path();

    // Create a template directory
    let templates_dir = cache_path.join("templates");
    let template_dir = templates_dir.join("my-template@2.0.0");
    let inner_dir = template_dir.join("my-template-2.0.0"); // Simulates tar extraction subdirectory
    std::fs::create_dir_all(&inner_dir).expect("Failed to create template dir");

    // Create a cdm-template.json in a subdirectory (like tar extraction)
    let manifest = serde_json::json!({
        "name": "my-template",
        "version": "2.0.0",
        "files": []
    });
    std::fs::write(
        inner_dir.join("cdm-template.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .expect("Failed to write manifest");

    // Also create metadata for this template
    let metadata_dir = cache_path.join("template_metadata").join("my-template");
    std::fs::create_dir_all(&metadata_dir).expect("Failed to create metadata dir");

    let metadata = serde_json::json!({
        "name": "my-template",
        "version": "2.0.0",
        "download_url": "https://example.com/template.tar.gz",
        "checksum": "sha256:abc123",
        "cached_at": 1234567890u64,
        "template_path": inner_dir.to_string_lossy()
    });
    std::fs::write(
        metadata_dir.join("2.0.0.json"),
        serde_json::to_string_pretty(&metadata).unwrap(),
    )
    .expect("Failed to write metadata");

    // List cached templates - should find exactly one entry (from metadata)
    let result = list_cached_templates(Some(cache_path));
    assert!(result.is_ok(), "list_cached_templates should succeed");

    let cached = result.unwrap();
    let matching: Vec<_> = cached
        .iter()
        .filter(|(name, version, _)| name == "my-template" && version == "2.0.0")
        .collect();

    assert_eq!(
        matching.len(),
        1,
        "Should find exactly one my-template@2.0.0 entry (no duplicates), got: {:?}",
        cached
    );
}

#[test]
fn test_is_template_cached_returns_true_for_cached_template() {
    use tempfile::tempdir;

    // Create a temp directory for the cache
    let temp_cache = tempdir().expect("Failed to create temp dir");
    let cache_path = temp_cache.path();

    // Create a template directory with cdm-template.json
    let templates_dir = cache_path.join("templates");
    let template_dir = templates_dir.join("my-template@1.0.0");
    std::fs::create_dir_all(&template_dir).expect("Failed to create template dir");

    let manifest = serde_json::json!({
        "name": "my-template",
        "version": "1.0.0",
        "files": []
    });
    std::fs::write(
        template_dir.join("cdm-template.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .expect("Failed to write manifest");

    // Check if template is cached - should return true
    let result = is_template_cached("my-template", "1.0.0", Some(cache_path));
    assert!(result.is_ok());
    assert!(result.unwrap(), "Template should be detected as cached");
}
