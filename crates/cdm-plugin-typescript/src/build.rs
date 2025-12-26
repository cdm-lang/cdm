use cdm_plugin_api::{CaseFormat, OutputFile, Schema, Utils, JSON};
use std::collections::HashMap;

use crate::type_mapper::map_type_to_typescript;

#[derive(Debug, Clone)]
struct Config {
    output_format: String,
    file_strategy: String,
    single_file_name: String,
    optional_strategy: String,
    strict_nulls: bool,
    export_all: bool,
    type_name_format: String,
    field_name_format: String,
}

impl Config {
    fn from_json(json: &JSON) -> Self {
        Self {
            output_format: json
                .get("output_format")
                .and_then(|v| v.as_str())
                .unwrap_or("interface")
                .to_string(),
            file_strategy: json
                .get("file_strategy")
                .and_then(|v| v.as_str())
                .unwrap_or("single")
                .to_string(),
            single_file_name: json
                .get("single_file_name")
                .and_then(|v| v.as_str())
                .unwrap_or("types.ts")
                .to_string(),
            optional_strategy: json
                .get("optional_strategy")
                .and_then(|v| v.as_str())
                .unwrap_or("native")
                .to_string(),
            strict_nulls: json
                .get("strict_nulls")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            export_all: json
                .get("export_all")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            type_name_format: json
                .get("type_name_format")
                .and_then(|v| v.as_str())
                .unwrap_or("preserve")
                .to_string(),
            field_name_format: json
                .get("field_name_format")
                .and_then(|v| v.as_str())
                .unwrap_or("preserve")
                .to_string(),
        }
    }
}

pub fn build(schema: Schema, config: JSON, utils: &Utils) -> Vec<OutputFile> {
    let cfg = Config::from_json(&config);

    match cfg.file_strategy.as_str() {
        "single" => build_single_file(schema, cfg, utils),
        "per_model" => build_per_model_files(schema, cfg, utils),
        _ => vec![],
    }
}

fn build_single_file(schema: Schema, cfg: Config, utils: &Utils) -> Vec<OutputFile> {
    let mut content = String::new();

    // Generate type aliases first
    for (name, alias) in &schema.type_aliases {
        if should_skip_type_alias(alias) {
            continue;
        }

        let formatted_name = format_name(&name, &cfg.type_name_format, utils);
        let type_str = map_type_to_typescript(&alias.alias_type, cfg.strict_nulls);

        let export = if cfg.export_all { "export " } else { "" };
        content.push_str(&format!("{}type {} = {};\n\n", export, formatted_name, type_str));
    }

    // Generate models
    for (name, model) in &schema.models {
        // Config is already filtered to this plugin by CDM core
        let model_config = &model.config;

        if should_skip_model(model_config) {
            continue;
        }

        let model_output_format = get_model_output_format(model_config, &cfg.output_format);
        let formatted_name = get_export_name(model_config, &name, &cfg.type_name_format, utils);

        match model_output_format.as_str() {
            "interface" => {
                content.push_str(&generate_interface(&formatted_name, model, &cfg, utils));
            }
            "class" => {
                content.push_str(&generate_class(&formatted_name, model, &cfg, utils));
            }
            "type" => {
                content.push_str(&generate_type_alias(&formatted_name, model, &cfg, utils));
            }
            _ => {}
        }
        content.push('\n');
    }

    vec![OutputFile {
        path: cfg.single_file_name.clone(),
        content,
    }]
}

fn build_per_model_files(schema: Schema, cfg: Config, utils: &Utils) -> Vec<OutputFile> {
    let mut files: HashMap<String, String> = HashMap::new();
    let mut model_to_file: HashMap<String, String> = HashMap::new();

    // First pass: determine which models go to which files
    for (name, model) in &schema.models {
        // Config is already filtered to this plugin by CDM core
        let model_config = &model.config;

        if should_skip_model(model_config) {
            continue;
        }

        let file_name = get_file_name(model_config, &name, utils);
        model_to_file.insert(name.clone(), file_name.clone());

        if !files.contains_key(&file_name) {
            files.insert(file_name.clone(), String::new());
        }
    }

    // Second pass: generate type aliases in a shared file
    if !schema.type_aliases.is_empty() {
        let mut types_content = String::new();
        for (name, alias) in &schema.type_aliases {
            if should_skip_type_alias(alias) {
                continue;
            }

            let formatted_name = format_name(&name, &cfg.type_name_format, utils);
            let type_str = map_type_to_typescript(&alias.alias_type, cfg.strict_nulls);

            let export = if cfg.export_all { "export " } else { "" };
            types_content.push_str(&format!("{}type {} = {};\n\n", export, formatted_name, type_str));
        }
        if !types_content.is_empty() {
            files.insert("types.ts".to_string(), types_content);
        }
    }

    // Third pass: generate models grouped by file
    for (name, model) in &schema.models {
        // Config is already filtered to this plugin by CDM core
        let model_config = &model.config;

        if should_skip_model(model_config) {
            continue;
        }

        let file_name = model_to_file.get(name).unwrap();
        let content = files.get_mut(file_name).unwrap();

        let model_output_format = get_model_output_format(model_config, &cfg.output_format);
        let formatted_name = get_export_name(model_config, &name, &cfg.type_name_format, utils);

        match model_output_format.as_str() {
            "interface" => {
                content.push_str(&generate_interface(&formatted_name, model, &cfg, utils));
            }
            "class" => {
                content.push_str(&generate_class(&formatted_name, model, &cfg, utils));
            }
            "type" => {
                content.push_str(&generate_type_alias(&formatted_name, model, &cfg, utils));
            }
            _ => {}
        }
        content.push('\n');
    }

    // Convert HashMap to Vec<OutputFile>
    files
        .into_iter()
        .map(|(path, content)| OutputFile { path, content })
        .collect()
}

