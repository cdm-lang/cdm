use cdm_plugin_interface::{CaseFormat, FieldDefinition, ModelDefinition, OutputFile, Schema, Utils, JSON};
use std::collections::BTreeSet;

use crate::decorator_builder::{build_join_column, build_join_table, DecoratorBuilder};
use crate::type_mapper::TypeMapper;

#[derive(Debug, Clone)]
struct Config {
    entity_file_strategy: String,
    entities_file_name: String,
    table_name_format: String,
    column_name_format: String,
    pluralize_table_names: bool,
    typeorm_import_path: String,
}

impl Config {
    fn from_json(json: &JSON) -> Self {
        Self {
            entity_file_strategy: json
                .get("entity_file_strategy")
                .and_then(|v| v.as_str())
                .unwrap_or("per_model")
                .to_string(),
            entities_file_name: json
                .get("entities_file_name")
                .and_then(|v| v.as_str())
                .unwrap_or("entities.ts")
                .to_string(),
            table_name_format: json
                .get("table_name_format")
                .and_then(|v| v.as_str())
                .unwrap_or("snake_case")
                .to_string(),
            column_name_format: json
                .get("column_name_format")
                .and_then(|v| v.as_str())
                .unwrap_or("snake_case")
                .to_string(),
            pluralize_table_names: json
                .get("pluralize_table_names")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            typeorm_import_path: json
                .get("typeorm_import_path")
                .and_then(|v| v.as_str())
                .unwrap_or("typeorm")
                .to_string(),
        }
    }
}

/// Tracks TypeORM imports needed for an entity
#[derive(Debug, Default)]
struct ImportCollector {
    typeorm_imports: BTreeSet<String>,
    entity_imports: BTreeSet<String>,
}

impl ImportCollector {
    fn new() -> Self {
        Self::default()
    }

    fn add_typeorm(&mut self, name: &str) {
        self.typeorm_imports.insert(name.to_string());
    }

    fn add_entity(&mut self, name: &str) {
        self.entity_imports.insert(name.to_string());
    }

    fn to_import_statements(&self, typeorm_path: &str, current_entity: &str) -> String {
        let mut result = String::new();

        // TypeORM imports
        if !self.typeorm_imports.is_empty() {
            let imports: Vec<&String> = self.typeorm_imports.iter().collect();
            result.push_str(&format!(
                "import {{ {} }} from \"{}\"\n",
                imports.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "),
                typeorm_path
            ));
        }

        // Entity imports (for relations)
        for entity in &self.entity_imports {
            if entity != current_entity {
                result.push_str(&format!("import {{ {} }} from \"./{}\"\n", entity, entity));
            }
        }

        if !result.is_empty() {
            result.push('\n');
        }

        result
    }
}

pub fn build(schema: Schema, config: JSON, utils: &Utils) -> Vec<OutputFile> {
    let cfg = Config::from_json(&config);
    let model_names: Vec<String> = schema.models.keys().cloned().collect();
    let type_mapper = TypeMapper::new(&config, &schema.type_aliases, model_names);

    match cfg.entity_file_strategy.as_str() {
        "single" => build_single_file(&schema, cfg, utils, &type_mapper),
        "per_model" => build_per_model_files(&schema, cfg, utils, &type_mapper),
        _ => vec![],
    }
}

fn build_single_file(
    schema: &Schema,
    cfg: Config,
    utils: &Utils,
    type_mapper: &TypeMapper,
) -> Vec<OutputFile> {
    let mut imports = ImportCollector::new();
    let mut entities_content = String::new();

    // Generate all entities
    for (name, model) in &schema.models {
        if should_skip_model(&model.config) {
            continue;
        }

        let entity_code = generate_entity(name, model, &cfg, utils, type_mapper, &mut imports);
        entities_content.push_str(&entity_code);
        entities_content.push_str("\n\n");
    }

    // Build final content with imports
    let import_statements = imports.to_import_statements(&cfg.typeorm_import_path, "");
    let content = format!("{}{}", import_statements, entities_content.trim_end());

    vec![OutputFile {
        path: cfg.entities_file_name,
        content,
    }]
}

