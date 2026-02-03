use cdm_plugin_interface::{CaseFormat, FieldDefinition, ModelDefinition, Utils, JSON};
use crate::type_mapper::{Dialect, TypeMapper};

/// Get the table name for a model, applying any overrides or formatting
pub fn get_table_name(model_name: &str, model_config: &JSON, global_config: &JSON) -> String {
    // Check for table_name override
    if let Some(override_name) = model_config.get("table_name").and_then(|v| v.as_str()) {
        return override_name.to_string();
    }

    // Apply table_name_format
    let format = global_config
        .get("table_name_format")
        .and_then(|v| v.as_str())
        .unwrap_or("snake_case");

    let formatted_name = apply_name_format(model_name, format);

    // Apply pluralization if enabled (default: true)
    let should_pluralize = global_config
        .get("pluralize_table_names")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    if should_pluralize {
        let utils = Utils;
        utils.pluralize(&formatted_name)
    } else {
        formatted_name
    }
}

/// Get the column name for a field, applying any overrides or formatting
pub fn get_column_name(field_name: &str, field_config: &JSON, global_config: &JSON) -> String {
    // Check for column_name override
    if let Some(override_name) = field_config.get("column_name").and_then(|v| v.as_str()) {
        return override_name.to_string();
    }

    // Apply column_name_format
    let format = global_config
        .get("column_name_format")
        .and_then(|v| v.as_str())
        .unwrap_or("snake_case");

    apply_name_format(field_name, format)
}

/// Quote an identifier for SQL (e.g., table or column name)
pub fn quote_identifier(name: &str, dialect: Dialect) -> String {
    match dialect {
        Dialect::PostgreSQL => format!("\"{}\"", name),
        Dialect::SQLite => format!("\"{}\"", name),
    }
}

/// Apply name formatting (snake_case, camelCase, PascalCase, or preserve)
pub fn apply_name_format(name: &str, format: &str) -> String {
    let case_format = match format {
        "snake_case" => CaseFormat::Snake,
        "camel_case" => CaseFormat::Camel,
        "pascal_case" => CaseFormat::Pascal,
        "preserve" => return name.to_string(),
        _ => CaseFormat::Snake,
    };

    let utils = Utils;
    utils.change_case(name, case_format)
}

/// Format a default value for SQL
pub fn format_default_value(value: &cdm_plugin_interface::Value) -> String {
    match value {
        cdm_plugin_interface::Value::String(s) => format!("'{}'", s.replace('\'', "''")),
        cdm_plugin_interface::Value::Number(n) => n.to_string(),
        cdm_plugin_interface::Value::Boolean(b) => {
            if *b { "TRUE" } else { "FALSE" }.to_string()
        }
        cdm_plugin_interface::Value::Null => "NULL".to_string(),
    }
}

/// Generate a CREATE TABLE statement for a model
pub fn generate_create_table(
    model_name: &str,
    model: &ModelDefinition,
    global_config: &JSON,
    type_mapper: &TypeMapper,
) -> String {
    let mut sql = String::new();

    // Get table name (with optional override)
    let table_name = get_table_name(model_name, &model.config, global_config);

    // Get schema prefix (PostgreSQL only)
    let schema_prefix = get_schema_prefix(&model.config, global_config, type_mapper.dialect());

    // Start CREATE TABLE
    sql.push_str(&format!("CREATE TABLE {}{} (\n", schema_prefix, quote_identifier(&table_name, type_mapper.dialect())));

    // Generate column definitions
    let mut column_defs = Vec::new();
    for field in &model.fields {
        if should_skip_field(&field.config) {
            continue;
        }

        let column_def = generate_column_definition(field, global_config, type_mapper);
        column_defs.push(column_def);
    }

    // Pre-compute constraints and indexes to determine if we need trailing commas
    // (only primary/unique indexes become inline constraints; regular indexes are standalone)
    let constraints_and_indexes = generate_constraints_and_indexes(&model.config, type_mapper.dialect());

    // Add columns
    for (i, col) in column_defs.iter().enumerate() {
        sql.push_str("  ");
        sql.push_str(col);
        if i < column_defs.len() - 1 || !constraints_and_indexes.is_empty() {
            sql.push(',');
        }
        sql.push('\n');
    }

    // Add constraints and indexes from model config
    if !constraints_and_indexes.is_empty() {
        sql.push_str(&constraints_and_indexes);
    }

    sql.push_str(");\n");

    // Add standalone indexes (non-primary, non-unique via constraint)
    let indexes_sql = generate_standalone_indexes(&table_name, &schema_prefix, &model.config, type_mapper.dialect());
    if !indexes_sql.is_empty() {
        sql.push('\n');
        sql.push_str(&indexes_sql);
    }

    sql
}

