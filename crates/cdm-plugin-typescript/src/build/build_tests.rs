use super::*;
use cdm_plugin_interface::{
    EntityId, FieldDefinition, ModelDefinition, TypeAliasDefinition, TypeExpression,
};
use serde_json::json;
use std::path::PathBuf;

fn local_id(id: u64) -> Option<EntityId> {
    Some(EntityId::local(id))
}

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
            entity_id: local_id(4),
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
            entity_id: local_id(3),
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
                entity_id: local_id(1),
            }],
            config: json!({}),
            entity_id: local_id(11),
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
                entity_id: local_id(1),
            }],
            config: json!({ "file_name": "models.ts" }),
            entity_id: local_id(11),
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

// Zod schema generation tests

#[test]
fn test_zod_generation_disabled_by_default() {
    let schema = create_test_schema();
    let config = json!({});
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    // Zod should not be generated by default
    assert!(!content.contains("import { z }"));
    assert!(!content.contains("Schema: z.ZodType"));
}

#[test]
fn test_zod_generation_global_enabled() {
    let schema = create_test_schema();
    let config = json!({ "generate_zod": true });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    // Should have zod import
    assert!(content.contains("import { z } from 'zod';"));

    // Should have UserSchema
    assert!(content.contains("export const UserSchema: z.ZodType<User> = z.object({"));
    assert!(content.contains("id: z.string(),"));
    assert!(content.contains("name: z.string(),"));
    assert!(content.contains("email: EmailSchema.optional(),"));
}

#[test]
fn test_zod_generation_model_override_enabled() {
    let mut schema = create_test_schema();
    // Enable Zod only for User model
    schema.models.get_mut("User").unwrap().config = json!({ "generate_zod": true });

    let config = json!({}); // Global is false
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    // Should have zod import
    assert!(content.contains("import { z } from 'zod';"));

    // Should have UserSchema
    assert!(content.contains("export const UserSchema: z.ZodType<User>"));
}

#[test]
fn test_zod_generation_model_override_disabled() {
    let mut schema = create_test_schema();
    // Disable Zod for User model even though global is enabled
    schema.models.get_mut("User").unwrap().config = json!({ "generate_zod": false });

    let config = json!({ "generate_zod": true }); // Global is true
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    // Should NOT have zod import (no models generate zod)
    assert!(!content.contains("import { z } from 'zod';"));

    // Should NOT have UserSchema
    assert!(!content.contains("UserSchema"));
}

#[test]
fn test_zod_generation_with_optional_fields() {
    let schema = create_test_schema();
    let config = json!({ "generate_zod": true });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    // email is optional in the test schema
    assert!(content.contains("email: EmailSchema.optional(),"));
}

#[test]
fn test_zod_generation_no_export() {
    let schema = create_test_schema();
    let config = json!({ "generate_zod": true, "export_all": false });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    // Should NOT have export keyword for schema
    assert!(content.contains("const UserSchema: z.ZodType<User>"));
    assert!(!content.contains("export const UserSchema"));
}

#[test]
fn test_zod_generation_with_type_override_uses_any() {
    let mut schema = create_test_schema();
    schema.models.get_mut("User").unwrap().fields[0].config = json!({ "type_override": "number" });

    let config = json!({ "generate_zod": true });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    // When type_override is used, we can't generate accurate Zod, so use z.any()
    assert!(content.contains("id: z.any(),"));
}

#[test]
fn test_zod_generation_per_model_strategy() {
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
                entity_id: local_id(1),
            }],
            config: json!({}),
            entity_id: local_id(11),
        },
    );

    let config = json!({ "file_strategy": "per_model", "generate_zod": true });
    let utils = Utils;

    let output = build(schema, config, &utils);

    // Find User.ts file
    let user_file = output.iter().find(|f| f.path == "User.ts").unwrap();
    assert!(user_file.content.contains("import { z } from 'zod';"));
    assert!(user_file.content.contains("export const UserSchema: z.ZodType<User>"));

    // Find Post.ts file
    let post_file = output.iter().find(|f| f.path == "Post.ts").unwrap();
    assert!(post_file.content.contains("import { z } from 'zod';"));
    assert!(post_file.content.contains("export const PostSchema: z.ZodType<Post>"));
}

#[test]
fn test_zod_generation_skips_skipped_fields() {
    let mut schema = create_test_schema();
    // Skip the 'name' field
    schema.models.get_mut("User").unwrap().fields[1].config = json!({ "skip": true });

    let config = json!({ "generate_zod": true });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    // UserSchema should not contain 'name'
    assert!(content.contains("id: z.string(),"));
    assert!(!content.contains("name: z.string(),"));
    assert!(content.contains("email: EmailSchema.optional(),"));
}

