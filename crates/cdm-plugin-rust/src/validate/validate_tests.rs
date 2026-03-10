use super::*;
use serde_json::json;

#[test]
fn test_valid_rust_identifiers() {
    assert!(is_valid_rust_identifier("foo"));
    assert!(is_valid_rust_identifier("_bar"));
    assert!(is_valid_rust_identifier("baz123"));
    assert!(is_valid_rust_identifier("my_type"));
    assert!(is_valid_rust_identifier("UserProfile"));
}

#[test]
fn test_invalid_rust_identifiers() {
    assert!(!is_valid_rust_identifier(""));
    assert!(!is_valid_rust_identifier("123abc"));
    assert!(!is_valid_rust_identifier("foo-bar"));
    assert!(!is_valid_rust_identifier("foo bar"));
    assert!(!is_valid_rust_identifier("$foo"));
}

#[test]
fn test_rust_reserved_keywords() {
    assert!(!is_valid_rust_identifier("struct"));
    assert!(!is_valid_rust_identifier("enum"));
    assert!(!is_valid_rust_identifier("fn"));
    assert!(!is_valid_rust_identifier("let"));
    assert!(!is_valid_rust_identifier("mut"));
    assert!(!is_valid_rust_identifier("pub"));
    assert!(!is_valid_rust_identifier("self"));
    assert!(!is_valid_rust_identifier("type"));
    assert!(!is_valid_rust_identifier("async"));
    assert!(!is_valid_rust_identifier("await"));
}

#[test]
fn test_valid_global_config() {
    let config = json!({
        "file_strategy": "single",
        "single_file_name": "types.rs",
        "number_type": "f64",
        "map_type": "HashMap",
        "visibility": "pub"
    });
    let utils = Utils;
    let errors = validate_config(ConfigLevel::Global, config, &utils);
    assert!(errors.is_empty());
}

#[test]
fn test_invalid_file_strategy() {
    let config = json!({ "file_strategy": "invalid" });
    let utils = Utils;
    let errors = validate_config(ConfigLevel::Global, config, &utils);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("file_strategy"));
}

#[test]
fn test_invalid_single_file_name() {
    let config = json!({ "single_file_name": "types.txt" });
    let utils = Utils;
    let errors = validate_config(ConfigLevel::Global, config, &utils);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains(".rs"));
}

#[test]
fn test_invalid_number_type() {
    let config = json!({ "number_type": "int" });
    let utils = Utils;
    let errors = validate_config(ConfigLevel::Global, config, &utils);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("number_type"));
}

#[test]
fn test_invalid_map_type() {
    let config = json!({ "map_type": "LinkedHashMap" });
    let utils = Utils;
    let errors = validate_config(ConfigLevel::Global, config, &utils);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("map_type"));
}

#[test]
fn test_invalid_visibility() {
    let config = json!({ "visibility": "protected" });
    let utils = Utils;
    let errors = validate_config(ConfigLevel::Global, config, &utils);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("visibility"));
}

#[test]
fn test_invalid_type_name_format() {
    let config = json!({ "type_name_format": "screamingSnake" });
    let utils = Utils;
    let errors = validate_config(ConfigLevel::Global, config, &utils);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("type_name_format"));
}

#[test]
fn test_invalid_field_name_format() {
    let config = json!({ "field_name_format": "UPPER" });
    let utils = Utils;
    let errors = validate_config(ConfigLevel::Global, config, &utils);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("field_name_format"));
}

#[test]
fn test_valid_model_config() {
    let config = json!({ "struct_name": "MyUser", "visibility": "pub_crate" });
    let utils = Utils;
    let errors = validate_config(
        ConfigLevel::Model {
            name: "User".to_string(),
        },
        config,
        &utils,
    );
    assert!(errors.is_empty());
}

#[test]
fn test_invalid_struct_name_keyword() {
    let config = json!({ "struct_name": "struct" });
    let utils = Utils;
    let errors = validate_config(
        ConfigLevel::Model {
            name: "User".to_string(),
        },
        config,
        &utils,
    );
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("not a valid Rust identifier"));
}

#[test]
fn test_invalid_field_name_override() {
    let config = json!({ "field_name": "fn" });
    let utils = Utils;
    let errors = validate_config(
        ConfigLevel::Field {
            model: "User".to_string(),
            field: "name".to_string(),
        },
        config,
        &utils,
    );
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("not a valid Rust identifier"));
}

#[test]
fn test_valid_type_alias_config() {
    let config = json!({ "export_name": "EmailAddress", "skip": false });
    let utils = Utils;
    let errors = validate_config(
        ConfigLevel::TypeAlias {
            name: "Email".to_string(),
        },
        config,
        &utils,
    );
    assert!(errors.is_empty());
}

#[test]
fn test_invalid_type_alias_export_name() {
    let config = json!({ "export_name": "123invalid" });
    let utils = Utils;
    let errors = validate_config(
        ConfigLevel::TypeAlias {
            name: "Email".to_string(),
        },
        config,
        &utils,
    );
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("not a valid Rust identifier"));
}

#[test]
fn test_field_visibility_validation() {
    let config = json!({ "visibility": "internal" });
    let utils = Utils;
    let errors = validate_config(
        ConfigLevel::Field {
            model: "User".to_string(),
            field: "name".to_string(),
        },
        config,
        &utils,
    );
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("visibility"));
}

#[test]
fn test_model_visibility_validation() {
    let config = json!({ "visibility": "internal" });
    let utils = Utils;
    let errors = validate_config(
        ConfigLevel::Model {
            name: "User".to_string(),
        },
        config,
        &utils,
    );
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("visibility"));
}

#[test]
fn test_empty_config_valid() {
    let config = json!({});
    let utils = Utils;

    let errors = validate_config(ConfigLevel::Global, config.clone(), &utils);
    assert!(errors.is_empty());

    let errors = validate_config(
        ConfigLevel::Model {
            name: "User".to_string(),
        },
        config.clone(),
        &utils,
    );
    assert!(errors.is_empty());

    let errors = validate_config(
        ConfigLevel::Field {
            model: "User".to_string(),
            field: "name".to_string(),
        },
        config.clone(),
        &utils,
    );
    assert!(errors.is_empty());

    let errors = validate_config(
        ConfigLevel::TypeAlias {
            name: "Email".to_string(),
        },
        config,
        &utils,
    );
    assert!(errors.is_empty());
}