fn generate_interface(
    name: &str,
    model: &cdm_plugin_api::ModelDefinition,
    cfg: &Config,
    utils: &Utils,
) -> String {
    let mut result = String::new();

    let export = if cfg.export_all { "export " } else { "" };
    result.push_str(&format!("{}interface {} {{\n", export, name));

    for field in &model.fields {
        // Config is already filtered to this plugin by CDM core
        let field_config = &field.config;

        if should_skip_field(field_config) {
            continue;
        }

        let field_name = get_field_name(field_config, &field.name, &cfg.field_name_format, utils);
        let readonly = if is_readonly_field(field_config) || is_readonly_model(&model.config) {
            "readonly "
        } else {
            ""
        };

        let type_str = get_field_type(field_config, &field.field_type, cfg.strict_nulls);
        let optional_marker = format_optional(field.optional, &cfg.optional_strategy);

        result.push_str(&format!("  {}{}{}: {};\n", readonly, field_name, optional_marker, type_str));
    }

    result.push('}');
    result
}

fn generate_class(
    name: &str,
    model: &cdm_plugin_api::ModelDefinition,
    cfg: &Config,
    utils: &Utils,
) -> String {
    let mut result = String::new();

    let export = if cfg.export_all { "export " } else { "" };
    result.push_str(&format!("{}class {} {{\n", export, name));

    // Properties
    for field in &model.fields {
        // Config is already filtered to this plugin by CDM core
        let field_config = &field.config;

        if should_skip_field(field_config) {
            continue;
        }

        let field_name = get_field_name(field_config, &field.name, &cfg.field_name_format, utils);
        let readonly = if is_readonly_field(field_config) || is_readonly_model(&model.config) {
            "readonly "
        } else {
            ""
        };

        let type_str = get_field_type(field_config, &field.field_type, cfg.strict_nulls);
        let optional_marker = format_optional(field.optional, &cfg.optional_strategy);

        result.push_str(&format!("  {}{}{}: {};\n", readonly, field_name, optional_marker, type_str));
    }

    // Constructor
    result.push_str(&format!("\n  constructor(data: Partial<{}>) {{\n", name));
    result.push_str("    Object.assign(this, data);\n");
    result.push_str("  }\n");

    result.push('}');
    result
}

fn generate_type_alias(
    name: &str,
    model: &cdm_plugin_api::ModelDefinition,
    cfg: &Config,
    utils: &Utils,
) -> String {
    let mut result = String::new();

    let export = if cfg.export_all { "export " } else { "" };
    result.push_str(&format!("{}type {} = {{\n", export, name));

    for field in &model.fields {
        // Config is already filtered to this plugin by CDM core
        let field_config = &field.config;

        if should_skip_field(field_config) {
            continue;
        }

        let field_name = get_field_name(field_config, &field.name, &cfg.field_name_format, utils);
        let readonly = if is_readonly_field(field_config) || is_readonly_model(&model.config) {
            "readonly "
        } else {
            ""
        };

        let type_str = get_field_type(field_config, &field.field_type, cfg.strict_nulls);
        let optional_marker = format_optional(field.optional, &cfg.optional_strategy);

        result.push_str(&format!("  {}{}{}: {};\n", readonly, field_name, optional_marker, type_str));
    }

    result.push_str("};");
    result
}

// Helper functions

