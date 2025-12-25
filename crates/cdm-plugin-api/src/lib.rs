use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod ffi;

pub type JSON = serde_json::Value;

/// Configuration level for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConfigLevel {
    Global,
    Model { name: String },
    Field { model: String, field: String },
}

/// Path segment for error reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathSegment {
    pub kind: String,
    pub name: String,
}

/// Error severity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

/// Validation error with structured path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub path: Vec<PathSegment>,
    pub message: String,
    pub severity: Severity,
}

/// Output file from build or migrate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputFile {
    pub path: String,
    pub content: String,
}

/// Case format for string conversion
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseFormat {
    Snake,
    Camel,
    Pascal,
    Kebab,
    Constant,
    Title,
}

/// Utility functions provided by CDM runtime
pub struct Utils;

impl Utils {
    pub fn change_case(&self, input: &str, format: CaseFormat) -> String {
        match format {
            CaseFormat::Snake => to_snake_case(input),
            CaseFormat::Camel => to_camel_case(input),
            CaseFormat::Pascal => to_pascal_case(input),
            CaseFormat::Kebab => to_kebab_case(input),
            CaseFormat::Constant => to_constant_case(input),
            CaseFormat::Title => to_title_case(input),
        }
    }
}

// Simple implementations for case conversion
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_is_upper = false;

    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 && !prev_is_upper {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap());
            prev_is_upper = true;
        } else {
            result.push(ch);
            prev_is_upper = false;
        }
    }

    result
}

fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;

    for (i, ch) in s.chars().enumerate() {
        if ch == '_' || ch == '-' || ch == ' ' {
            capitalize_next = true;
        } else if i == 0 {
            result.push(ch.to_lowercase().next().unwrap());
        } else if capitalize_next {
            result.push(ch.to_uppercase().next().unwrap());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }

    result
}

fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for ch in s.chars() {
        if ch == '_' || ch == '-' || ch == ' ' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(ch.to_uppercase().next().unwrap());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }

    result
}

fn to_kebab_case(s: &str) -> String {
    to_snake_case(s).replace('_', "-")
}

fn to_constant_case(s: &str) -> String {
    to_snake_case(s).to_uppercase()
}