#[test]
fn test_zod_with_array_type() {
    let mut schema = create_test_schema();
    schema.models.get_mut("User").unwrap().fields.push(FieldDefinition {
        name: "tags".to_string(),
        field_type: TypeExpression::Array {
            element_type: Box::new(TypeExpression::Identifier {
                name: "string".to_string(),
            }),
        },
        optional: false,
        default: None,
        config: json!({}),
        entity_id: local_id(5),
    });

    let config = json!({ "generate_zod": true });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("tags: z.array(z.string()),"));
}

#[test]
fn test_zod_with_union_type() {
    let mut schema = create_test_schema();
    schema.models.get_mut("User").unwrap().fields.push(FieldDefinition {
        name: "status".to_string(),
        field_type: TypeExpression::Union {
            types: vec![
                TypeExpression::StringLiteral { value: "active".to_string() },
                TypeExpression::StringLiteral { value: "inactive".to_string() },
            ],
        },
        optional: false,
        default: None,
        config: json!({}),
        entity_id: local_id(6),
    });

    let config = json!({ "generate_zod": true });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("status: z.union([z.literal(\"active\"), z.literal(\"inactive\")]),"));
}

// Type alias Zod generation tests

#[test]
fn test_type_alias_zod_generation() {
    let schema = create_test_schema();
    let config = json!({ "generate_zod": true });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    // Should generate Zod schemas for type aliases
    assert!(content.contains("export const EmailSchema = z.string();"));
    assert!(content.contains("export const StatusSchema = z.union([z.literal(\"active\"), z.literal(\"inactive\")]);"));
}

#[test]
fn test_type_alias_zod_no_export() {
    let schema = create_test_schema();
    let config = json!({ "generate_zod": true, "export_all": false });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    // Should NOT have export keyword for type alias schemas
    assert!(content.contains("const EmailSchema = z.string();"));
    assert!(!content.contains("export const EmailSchema"));
}

#[test]
fn test_type_alias_zod_not_generated_when_disabled() {
    let schema = create_test_schema();
    let config = json!({});
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    // Should NOT generate Zod schemas when disabled
    assert!(!content.contains("EmailSchema"));
    assert!(!content.contains("StatusSchema"));
}

#[test]
fn test_type_alias_zod_per_model_strategy() {
    let schema = create_test_schema();
    let config = json!({ "file_strategy": "per_model", "generate_zod": true });
    let utils = Utils;

    let output = build(schema, config, &utils);

    // Find types.ts file
    let types_file = output.iter().find(|f| f.path == "types.ts").unwrap();

    // types.ts should have Zod import and type alias schemas
    assert!(types_file.content.contains("import { z } from 'zod';"));
    assert!(types_file.content.contains("export const EmailSchema = z.string();"));
    assert!(types_file.content.contains("export const StatusSchema = z.union([z.literal(\"active\"), z.literal(\"inactive\")]);"));
}

#[test]
fn test_skipped_type_alias_no_zod() {
    let mut schema = create_test_schema();
    // Skip the Email type alias
    schema.type_aliases.get_mut("Email").unwrap().config = json!({ "skip": true });

    let config = json!({ "generate_zod": true });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    // EmailSchema declaration should NOT be generated (skipped)
    // Note: EmailSchema may still appear as a reference in model field types
    assert!(!content.contains("const EmailSchema = z.string();"));
    // But StatusSchema should still be generated
    assert!(content.contains("StatusSchema"));
}

// Import generation tests for per_model strategy

#[test]
fn test_per_model_imports_model_reference() {
    let mut schema = create_test_schema();

    // Add a Post model that references User
    schema.models.insert(
        "Post".to_string(),
        ModelDefinition {
            name: "Post".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "title".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(1),
                },
                FieldDefinition {
                    name: "author".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "User".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(2),
                },
            ],
            config: json!({}),
            entity_id: local_id(11),
        },
    );

    let config = json!({ "file_strategy": "per_model" });
    let utils = Utils;

    let output = build(schema, config, &utils);

    // Find Post.ts file
    let post_file = output.iter().find(|f| f.path == "Post.ts").unwrap();

    // Post.ts should import User from "./User"
    assert!(
        post_file.content.contains("import { User } from \"./User\""),
        "Post.ts should import User. Content:\n{}",
        post_file.content
    );
}

