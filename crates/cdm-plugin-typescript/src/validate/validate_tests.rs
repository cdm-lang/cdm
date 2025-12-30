use super::*;
use serde_json::json;

#[test]
fn test_valid_typescript_identifiers() {
    assert!(is_valid_typescript_identifier("User"));
    assert!(is_valid_typescript_identifier("_private"));
    assert!(is_valid_typescript_identifier("$value"));
    assert!(is_valid_typescript_identifier("myVar123"));
}

#[test]
fn test_invalid_typescript_identifiers() {
    assert!(!is_valid_typescript_identifier(""));
    assert!(!is_valid_typescript_identifier("123abc"));
    assert!(!is_valid_typescript_identifier("my-var"));
    assert!(!is_valid_typescript_identifier("class")); // reserved keyword
    assert!(!is_valid_typescript_identifier("interface")); // reserved keyword
}

#[test]
fn test_validate_global_output_format() {
    let utils = Utils;
    let config = json!({ "output_format": "invalid" });
    let errors = validate_config(ConfigLevel::Global, config, &utils);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("output_format"));
}

#[test]
fn test_validate_global_file_strategy() {
    let utils = Utils;
    let config = json!({ "file_strategy": "invalid" });
    let errors = validate_config(ConfigLevel::Global, config, &utils);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("file_strategy"));
}

#[test]
fn test_validate_single_file_name_extension() {
    let utils = Utils;
    let config = json!({ "single_file_name": "types.js" });
    let errors = validate_config(ConfigLevel::Global, config, &utils);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains(".ts"));
}

#[test]
fn test_validate_model_export_name() {
    let utils = Utils;
    let config = json!({ "export_name": "123Invalid" });
    let errors = validate_config(
        ConfigLevel::Model {
            name: "User".to_string(),
        },
        config,
        &utils,
    );
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("not a valid TypeScript identifier"));
}