fn build_per_model_files(
    schema: &Schema,
    cfg: Config,
    utils: &Utils,
    type_mapper: &TypeMapper,
) -> Vec<OutputFile> {
    let mut files = Vec::new();

    for (name, model) in &schema.models {
        if should_skip_model(&model.config) {
            continue;
        }

        let mut imports = ImportCollector::new();
        let entity_code = generate_entity(name, model, &cfg, utils, type_mapper, &mut imports);

        let import_statements = imports.to_import_statements(&cfg.typeorm_import_path, name);
        let content = format!("{}{}", import_statements, entity_code);

        files.push(OutputFile {
            path: format!("{}.ts", name),
            content,
        });
    }

    files
}

fn generate_entity(
    name: &str,
    model: &ModelDefinition,
    cfg: &Config,
    utils: &Utils,
    type_mapper: &TypeMapper,
    imports: &mut ImportCollector,
) -> String {
    let mut result = String::new();

    // Add Entity import
    imports.add_typeorm("Entity");

    // Generate entity decorator
    let table_name = get_table_name(name, &model.config, cfg, utils);
    let mut entity_decorator = DecoratorBuilder::entity();

    // Only add name option if it differs from the default
    let default_table_name = if cfg.pluralize_table_names {
        utils.pluralize(&apply_name_format(name, &cfg.table_name_format, utils))
    } else {
        apply_name_format(name, &cfg.table_name_format, utils)
    };

    if table_name != default_table_name {
        entity_decorator = entity_decorator.string_option("name", &table_name);
    } else {
        entity_decorator = entity_decorator.string_option("name", &table_name);
    }

    result.push_str(&entity_decorator.build());
    result.push('\n');

    // Generate index decorators at class level
    if let Some(indexes) = model.config.get("indexes").and_then(|v| v.as_array()) {
        for index in indexes {
            if let Some(fields) = index.get("fields").and_then(|v| v.as_array()) {
                let field_names: Vec<String> = fields
                    .iter()
                    .filter_map(|f| f.as_str())
                    .map(|s| s.to_string())
                    .collect();

                if !field_names.is_empty() {
                    imports.add_typeorm("Index");
                    let mut index_builder = DecoratorBuilder::index(&field_names);

                    if let Some(true) = index.get("unique").and_then(|v| v.as_bool()) {
                        index_builder = index_builder.bool_option("unique", true);
                    }

                    result.push_str(&index_builder.build());
                    result.push('\n');
                }
            }
        }
    }

    // Generate class
    result.push_str(&format!("export class {} {{\n", name));

    // Generate fields
    for field in &model.fields {
        if should_skip_field(&field.config) {
            continue;
        }

        let field_code = generate_field(field, name, cfg, utils, type_mapper, imports);
        result.push_str(&field_code);
    }

    result.push('}');
    result
}

fn generate_field(
    field: &FieldDefinition,
    model_name: &str,
    cfg: &Config,
    utils: &Utils,
    type_mapper: &TypeMapper,
    imports: &mut ImportCollector,
) -> String {
    // Check for relation config first
    if let Some(relation) = field.config.get("relation") {
        return generate_relation_field(field, relation, model_name, type_mapper, imports);
    }

    // Check for primary key config
    if let Some(primary) = field.config.get("primary") {
        return generate_primary_field(field, primary, cfg, utils, type_mapper, imports);
    }

    // Regular column
    generate_column_field(field, cfg, utils, type_mapper, imports)
}