#[test]
fn test_per_model_imports_type_alias_reference() {
    let schema = create_test_schema();

    let config = json!({ "file_strategy": "per_model" });
    let utils = Utils;

    let output = build(schema, config, &utils);

    // Find User.ts file
    let user_file = output.iter().find(|f| f.path == "User.ts").unwrap();

    // User.ts should import Email from "./types"
    assert!(
        user_file.content.contains("import { Email } from \"./types\""),
        "User.ts should import Email from types. Content:\n{}",
        user_file.content
    );
}

#[test]
fn test_per_model_imports_array_of_models() {
    let mut schema = create_test_schema();

    // Add a Team model that has an array of Users
    schema.models.insert(
        "Team".to_string(),
        ModelDefinition {
            name: "Team".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "name".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(1),
                },
                FieldDefinition {
                    name: "members".to_string(),
                    field_type: TypeExpression::Array {
                        element_type: Box::new(TypeExpression::Identifier {
                            name: "User".to_string(),
                        }),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(2),
                },
            ],
            config: json!({}),
            entity_id: local_id(12),
        },
    );

    let config = json!({ "file_strategy": "per_model" });
    let utils = Utils;

    let output = build(schema, config, &utils);

    // Find Team.ts file
    let team_file = output.iter().find(|f| f.path == "Team.ts").unwrap();

    // Team.ts should import User from "./User"
    assert!(
        team_file.content.contains("import { User } from \"./User\""),
        "Team.ts should import User for array field. Content:\n{}",
        team_file.content
    );
}

#[test]
fn test_per_model_no_self_import() {
    let mut schema = create_test_schema();

    // Add a Node model that references itself (tree structure)
    schema.models.insert(
        "Node".to_string(),
        ModelDefinition {
            name: "Node".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "value".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(1),
                },
                FieldDefinition {
                    name: "parent".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "Node".to_string(),
                    },
                    optional: true,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(2),
                },
            ],
            config: json!({}),
            entity_id: local_id(13),
        },
    );

    let config = json!({ "file_strategy": "per_model" });
    let utils = Utils;

    let output = build(schema, config, &utils);

    // Find Node.ts file
    let node_file = output.iter().find(|f| f.path == "Node.ts").unwrap();

    // Node.ts should NOT import Node from itself
    assert!(
        !node_file.content.contains("import { Node } from \"./Node\""),
        "Node.ts should NOT self-import. Content:\n{}",
        node_file.content
    );
}

#[test]
fn test_per_model_imports_multiple_types() {
    let mut schema = create_test_schema();

    // Add a Comment model that references User and uses Email type alias
    schema.models.insert(
        "Comment".to_string(),
        ModelDefinition {
            name: "Comment".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "text".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(1),
                },
                FieldDefinition {
                    name: "author".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "User".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(2),
                },
                FieldDefinition {
                    name: "authorEmail".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "Email".to_string(),
                    },
                    optional: true,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(3),
                },
            ],
            config: json!({}),
            entity_id: local_id(14),
        },
    );

    let config = json!({ "file_strategy": "per_model" });
    let utils = Utils;

    let output = build(schema, config, &utils);

    // Find Comment.ts file
    let comment_file = output.iter().find(|f| f.path == "Comment.ts").unwrap();

    // Comment.ts should import User from "./User"
    assert!(
        comment_file.content.contains("import { User } from \"./User\""),
        "Comment.ts should import User. Content:\n{}",
        comment_file.content
    );

    // Comment.ts should import Email from "./types"
    assert!(
        comment_file.content.contains("import { Email } from \"./types\""),
        "Comment.ts should import Email from types. Content:\n{}",
        comment_file.content
    );
}

#[test]
fn test_per_model_imports_with_zod() {
    let mut schema = create_test_schema();

    // Add a Post model that references User
    schema.models.insert(
        "Post".to_string(),
        ModelDefinition {
            name: "Post".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "title".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(1),
                },
                FieldDefinition {
                    name: "author".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "User".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(2),
                },
            ],
            config: json!({}),
            entity_id: local_id(11),
        },
    );

    let config = json!({ "file_strategy": "per_model", "generate_zod": true });
    let utils = Utils;

    let output = build(schema, config, &utils);

    // Find Post.ts file
    let post_file = output.iter().find(|f| f.path == "Post.ts").unwrap();

    // Post.ts should have both zod import and User import
    assert!(
        post_file.content.contains("import { z } from 'zod'"),
        "Post.ts should have zod import. Content:\n{}",
        post_file.content
    );
    assert!(
        post_file.content.contains("import { User, UserSchema } from \"./User\""),
        "Post.ts should import User and UserSchema. Content:\n{}",
        post_file.content
    );
}

