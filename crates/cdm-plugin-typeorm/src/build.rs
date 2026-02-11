use cdm_plugin_interface::{CaseFormat, FieldDefinition, ModelDefinition, OutputFile, Schema, Utils, JSON};
use std::collections::BTreeSet;

use crate::decorator_builder::{build_join_column, build_join_table, DecoratorBuilder};
use crate::type_mapper::{TsTypeInfo, TypeMapper};

#[derive(Debug, Clone)]
struct Config {
    entity_file_strategy: String,
    entities_file_name: String,
    table_name_format: String,
    column_name_format: String,
    pluralize_table_names: bool,
    typeorm_import_path: String,
    definite_assignment: bool,
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
            definite_assignment: json
                .get("definite_assignment")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
        }
    }
}

/// Resolves whether to use definite assignment assertion (!) for a field.
/// Precedence: Field config → Model config → Global config
fn should_use_definite_assignment(
    field: &FieldDefinition,
    model_config: &JSON,
    global_setting: bool,
) -> bool {
    // Field level takes precedence
    if let Some(field_setting) = field.config.get("definite_assignment").and_then(|v| v.as_bool()) {
        return field_setting;
    }
    // Then model level
    if let Some(model_setting) = model_config.get("definite_assignment").and_then(|v| v.as_bool()) {
        return model_setting;
    }
    // Fall back to global
    global_setting
}

/// Tracks TypeORM imports needed for an entity
#[derive(Debug, Default)]
struct ImportCollector {
    typeorm_imports: BTreeSet<String>,
    entity_imports: BTreeSet<String>,
    /// Custom imports for hook functions: (function_name, import_path)
    hook_imports: BTreeSet<(String, String)>,
    /// Custom type imports: (type_name, import_path, is_default_import)
    type_imports: Vec<(String, String, bool)>,
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

    fn add_hook_import(&mut self, function_name: &str, import_path: &str) {
        self.hook_imports
            .insert((function_name.to_string(), import_path.to_string()));
    }