/// Check if a field should be skipped in SQL generation
/// The skip flag can be at the top level or nested inside the "sql" config
pub fn should_skip_field(config: &JSON) -> bool {
    // First check top-level skip (for backwards compatibility)
    if let Some(skip) = config.get("skip").and_then(|v| v.as_bool()) {
        return skip;
    }
    // Then check inside the "sql" config object
    config
        .get("sql")
        .and_then(|sql| sql.get("skip"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn get_schema_prefix(model_config: &JSON, global_config: &JSON, dialect: Dialect) -> String {
    if dialect == Dialect::SQLite {
        return String::new();
    }

    // Check model-level schema override
    if let Some(schema) = model_config.get("schema").and_then(|v| v.as_str()) {
        return format!("{}.", quote_identifier(schema, dialect));
    }

    // Check global schema
    if let Some(schema) = global_config.get("schema").and_then(|v| v.as_str()) {
        return format!("{}.", quote_identifier(schema, dialect));
    }

    String::new()
}

fn generate_column_definition(
    field: &FieldDefinition,
    global_config: &JSON,
    type_mapper: &TypeMapper,
) -> String {
    let mut def = String::new();

    // Get column name
    let column_name = get_column_name(&field.name, &field.config, global_config);
    def.push_str(&quote_identifier(&column_name, type_mapper.dialect()));
    def.push(' ');

    // Get SQL type (check for override first)
    let sql_type = if let Some(type_override) = field.config.get("type").and_then(|v| v.as_str()) {
        type_override.to_string()
    } else {
        type_mapper.map_type(&field.field_type, field.optional)
    };
    def.push_str(&sql_type);

    // Add NOT NULL if field is required
    if !field.optional && should_infer_not_null(global_config) {
        def.push_str(" NOT NULL");
    }

    // Add DEFAULT value with the following priority:
    // 1. Field config default (SQL expression) - highest priority
    // 2. Type alias default (SQL expression)
    // 3. CDM field default (formatted value) - lowest priority
    if let Some(field_config_default) = field.config.get("default").and_then(|v| v.as_str()) {
        // Field-level config default (SQL expression)
        def.push_str(&format!(" DEFAULT {}", field_config_default));
    } else if let Some(type_alias_default) = type_mapper.get_type_alias_default(&field.field_type) {
        // Type alias default (SQL expression)
        def.push_str(&format!(" DEFAULT {}", type_alias_default));
    } else if let Some(default) = &field.default {
        // CDM schema default (formatted value)
        if should_apply_cdm_defaults(global_config) {
            def.push_str(&format!(" DEFAULT {}", format_default_value(default)));
        }
    }

    // Add REFERENCES clause for foreign key constraints
    if let Some(references) = field.config.get("references") {
        if let Some(ref_table) = references.get("table").and_then(|v| v.as_str()) {
            def.push_str(" REFERENCES ");
            def.push_str(&quote_identifier(ref_table, type_mapper.dialect()));

            if let Some(ref_column) = references.get("column").and_then(|v| v.as_str()) {
                def.push('(');
                def.push_str(&quote_identifier(ref_column, type_mapper.dialect()));
                def.push(')');
            }

            if let Some(on_delete) = references.get("on_delete").and_then(|v| v.as_str()) {
                def.push_str(" ON DELETE ");
                def.push_str(&on_delete.to_uppercase());
            }

            if let Some(on_update) = references.get("on_update").and_then(|v| v.as_str()) {
                def.push_str(" ON UPDATE ");
                def.push_str(&on_update.to_uppercase());
            }
        }
    }

    def
}

fn should_infer_not_null(global_config: &JSON) -> bool {
    global_config
        .get("infer_not_null")
        .and_then(|v| v.as_bool())
        .unwrap_or(true)
}

fn should_apply_cdm_defaults(global_config: &JSON) -> bool {
    global_config
        .get("apply_cdm_defaults")
        .and_then(|v| v.as_bool())
        .unwrap_or(true)
}

fn generate_constraints_and_indexes(config: &JSON, dialect: Dialect) -> String {
    let mut sql = String::new();

    // Generate primary key and unique constraints from indexes
    if let Some(indexes) = config.get("indexes").and_then(|v| v.as_array()) {
        for index in indexes {
            if let Some(true) = index.get("primary").and_then(|v| v.as_bool()) {
                // Primary key
                if let Some(fields) = index.get("fields").and_then(|v| v.as_array()) {
                    let field_names: Vec<String> = fields
                        .iter()
                        .filter_map(|f| f.as_str())
                        .map(|f| quote_identifier(f, dialect))
                        .collect();

                    sql.push_str("  PRIMARY KEY (");
                    sql.push_str(&field_names.join(", "));
                    sql.push_str("),\n");
                }
            } else if let Some(true) = index.get("unique").and_then(|v| v.as_bool()) {
                // Unique constraint
                if let Some(fields) = index.get("fields").and_then(|v| v.as_array()) {
                    let field_names: Vec<String> = fields
                        .iter()
                        .filter_map(|f| f.as_str())
                        .map(|f| quote_identifier(f, dialect))
                        .collect();

                    sql.push_str("  UNIQUE (");
                    sql.push_str(&field_names.join(", "));
                    sql.push_str("),\n");
                }
            }
        }
    }

    // Remove trailing comma and newline if present
    if sql.ends_with(",\n") {
        sql.truncate(sql.len() - 2);
        sql.push('\n');
    }

    sql
}

/// Represents an index extracted from config for comparison and SQL generation
#[derive(Debug, Clone)]
pub struct IndexInfo {
    pub name: String,
    pub fields: Vec<String>,
    pub is_unique: bool,
    pub is_primary: bool,
    pub method: Option<String>,
    pub where_clause: Option<String>,
}

/// Extract indexes from a model config JSON
pub fn extract_indexes(config: &JSON, table_name: &str) -> Vec<IndexInfo> {
    let mut indexes = Vec::new();

    if let Some(index_array) = config.get("indexes").and_then(|v| v.as_array()) {
        for (i, index) in index_array.iter().enumerate() {
            let fields: Vec<String> = index
                .get("fields")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|f| f.as_str().map(String::from)).collect())
                .unwrap_or_default();

            if fields.is_empty() {
                continue;
            }

            let is_primary = index.get("primary").and_then(|v| v.as_bool()).unwrap_or(false);
            let is_unique = index.get("unique").and_then(|v| v.as_bool()).unwrap_or(false);

            let name = index
                .get("name")
                .and_then(|v| v.as_str())
                .map(String::from)
                .unwrap_or_else(|| format!("idx_{}_{}", table_name, i));

            let method = index.get("method").and_then(|v| v.as_str()).map(String::from);
            let where_clause = index.get("where").and_then(|v| v.as_str()).map(String::from);

            indexes.push(IndexInfo {
                name,
                fields,
                is_unique,
                is_primary,
                method,
                where_clause,
            });
        }
    }

    indexes
}

/// Generate CREATE INDEX SQL for a single index
pub fn generate_create_index_sql(
    index: &IndexInfo,
    table_name: &str,
    schema_prefix: &str,
    dialect: Dialect,
) -> String {
    let mut sql = String::new();

    let field_names: Vec<String> = index
        .fields
        .iter()
        .map(|f| quote_identifier(f, dialect))
        .collect();

    if index.is_unique {
        sql.push_str(&format!(
            "CREATE UNIQUE INDEX {} ON {}{} ({});\n",
            quote_identifier(&index.name, dialect),
            schema_prefix,
            quote_identifier(table_name, dialect),
            field_names.join(", ")
        ));
    } else {
        sql.push_str(&format!(
            "CREATE INDEX {} ON {}{} ({})",
            quote_identifier(&index.name, dialect),
            schema_prefix,
            quote_identifier(table_name, dialect),
            field_names.join(", ")
        ));

        // Add index method (PostgreSQL only)
        if dialect == Dialect::PostgreSQL {
            if let Some(method) = &index.method {
                sql.push_str(&format!(" USING {}", method.to_uppercase()));
            }
        }

        // Add WHERE clause (PostgreSQL only)
        if dialect == Dialect::PostgreSQL {
            if let Some(where_clause) = &index.where_clause {
                sql.push_str(&format!(" WHERE {}", where_clause));
            }
        }

        sql.push_str(";\n");
    }

    sql
}

/// Generate DROP INDEX SQL for a single index
pub fn generate_drop_index_sql(
    index: &IndexInfo,
    schema_prefix: &str,
    dialect: Dialect,
) -> String {
    match dialect {
        Dialect::PostgreSQL => {
            format!("DROP INDEX {}{};\n", schema_prefix, quote_identifier(&index.name, dialect))
        }
        Dialect::SQLite => {
            format!("DROP INDEX {};\n", quote_identifier(&index.name, dialect))
        }
    }
}

fn generate_standalone_indexes(
    table_name: &str,
    schema_prefix: &str,
    config: &JSON,
    dialect: Dialect,
) -> String {
    let mut sql = String::new();
    let indexes = extract_indexes(config, table_name);

    for index in indexes {
        // Skip primary keys and unique constraints (handled as table constraints in CREATE TABLE)
        if index.is_primary || index.is_unique {
            continue;
        }

        sql.push_str(&generate_create_index_sql(&index, table_name, schema_prefix, dialect));
    }

    sql
}


#[cfg(test)]
#[path = "utils/utils_tests.rs"]
mod utils_tests;