fn to_title_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Schema types

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub models: HashMap<String, ModelDefinition>,
    pub type_aliases: HashMap<String, TypeAliasDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDefinition {
    pub name: String,
    pub parents: Vec<String>,
    pub fields: Vec<FieldDefinition>,
    pub config: JSON,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    pub name: String,
    pub field_type: TypeExpression,
    pub optional: bool,
    pub default: Option<Value>,
    pub config: JSON,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeAliasDefinition {
    pub name: String,
    pub alias_type: TypeExpression,
    pub config: JSON,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TypeExpression {
    Identifier { name: String },
    Array { element_type: Box<TypeExpression> },
    Union { types: Vec<TypeExpression> },
    StringLiteral { value: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    String(String),
    Number(f64),
    Boolean(bool),
    Null,
}

impl From<&serde_json::Value> for Value {
    fn from(json: &serde_json::Value) -> Self {
        match json {
            serde_json::Value::String(s) => Value::String(s.clone()),
            serde_json::Value::Number(n) => Value::Number(n.as_f64().unwrap_or(0.0)),
            serde_json::Value::Bool(b) => Value::Boolean(*b),
            serde_json::Value::Null => Value::Null,
            // Arrays and objects are not supported - convert to Null
            _ => Value::Null,
        }
    }
}

/// Delta types for migrations

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Delta {
    // Models
    ModelAdded {
        name: String,
        after: ModelDefinition,
    },
    ModelRemoved {
        name: String,
        before: ModelDefinition,
    },
    ModelRenamed {
        old_name: String,
        new_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<u64>,
        before: ModelDefinition,
        after: ModelDefinition,
    },

    // Fields
    FieldAdded {
        model: String,
        field: String,
        after: FieldDefinition,
    },
    FieldRemoved {
        model: String,
        field: String,
        before: FieldDefinition,
    },
    FieldRenamed {
        model: String,
        old_name: String,
        new_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<u64>,
        before: FieldDefinition,
        after: FieldDefinition,
    },
    FieldTypeChanged {
        model: String,
        field: String,
        before: TypeExpression,
        after: TypeExpression,
    },
    FieldOptionalityChanged {
        model: String,
        field: String,
        before: bool,
        after: bool,
    },
    FieldDefaultChanged {
        model: String,
        field: String,
        before: Option<Value>,
        after: Option<Value>,
    },

    // Type Aliases
    TypeAliasAdded {
        name: String,
        after: TypeAliasDefinition,
    },
    TypeAliasRemoved {
        name: String,
        before: TypeAliasDefinition,
    },
    TypeAliasRenamed {
        old_name: String,
        new_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<u64>,
        before: TypeAliasDefinition,
        after: TypeAliasDefinition,
    },
    TypeAliasTypeChanged {
        name: String,
        before: TypeExpression,
        after: TypeExpression,
    },

    // Inheritance
    InheritanceAdded {
        model: String,
        parent: String,
    },
    InheritanceRemoved {
        model: String,
        parent: String,
    },

    // Config Changes
    GlobalConfigChanged {
        before: JSON,
        after: JSON,
    },
    ModelConfigChanged {
        model: String,
        before: JSON,
        after: JSON,
    },
    FieldConfigChanged {
        model: String,
        field: String,
        before: JSON,
        after: JSON,
    },
}

/// Export macro placeholder - in a real implementation, this would be a proc macro
/// For now, we'll just use it as a marker
pub use export_plugin_impl as export_plugin;

/// Placeholder for the actual proc macro
/// In a real implementation, this would be in a separate proc-macro crate
pub fn export_plugin_impl(_attr: &str, _item: &str) -> String {
    // This is a placeholder - actual implementation would generate WASM exports
    String::new()
}

/// Helper macro to embed a schema.cdm file and export it via the WASM `_schema()` function.
///
/// This is a Rust-specific convenience for plugin development. It uses `include_str!()`
/// to embed the schema file at compile time, making the WASM binary self-contained.
///
/// # Example
///
/// ```rust,ignore
/// use cdm_plugin_api::schema_from_file;
///
/// // Embeds ../schema.cdm and creates the _schema() export
/// schema_from_file!("../schema.cdm");
/// ```
///
/// # How it works
///
/// The macro expands to:
/// - Load the file contents at compile time using `include_str!()`
/// - Create a function that returns the embedded schema string
/// - Use the `export_schema!` macro to wrap it with proper FFI handling
///
/// # Note
///
/// This is optional - plugins can implement `_schema()` however they want.
/// This macro is just a convenience for Rust plugins that want to keep
/// their schema in a separate `.cdm` file.
#[macro_export]
macro_rules! schema_from_file {
    ($path:expr) => {
        pub fn __cdm_schema_content() -> String {
            include_str!($path).to_string()
        }
        $crate::export_schema!(__cdm_schema_content);
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    // Case conversion tests
    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("HelloWorld"), "hello_world");
        assert_eq!(to_snake_case("helloWorld"), "hello_world");
        assert_eq!(to_snake_case("hello"), "hello");
        assert_eq!(to_snake_case("HELLO"), "hello");  // All uppercase becomes lowercase without underscores between consecutive uppers
        assert_eq!(to_snake_case(""), "");
        assert_eq!(to_snake_case("ID"), "id");  // Consecutive uppercase letters don't get underscores between them
        assert_eq!(to_snake_case("MyHTTPServer"), "my_httpserver");  // Consecutive uppers treated as one block
    }

    #[test]
    fn test_to_camel_case() {
        assert_eq!(to_camel_case("hello_world"), "helloWorld");
        assert_eq!(to_camel_case("hello-world"), "helloWorld");
        assert_eq!(to_camel_case("hello world"), "helloWorld");
        assert_eq!(to_camel_case("hello"), "hello");
        assert_eq!(to_camel_case("HelloWorld"), "helloWorld");
        assert_eq!(to_camel_case(""), "");
        assert_eq!(to_camel_case("one_two_three"), "oneTwoThree");
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("hello_world"), "HelloWorld");
        assert_eq!(to_pascal_case("hello-world"), "HelloWorld");
        assert_eq!(to_pascal_case("hello world"), "HelloWorld");
        assert_eq!(to_pascal_case("hello"), "Hello");
        assert_eq!(to_pascal_case("HelloWorld"), "HelloWorld");
        assert_eq!(to_pascal_case(""), "");
        assert_eq!(to_pascal_case("one_two_three"), "OneTwoThree");
    }

    #[test]
    fn test_to_kebab_case() {
        assert_eq!(to_kebab_case("HelloWorld"), "hello-world");
        assert_eq!(to_kebab_case("helloWorld"), "hello-world");
        assert_eq!(to_kebab_case("hello"), "hello");
        assert_eq!(to_kebab_case(""), "");
    }

    #[test]
    fn test_to_constant_case() {
        assert_eq!(to_constant_case("HelloWorld"), "HELLO_WORLD");
        assert_eq!(to_constant_case("helloWorld"), "HELLO_WORLD");
        assert_eq!(to_constant_case("hello"), "HELLO");
        assert_eq!(to_constant_case(""), "");
    }

    #[test]
    fn test_to_title_case() {
        assert_eq!(to_title_case("hello_world"), "Hello World");
        assert_eq!(to_title_case("hello"), "Hello");
        assert_eq!(to_title_case("one_two_three"), "One Two Three");
        assert_eq!(to_title_case(""), "");
    }

    #[test]
    fn test_utils_change_case() {
        let utils = Utils;

        assert_eq!(utils.change_case("HelloWorld", CaseFormat::Snake), "hello_world");
        assert_eq!(utils.change_case("hello_world", CaseFormat::Camel), "helloWorld");
        assert_eq!(utils.change_case("hello_world", CaseFormat::Pascal), "HelloWorld");
        assert_eq!(utils.change_case("HelloWorld", CaseFormat::Kebab), "hello-world");
        assert_eq!(utils.change_case("HelloWorld", CaseFormat::Constant), "HELLO_WORLD");
        assert_eq!(utils.change_case("hello_world", CaseFormat::Title), "Hello World");
    }

    // Serialization tests
    #[test]
    fn test_config_level_serialization() {
        // Global level
        let global = ConfigLevel::Global;
        let json = serde_json::to_string(&global).unwrap();
        assert!(json.contains("\"type\":\"global\""));

        let deserialized: ConfigLevel = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, ConfigLevel::Global));

        // Model level
        let model = ConfigLevel::Model { name: "User".to_string() };
        let json = serde_json::to_string(&model).unwrap();
        assert!(json.contains("\"type\":\"model\""));
        assert!(json.contains("\"name\":\"User\""));

        // Field level
        let field = ConfigLevel::Field {
            model: "User".to_string(),
            field: "id".to_string()
        };
        let json = serde_json::to_string(&field).unwrap();
        assert!(json.contains("\"type\":\"field\""));
        assert!(json.contains("\"model\":\"User\""));
        assert!(json.contains("\"field\":\"id\""));
    }

    #[test]
    fn test_severity_serialization() {
        let error = Severity::Error;
        let json = serde_json::to_string(&error).unwrap();
        assert_eq!(json, "\"error\"");

        let warning = Severity::Warning;
        let json = serde_json::to_string(&warning).unwrap();
        assert_eq!(json, "\"warning\"");
    }

    #[test]
    fn test_validation_error_serialization() {
        let error = ValidationError {
            path: vec![
                PathSegment {
                    kind: "field".to_string(),
                    name: "email".to_string(),
                },
            ],
            message: "Invalid email format".to_string(),
            severity: Severity::Error,
        };

        let json = serde_json::to_string(&error).unwrap();
        let deserialized: ValidationError = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.path.len(), 1);
        assert_eq!(deserialized.path[0].kind, "field");
        assert_eq!(deserialized.path[0].name, "email");
        assert_eq!(deserialized.message, "Invalid email format");
        assert_eq!(deserialized.severity, Severity::Error);
    }

    #[test]
    fn test_output_file_serialization() {
        let file = OutputFile {
            path: "output.txt".to_string(),
            content: "Hello, world!".to_string(),
        };

        let json = serde_json::to_string(&file).unwrap();
        let deserialized: OutputFile = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.path, "output.txt");
        assert_eq!(deserialized.content, "Hello, world!");
    }

    #[test]
    fn test_type_expression_serialization() {
        // Identifier
        let identifier = TypeExpression::Identifier {
            name: "string".to_string()
        };
        let json = serde_json::to_string(&identifier).unwrap();
        assert!(json.contains("\"type\":\"identifier\""));
        assert!(json.contains("\"name\":\"string\""));

        // Array
        let array = TypeExpression::Array {
            element_type: Box::new(TypeExpression::Identifier {
                name: "number".to_string()
            })
        };
        let json = serde_json::to_string(&array).unwrap();
        assert!(json.contains("\"type\":\"array\""));

        // Union
        let union = TypeExpression::Union {
            types: vec![
                TypeExpression::Identifier { name: "string".to_string() },
                TypeExpression::Identifier { name: "number".to_string() },
            ]
        };
        let json = serde_json::to_string(&union).unwrap();
        assert!(json.contains("\"type\":\"union\""));

        // String literal
        let literal = TypeExpression::StringLiteral {
            value: "active".to_string()
        };
        let json = serde_json::to_string(&literal).unwrap();
        assert!(json.contains("\"type\":\"string_literal\""));
    }

    #[test]
    fn test_value_serialization() {
        // String
        let string_val = Value::String("test".to_string());
        let json = serde_json::to_string(&string_val).unwrap();
        assert_eq!(json, "\"test\"");

        // Number
        let number_val = Value::Number(42.5);
        let json = serde_json::to_string(&number_val).unwrap();
        assert_eq!(json, "42.5");

        // Boolean
        let bool_val = Value::Boolean(true);
        let json = serde_json::to_string(&bool_val).unwrap();
        assert_eq!(json, "true");

        // Null
        let null_val = Value::Null;
        let json = serde_json::to_string(&null_val).unwrap();
        assert_eq!(json, "null");
    }

    #[test]
    fn test_delta_model_added_serialization() {
        let delta = Delta::ModelAdded {
            name: "User".to_string(),
            after: ModelDefinition {
                name: "User".to_string(),
                parents: vec![],
                fields: vec![],
                config: serde_json::json!({}),
                entity_id: None,
            },
        };

        let json = serde_json::to_string(&delta).unwrap();
        assert!(json.contains("\"type\":\"model_added\""));
        assert!(json.contains("\"name\":\"User\""));

        let deserialized: Delta = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, Delta::ModelAdded { .. }));
    }

    #[test]
    fn test_delta_field_added_serialization() {
        let delta = Delta::FieldAdded {
            model: "User".to_string(),
            field: "email".to_string(),
            after: FieldDefinition {
                name: "email".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({}),
                entity_id: None,
            },
        };

        let json = serde_json::to_string(&delta).unwrap();
        assert!(json.contains("\"type\":\"field_added\""));
        assert!(json.contains("\"model\":\"User\""));
        assert!(json.contains("\"field\":\"email\""));

        let deserialized: Delta = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, Delta::FieldAdded { .. }));
    }

    #[test]
    fn test_schema_serialization() {
        let mut models = HashMap::new();
        models.insert(
            "User".to_string(),
            ModelDefinition {
                name: "User".to_string(),
                parents: vec![],
                fields: vec![
                    FieldDefinition {
                        name: "id".to_string(),
                        field_type: TypeExpression::Identifier {
                            name: "number".to_string(),
                        },
                        optional: false,
                        default: None,
                        config: serde_json::json!({}),
                        entity_id: None,
                    },
                ],
                config: serde_json::json!({}),
                entity_id: None,
            },
        );

        let schema = Schema {
            models,
            type_aliases: HashMap::new(),
        };

        let json = serde_json::to_string(&schema).unwrap();
        let deserialized: Schema = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.models.len(), 1);
        assert!(deserialized.models.contains_key("User"));
        assert_eq!(deserialized.type_aliases.len(), 0);
    }

    #[test]
    fn test_case_format_serialization() {
        assert_eq!(
            serde_json::to_string(&CaseFormat::Snake).unwrap(),
            "\"snake\""
        );
        assert_eq!(
            serde_json::to_string(&CaseFormat::Camel).unwrap(),
            "\"camel\""
        );
        assert_eq!(
            serde_json::to_string(&CaseFormat::Pascal).unwrap(),
            "\"pascal\""
        );
        assert_eq!(
            serde_json::to_string(&CaseFormat::Kebab).unwrap(),
            "\"kebab\""
        );
        assert_eq!(
            serde_json::to_string(&CaseFormat::Constant).unwrap(),
            "\"constant\""
        );
        assert_eq!(
            serde_json::to_string(&CaseFormat::Title).unwrap(),
            "\"title\""
        );
    }
}
