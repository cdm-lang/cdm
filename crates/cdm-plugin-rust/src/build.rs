use cdm_plugin_interface::{CaseFormat, OutputFile, Schema, TypeExpression, Utils, JSON};
#[cfg(test)]
use std::collections::HashMap;

use crate::type_mapper::{
    is_string_literal_union, is_type_reference_union, is_union_type, map_type_to_rust,
    TypeMapperConfig,
};

#[derive(Debug, Clone)]
struct Config {
    file_strategy: String,
    single_file_name: String,
    derive_macros: Vec<String>,
    serde_support: bool,
    type_name_format: String,
    field_name_format: String,
    number_type: String,
    map_type: String,
    visibility: String,
    allow_unused_imports: bool,
}

impl Config {
    fn from_json(json: &JSON) -> Self {
        let derive_macros_str = json
            .get("derive_macros")
            .and_then(|v| v.as_str())
            .unwrap_or("Debug, Clone, Serialize, Deserialize");

        let derive_macros: Vec<String> = derive_macros_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Self {
            file_strategy: json
                .get("file_strategy")
                .and_then(|v| v.as_str())
                .unwrap_or("single")
                .to_string(),
            single_file_name: json
                .get("single_file_name")
                .and_then(|v| v.as_str())
                .unwrap_or("types.rs")
                .to_string(),
            derive_macros,
            serde_support: json
                .get("serde_support")
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
                .unwrap_or("snake")
                .to_string(),
            number_type: json
                .get("number_type")
                .and_then(|v| v.as_str())
                .unwrap_or("f64")
                .to_string(),
            map_type: json
                .get("map_type")
                .and_then(|v| v.as_str())
                .unwrap_or("HashMap")
                .to_string(),
            visibility: json
                .get("visibility")
                .and_then(|v| v.as_str())
                .unwrap_or("pub")
                .to_string(),
            allow_unused_imports: json
                .get("allow_unused_imports")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        }
    }