#[test]
fn test_per_model_imports_zod_type_alias() {
    let schema = create_test_schema();

    let config = json!({ "file_strategy": "per_model", "generate_zod": true });
    let utils = Utils;

    let output = build(schema, config, &utils);

    // Find User.ts file
    let user_file = output.iter().find(|f| f.path == "User.ts").unwrap();

    // User.ts should import Email and EmailSchema from types
    assert!(
        user_file.content.contains("import { Email, EmailSchema } from \"./types\""),
        "User.ts should import Email and EmailSchema from types. Content:\n{}",
        user_file.content
    );
}

#[test]
fn test_per_model_grouped_files_no_imports_within_same_file() {
    let mut schema = create_test_schema();

    // Add Post model in same file as User
    schema.models.get_mut("User").unwrap().config = json!({ "file_name": "models.ts" });

    schema.models.insert(
        "Post".to_string(),
        ModelDefinition {
            name: "Post".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "title".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(1),
                },
                FieldDefinition {
                    name: "author".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "User".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(2),
                },
            ],
            config: json!({ "file_name": "models.ts" }),
            entity_id: local_id(11),
        },
    );

    let config = json!({ "file_strategy": "per_model" });
    let utils = Utils;

    let output = build(schema, config, &utils);

    // Find models.ts file (both User and Post should be in this file)
    let models_file = output.iter().find(|f| f.path == "models.ts").unwrap();

    // models.ts should NOT have an import for User since Post and User are in same file
    assert!(
        !models_file.content.contains("import { User }"),
        "models.ts should NOT import User since they're in same file. Content:\n{}",
        models_file.content
    );

    // But should still import type alias from types.ts
    assert!(
        models_file.content.contains("import { Email } from \"./types\""),
        "models.ts should import Email from types. Content:\n{}",
        models_file.content
    );
}

// Zod schema ordering tests

#[test]
fn test_zod_schemas_declared_before_referenced_single_file() {
    let mut schema = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };

    // Create Address model (no dependencies)
    schema.models.insert(
        "Address".to_string(),
        ModelDefinition {
            name: "Address".to_string(),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "street".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: json!({}),
                entity_id: local_id(1),
            }],
            config: json!({}),
            entity_id: local_id(10),
        },
    );

    // Create User model that depends on Address
    schema.models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "name".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(2),
                },
                FieldDefinition {
                    name: "address".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "Address".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(3),
                },
            ],
            config: json!({}),
            entity_id: local_id(11),
        },
    );

    // Create Post model that depends on User (which depends on Address)
    schema.models.insert(
        "Post".to_string(),
        ModelDefinition {
            name: "Post".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "title".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(4),
                },
                FieldDefinition {
                    name: "author".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "User".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(5),
                },
            ],
            config: json!({}),
            entity_id: local_id(12),
        },
    );

    let config = json!({ "generate_zod": true });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    // AddressSchema should appear before UserSchema (since User depends on Address)
    let address_schema_pos = content.find("const AddressSchema").unwrap();
    let user_schema_pos = content.find("const UserSchema").unwrap();
    let post_schema_pos = content.find("const PostSchema").unwrap();

    assert!(
        address_schema_pos < user_schema_pos,
        "AddressSchema should be declared before UserSchema.\nContent:\n{}",
        content
    );
    assert!(
        user_schema_pos < post_schema_pos,
        "UserSchema should be declared before PostSchema.\nContent:\n{}",
        content
    );
}

#[test]
fn test_zod_schemas_type_aliases_before_models_that_reference_them() {
    let mut schema = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };

    // Create Email type alias
    schema.type_aliases.insert(
        "Email".to_string(),
        TypeAliasDefinition {
            name: "Email".to_string(),
            alias_type: TypeExpression::Identifier {
                name: "string".to_string(),
            },
            config: json!({}),
            entity_id: local_id(1),
        },
    );

    // Create User model that references Email type alias
    schema.models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "email".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "Email".to_string(),
                },
                optional: false,
                default: None,
                config: json!({}),
                entity_id: local_id(2),
            }],
            config: json!({}),
            entity_id: local_id(10),
        },
    );

    let config = json!({ "generate_zod": true });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    // EmailSchema should appear before UserSchema
    let email_schema_pos = content.find("const EmailSchema").unwrap();
    let user_schema_pos = content.find("const UserSchema").unwrap();

    assert!(
        email_schema_pos < user_schema_pos,
        "EmailSchema should be declared before UserSchema.\nContent:\n{}",
        content
    );
}

