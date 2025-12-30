use cdm_plugin_interface::{TypeExpression, JSON};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    PostgreSQL,
    SQLite,
}

impl Dialect {
    pub fn from_config(config: &JSON) -> Self {
        config
            .get("dialect")
            .and_then(|d| d.as_str())
            .map(|s| match s {
                "sqlite" => Dialect::SQLite,
                _ => Dialect::PostgreSQL,
            })
            .unwrap_or(Dialect::PostgreSQL)
    }
}

pub struct TypeMapper {
    dialect: Dialect,
    default_string_length: i64,
    number_type: String,
}

impl TypeMapper {
    pub fn new(config: &JSON) -> Self {
        let dialect = Dialect::from_config(config);

        let default_string_length = config
            .get("default_string_length")
            .and_then(|v| v.as_i64())
            .unwrap_or(255);

        let number_type = config
            .get("number_type")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "double".to_string());

        TypeMapper {
            dialect,
            default_string_length,
            number_type,
        }
    }

    /// Map a CDM type expression to a SQL type
    pub fn map_type(&self, type_expr: &TypeExpression, _is_optional: bool) -> String {
        let base_type = self.map_base_type(type_expr);

        // NULL modifier is handled separately at the column definition level
        // This function only returns the base SQL type
        base_type
    }

    fn map_base_type(&self, type_expr: &TypeExpression) -> String {
        match type_expr {
            TypeExpression::Identifier { name } => {
                // Check for built-in types
                match name.as_str() {
                    "string" => match self.dialect {
                        Dialect::PostgreSQL => format!("VARCHAR({})", self.default_string_length),
                        Dialect::SQLite => "TEXT".to_string(),
                    },
                    "number" => match self.dialect {
                        Dialect::PostgreSQL => match self.number_type.as_str() {
                            "real" => "REAL".to_string(),
                            "numeric" => "NUMERIC".to_string(),
                            _ => "DOUBLE PRECISION".to_string(),
                        },
                        Dialect::SQLite => match self.number_type.as_str() {
                            "numeric" => "NUMERIC".to_string(),
                            _ => "REAL".to_string(),
                        },
                    },
                    "boolean" => match self.dialect {
                        Dialect::PostgreSQL => "BOOLEAN".to_string(),
                        Dialect::SQLite => "INTEGER".to_string(), // SQLite uses 0/1 for boolean
                    },
                    "JSON" => match self.dialect {
                        Dialect::PostgreSQL => "JSONB".to_string(),
                        Dialect::SQLite => "TEXT".to_string(),
                    },
                    // Model references or type aliases
                    _ => match self.dialect {
                        Dialect::PostgreSQL => "JSONB".to_string(),
                        Dialect::SQLite => "TEXT".to_string(),
                    },
                }
            }

            TypeExpression::Array { element_type } => match self.dialect {
                Dialect::PostgreSQL => {
                    let inner_type = self.map_base_type(element_type);
                    format!("{}[]", inner_type)
                }
                Dialect::SQLite => "TEXT".to_string(), // SQLite stores arrays as JSON
            },

            TypeExpression::Union { types: _ } => {
                // Unions default to JSON or VARCHAR depending on dialect
                match self.dialect {
                    Dialect::PostgreSQL => format!("VARCHAR({})", self.default_string_length),
                    Dialect::SQLite => "TEXT".to_string(),
                }
            }

            TypeExpression::StringLiteral { value: _ } => {
                // String literals are mapped to their SQL type
                match self.dialect {
                    Dialect::PostgreSQL => format!("VARCHAR({})", self.default_string_length),
                    Dialect::SQLite => "TEXT".to_string(),
                }
            }
        }
    }

    /// Get the dialect
    pub fn dialect(&self) -> Dialect {
        self.dialect
    }
}


#[cfg(test)]
#[path = "type_mapper/type_mapper_tests.rs"]
mod type_mapper_tests;
