use super::*;
use serde_json::json;

#[test]
fn test_apply_name_format() {
    assert_eq!(apply_name_format("UserProfile", "snake_case"), "user_profile");
    assert_eq!(apply_name_format("UserProfile", "preserve"), "UserProfile");
    assert_eq!(apply_name_format("user_profile", "pascal_case"), "UserProfile");
    assert_eq!(apply_name_format("user_profile", "camel_case"), "userProfile");
}

#[test]
fn test_get_table_name_with_override() {
    let model_config = json!({ "table_name": "custom_users" });
    let global_config = json!({});

    let name = get_table_name("User", &model_config, &global_config);
    assert_eq!(name, "custom_users");
}

#[test]
fn test_get_table_name_with_formatting() {
    let model_config = json!({});
    let global_config = json!({
        "table_name_format": "snake_case",
        "pluralize_table_names": false  // Disable for this test
    });

    let name = get_table_name("UserProfile", &model_config, &global_config);
    assert_eq!(name, "user_profile");
}

#[test]
fn test_get_table_name_with_pluralization() {
    let model_config = json!({});
    let global_config = json!({
        "table_name_format": "snake_case",
        "pluralize_table_names": true
    });

    let name = get_table_name("User", &model_config, &global_config);
    assert_eq!(name, "users");
}

#[test]
fn test_get_table_name_with_pluralization_default() {
    let model_config = json!({});
    let global_config = json!({
        "table_name_format": "snake_case"
        // pluralize_table_names not specified, should default to true
    });

    let name = get_table_name("Category", &model_config, &global_config);
    assert_eq!(name, "categories");
}

#[test]
fn test_get_table_name_pluralization_disabled() {
    let model_config = json!({});
    let global_config = json!({
        "table_name_format": "snake_case",
        "pluralize_table_names": false
    });

    let name = get_table_name("User", &model_config, &global_config);
    assert_eq!(name, "user");
}

#[test]
fn test_get_table_name_pluralization_irregular() {
    let model_config = json!({});
    let global_config = json!({
        "table_name_format": "snake_case",
        "pluralize_table_names": true
    });

    let name = get_table_name("Person", &model_config, &global_config);
    assert_eq!(name, "people");
}

#[test]
fn test_get_table_name_override_ignores_pluralization() {
    let model_config = json!({ "table_name": "my_custom_table" });
    let global_config = json!({
        "table_name_format": "snake_case",
        "pluralize_table_names": true
    });

    let name = get_table_name("User", &model_config, &global_config);
    assert_eq!(name, "my_custom_table");
}

#[test]
fn test_get_column_name_with_override() {
    let field_config = json!({ "column_name": "custom_id" });
    let global_config = json!({});

    let name = get_column_name("userId", &field_config, &global_config);
    assert_eq!(name, "custom_id");
}

#[test]
fn test_get_column_name_with_formatting() {
    let field_config = json!({});
    let global_config = json!({ "column_name_format": "snake_case" });

    let name = get_column_name("firstName", &field_config, &global_config);
    assert_eq!(name, "first_name");
}

#[test]
fn test_quote_identifier() {
    assert_eq!(quote_identifier("user", Dialect::PostgreSQL), "\"user\"");
    assert_eq!(quote_identifier("user", Dialect::SQLite), "\"user\"");
    assert_eq!(quote_identifier("first_name", Dialect::PostgreSQL), "\"first_name\"");
}

#[test]
fn test_format_default_value() {
    use cdm_plugin_interface::Value;

    assert_eq!(format_default_value(&Value::String("hello".to_string())), "'hello'");
    assert_eq!(format_default_value(&Value::String("it's".to_string())), "'it''s'");
    assert_eq!(format_default_value(&Value::Number(42.0)), "42");
    assert_eq!(format_default_value(&Value::Boolean(true)), "TRUE");
    assert_eq!(format_default_value(&Value::Boolean(false)), "FALSE");
    assert_eq!(format_default_value(&Value::Null), "NULL");
}