    fn add_type_import(&mut self, type_name: &str, import_path: &str, is_default: bool) {
        // Check if this exact import already exists to avoid duplicates
        let exists = self.type_imports.iter().any(|(t, p, d)| {
            t == type_name && p == import_path && *d == is_default
        });
        if !exists {
            self.type_imports.push((type_name.to_string(), import_path.to_string(), is_default));
        }
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

        // Custom type imports - group named imports by path, default imports are separate
        let mut named_imports_by_path: std::collections::BTreeMap<&str, BTreeSet<&str>> =
            std::collections::BTreeMap::new();
        let mut default_imports: Vec<(&str, &str)> = Vec::new();

        for (type_name, import_path, is_default) in &self.type_imports {
            if *is_default {
                default_imports.push((type_name.as_str(), import_path.as_str()));
            } else {
                named_imports_by_path
                    .entry(import_path.as_str())
                    .or_default()
                    .insert(type_name.as_str());
            }
        }

        // Generate default import statements
        for (type_name, path) in default_imports {
            result.push_str(&format!("import {} from \"{}\"\n", type_name, path));
        }

        // Generate named import statements, grouped by path
        for (path, types) in named_imports_by_path {
            let sorted_types: Vec<&str> = types.into_iter().collect();
            result.push_str(&format!("import {{ {} }} from \"{}\"\n", sorted_types.join(", "), path));
        }

        // Hook function imports - group by import path
        let mut imports_by_path: std::collections::BTreeMap<&str, Vec<&str>> =
            std::collections::BTreeMap::new();
        for (func_name, import_path) in &self.hook_imports {
            imports_by_path
                .entry(import_path.as_str())
                .or_default()
                .push(func_name.as_str());
        }
        for (path, funcs) in imports_by_path {
            result.push_str(&format!("import {{ {} }} from \"{}\"\n", funcs.join(", "), path));
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
    // Indexes are keyed by name (map type: Index[string])
    if let Some(indexes) = model.config.get("indexes").and_then(|v| v.as_object()) {
        for (index_name, index) in indexes {
            if let Some(fields) = index.get("fields").and_then(|v| v.as_array()) {
                let field_names: Vec<String> = fields
                    .iter()
                    .filter_map(|f| f.as_str())
                    .map(|s| s.to_string())
                    .collect();

                if !field_names.is_empty() {
                    imports.add_typeorm("Index");
                    let mut index_builder = DecoratorBuilder::index(&field_names);

                    // Set the index name from the map key
                    index_builder = index_builder.string_option("name", index_name);

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

        let field_code = generate_field(field, name, &model.config, cfg, utils, type_mapper, imports);
        result.push_str(&field_code);
    }

    // Generate hook methods
    if let Some(hooks) = model.config.get("hooks") {
        let hook_code = generate_hooks(hooks, imports);
        result.push_str(&hook_code);
    }

    result.push('}');
    result
}

fn generate_field(
    field: &FieldDefinition,
    model_name: &str,
    model_config: &JSON,
    cfg: &Config,
    utils: &Utils,
    type_mapper: &TypeMapper,
    imports: &mut ImportCollector,
) -> String {
    // Check for relation config first
    if let Some(relation) = field.config.get("relation") {
        return generate_relation_field(field, relation, model_name, model_config, cfg, type_mapper, imports);
    }

    // Check for primary key config
    if let Some(primary) = field.config.get("primary") {
        return generate_primary_field(field, primary, model_config, cfg, utils, type_mapper, imports);
    }

    // Check for special date column decorators
    if field.config.get("create_date").and_then(|v| v.as_bool()).unwrap_or(false) {
        return generate_column_field_with_decorator(field, "CreateDateColumn", model_config, cfg, utils, type_mapper, imports);
    }
    if field.config.get("update_date").and_then(|v| v.as_bool()).unwrap_or(false) {
        return generate_column_field_with_decorator(field, "UpdateDateColumn", model_config, cfg, utils, type_mapper, imports);
    }
    if field.config.get("delete_date").and_then(|v| v.as_bool()).unwrap_or(false) {
        return generate_column_field_with_decorator(field, "DeleteDateColumn", model_config, cfg, utils, type_mapper, imports);
    }

    // Regular column
    generate_column_field(field, model_config, cfg, utils, type_mapper, imports)
}

fn generate_primary_field(
    field: &FieldDefinition,
    primary_config: &JSON,
    model_config: &JSON,
    cfg: &Config,
    _utils: &Utils,
    type_mapper: &TypeMapper,
    imports: &mut ImportCollector,
) -> String {
    let mut result = String::new();

    let generation = primary_config
        .get("generation")
        .and_then(|v| v.as_str());

    // Check for type override in field config
    let type_override = field.config.get("type").and_then(|v| v.as_str());

    let mut decorator = if let Some(gen_strategy) = generation {
        imports.add_typeorm("PrimaryGeneratedColumn");
        DecoratorBuilder::primary_generated_column(Some(gen_strategy))
    } else {
        imports.add_typeorm("PrimaryColumn");
        DecoratorBuilder::primary_column()
    };

    // Add type option if specified
    if let Some(col_type) = type_override {
        decorator = decorator.string_option("type", col_type);
    }

    result.push_str("    ");
    result.push_str(&decorator.build());
    result.push('\n');

    // Property declaration - check for ts_type override
    let ts_type = resolve_typescript_type(field, type_mapper, imports);
    let property_marker = if field.optional {
        "?"
    } else if should_use_definite_assignment(field, model_config, cfg.definite_assignment) {
        "!"
    } else {
        ""
    };
    result.push_str(&format!("    {}{}: {}\n\n", field.name, property_marker, ts_type));

    result
}

fn generate_column_field(
    field: &FieldDefinition,
    model_config: &JSON,
    cfg: &Config,
    utils: &Utils,
    type_mapper: &TypeMapper,
    imports: &mut ImportCollector,
) -> String {
    generate_column_field_with_decorator(field, "Column", model_config, cfg, utils, type_mapper, imports)
}

fn generate_column_field_with_decorator(
    field: &FieldDefinition,
    decorator_name: &str,
    model_config: &JSON,
    cfg: &Config,
    utils: &Utils,
    type_mapper: &TypeMapper,
    imports: &mut ImportCollector,
) -> String {
    let mut result = String::new();

    imports.add_typeorm(decorator_name);

    let mut column_builder = match decorator_name {
        "CreateDateColumn" => DecoratorBuilder::create_date_column(),
        "UpdateDateColumn" => DecoratorBuilder::update_date_column(),
        "DeleteDateColumn" => DecoratorBuilder::delete_date_column(),
        _ => DecoratorBuilder::column(),
    };

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

    // Property declaration - check for ts_type override
    let ts_type = resolve_typescript_type(field, type_mapper, imports);
    let property_marker = if field.optional {
        "?"
    } else if should_use_definite_assignment(field, model_config, cfg.definite_assignment) {
        "!"
    } else {
        ""
    };
    result.push_str(&format!("    {}{}: {}\n\n", field.name, property_marker, ts_type));

    result
}

fn generate_relation_field(
    field: &FieldDefinition,
    relation_config: &JSON,
    _model_name: &str,
    model_config: &JSON,
    cfg: &Config,
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
    // Check field level first, then nested inside relation for backward compatibility
    let join_column = field.config.get("join_column")
        .or_else(|| relation_config.get("join_column"));

    if let Some(join_column) = join_column {
        imports.add_typeorm("JoinColumn");
        let jc_name = join_column.get("name").and_then(|v| v.as_str());
        let jc_ref = join_column.get("referenced_column").and_then(|v| v.as_str());
        result.push_str("    ");
        result.push_str(&build_join_column(jc_name, jc_ref));
        result.push('\n');
    }

    // Handle JoinTable for ManyToMany
    // Check field level first, then nested inside relation for backward compatibility
    let join_table = field.config.get("join_table")
        .or_else(|| relation_config.get("join_table"));

    if let Some(join_table) = join_table {
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

    // Property declaration - check for ts_type override
    let ts_type = resolve_typescript_type(field, type_mapper, imports);
    let property_marker = if field.optional {
        "?"
    } else if should_use_definite_assignment(field, model_config, cfg.definite_assignment) {
        "!"
    } else {
        ""
    };
    result.push_str(&format!("    {}{}: {}\n\n", field.name, property_marker, ts_type));

    result
}

/// Parsed hook configuration
struct HookInfo {
    method_name: String,
    import_path: Option<String>,
}

impl HookInfo {
    fn from_json(value: &JSON) -> Option<Self> {
        if let Some(method_str) = value.as_str() {
            // String format: just the method name
            Some(HookInfo {
                method_name: method_str.to_string(),
                import_path: None,
            })
        } else if value.is_object() {
            // Object format: { method: string, import: string }
            let method_name = value.get("method")?.as_str()?.to_string();
            let import_path = value.get("import")?.as_str().map(|s| s.to_string());
            Some(HookInfo {
                method_name,
                import_path,
            })
        } else {
            None
        }
    }
}

fn generate_hooks(hooks: &JSON, imports: &mut ImportCollector) -> String {
    use crate::decorator_builder::DecoratorBuilder;

    let mut result = String::new();

    // Process each hook type
    let hook_configs: &[(&str, &str, fn() -> DecoratorBuilder)] = &[
        ("before_insert", "BeforeInsert", DecoratorBuilder::before_insert),
        ("after_insert", "AfterInsert", DecoratorBuilder::after_insert),
        ("before_update", "BeforeUpdate", DecoratorBuilder::before_update),
        ("after_update", "AfterUpdate", DecoratorBuilder::after_update),
        ("before_remove", "BeforeRemove", DecoratorBuilder::before_remove),
        ("after_remove", "AfterRemove", DecoratorBuilder::after_remove),
        ("after_load", "AfterLoad", DecoratorBuilder::after_load),
        ("before_soft_remove", "BeforeSoftRemove", DecoratorBuilder::before_soft_remove),
        ("after_soft_remove", "AfterSoftRemove", DecoratorBuilder::after_soft_remove),
        ("after_recover", "AfterRecover", DecoratorBuilder::after_recover),
    ];

    for (config_key, import_name, builder_fn) in hook_configs {
        if let Some(hook_value) = hooks.get(*config_key) {
            if let Some(hook_info) = HookInfo::from_json(hook_value) {
                imports.add_typeorm(import_name);
                let decorator = builder_fn().build();

                if let Some(import_path) = &hook_info.import_path {
                    // With import: generate method that delegates to imported function
                    imports.add_hook_import(&hook_info.method_name, import_path);
                    result.push_str(&format!(
                        "    {}\n    {}() {{\n        {}.call(this)\n    }}\n\n",
                        decorator, hook_info.method_name, hook_info.method_name
                    ));
                } else {
                    // Without import: generate stub method
                    result.push_str(&format!(
                        "    {}\n    {}() {{\n        // Implementation required\n    }}\n\n",
                        decorator, hook_info.method_name
                    ));
                }
            }
        }
    }

    result
}

// Helper functions

/// Resolves the TypeScript type for a field, checking for ts_type overrides.
/// Priority: field-level ts_type > type alias-level ts_type > default mapping
fn resolve_typescript_type(
    field: &FieldDefinition,
    type_mapper: &TypeMapper,
    imports: &mut ImportCollector,
) -> String {
    // 1. Check for field-level ts_type override
    if let Some(ts_type_config) = field.config.get("ts_type") {
        if let Some(ts_type_info) = TsTypeInfo::from_ts_type_config(ts_type_config) {
            // Add import if specified
            if let Some(import_path) = &ts_type_info.import_path {
                imports.add_type_import(&ts_type_info.type_name, import_path, ts_type_info.is_default_import);
            }
            return ts_type_info.type_name;
        }
    }

    // 2. Check for type alias-level ts_type override
    if let Some(ts_type_info) = type_mapper.get_type_alias_ts_type(&field.field_type) {
        // Add import if specified
        if let Some(import_path) = &ts_type_info.import_path {
            imports.add_type_import(&ts_type_info.type_name, import_path, ts_type_info.is_default_import);
        }
        return ts_type_info.type_name;
    }

    // 3. Fall back to default type mapping
    type_mapper.map_to_typescript_type(&field.field_type)
}

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
    // Check for explicit table name override (consistent with SQL plugin)
    if let Some(table_name) = model_config.get("table_name").and_then(|v| v.as_str()) {
        return table_name.to_string();
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