    fn type_mapper_config(&self) -> TypeMapperConfig {
        TypeMapperConfig {
            number_type: self.number_type.clone(),
            map_type: self.map_type.clone(),
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

    // Generate use statements
    content.push_str(&generate_use_statements(&schema, &cfg));

    // Generate type aliases (sorted for deterministic output)
    let mut sorted_aliases: Vec<_> = schema.type_aliases.iter().collect();
    sorted_aliases.sort_by_key(|(name, _)| (*name).clone());

    for (name, alias) in &sorted_aliases {
        if should_skip_type_alias(&alias.config) {
            continue;
        }

        let formatted_name = get_type_alias_export_name(&alias.config, name, &cfg, utils);
        content.push_str(&generate_type_alias_code(
            &formatted_name,
            &alias.alias_type,
            &cfg,
            utils,
        ));
        content.push('\n');
    }

    // Generate models (sorted for deterministic output)
    let mut sorted_models: Vec<_> = schema.models.iter().collect();
    sorted_models.sort_by_key(|(name, _)| (*name).clone());

    for (name, model) in &sorted_models {
        let model_config = &model.config;

        if should_skip_model(model_config) {
            continue;
        }

        let formatted_name = get_struct_name(model_config, name, &cfg, utils);

        // Generate inline enums for union-typed fields
        let inline_enums = collect_inline_enums(name, model, &schema, &cfg, utils);
        for enum_code in &inline_enums {
            content.push_str(enum_code);
            content.push('\n');
        }

        content.push_str(&generate_struct(&formatted_name, model, &cfg, utils, &schema));
        content.push('\n');
    }

    // Remove trailing newline
    if content.ends_with('\n') {
        content.pop();
    }

    vec![OutputFile {
        path: cfg.single_file_name.clone(),
        content,
    }]
}

fn build_per_model_files(schema: Schema, cfg: Config, utils: &Utils) -> Vec<OutputFile> {
    let mut files: Vec<OutputFile> = Vec::new();
    let mut module_names: Vec<String> = Vec::new();

    // Generate type aliases in types.rs
    let mut types_content = String::new();
    let has_type_aliases = schema
        .type_aliases
        .iter()
        .any(|(_, alias)| !should_skip_type_alias(&alias.config));

    if has_type_aliases {
        types_content.push_str(&generate_use_statements_for_types(&schema, &cfg));

        let mut sorted_aliases: Vec<_> = schema.type_aliases.iter().collect();
        sorted_aliases.sort_by_key(|(name, _)| (*name).clone());

        for (name, alias) in &sorted_aliases {
            if should_skip_type_alias(&alias.config) {
                continue;
            }

            let formatted_name = get_type_alias_export_name(&alias.config, name, &cfg, utils);
            types_content.push_str(&generate_type_alias_code(
                &formatted_name,
                &alias.alias_type,
                &cfg,
                utils,
            ));
            types_content.push('\n');
        }

        if types_content.ends_with('\n') {
            types_content.pop();
        }

        files.push(OutputFile {
            path: "types.rs".to_string(),
            content: types_content,
        });
        module_names.push("types".to_string());
    }

    // Generate each model in its own file
    let mut sorted_models: Vec<_> = schema.models.iter().collect();
    sorted_models.sort_by_key(|(name, _)| (*name).clone());

    for (name, model) in &sorted_models {
        let model_config = &model.config;

        if should_skip_model(model_config) {
            continue;
        }

        let formatted_name = get_struct_name(model_config, name, &cfg, utils);
        let file_name = get_file_name(model_config, name, utils);
        let module_name = file_name.trim_end_matches(".rs").to_string();
        module_names.push(module_name);

        let mut model_content = String::new();
        if cfg.allow_unused_imports {
            model_content.push_str("#[allow(unused_imports)]\n");
        }
        model_content.push_str("use super::*;\n\n");

        // Generate inline enums
        let inline_enums = collect_inline_enums(name, model, &schema, &cfg, utils);
        for enum_code in &inline_enums {
            model_content.push_str(enum_code);
            model_content.push('\n');
        }

        model_content.push_str(&generate_struct(&formatted_name, model, &cfg, utils, &schema));

        files.push(OutputFile {
            path: file_name,
            content: model_content,
        });
    }

    // Generate mod.rs
    let mut mod_content = String::new();
    mod_content.push_str(&generate_use_statements(&schema, &cfg));

    for module_name in &module_names {
        mod_content.push_str(&format!("mod {};\n", module_name));
    }
    if !module_names.is_empty() {
        mod_content.push('\n');
    }
    for module_name in &module_names {
        mod_content.push_str(&format!("pub use {}::*;\n", module_name));
    }

    files.push(OutputFile {
        path: "mod.rs".to_string(),
        content: mod_content,
    });

    files
}

fn generate_use_statements(schema: &Schema, cfg: &Config) -> String {
    let mut result = String::new();
    let mut needs_serde = false;
    let mut needs_map = false;
    let mut needs_json = false;

    if cfg.serde_support && has_serde_derives(&cfg.derive_macros) {
        needs_serde = true;
    }

    // Check all fields for map and JSON types
    for (_, model) in &schema.models {
        for field in &model.fields {
            check_type_needs(&field.field_type, &mut needs_map, &mut needs_json);
        }
    }
    for (_, alias) in &schema.type_aliases {
        check_type_needs(&alias.alias_type, &mut needs_map, &mut needs_json);
    }

    let allow_attr = if cfg.allow_unused_imports {
        "#[allow(unused_imports)]\n"
    } else {
        ""
    };

    if needs_serde {
        result.push_str(allow_attr);
        result.push_str("use serde::{Serialize, Deserialize};\n");
    }
    if needs_map {
        result.push_str(allow_attr);
        result.push_str(&format!("use std::collections::{};\n", cfg.map_type));
    }
    if needs_json {
        // serde_json::Value is used with full path, no import needed
    }

    if !result.is_empty() {
        result.push('\n');
    }

    result
}

fn generate_use_statements_for_types(schema: &Schema, cfg: &Config) -> String {
    let mut result = String::new();
    let mut needs_serde = false;
    let mut needs_map = false;

    if cfg.serde_support && has_serde_derives(&cfg.derive_macros) {
        // Check if any type alias is a union (which generates an enum needing serde)
        for (_, alias) in &schema.type_aliases {
            if !should_skip_type_alias(&alias.config) && is_union_type(&alias.alias_type) {
                needs_serde = true;
                break;
            }
        }
    }

    for (_, alias) in &schema.type_aliases {
        let mut _needs_json = false;
        check_type_needs(&alias.alias_type, &mut needs_map, &mut _needs_json);
    }

    let allow_attr = if cfg.allow_unused_imports {
        "#[allow(unused_imports)]\n"
    } else {
        ""
    };

    if needs_serde {
        result.push_str(allow_attr);
        result.push_str("use serde::{Serialize, Deserialize};\n");
    }
    if needs_map {
        result.push_str(allow_attr);
        result.push_str(&format!("use std::collections::{};\n", cfg.map_type));
    }

    if !result.is_empty() {
        result.push('\n');
    }

    result
}

fn check_type_needs(type_expr: &TypeExpression, needs_map: &mut bool, needs_json: &mut bool) {
    match type_expr {
        TypeExpression::Identifier { name } => {
            if name == "JSON" {
                *needs_json = true;
            }
        }
        TypeExpression::Array { element_type } => {
            check_type_needs(element_type, needs_map, needs_json);
        }
        TypeExpression::Map {
            value_type,
            key_type,
        } => {
            *needs_map = true;
            check_type_needs(key_type, needs_map, needs_json);
            check_type_needs(value_type, needs_map, needs_json);
        }
        TypeExpression::Union { types } => {
            for t in types {
                check_type_needs(t, needs_map, needs_json);
            }
        }
        TypeExpression::StringLiteral { .. } | TypeExpression::NumberLiteral { .. } => {}
    }
}

fn has_serde_derives(derive_macros: &[String]) -> bool {
    derive_macros
        .iter()
        .any(|m| m == "Serialize" || m == "Deserialize")
}

fn generate_type_alias_code(
    name: &str,
    alias_type: &TypeExpression,
    cfg: &Config,
    utils: &Utils,
) -> String {
    let vis = visibility_prefix(&cfg.visibility);

    if is_string_literal_union(alias_type) {
        // Generate a Rust enum for string literal unions
        return generate_string_literal_enum(name, alias_type, cfg, utils);
    }

    if is_type_reference_union(alias_type) {
        // Generate a Rust enum with newtype variants
        return generate_type_reference_enum(name, alias_type, cfg, utils);
    }

    // Simple type alias
    let tm_config = cfg.type_mapper_config();
    let rust_type = map_type_to_rust(alias_type, &tm_config);
    format!("{}type {} = {};\n", vis, name, rust_type)
}

fn generate_string_literal_enum(
    name: &str,
    type_expr: &TypeExpression,
    cfg: &Config,
    utils: &Utils,
) -> String {
    let mut result = String::new();
    let vis = visibility_prefix(&cfg.visibility);

    // Derive attribute
    result.push_str(&generate_derive_attr(&cfg.derive_macros));

    result.push_str(&format!("{}enum {} {{\n", vis, name));

    if let TypeExpression::Union { types } = type_expr {
        for t in types {
            if let TypeExpression::StringLiteral { value } = t {
                let variant_name = format_name(value, "pascal", utils);
                if cfg.serde_support {
                    result.push_str(&format!(
                        "    #[serde(rename = \"{}\")]\n",
                        value
                    ));
                }
                result.push_str(&format!("    {},\n", variant_name));
            }
        }
    }

    result.push_str("}\n");
    result
}

fn generate_type_reference_enum(
    name: &str,
    type_expr: &TypeExpression,
    cfg: &Config,
    _utils: &Utils,
) -> String {
    let mut result = String::new();
    let vis = visibility_prefix(&cfg.visibility);

    // Derive attribute
    result.push_str(&generate_derive_attr(&cfg.derive_macros));

    if cfg.serde_support {
        result.push_str("#[serde(untagged)]\n");
    }

    result.push_str(&format!("{}enum {} {{\n", vis, name));

    if let TypeExpression::Union { types } = type_expr {
        for t in types {
            if let TypeExpression::Identifier { name: type_name } = t {
                result.push_str(&format!("    {}({}),\n", type_name, type_name));
            }
        }
    }

    result.push_str("}\n");
    result
}

fn generate_derive_attr(derive_macros: &[String]) -> String {
    if derive_macros.is_empty() {
        return String::new();
    }
    format!("#[derive({})]\n", derive_macros.join(", "))
}

fn generate_struct(
    name: &str,
    model: &cdm_plugin_interface::ModelDefinition,
    cfg: &Config,
    utils: &Utils,
    schema: &Schema,
) -> String {
    let mut result = String::new();
    let model_config = &model.config;

    let model_visibility = get_model_visibility(model_config, &cfg.visibility);
    let vis = visibility_prefix(&model_visibility);

    let model_derive_macros = get_model_derive_macros(model_config, &cfg.derive_macros);

    // Derive attribute
    result.push_str(&generate_derive_attr(&model_derive_macros));

    result.push_str(&format!("{}struct {} {{\n", vis, name));

    let tm_config = cfg.type_mapper_config();

    for field in &model.fields {
        let field_config = &field.config;

        if should_skip_field(field_config) {
            continue;
        }

        let field_name = get_field_name(field_config, &field.name, &cfg.field_name_format, utils);
        let field_vis = get_field_visibility(field_config, &model_visibility);
        let field_vis_prefix = visibility_prefix(&field_vis);

        // Serde rename if field name differs from original CDM name
        if cfg.serde_support && field_name != field.name {
            result.push_str(&format!("    #[serde(rename = \"{}\")]\n", field.name));
        }

        // Serde rename from explicit config
        if cfg.serde_support {
            if let Some(serde_rename) = field_config.get("serde_rename").and_then(|v| v.as_str()) {
                // If we already added a rename from field_name diff, replace it
                if field_name != field.name {
                    // Remove the last line (the auto-generated rename)
                    if let Some(pos) = result.rfind("    #[serde(rename = ") {
                        result.truncate(pos);
                    }
                }
                result.push_str(&format!("    #[serde(rename = \"{}\")]\n", serde_rename));
            }
        }

        // Determine the type string
        let type_str = get_field_type_str(field_config, &field.field_type, &tm_config, name, &field.name, schema, cfg, utils);

        // Wrap in Option if optional
        let final_type = if field.optional {
            format!("Option<{}>", type_str)
        } else {
            type_str
        };

        result.push_str(&format!(
            "    {}{}: {},\n",
            field_vis_prefix, field_name, final_type
        ));
    }

    result.push_str("}\n");
    result
}

fn get_field_type_str(
    field_config: &serde_json::Value,
    field_type: &TypeExpression,
    tm_config: &TypeMapperConfig,
    model_name: &str,
    field_name: &str,
    schema: &Schema,
    cfg: &Config,
    utils: &Utils,
) -> String {
    // Check for type_override
    if let Some(type_override) = field_config.get("type_override").and_then(|v| v.as_str()) {
        return type_override.to_string();
    }

    // If the field type is a union, use the generated enum name
    if is_union_type(field_type) {
        let enum_name = generate_inline_enum_name(model_name, field_name, cfg, utils);
        return enum_name;
    }

    // Check if the field references a type alias that is a union
    // In that case, the type alias name is used directly (enum was already generated)
    if let TypeExpression::Identifier { name } = field_type {
        if let Some(alias) = schema.type_aliases.get(name) {
            if is_union_type(&alias.alias_type) {
                let formatted = get_type_alias_export_name(&alias.config, name, cfg, utils);
                return formatted;
            }
        }
    }

    map_type_to_rust(field_type, tm_config)
}

fn generate_inline_enum_name(
    model_name: &str,
    field_name: &str,
    _cfg: &Config,
    utils: &Utils,
) -> String {
    let pascal_field = utils.change_case(field_name, CaseFormat::Pascal);
    format!("{}{}", model_name, pascal_field)
}

fn collect_inline_enums(
    model_name: &str,
    model: &cdm_plugin_interface::ModelDefinition,
    schema: &Schema,
    cfg: &Config,
    utils: &Utils,
) -> Vec<String> {
    let mut enums = Vec::new();

    for field in &model.fields {
        if should_skip_field(&field.config) {
            continue;
        }

        // Skip if type_override is set
        if field.config.get("type_override").is_some() {
            continue;
        }

        if is_union_type(&field.field_type) {
            // Check this isn't referencing a named type alias
            if let TypeExpression::Identifier { name } = &field.field_type {
                if schema.type_aliases.contains_key(name) {
                    continue;
                }
            }

            let enum_name = generate_inline_enum_name(model_name, &field.name, cfg, utils);

            if is_string_literal_union(&field.field_type) {
                enums.push(generate_string_literal_enum(
                    &enum_name,
                    &field.field_type,
                    cfg,
                    utils,
                ));
            } else if is_type_reference_union(&field.field_type) {
                enums.push(generate_type_reference_enum(
                    &enum_name,
                    &field.field_type,
                    cfg,
                    utils,
                ));
            }
        }
    }

    enums
}

// Helper functions

fn should_skip_type_alias(config: &serde_json::Value) -> bool {
    config
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

fn visibility_prefix(visibility: &str) -> String {
    match visibility {
        "pub" => "pub ".to_string(),
        "pub_crate" => "pub(crate) ".to_string(),
        "private" => String::new(),
        _ => "pub ".to_string(),
    }
}

fn get_model_visibility(model_config: &serde_json::Value, default: &str) -> String {
    model_config
        .get("visibility")
        .and_then(|v| v.as_str())
        .unwrap_or(default)
        .to_string()
}

fn get_model_derive_macros(model_config: &serde_json::Value, default: &[String]) -> Vec<String> {
    model_config
        .get("derive_macros")
        .and_then(|v| v.as_str())
        .map(|s| {
            s.split(',')
                .map(|m| m.trim().to_string())
                .filter(|m| !m.is_empty())
                .collect()
        })
        .unwrap_or_else(|| default.to_vec())
}

fn get_field_visibility(field_config: &serde_json::Value, model_visibility: &str) -> String {
    field_config
        .get("visibility")
        .and_then(|v| v.as_str())
        .unwrap_or(model_visibility)
        .to_string()
}

fn get_struct_name(
    model_config: &serde_json::Value,
    default_name: &str,
    cfg: &Config,
    utils: &Utils,
) -> String {
    model_config
        .get("struct_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format_name(default_name, &cfg.type_name_format, utils))
}

fn get_type_alias_export_name(
    config: &serde_json::Value,
    default_name: &str,
    cfg: &Config,
    utils: &Utils,
) -> String {
    config
        .get("export_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format_name(default_name, &cfg.type_name_format, utils))
}

fn get_field_name(
    field_config: &serde_json::Value,
    default_name: &str,
    format: &str,
    utils: &Utils,
) -> String {
    field_config
        .get("field_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format_name(default_name, format, utils))
}

fn get_file_name(
    model_config: &serde_json::Value,
    model_name: &str,
    utils: &Utils,
) -> String {
    model_config
        .get("file_name")
        .and_then(|v| v.as_str())
        .map(|s| {
            if s.ends_with(".rs") {
                s.to_string()
            } else {
                format!("{}.rs", s)
            }
        })
        .unwrap_or_else(|| {
            format!("{}.rs", utils.change_case(model_name, CaseFormat::Snake))
        })
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


#[cfg(test)]
#[path = "build/build_tests.rs"]
mod build_tests;
