use super::*;
use cdm_plugin_interface::{FieldDefinition, ModelDefinition, TypeAliasDefinition, TypeExpression};
use serde_json::json;
use std::path::PathBuf;

fn fixtures_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_fixtures")
}

fn load_test_schema(fixture_name: &str) -> Schema {
    let path = fixtures_path().join(fixture_name);
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to load fixture {}: {}", path.display(), e));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse fixture {}: {}", path.display(), e))
}

fn create_test_schema() -> Schema {
    load_test_schema("basic_schema.json")
}

#[test]
fn test_default_config() {
    let config = Config::from_json(&json!({}));
    assert_eq!(config.output_format, "interface");
    assert_eq!(config.file_strategy, "single");
    assert_eq!(config.single_file_name, "types.ts");
    assert_eq!(config.optional_strategy, "native");
    assert_eq!(config.strict_nulls, true);
    assert_eq!(config.export_all, true);
    assert_eq!(config.type_name_format, "preserve");
    assert_eq!(config.field_name_format, "preserve");
}

#[test]
fn test_custom_config() {
    let config = Config::from_json(&json!({
        "output_format": "class",
        "file_strategy": "per_model",
        "single_file_name": "models.ts",
        "optional_strategy": "union_undefined",
        "strict_nulls": false,
        "export_all": false,
        "type_name_format": "pascal",
        "field_name_format": "camel"
    }));

    assert_eq!(config.output_format, "class");
    assert_eq!(config.file_strategy, "per_model");
    assert_eq!(config.single_file_name, "models.ts");
    assert_eq!(config.optional_strategy, "union_undefined");
    assert_eq!(config.strict_nulls, false);
    assert_eq!(config.export_all, false);
    assert_eq!(config.type_name_format, "pascal");
    assert_eq!(config.field_name_format, "camel");
}

#[test]
fn test_single_file_interface_generation() {
    let schema = create_test_schema();
    let config = json!({});
    let utils = Utils;

    let output = build(schema, config, &utils);

    assert_eq!(output.len(), 1);
    assert_eq!(output[0].path, "types.ts");

    let content = &output[0].content;
    assert!(content.contains("export type Email = string;"));
    assert!(content.contains("export type Status = \"active\" | \"inactive\";"));
    assert!(content.contains("export interface User {"));
    assert!(content.contains("  id: string;"));
    assert!(content.contains("  name: string;"));
    assert!(content.contains("  email?: Email;"));
}

#[test]
fn test_class_generation() {
    let schema = create_test_schema();
    let config = json!({ "output_format": "class" });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("export class User {"));
    assert!(content.contains("  id: string;"));
    assert!(content.contains("  name: string;"));
    assert!(content.contains("  email?: Email;"));
    assert!(content.contains("  constructor(data: Partial<User>) {"));
    assert!(content.contains("    Object.assign(this, data);"));
}

#[test]
fn test_type_alias_generation() {
    let schema = create_test_schema();
    let config = json!({ "output_format": "type" });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("export type User = {"));
    assert!(content.contains("  id: string;"));
    assert!(content.contains("  name: string;"));
    assert!(content.contains("  email?: Email;"));
    assert!(content.contains("};"));
}

#[test]
fn test_no_exports() {
    let schema = create_test_schema();
    let config = json!({ "export_all": false });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("type Email = string;"));
    assert!(content.contains("interface User {"));
    assert!(!content.contains("export type Email"));
    assert!(!content.contains("export interface User"));
}

#[test]
fn test_field_name_format_camel() {
    let mut schema = create_test_schema();
    schema
        .models
        .get_mut("User")
        .unwrap()
        .fields
        .push(FieldDefinition {
            name: "created_at".to_string(),
            field_type: TypeExpression::Identifier {
                name: "string".to_string(),
            },
            optional: false,
            default: None,
            config: json!({}),
            entity_id: Some(4),
        });

    let config = json!({ "field_name_format": "camel" });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("  createdAt: string;"));
    assert!(!content.contains("  created_at: string;"));
}

#[test]
fn test_type_name_format_pascal() {
    let mut schema = create_test_schema();
    schema.type_aliases.insert(
        "user_status".to_string(),
        TypeAliasDefinition {
            name: "user_status".to_string(),
            alias_type: TypeExpression::Identifier {
                name: "string".to_string(),
            },
            config: json!({}),
            entity_id: Some(3),
        },
    );

    let config = json!({ "type_name_format": "pascal" });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("export type UserStatus = string;"));
    assert!(!content.contains("export type user_status"));
}

#[test]
fn test_readonly_field() {
    let mut schema = create_test_schema();
    schema.models.get_mut("User").unwrap().fields[0].config = json!({ "readonly": true });

    let config = json!({});
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("  readonly id: string;"));
}

#[test]
fn test_readonly_model() {
    let mut schema = create_test_schema();
    schema.models.get_mut("User").unwrap().config = json!({ "readonly": true });

    let config = json!({});
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("  readonly id: string;"));
    assert!(content.contains("  readonly name: string;"));
    assert!(content.contains("  readonly email?: Email;"));
}

