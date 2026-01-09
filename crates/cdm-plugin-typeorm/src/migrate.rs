use cdm_plugin_interface::{CaseFormat, Delta, FieldDefinition, OutputFile, Schema, Utils, Value, JSON};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::type_mapper::TypeMapper;

/// Configuration for migration generation
#[derive(Debug, Clone)]
struct MigrateConfig {
    table_name_format: String,
    column_name_format: String,
    pluralize_table_names: bool,
}

impl MigrateConfig {
    fn from_json(json: &JSON) -> Self {
        Self {
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
        }
    }
}

/// Generate TypeORM migration files from schema deltas
pub fn migrate(
    current_schema: Schema,
    deltas: Vec<Delta>,
    config: JSON,
    utils: &Utils,
) -> Vec<OutputFile> {
    if deltas.is_empty() {
        return vec![];
    }

    let cfg = MigrateConfig::from_json(&config);
    let model_names: Vec<String> = current_schema.models.keys().cloned().collect();
    let type_mapper = TypeMapper::new(&config, &current_schema.type_aliases, model_names);

    // Generate timestamp for migration name
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    // Generate migration name from deltas
    let migration_name = generate_migration_name(&deltas);

    // Collect up and down queries
    let mut up_queries = Vec::new();
    let mut down_queries = Vec::new();

    for delta in &deltas {
        let (up, down) = delta_to_sql(delta, &current_schema, &cfg, &type_mapper, utils);
        if !up.is_empty() {
            up_queries.push(up);
        }
        if !down.is_empty() {
            down_queries.push(down);
        }
    }

    // Generate migration class content
    let content = generate_migration_class(&migration_name, timestamp, &up_queries, &down_queries);

    vec![OutputFile {
        path: format!("{}-{}.ts", timestamp, to_kebab_case(&migration_name)),
        content,
    }]
}

/// Generate a descriptive migration name from deltas
fn generate_migration_name(deltas: &[Delta]) -> String {
    if deltas.is_empty() {
        return "EmptyMigration".to_string();
    }

    // Use the first significant delta to name the migration
    for delta in deltas {
        match delta {
            Delta::ModelAdded { name, .. } => return format!("Add{}", name),
            Delta::ModelRemoved { name, .. } => return format!("Remove{}", name),
            Delta::ModelRenamed { new_name, .. } => return format!("Rename{}", new_name),
            Delta::FieldAdded { model, field, .. } => {
                return format!("Add{}To{}", capitalize(field), model)
            }
            Delta::FieldRemoved { model, field, .. } => {
                return format!("Remove{}From{}", capitalize(field), model)
            }
            _ => continue,
        }
    }

    "SchemaMigration".to_string()
}

/// Generate the TypeORM migration class
fn generate_migration_class(
    name: &str,
    timestamp: u64,
    up_queries: &[String],
    down_queries: &[String],
) -> String {
    let class_name = format!("{}{}", name, timestamp);

    let up_statements = up_queries
        .iter()
        .map(|q| format!("        await queryRunner.query(`{}`)", q))
        .collect::<Vec<_>>()
        .join("\n");

    let down_statements = down_queries
        .iter()
        .rev() // Reverse order for down migrations
        .map(|q| format!("        await queryRunner.query(`{}`)", q))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"import {{ MigrationInterface, QueryRunner }} from "typeorm"

export class {class_name} implements MigrationInterface {{
    name = '{class_name}'

    public async up(queryRunner: QueryRunner): Promise<void> {{
{up_statements}
    }}

    public async down(queryRunner: QueryRunner): Promise<void> {{
{down_statements}
    }}
}}
"#,
        class_name = class_name,
        up_statements = if up_statements.is_empty() {
            "        // No changes"
        } else {
            &up_statements
        },
        down_statements = if down_statements.is_empty() {
            "        // No changes"
        } else {
            &down_statements
        },
    )
}