fn should_skip_type_alias(alias: &cdm_plugin_api::TypeAliasDefinition) -> bool {
    alias.config
        .get("skip")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn should_skip_model(model_config: &serde_json::Value) -> bool {
    model_config
        .get("skip")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn should_skip_field(field_config: &serde_json::Value) -> bool {
    field_config
        .get("skip")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn is_readonly_model(model_config: &serde_json::Value) -> bool {
    model_config
        .get("readonly")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn is_readonly_field(field_config: &serde_json::Value) -> bool {
    field_config
        .get("readonly")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn get_model_output_format(model_config: &serde_json::Value, default: &str) -> String {
    model_config
        .get("output_format")
        .and_then(|v| v.as_str())
        .unwrap_or(default)
        .to_string()
}

fn get_export_name(model_config: &serde_json::Value, default_name: &str, format: &str, utils: &Utils) -> String {
    model_config
        .get("export_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format_name(default_name, format, utils))
}

fn get_file_name(model_config: &serde_json::Value, model_name: &str, utils: &Utils) -> String {
    model_config
        .get("file_name")
        .and_then(|v| v.as_str())
        .map(|s| {
            if s.ends_with(".ts") {
                s.to_string()
            } else {
                format!("{}.ts", s)
            }
        })
        .unwrap_or_else(|| format!("{}.ts", utils.change_case(model_name, CaseFormat::Pascal)))
}

fn get_field_name(field_config: &serde_json::Value, default_name: &str, format: &str, utils: &Utils) -> String {
    field_config
        .get("field_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format_name(default_name, format, utils))
}

fn get_field_type(field_config: &serde_json::Value, default_type: &cdm_plugin_api::TypeExpression, strict_nulls: bool) -> String {
    field_config
        .get("type_override")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| map_type_to_typescript(default_type, strict_nulls))
}

fn format_name(name: &str, format: &str, utils: &Utils) -> String {
    match format {
        "preserve" => name.to_string(),
        "pascal" => utils.change_case(name, CaseFormat::Pascal),
        "camel" => utils.change_case(name, CaseFormat::Camel),
        "snake" => utils.change_case(name, CaseFormat::Snake),
        "kebab" => utils.change_case(name, CaseFormat::Kebab),
        "constant" => utils.change_case(name, CaseFormat::Constant),
        _ => name.to_string(),
    }
}

fn format_optional(is_optional: bool, strategy: &str) -> String {
    if !is_optional {
        return String::new();
    }

    match strategy {
        "native" => "?".to_string(),
        "union_undefined" => String::new(),
        _ => "?".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cdm_plugin_api::{FieldDefinition, ModelDefinition, TypeAliasDefinition, TypeExpression};
    use serde_json::json;

    fn create_test_schema() -> Schema {
        let mut models = HashMap::new();
        let mut type_aliases = HashMap::new();

        // Create a simple type alias
        type_aliases.insert(
            "Email".to_string(),
            TypeAliasDefinition {
                name: "Email".to_string(),
                alias_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                config: json!({}),
                entity_id: Some(1),
            },
        );

        // Create a Status union type
        type_aliases.insert(
            "Status".to_string(),
            TypeAliasDefinition {
                name: "Status".to_string(),
                alias_type: TypeExpression::Union {
                    types: vec![
                        TypeExpression::StringLiteral {
                            value: "active".to_string(),
                        },
                        TypeExpression::StringLiteral {
                            value: "inactive".to_string(),
                        },
                    ],
                },
                config: json!({}),
                entity_id: Some(2),
            },
        );

        // Create User model
        models.insert(
            "User".to_string(),
            ModelDefinition {
                name: "User".to_string(),
                parents: vec![],
                fields: vec![
                    FieldDefinition {
                        name: "id".to_string(),
                        field_type: TypeExpression::Identifier {
                            name: "string".to_string(),
                        },
                        optional: false,
                        default: None,
                        config: json!({}),
                        entity_id: Some(1),
                    },
                    FieldDefinition {
                        name: "name".to_string(),
                        field_type: TypeExpression::Identifier {
                            name: "string".to_string(),
                        },
                        optional: false,
                        default: None,
                        config: json!({}),
                        entity_id: Some(2),
                    },
                    FieldDefinition {
                        name: "email".to_string(),
                        field_type: TypeExpression::Identifier {
                            name: "Email".to_string(),
                        },
                        optional: true,
                        default: None,
                        config: json!({}),
                        entity_id: Some(3),
                    },
                ],
                config: json!({}),
                entity_id: Some(10),
            },
        );

        Schema {
            models,
            type_aliases,
        }
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
        schema.models.get_mut("User").unwrap().fields.push(FieldDefinition {
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
        assert_eq!(format_name("UserProfile", "preserve", &utils), "UserProfile");
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
}