#[test]
fn test_skip_field() {
    let mut schema = create_test_schema();
    schema.models.get_mut("User").unwrap().fields[1].config = json!({ "skip": true });

    let config = json!({});
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("  id: string;"));
    assert!(!content.contains("  name: string;"));
    assert!(content.contains("  email?: Email;"));
}

#[test]
fn test_skip_model() {
    let mut schema = create_test_schema();
    schema.models.get_mut("User").unwrap().config = json!({ "skip": true });

    let config = json!({});
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(!content.contains("interface User"));
    assert!(content.contains("export type Email"));
}

#[test]
fn test_type_override() {
    let mut schema = create_test_schema();
    schema.models.get_mut("User").unwrap().fields[0].config = json!({ "type_override": "number" });

    let config = json!({});
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("  id: number;"));
}

#[test]
fn test_field_name_override() {
    let mut schema = create_test_schema();
    schema.models.get_mut("User").unwrap().fields[0].config = json!({ "field_name": "userId" });

    let config = json!({});
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("  userId: string;"));
    assert!(!content.contains("  id: string;"));
}

#[test]
fn test_export_name_override() {
    let mut schema = create_test_schema();
    schema.models.get_mut("User").unwrap().config = json!({ "export_name": "UserModel" });

    let config = json!({});
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("export interface UserModel {"));
    assert!(!content.contains("export interface User {"));
}

#[test]
fn test_per_model_file_strategy() {
    let mut schema = create_test_schema();

    // Add another model
    schema.models.insert(
        "Post".to_string(),
        ModelDefinition {
            name: "Post".to_string(),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "title".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: json!({}),
                entity_id: Some(1),
            }],
            config: json!({}),
            entity_id: Some(11),
        },
    );

    let config = json!({ "file_strategy": "per_model" });
    let utils = Utils;

    let output = build(schema, config, &utils);

    assert_eq!(output.len(), 3); // User.ts, Post.ts, types.ts

    let file_names: Vec<&str> = output.iter().map(|f| f.path.as_str()).collect();
    assert!(file_names.contains(&"User.ts"));
    assert!(file_names.contains(&"Post.ts"));
    assert!(file_names.contains(&"types.ts"));
}

#[test]
fn test_file_grouping() {
    let mut schema = create_test_schema();

    // Add two models with the same file_name
    schema.models.get_mut("User").unwrap().config = json!({ "file_name": "models.ts" });

    schema.models.insert(
        "Post".to_string(),
        ModelDefinition {
            name: "Post".to_string(),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "title".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: json!({}),
                entity_id: Some(1),
            }],
            config: json!({ "file_name": "models.ts" }),
            entity_id: Some(11),
        },
    );

    let config = json!({ "file_strategy": "per_model" });
    let utils = Utils;

    let output = build(schema, config, &utils);

    assert_eq!(output.len(), 2); // models.ts (with User and Post), types.ts

    let models_file = output.iter().find(|f| f.path == "models.ts").unwrap();
    assert!(models_file.content.contains("interface User"));
    assert!(models_file.content.contains("interface Post"));
}

#[test]
fn test_optional_native_strategy() {
    let schema = create_test_schema();
    let config = json!({ "optional_strategy": "native" });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("  email?: Email;"));
}

#[test]
fn test_optional_union_undefined_strategy() {
    let schema = create_test_schema();
    let config = json!({ "optional_strategy": "union_undefined" });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    // With union_undefined, there's no ? marker
    assert!(content.contains("  email: Email;"));
    assert!(!content.contains("  email?: Email;"));
}

#[test]
fn test_model_output_format_override() {
    let mut schema = create_test_schema();
    schema.models.get_mut("User").unwrap().config = json!({ "output_format": "class" });

    let config = json!({ "output_format": "interface" });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    // Model config should override global config
    assert!(content.contains("export class User {"));
    assert!(content.contains("  constructor(data: Partial<User>)"));
}

#[test]
fn test_custom_single_file_name() {
    let schema = create_test_schema();
    let config = json!({ "single_file_name": "my-types.ts" });
    let utils = Utils;

    let output = build(schema, config, &utils);

    assert_eq!(output.len(), 1);
    assert_eq!(output[0].path, "my-types.ts");
}

#[test]
fn test_format_name_preserve() {
    let utils = Utils;
    assert_eq!(
        format_name("UserProfile", "preserve", &utils),
        "UserProfile"
    );
}

#[test]
fn test_format_name_pascal() {
    let utils = Utils;
    assert_eq!(format_name("user_profile", "pascal", &utils), "UserProfile");
}

#[test]
fn test_format_name_camel() {
    let utils = Utils;
    assert_eq!(format_name("user_profile", "camel", &utils), "userProfile");
}

#[test]
fn test_format_name_snake() {
    let utils = Utils;
    assert_eq!(format_name("UserProfile", "snake", &utils), "user_profile");
}

#[test]
fn test_format_optional_native() {
    assert_eq!(format_optional(true, "native"), "?");
    assert_eq!(format_optional(false, "native"), "");
}

#[test]
fn test_format_optional_union_undefined() {
    assert_eq!(format_optional(true, "union_undefined"), "");
    assert_eq!(format_optional(false, "union_undefined"), "");
}
