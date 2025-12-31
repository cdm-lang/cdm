use super::*;

#[test]
fn test_validate_empty_config() {
    let config = serde_json::json!({});
    let utils = Utils;

    let errors = validate_config(ConfigLevel::Global, config, &utils);
    assert!(errors.is_empty());
}

#[test]
fn test_validate_invalid_pattern() {
    let config = serde_json::json!({
        "pattern": "[invalid"
    });
    let utils = Utils;

    let errors = validate_config(
        ConfigLevel::Field {
            model: "User".to_string(),
            field: "email".to_string(),
        },
        config,
        &utils,
    );

    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("Invalid regex pattern"));
}

#[test]
fn test_validate_min_max_length() {
    let config = serde_json::json!({
        "min_length": 100,
        "max_length": 50
    });
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
    assert!(errors[0].message.contains("min_length"));
    assert!(errors[0].message.contains("max_length"));
}