fn generate_primary_field(
    field: &FieldDefinition,
    primary_config: &JSON,
    _cfg: &Config,
    _utils: &Utils,
    type_mapper: &TypeMapper,
    imports: &mut ImportCollector,
) -> String {
    let mut result = String::new();

    let generation = primary_config
        .get("generation")
        .and_then(|v| v.as_str());

    let decorator = if let Some(gen_strategy) = generation {
        imports.add_typeorm("PrimaryGeneratedColumn");
        DecoratorBuilder::primary_generated_column(Some(gen_strategy))
    } else {
        imports.add_typeorm("PrimaryColumn");
        DecoratorBuilder::primary_column()
    };

    result.push_str("    ");
    result.push_str(&decorator.build());
    result.push('\n');

    // Property declaration
    let ts_type = type_mapper.map_to_typescript_type(&field.field_type);
    let optional_marker = if field.optional { "?" } else { "" };
    result.push_str(&format!("    {}{}: {}\n\n", field.name, optional_marker, ts_type));

    result
}

fn generate_column_field(
    field: &FieldDefinition,
    cfg: &Config,
    utils: &Utils,
    type_mapper: &TypeMapper,
    imports: &mut ImportCollector,
) -> String {
    let mut result = String::new();

    imports.add_typeorm("Column");

    let mut column_builder = DecoratorBuilder::column();

    // Column name override
    let column_name = get_column_name(&field.name, &field.config, cfg, utils);
    let default_column_name = apply_name_format(&field.name, &cfg.column_name_format, utils);
    if column_name != default_column_name {
        column_builder = column_builder.string_option("name", &column_name);
    }

    // Type override
    if let Some(type_override) = field.config.get("type").and_then(|v| v.as_str()) {
        column_builder = column_builder.string_option("type", type_override);
    }

    // Nullable (inferred from optional unless explicitly set)
    let nullable = field.config
        .get("nullable")
        .and_then(|v| v.as_bool())
        .unwrap_or(field.optional);
    if nullable {
        column_builder = column_builder.bool_option("nullable", true);
    }

    // Unique
    if let Some(true) = field.config.get("unique").and_then(|v| v.as_bool()) {
        column_builder = column_builder.bool_option("unique", true);
    }

    // Default
    if let Some(default) = field.config.get("default").and_then(|v| v.as_str()) {
        column_builder = column_builder.string_option("default", default);
    }

    // Length
    if let Some(length) = field.config.get("length").and_then(|v| v.as_i64()) {
        column_builder = column_builder.number_option("length", length);
    }

    // Array
    if let Some(true) = field.config.get("array").and_then(|v| v.as_bool()) {
        column_builder = column_builder.bool_option("array", true);
    }

    result.push_str("    ");
    result.push_str(&column_builder.build());
    result.push('\n');

    // Property declaration
    let ts_type = type_mapper.map_to_typescript_type(&field.field_type);
    let optional_marker = if field.optional { "?" } else { "" };
    result.push_str(&format!("    {}{}: {}\n\n", field.name, optional_marker, ts_type));

    result
}

