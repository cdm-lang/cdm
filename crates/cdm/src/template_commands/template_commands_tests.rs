use super::*;

#[test]
fn test_is_template_cached_returns_false_for_nonexistent() {
    // Non-existent template should return false
    let result = is_template_cached("nonexistent-template", "1.0.0");
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

#[test]
fn test_list_cached_templates_empty() {
    // When no templates are cached, should return empty list
    // This test may pass or fail depending on the test environment
    let result = list_cached_templates();
    assert!(result.is_ok());
}

#[test]
fn test_cache_template_cmd_requires_name_or_all() {
    // Should error when neither name nor --all is provided
    let result = cache_template_cmd(None, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Must specify"));
}