/// Convert a delta to up/down SQL queries
fn delta_to_sql(
    delta: &Delta,
    schema: &Schema,
    cfg: &MigrateConfig,
    type_mapper: &TypeMapper,
    utils: &Utils,
) -> (String, String) {
    match delta {
        Delta::ModelAdded { name, after } => {
            let table_name = get_table_name(name, &after.config, cfg, utils);
            let create_sql = generate_create_table_sql(&table_name, after, cfg, type_mapper, utils);
            let drop_sql = format!("DROP TABLE \"{}\"", table_name);
            (create_sql, drop_sql)
        }

        Delta::ModelRemoved { name, before } => {
            let table_name = get_table_name(name, &before.config, cfg, utils);
            let drop_sql = format!("DROP TABLE \"{}\"", table_name);
            let create_sql = generate_create_table_sql(&table_name, before, cfg, type_mapper, utils);
            (drop_sql, create_sql)
        }

        Delta::ModelRenamed {
            old_name,
            new_name,
            before,
            after,
            ..
        } => {
            let old_table = get_table_name(old_name, &before.config, cfg, utils);
            let new_table = get_table_name(new_name, &after.config, cfg, utils);
            let up_sql = format!("ALTER TABLE \"{}\" RENAME TO \"{}\"", old_table, new_table);
            let down_sql = format!("ALTER TABLE \"{}\" RENAME TO \"{}\"", new_table, old_table);
            (up_sql, down_sql)
        }

        Delta::FieldAdded { model, field, after } => {
            if let Some(model_def) = schema.models.get(model) {
                let table_name = get_table_name(model, &model_def.config, cfg, utils);
                let column_name = get_column_name(field, &after.config, cfg, utils);
                let column_def = generate_column_sql(after, &column_name, type_mapper);

                let up_sql = format!(
                    "ALTER TABLE \"{}\" ADD COLUMN {}",
                    table_name, column_def
                );
                let down_sql = format!(
                    "ALTER TABLE \"{}\" DROP COLUMN \"{}\"",
                    table_name, column_name
                );
                (up_sql, down_sql)
            } else {
                (String::new(), String::new())
            }
        }

        Delta::FieldRemoved { model, field, before } => {
            if let Some(model_def) = schema.models.get(model) {
                let table_name = get_table_name(model, &model_def.config, cfg, utils);
                let column_name = get_column_name(field, &before.config, cfg, utils);
                let column_def = generate_column_sql(before, &column_name, type_mapper);

                let up_sql = format!(
                    "ALTER TABLE \"{}\" DROP COLUMN \"{}\"",
                    table_name, column_name
                );
                let down_sql = format!(
                    "ALTER TABLE \"{}\" ADD COLUMN {}",
                    table_name, column_def
                );
                (up_sql, down_sql)
            } else {
                (String::new(), String::new())
            }
        }

        Delta::FieldRenamed {
            model,
            old_name,
            new_name,
            before,
            after,
            ..
        } => {
            if let Some(model_def) = schema.models.get(model) {
                let table_name = get_table_name(model, &model_def.config, cfg, utils);
                let old_column = get_column_name(old_name, &before.config, cfg, utils);
                let new_column = get_column_name(new_name, &after.config, cfg, utils);

                let up_sql = format!(
                    "ALTER TABLE \"{}\" RENAME COLUMN \"{}\" TO \"{}\"",
                    table_name, old_column, new_column
                );
                let down_sql = format!(
                    "ALTER TABLE \"{}\" RENAME COLUMN \"{}\" TO \"{}\"",
                    table_name, new_column, old_column
                );
                (up_sql, down_sql)
            } else {
                (String::new(), String::new())
            }
        }

        Delta::FieldTypeChanged {
            model,
            field,
            before,
            after,
        } => {
            if let Some(model_def) = schema.models.get(model) {
                let table_name = get_table_name(model, &model_def.config, cfg, utils);
                let field_def = model_def.fields.iter().find(|f| &f.name == field);
                let column_name = field_def
                    .map(|f| get_column_name(field, &f.config, cfg, utils))
                    .unwrap_or_else(|| apply_name_format(field, &cfg.column_name_format, utils));

                let new_type = type_mapper.map_to_column_type(after);
                let old_type = type_mapper.map_to_column_type(before);

                let up_sql = format!(
                    "ALTER TABLE \"{}\" ALTER COLUMN \"{}\" TYPE {}",
                    table_name, column_name, new_type
                );
                let down_sql = format!(
                    "ALTER TABLE \"{}\" ALTER COLUMN \"{}\" TYPE {}",
                    table_name, column_name, old_type
                );
                (up_sql, down_sql)
            } else {
                (String::new(), String::new())
            }
        }

        Delta::FieldOptionalityChanged {
            model,
            field,
            after,
            ..
        } => {
            if let Some(model_def) = schema.models.get(model) {
                let table_name = get_table_name(model, &model_def.config, cfg, utils);
                let field_def = model_def.fields.iter().find(|f| &f.name == field);
                let column_name = field_def
                    .map(|f| get_column_name(field, &f.config, cfg, utils))
                    .unwrap_or_else(|| apply_name_format(field, &cfg.column_name_format, utils));

                let (up_sql, down_sql) = if *after {
                    // Became optional
                    (
                        format!(
                            "ALTER TABLE \"{}\" ALTER COLUMN \"{}\" DROP NOT NULL",
                            table_name, column_name
                        ),
                        format!(
                            "ALTER TABLE \"{}\" ALTER COLUMN \"{}\" SET NOT NULL",
                            table_name, column_name
                        ),
                    )
                } else {
                    // Became required
                    (
                        format!(
                            "ALTER TABLE \"{}\" ALTER COLUMN \"{}\" SET NOT NULL",
                            table_name, column_name
                        ),
                        format!(
                            "ALTER TABLE \"{}\" ALTER COLUMN \"{}\" DROP NOT NULL",
                            table_name, column_name
                        ),
                    )
                };
                (up_sql, down_sql)
            } else {
                (String::new(), String::new())
            }
        }

        Delta::FieldDefaultChanged {
            model,
            field,
            before,
            after,
        } => {
            if let Some(model_def) = schema.models.get(model) {
                let table_name = get_table_name(model, &model_def.config, cfg, utils);
                let field_def = model_def.fields.iter().find(|f| &f.name == field);
                let column_name = field_def
                    .map(|f| get_column_name(field, &f.config, cfg, utils))
                    .unwrap_or_else(|| apply_name_format(field, &cfg.column_name_format, utils));

                let up_sql = match after {
                    Some(val) => format!(
                        "ALTER TABLE \"{}\" ALTER COLUMN \"{}\" SET DEFAULT {}",
                        table_name,
                        column_name,
                        format_default_value(val)
                    ),
                    None => format!(
                        "ALTER TABLE \"{}\" ALTER COLUMN \"{}\" DROP DEFAULT",
                        table_name, column_name
                    ),
                };

                let down_sql = match before {
                    Some(val) => format!(
                        "ALTER TABLE \"{}\" ALTER COLUMN \"{}\" SET DEFAULT {}",
                        table_name,
                        column_name,
                        format_default_value(val)
                    ),
                    None => format!(
                        "ALTER TABLE \"{}\" ALTER COLUMN \"{}\" DROP DEFAULT",
                        table_name, column_name
                    ),
                };

                (up_sql, down_sql)
            } else {
                (String::new(), String::new())
            }
        }

        // Type alias changes don't affect database
        Delta::TypeAliasAdded { .. }
        | Delta::TypeAliasRemoved { .. }
        | Delta::TypeAliasRenamed { .. }
        | Delta::TypeAliasTypeChanged { .. } => (String::new(), String::new()),

        // Inheritance changes don't directly affect database (fields are flattened)
        Delta::InheritanceAdded { .. } | Delta::InheritanceRemoved { .. } => {
            (String::new(), String::new())
        }

        // Config changes may need manual review
        Delta::GlobalConfigChanged { .. }
        | Delta::ModelConfigChanged { .. }
        | Delta::FieldConfigChanged { .. } => (String::new(), String::new()),
    }
}