fn generate_relation_field(
    field: &FieldDefinition,
    relation_config: &JSON,
    _model_name: &str,
    type_mapper: &TypeMapper,
    imports: &mut ImportCollector,
) -> String {
    let mut result = String::new();

    let relation_type = relation_config
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("many_to_one");

    let inverse_side = relation_config
        .get("inverse_side")
        .and_then(|v| v.as_str());

    // Get target entity from field type
    let target_entity = type_mapper
        .is_model_reference(&field.field_type)
        .unwrap_or_else(|| "Unknown".to_string());

    // Add entity import
    imports.add_entity(&target_entity);

    // Build relation decorator
    let relation_decorator = match relation_type {
        "one_to_one" => {
            imports.add_typeorm("OneToOne");
            DecoratorBuilder::one_to_one(&target_entity, inverse_side)
        }
        "one_to_many" => {
            imports.add_typeorm("OneToMany");
            let inv = inverse_side.unwrap_or("unknown");
            DecoratorBuilder::one_to_many(&target_entity, inv)
        }
        "many_to_one" => {
            imports.add_typeorm("ManyToOne");
            DecoratorBuilder::many_to_one(&target_entity, inverse_side)
        }
        "many_to_many" => {
            imports.add_typeorm("ManyToMany");
            DecoratorBuilder::many_to_many(&target_entity, inverse_side)
        }
        _ => DecoratorBuilder::many_to_one(&target_entity, inverse_side),
    };

    // Add relation options
    let cascade = relation_config.get("cascade").and_then(|v| v.as_bool());
    let eager = relation_config.get("eager").and_then(|v| v.as_bool());
    let lazy = relation_config.get("lazy").and_then(|v| v.as_bool());
    let nullable = relation_config.get("nullable").and_then(|v| v.as_bool());
    let on_delete = relation_config.get("on_delete").and_then(|v| v.as_str());
    let on_update = relation_config.get("on_update").and_then(|v| v.as_str());

    let relation_decorator =
        relation_decorator.with_relation_options(cascade, eager, lazy, nullable, on_delete, on_update);

    result.push_str("    ");
    result.push_str(&relation_decorator.build());
    result.push('\n');

    // Handle JoinColumn for owning side (ManyToOne, OneToOne)
    if let Some(join_column) = relation_config.get("join_column") {
        imports.add_typeorm("JoinColumn");
        let jc_name = join_column.get("name").and_then(|v| v.as_str());
        let jc_ref = join_column.get("referenced_column").and_then(|v| v.as_str());
        result.push_str("    ");
        result.push_str(&build_join_column(jc_name, jc_ref));
        result.push('\n');
    }

    // Handle JoinTable for ManyToMany
    if let Some(join_table) = relation_config.get("join_table") {
        imports.add_typeorm("JoinTable");
        let jt_name = join_table
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("junction");

        let jc = join_table.get("join_column");
        let ijc = join_table.get("inverse_join_column");

        let jc_name = jc.and_then(|c| c.get("name")).and_then(|v| v.as_str());
        let jc_ref = jc
            .and_then(|c| c.get("referenced_column"))
            .and_then(|v| v.as_str());
        let ijc_name = ijc.and_then(|c| c.get("name")).and_then(|v| v.as_str());
        let ijc_ref = ijc
            .and_then(|c| c.get("referenced_column"))
            .and_then(|v| v.as_str());

        result.push_str("    ");
        result.push_str(&build_join_table(jt_name, jc_name, jc_ref, ijc_name, ijc_ref));
        result.push('\n');
    }

    // Property declaration
    let ts_type = type_mapper.map_to_typescript_type(&field.field_type);
    let optional_marker = if field.optional { "?" } else { "" };
    result.push_str(&format!("    {}{}: {}\n\n", field.name, optional_marker, ts_type));

    result
}

// Helper functions

fn should_skip_model(config: &JSON) -> bool {
    config
        .get("skip")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn should_skip_field(config: &JSON) -> bool {
    config
        .get("skip")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn get_table_name(model_name: &str, model_config: &JSON, cfg: &Config, utils: &Utils) -> String {
    // Check for explicit table name override
    if let Some(table) = model_config.get("table").and_then(|v| v.as_str()) {
        return table.to_string();
    }

    // Apply naming format
    let formatted = apply_name_format(model_name, &cfg.table_name_format, utils);

    // Apply pluralization
    if cfg.pluralize_table_names {
        utils.pluralize(&formatted)
    } else {
        formatted
    }
}

fn get_column_name(field_name: &str, field_config: &JSON, cfg: &Config, utils: &Utils) -> String {
    // Check for explicit column name override
    if let Some(column) = field_config.get("column").and_then(|v| v.as_str()) {
        return column.to_string();
    }

    // Apply naming format
    apply_name_format(field_name, &cfg.column_name_format, utils)
}

fn apply_name_format(name: &str, format: &str, utils: &Utils) -> String {
    match format {
        "snake_case" => utils.change_case(name, CaseFormat::Snake),
        "camel_case" => utils.change_case(name, CaseFormat::Camel),
        "pascal_case" => utils.change_case(name, CaseFormat::Pascal),
        "preserve" => name.to_string(),
        _ => utils.change_case(name, CaseFormat::Snake),
    }
}

#[cfg(test)]
#[path = "build/build_tests.rs"]
mod build_tests;
