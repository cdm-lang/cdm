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

    pub fn pluralize(&self, input: &str) -> String {
        pluralize(input)
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

fn pluralize(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }

    let s_lower = s.to_lowercase();

    // Irregular plurals
    let irregular = [
        ("person", "people"),
        ("child", "children"),
        ("man", "men"),
        ("woman", "women"),
        ("tooth", "teeth"),
        ("foot", "feet"),
        ("mouse", "mice"),
        ("goose", "geese"),
    ];

    for (singular, plural) in &irregular {
        if s_lower == *singular {
            // Preserve the original case
            return if s.chars().next().unwrap().is_uppercase() {
                capitalize(plural)
            } else {
                plural.to_string()
            };
        }
    }

    // Words that don't change
    let unchanging = ["sheep", "fish", "deer", "species", "series"];
    if unchanging.contains(&s_lower.as_str()) {
        return s.to_string();
    }

    // Rules-based pluralization
    if s_lower.ends_with("s")
        || s_lower.ends_with("x")
        || s_lower.ends_with("z")
        || s_lower.ends_with("ch")
        || s_lower.ends_with("sh")
    {
        return format!("{}es", s);
    }

    if s_lower.ends_with("y") {
        if let Some(second_last) = s_lower.chars().rev().nth(1) {
            if !"aeiou".contains(second_last) {
                // Consonant + y -> ies
                return format!("{}ies", &s[..s.len() - 1]);
            }
        }
        // Vowel + y -> ys
        return format!("{}s", s);
    }

    if s_lower.ends_with("f") {
        return format!("{}ves", &s[..s.len() - 1]);
    }

    if s_lower.ends_with("fe") {
        return format!("{}ves", &s[..s.len() - 2]);
    }

    if s_lower.ends_with("o") {
        if let Some(second_last) = s_lower.chars().rev().nth(1) {
            if !"aeiou".contains(second_last) {
                // Consonant + o -> oes (for most cases)
                return format!("{}es", s);
            }
        }
    }

    // Default: just add 's'
    format!("{}s", s)
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
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
#[path = "lib/lib_tests.rs"]
mod lib_tests;
