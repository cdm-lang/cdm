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

    // Add columns
    for (i, col) in column_defs.iter().enumerate() {
        sql.push_str("  ");
        sql.push_str(col);
        if i < column_defs.len() - 1 || has_constraints_or_indexes(&model.config) {
            sql.push(',');
        }
        sql.push('\n');
    }

    // Add constraints and indexes from model config
    let constraints_and_indexes = generate_constraints_and_indexes(&model.config, type_mapper.dialect());
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

fn should_skip_field(config: &JSON) -> bool {
    config
        .get("skip")
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

    // Add DEFAULT if field has default value
    if let Some(default) = &field.default {
        if should_apply_cdm_defaults(global_config) {
            def.push_str(&format!(" DEFAULT {}", format_default_value(default)));
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

fn has_constraints_or_indexes(config: &JSON) -> bool {
    config.get("indexes").is_some() || config.get("constraints").is_some()
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

fn generate_standalone_indexes(
    table_name: &str,
    schema_prefix: &str,
    config: &JSON,
    dialect: Dialect,
) -> String {
    let mut sql = String::new();

    if let Some(indexes) = config.get("indexes").and_then(|v| v.as_array()) {
        for (i, index) in indexes.iter().enumerate() {
            // Skip primary keys and unique constraints (already handled)
            if index.get("primary").and_then(|v| v.as_bool()).unwrap_or(false) {
                continue;
            }
            if index.get("unique").and_then(|v| v.as_bool()).unwrap_or(false) {
                continue;
            }

            if let Some(fields) = index.get("fields").and_then(|v| v.as_array()) {
                let field_names: Vec<String> = fields
                    .iter()
                    .filter_map(|f| f.as_str())
                    .map(|f| quote_identifier(f, dialect))
                    .collect();

                // Generate index name
                let index_name = if let Some(name) = index.get("name").and_then(|v| v.as_str()) {
                    name.to_string()
                } else {
                    format!("idx_{}_{}", table_name, i)
                };

                sql.push_str(&format!("CREATE INDEX {} ON {}{} (",
                    quote_identifier(&index_name, dialect),
                    schema_prefix,
                    quote_identifier(table_name, dialect)
                ));
                sql.push_str(&field_names.join(", "));
                sql.push(')');

                // Add index method (PostgreSQL only)
                if dialect == Dialect::PostgreSQL {
                    if let Some(method) = index.get("method").and_then(|v| v.as_str()) {
                        sql.push_str(&format!(" USING {}", method.to_uppercase()));
                    }
                }

                // Add WHERE clause (PostgreSQL only)
                if dialect == Dialect::PostgreSQL {
                    if let Some(where_clause) = index.get("where").and_then(|v| v.as_str()) {
                        sql.push_str(&format!(" WHERE {}", where_clause));
                    }
                }

                sql.push_str(";\n");
            }
        }
    }

    sql
}

#[cfg(test)]
mod tests {
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
}