#[test]
fn test_zod_schemas_handles_array_dependencies() {
    let mut schema = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };

    // Create Tag model
    schema.models.insert(
        "Tag".to_string(),
        ModelDefinition {
            name: "Tag".to_string(),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "name".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: json!({}),
                entity_id: local_id(1),
            }],
            config: json!({}),
            entity_id: local_id(10),
        },
    );

    // Create Post model that has an array of Tags
    schema.models.insert(
        "Post".to_string(),
        ModelDefinition {
            name: "Post".to_string(),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "tags".to_string(),
                field_type: TypeExpression::Array {
                    element_type: Box::new(TypeExpression::Identifier {
                        name: "Tag".to_string(),
                    }),
                },
                optional: false,
                default: None,
                config: json!({}),
                entity_id: local_id(2),
            }],
            config: json!({}),
            entity_id: local_id(11),
        },
    );

    let config = json!({ "generate_zod": true });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    // TagSchema should appear before PostSchema
    let tag_schema_pos = content.find("const TagSchema").unwrap();
    let post_schema_pos = content.find("const PostSchema").unwrap();

    assert!(
        tag_schema_pos < post_schema_pos,
        "TagSchema should be declared before PostSchema.\nContent:\n{}",
        content
    );
}

#[test]
fn test_zod_schemas_per_model_grouped_file_ordering() {
    let mut schema = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };

    // Create Address model in same file as User
    schema.models.insert(
        "Address".to_string(),
        ModelDefinition {
            name: "Address".to_string(),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "street".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: json!({}),
                entity_id: local_id(1),
            }],
            config: json!({ "file_name": "models.ts" }),
            entity_id: local_id(10),
        },
    );

    // Create User model that depends on Address, in same file
    schema.models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "name".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(2),
                },
                FieldDefinition {
                    name: "address".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "Address".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(3),
                },
            ],
            config: json!({ "file_name": "models.ts" }),
            entity_id: local_id(11),
        },
    );

    let config = json!({ "file_strategy": "per_model", "generate_zod": true });
    let utils = Utils;

    let output = build(schema, config, &utils);

    // Find models.ts file (both User and Address should be in this file)
    let models_file = output.iter().find(|f| f.path == "models.ts").unwrap();
    let content = &models_file.content;

    // AddressSchema should appear before UserSchema since User depends on Address
    let address_schema_pos = content.find("const AddressSchema").unwrap();
    let user_schema_pos = content.find("const UserSchema").unwrap();

    assert!(
        address_schema_pos < user_schema_pos,
        "AddressSchema should be declared before UserSchema in grouped file.\nContent:\n{}",
        content
    );
}

#[test]
fn test_zod_schemas_circular_reference_uses_lazy() {
    let mut schema = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };

    // Create User model that references Post
    schema.models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "name".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(1),
                },
                FieldDefinition {
                    name: "posts".to_string(),
                    field_type: TypeExpression::Array {
                        element_type: Box::new(TypeExpression::Identifier {
                            name: "Post".to_string(),
                        }),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(2),
                },
            ],
            config: json!({}),
            entity_id: local_id(10),
        },
    );

    // Create Post model that references User (circular dependency)
    schema.models.insert(
        "Post".to_string(),
        ModelDefinition {
            name: "Post".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "title".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(3),
                },
                FieldDefinition {
                    name: "author".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "User".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(4),
                },
            ],
            config: json!({}),
            entity_id: local_id(11),
        },
    );

    let config = json!({ "generate_zod": true });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    // Both schemas should use z.lazy() for the circular reference
    // At least one of them must use z.lazy() to break the cycle
    let has_lazy = content.contains("z.lazy(");
    assert!(
        has_lazy,
        "Circular references should use z.lazy() to break the cycle.\nContent:\n{}",
        content
    );

    // Verify the lazy pattern is correct (wraps a schema reference)
    assert!(
        content.contains("z.lazy(() => PostSchema)") || content.contains("z.lazy(() => UserSchema)"),
        "z.lazy() should wrap the schema reference.\nContent:\n{}",
        content
    );
}

#[test]
fn test_zod_schemas_self_reference_uses_lazy() {
    let mut schema = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };

    // Create Node model that references itself (tree structure)
    schema.models.insert(
        "Node".to_string(),
        ModelDefinition {
            name: "Node".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "value".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(1),
                },
                FieldDefinition {
                    name: "children".to_string(),
                    field_type: TypeExpression::Array {
                        element_type: Box::new(TypeExpression::Identifier {
                            name: "Node".to_string(),
                        }),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(2),
                },
            ],
            config: json!({}),
            entity_id: local_id(10),
        },
    );

    let config = json!({ "generate_zod": true });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    // Self-reference should use z.lazy()
    assert!(
        content.contains("z.lazy(() => NodeSchema)"),
        "Self-references should use z.lazy().\nContent:\n{}",
        content
    );
}
