use cdm_plugin_interface::{CaseFormat, OutputFile, Schema, Utils, JSON};
use std::collections::HashMap;

use crate::type_mapper::map_type_to_typescript;
use crate::zod_mapper::map_type_to_zod;

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
    generate_zod: bool,
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
            generate_zod: json
                .get("generate_zod")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
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
    let mut zod_content = String::new();

    // Check if any models need Zod schemas
    let needs_zod = schema.models.iter().any(|(_, model)| {
        !should_skip_model(&model.config) && should_generate_zod(&model.config, cfg.generate_zod)
    });

    // Add Zod import if needed
    if needs_zod {
        content.push_str("import { z } from 'zod';\n\n");
    }

    // Generate type aliases first
    for (name, alias) in &schema.type_aliases {
        if should_skip_type_alias(alias) {
            continue;
        }

        let formatted_name = format_name(&name, &cfg.type_name_format, utils);
        let type_str = map_type_to_typescript(&alias.alias_type, cfg.strict_nulls);

        let export = if cfg.export_all { "export " } else { "" };
        content.push_str(&format!("{}type {} = {};\n\n", export, formatted_name, type_str));

        // Generate Zod schema for type alias if Zod is enabled
        if needs_zod {
            zod_content.push_str(&generate_type_alias_zod_schema(&formatted_name, alias, &cfg));
            zod_content.push_str("\n\n");
        }
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

        // Generate Zod schema if enabled for this model
        if should_generate_zod(model_config, cfg.generate_zod) {
            zod_content.push_str(&generate_zod_schema(&formatted_name, model, &cfg, utils));
            zod_content.push_str("\n\n");
        }
    }

    // Append Zod schemas after type definitions
    if !zod_content.is_empty() {
        content.push('\n');
        content.push_str(&zod_content);
    }

    vec![OutputFile {
        path: cfg.single_file_name.clone(),
        content,
    }]
}

fn build_per_model_files(schema: Schema, cfg: Config, utils: &Utils) -> Vec<OutputFile> {
    let mut files: HashMap<String, String> = HashMap::new();
    let mut model_to_file: HashMap<String, String> = HashMap::new();
    // Track which files need Zod import
    let mut files_needing_zod: std::collections::HashSet<String> = std::collections::HashSet::new();

    // First pass: determine which models go to which files and which need Zod
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

        // Track if this file needs Zod import
        if should_generate_zod(model_config, cfg.generate_zod) {
            files_needing_zod.insert(file_name);
        }
    }

    // Check if any models need Zod schemas
    let needs_zod = schema.models.iter().any(|(_, model)| {
        !should_skip_model(&model.config) && should_generate_zod(&model.config, cfg.generate_zod)
    });

    // Second pass: generate type aliases in a shared file
    if !schema.type_aliases.is_empty() {
        let mut types_content = String::new();
        let mut types_zod_content = String::new();

        // Add Zod import if needed
        if needs_zod {
            types_content.push_str("import { z } from 'zod';\n\n");
        }

        for (name, alias) in &schema.type_aliases {
            if should_skip_type_alias(alias) {
                continue;
            }

            let formatted_name = format_name(&name, &cfg.type_name_format, utils);
            let type_str = map_type_to_typescript(&alias.alias_type, cfg.strict_nulls);

            let export = if cfg.export_all { "export " } else { "" };
            types_content.push_str(&format!("{}type {} = {};\n\n", export, formatted_name, type_str));

            // Generate Zod schema for type alias if Zod is enabled
            if needs_zod {
                types_zod_content.push_str(&generate_type_alias_zod_schema(&formatted_name, alias, &cfg));
                types_zod_content.push_str("\n\n");
            }
        }

        // Append Zod schemas after type definitions
        if !types_zod_content.is_empty() {
            types_content.push_str(&types_zod_content);
        }

        if !types_content.is_empty() {
            files.insert("types.ts".to_string(), types_content);
        }
    }

    // Add Zod imports to files that need them
    for file_name in &files_needing_zod {
        if let Some(content) = files.get_mut(file_name) {
            let import_line = "import { z } from 'zod';\n\n";
            *content = format!("{}{}", import_line, content);
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

        // Generate Zod schema if enabled for this model
        if should_generate_zod(model_config, cfg.generate_zod) {
            content.push('\n');
            content.push_str(&generate_zod_schema(&formatted_name, model, &cfg, utils));
            content.push('\n');
        }
    }

    // Convert HashMap to Vec<OutputFile>
    files
        .into_iter()
        .map(|(path, content)| OutputFile { path, content })
        .collect()
}

fn generate_interface(
    name: &str,
    model: &cdm_plugin_interface::ModelDefinition,
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
    model: &cdm_plugin_interface::ModelDefinition,
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
    model: &cdm_plugin_interface::ModelDefinition,
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

fn should_skip_type_alias(alias: &cdm_plugin_interface::TypeAliasDefinition) -> bool {
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

fn get_field_type(field_config: &serde_json::Value, default_type: &cdm_plugin_interface::TypeExpression, strict_nulls: bool) -> String {
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

/// Determines if a model should have a Zod schema generated.
/// Model-level setting overrides global setting.
fn should_generate_zod(model_config: &serde_json::Value, global_generate_zod: bool) -> bool {
    model_config
        .get("generate_zod")
        .and_then(|v| v.as_bool())
        .unwrap_or(global_generate_zod)
}

/// Generates a Zod schema for a model
fn generate_zod_schema(
    name: &str,
    model: &cdm_plugin_interface::ModelDefinition,
    cfg: &Config,
    utils: &Utils,
) -> String {
    let mut result = String::new();

    let export = if cfg.export_all { "export " } else { "" };
    result.push_str(&format!(
        "{}const {}Schema: z.ZodType<{}> = z.object({{\n",
        export, name, name
    ));

    for field in &model.fields {
        let field_config = &field.config;

        if should_skip_field(field_config) {
            continue;
        }

        let field_name = get_field_name(field_config, &field.name, &cfg.field_name_format, utils);
        let zod_type = get_field_zod_type(field_config, &field.field_type, cfg.strict_nulls);

        // Handle optional fields
        let final_type = if field.optional {
            format!("{}.optional()", zod_type)
        } else {
            zod_type
        };

        result.push_str(&format!("  {}: {},\n", field_name, final_type));
    }

    result.push_str("});");
    result
}

/// Generates a Zod schema for a type alias
fn generate_type_alias_zod_schema(
    name: &str,
    alias: &cdm_plugin_interface::TypeAliasDefinition,
    cfg: &Config,
) -> String {
    let export = if cfg.export_all { "export " } else { "" };
    let zod_type = map_type_to_zod(&alias.alias_type, cfg.strict_nulls);
    format!("{}const {}Schema = {};", export, name, zod_type)
}

/// Gets the Zod type for a field, respecting type_override if present
fn get_field_zod_type(
    field_config: &serde_json::Value,
    default_type: &cdm_plugin_interface::TypeExpression,
    strict_nulls: bool,
) -> String {
    // If there's a type_override, we can't generate accurate Zod - use z.any()
    if field_config.get("type_override").is_some() {
        return "z.any()".to_string();
    }
    map_type_to_zod(default_type, strict_nulls)
}


#[cfg(test)]
#[path = "build/build_tests.rs"]
mod build_tests;
