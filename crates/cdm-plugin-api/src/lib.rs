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

/// Output file from generate or migrate
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    pub name: String,
    pub field_type: TypeExpression,
    pub optional: bool,
    pub default: Option<Value>,
    pub config: JSON,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeAliasDefinition {
    pub name: String,
    pub alias_type: TypeExpression,
    pub config: JSON,
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