/// Generate CREATE TABLE SQL for a model
fn generate_create_table_sql(
    table_name: &str,
    model: &cdm_plugin_interface::ModelDefinition,
    cfg: &MigrateConfig,
    type_mapper: &TypeMapper,
    utils: &Utils,
) -> String {
    let mut columns = Vec::new();
    let mut primary_keys = Vec::new();

    for field in &model.fields {
        // Skip relation fields (they don't have columns)
        if field.config.get("relation").is_some() {
            continue;
        }

        // Skip explicitly skipped fields
        if field
            .config
            .get("skip")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            continue;
        }

        let column_name = get_column_name(&field.name, &field.config, cfg, utils);
        let column_sql = generate_column_sql(field, &column_name, type_mapper);
        columns.push(column_sql);

        // Track primary keys
        if field.config.get("primary").is_some() {
            primary_keys.push(format!("\"{}\"", column_name));
        }
    }

    let mut sql = format!("CREATE TABLE \"{}\" (", table_name);
    sql.push_str(&columns.join(", "));

    // Add primary key constraint if we have PKs
    if !primary_keys.is_empty() {
        sql.push_str(&format!(", PRIMARY KEY ({})", primary_keys.join(", ")));
    }

    sql.push(')');
    sql
}

/// Generate column definition SQL
fn generate_column_sql(
    field: &FieldDefinition,
    column_name: &str,
    type_mapper: &TypeMapper,
) -> String {
    let mut parts = vec![format!("\"{}\"", column_name)];

    // Get column type
    let col_type = field
        .config
        .get("type")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| type_mapper.map_to_column_type(&field.field_type));

    // Handle primary key with generation
    if let Some(primary) = field.config.get("primary") {
        if let Some(generation) = primary.get("generation").and_then(|v| v.as_str()) {
            match generation {
                "uuid" => {
                    parts.push("uuid".to_string());
                    parts.push("DEFAULT uuid_generate_v4()".to_string());
                }
                "increment" | "identity" => {
                    parts.push("SERIAL".to_string());
                }
                _ => {
                    parts.push(col_type);
                }
            }
        } else {
            parts.push(col_type);
        }
    } else {
        parts.push(col_type);
    }

    // NOT NULL
    let nullable = field
        .config
        .get("nullable")
        .and_then(|v| v.as_bool())
        .unwrap_or(field.optional);
    if !nullable {
        parts.push("NOT NULL".to_string());
    }

    // UNIQUE
    if field
        .config
        .get("unique")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        parts.push("UNIQUE".to_string());
    }

    // DEFAULT
    if let Some(default) = &field.default {
        parts.push(format!("DEFAULT {}", format_default_value(default)));
    } else if let Some(default_str) = field.config.get("default").and_then(|v| v.as_str()) {
        parts.push(format!("DEFAULT {}", default_str));
    }

    parts.join(" ")
}

// Helper functions

fn get_table_name(model_name: &str, model_config: &JSON, cfg: &MigrateConfig, utils: &Utils) -> String {
    if let Some(table) = model_config.get("table").and_then(|v| v.as_str()) {
        return table.to_string();
    }

    let formatted = apply_name_format(model_name, &cfg.table_name_format, utils);
    if cfg.pluralize_table_names {
        utils.pluralize(&formatted)
    } else {
        formatted
    }
}

fn get_column_name(field_name: &str, field_config: &JSON, cfg: &MigrateConfig, utils: &Utils) -> String {
    if let Some(column) = field_config.get("column").and_then(|v| v.as_str()) {
        return column.to_string();
    }
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

fn format_default_value(value: &Value) -> String {
    match value {
        Value::String(s) => format!("'{}'", s.replace('\'', "''")),
        Value::Number(n) => n.to_string(),
        Value::Boolean(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        Value::Null => "NULL".to_string(),
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('-');
        }
        result.push(c.to_lowercase().next().unwrap_or(c));
    }
    result
}

#[cfg(test)]
#[path = "migrate/migrate_tests.rs"]
mod migrate_tests;
