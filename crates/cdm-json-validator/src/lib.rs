//! JSON validator for CDM schemas
//!
//! This crate provides validation of JSON values against CDM ResolvedSchema types.
//! It validates structure, types, and optionality according to the parsed type definitions.

use cdm_utils::{ParsedType, PrimitiveType, ResolvedSchema};
use cdm_plugin_api::{PathSegment, Severity, ValidationError};
use serde_json::Value as JSON;
use std::collections::HashSet;

/// Validate a JSON value against a model in the schema
///
/// # Arguments
/// * `schema` - The resolved schema containing all type definitions
/// * `json` - The JSON value to validate
/// * `model_name` - The name of the model to validate against
///
/// # Returns
/// A vector of validation errors. Empty if validation succeeds.
///
/// # Example
/// ```ignore
/// let schema = build_resolved_schema(...);
/// let json = serde_json::json!({"name": "Alice", "age": 30});
/// let errors = validate_json(&schema, &json, "User");
/// ```
pub fn validate_json(
    schema: &ResolvedSchema,
    json: &JSON,
    model_name: &str,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // Look up the model
    let model = match schema.models.get(model_name) {
        Some(m) => m,
        None => {
            errors.push(ValidationError {
                path: vec![],
                message: format!("Model '{}' not found in schema", model_name),
                severity: Severity::Error,
            });
            return errors;
        }
    };

    // JSON must be an object for model validation
    let obj = match json.as_object() {
        Some(o) => o,
        None => {
            errors.push(ValidationError {
                path: vec![],
                message: format!("Expected object for model '{}', got {}", model_name, json_type_name(json)),
                severity: Severity::Error,
            });
            return errors;
        }
    };

    // Track which fields we've seen
    let mut seen_fields = HashSet::new();

    // Validate each field in the model
    for field in &model.fields {
        seen_fields.insert(&field.name);

        let field_value = obj.get(&field.name);

        // Check if field is present
        if field_value.is_none() {
            if !field.optional {
                errors.push(ValidationError {
                    path: vec![PathSegment {
                        kind: "field".to_string(),
                        name: field.name.clone(),
                    }],
                    message: format!("Required field '{}' is missing", field.name),
                    severity: Severity::Error,
                });
            }
            continue;
        }

        let value = field_value.unwrap();

        // Get the parsed type
        let parsed_type = match field.parsed_type() {
            Ok(t) => t,
            Err(e) => {
                errors.push(ValidationError {
                    path: vec![PathSegment {
                        kind: "field".to_string(),
                        name: field.name.clone(),
                    }],
                    message: format!("Failed to parse field type: {}", e),
                    severity: Severity::Error,
                });
                continue;
            }
        };

        // Validate the field value against its type
        let field_path = vec![PathSegment {
            kind: "field".to_string(),
            name: field.name.clone(),
        }];

        let field_errors = validate_value(schema, value, &parsed_type, &field_path);
        errors.extend(field_errors);
    }

    // Check for unknown fields
    for (key, _) in obj.iter() {
        if !seen_fields.contains(key) {
            errors.push(ValidationError {
                path: vec![PathSegment {
                    kind: "field".to_string(),
                    name: key.clone(),
                }],
                message: format!("Unknown field '{}'", key),
                severity: Severity::Error,
            });
        }
    }

    errors
}

/// Validate a JSON value against a parsed type
///
/// This is the core recursive validation function that handles all type variants.
///
/// # Arguments
/// * `schema` - The resolved schema (for resolving references)
/// * `json` - The JSON value to validate
/// * `parsed_type` - The parsed type to validate against
/// * `path` - The current path in the JSON structure (for error reporting)
///
/// # Returns
/// A vector of validation errors. Empty if validation succeeds.
pub fn validate_value(
    schema: &ResolvedSchema,
    json: &JSON,
    parsed_type: &ParsedType,
    path: &[PathSegment],
) -> Vec<ValidationError> {
    match parsed_type {
        ParsedType::Primitive(prim) => validate_primitive(json, prim, path),

        ParsedType::Literal(expected) => validate_literal(json, expected, path),

        ParsedType::Reference(name) => validate_reference(schema, json, name, path),

        ParsedType::Array(element_type) => validate_array(schema, json, element_type, path),

        ParsedType::Union(types) => validate_union(schema, json, types, path),

        ParsedType::Null => validate_null(json, path),
    }
}

/// Validate a primitive type
fn validate_primitive(
    json: &JSON,
    prim: &PrimitiveType,
    path: &[PathSegment],
) -> Vec<ValidationError> {
    let expected = match prim {
        PrimitiveType::String => {
            if json.is_string() {
                return vec![];
            }
            "string"
        }
        PrimitiveType::Number => {
            if json.is_number() {
                return vec![];
            }
            "number"
        }
        PrimitiveType::Boolean => {
            if json.is_boolean() {
                return vec![];
            }
            "boolean"
        }
    };

    vec![type_error(path.to_vec(), expected, json_type_name(json))]
}

/// Validate a string literal
fn validate_literal(
    json: &JSON,
    expected: &str,
    path: &[PathSegment],
) -> Vec<ValidationError> {
    match json.as_str() {
        Some(s) if s == expected => vec![],
        Some(s) => vec![ValidationError {
            path: path.to_vec(),
            message: format!("Expected literal '{}', got '{}'", expected, s),
            severity: Severity::Error,
        }],
        None => vec![type_error(path.to_vec(), &format!("literal '{}'", expected), json_type_name(json))],
    }
}

/// Validate a reference to a model or type alias
fn validate_reference(
    schema: &ResolvedSchema,
    json: &JSON,
    name: &str,
    path: &[PathSegment],
) -> Vec<ValidationError> {
    // Check if it's a type alias first
    if let Some(alias) = schema.type_aliases.get(name) {
        let alias_type = match alias.parsed_type() {
            Ok(t) => t,
            Err(e) => {
                return vec![ValidationError {
                    path: path.to_vec(),
                    message: format!("Failed to parse type alias '{}': {}", name, e),
                    severity: Severity::Error,
                }];
            }
        };
        return validate_value(schema, json, &alias_type, path);
    }

    // Check if it's a model
    if let Some(model) = schema.models.get(name) {
        // JSON must be an object
        let obj = match json.as_object() {
            Some(o) => o,
            None => {
                return vec![type_error(path.to_vec(), &format!("model '{}'", name), json_type_name(json))];
            }
        };

        let mut errors = Vec::new();
        let mut seen_fields = HashSet::new();

        // Validate each field
        for field in &model.fields {
            seen_fields.insert(&field.name);

            let field_value = obj.get(&field.name);

            if field_value.is_none() {
                if !field.optional {
                    let mut field_path = path.to_vec();
                    field_path.push(PathSegment {
                        kind: "field".to_string(),
                        name: field.name.clone(),
                    });
                    errors.push(ValidationError {
                        path: field_path,
                        message: format!("Required field '{}' is missing", field.name),
                        severity: Severity::Error,
                    });
                }
                continue;
            }

            let value = field_value.unwrap();
            let parsed_type = match field.parsed_type() {
                Ok(t) => t,
                Err(e) => {
                    let mut field_path = path.to_vec();
                    field_path.push(PathSegment {
                        kind: "field".to_string(),
                        name: field.name.clone(),
                    });
                    errors.push(ValidationError {
                        path: field_path,
                        message: format!("Failed to parse field type: {}", e),
                        severity: Severity::Error,
                    });
                    continue;
                }
            };

            let mut field_path = path.to_vec();
            field_path.push(PathSegment {
                kind: "field".to_string(),
                name: field.name.clone(),
            });

            let field_errors = validate_value(schema, value, &parsed_type, &field_path);
            errors.extend(field_errors);
        }

        // Check for unknown fields
        for (key, _) in obj.iter() {
            if !seen_fields.contains(key) {
                let mut field_path = path.to_vec();
                field_path.push(PathSegment {
                    kind: "field".to_string(),
                    name: key.clone(),
                });
                errors.push(ValidationError {
                    path: field_path,
                    message: format!("Unknown field '{}'", key),
                    severity: Severity::Error,
                });
            }
        }

        return errors;
    }

    // Reference not found
    vec![ValidationError {
        path: path.to_vec(),
        message: format!("Type '{}' not found in schema", name),
        severity: Severity::Error,
    }]
}

/// Validate an array
fn validate_array(
    schema: &ResolvedSchema,
    json: &JSON,
    element_type: &ParsedType,
    path: &[PathSegment],
) -> Vec<ValidationError> {
    let arr = match json.as_array() {
        Some(a) => a,
        None => {
            return vec![type_error(path.to_vec(), "array", json_type_name(json))];
        }
    };

    let mut errors = Vec::new();

    for (index, element) in arr.iter().enumerate() {
        let mut element_path = path.to_vec();
        element_path.push(PathSegment {
            kind: "index".to_string(),
            name: index.to_string(),
        });

        let element_errors = validate_value(schema, element, element_type, &element_path);
        errors.extend(element_errors);
    }

    errors
}

/// Validate a union type
fn validate_union(
    schema: &ResolvedSchema,
    json: &JSON,
    types: &[ParsedType],
    path: &[PathSegment],
) -> Vec<ValidationError> {
    // Try each type in the union
    for typ in types {
        let errors = validate_value(schema, json, typ, path);
        if errors.is_empty() {
            // Found a matching type
            return vec![];
        }
    }

    // No type matched
    let type_names: Vec<String> = types.iter().map(|t| format_type(t)).collect();
    vec![ValidationError {
        path: path.to_vec(),
        message: format!(
            "Value does not match any type in union ({})",
            type_names.join(" | ")
        ),
        severity: Severity::Error,
    }]
}

/// Validate null
fn validate_null(json: &JSON, path: &[PathSegment]) -> Vec<ValidationError> {
    if json.is_null() {
        vec![]
    } else {
        vec![type_error(path.to_vec(), "null", json_type_name(json))]
    }
}

/// Helper to create a type mismatch error
fn type_error(path: Vec<PathSegment>, expected: &str, actual: &str) -> ValidationError {
    ValidationError {
        path,
        message: format!("Expected {}, got {}", expected, actual),
        severity: Severity::Error,
    }
}

/// Get the type name of a JSON value for error messages
fn json_type_name(json: &JSON) -> &'static str {
    match json {
        JSON::Null => "null",
        JSON::Bool(_) => "boolean",
        JSON::Number(_) => "number",
        JSON::String(_) => "string",
        JSON::Array(_) => "array",
        JSON::Object(_) => "object",
    }
}

/// Format a ParsedType for display in error messages
fn format_type(typ: &ParsedType) -> String {
    match typ {
        ParsedType::Primitive(PrimitiveType::String) => "string".to_string(),
        ParsedType::Primitive(PrimitiveType::Number) => "number".to_string(),
        ParsedType::Primitive(PrimitiveType::Boolean) => "boolean".to_string(),
        ParsedType::Literal(s) => format!("\"{}\"", s),
        ParsedType::Reference(name) => name.clone(),
        ParsedType::Array(inner) => format!("{}[]", format_type(inner)),
        ParsedType::Union(types) => {
            let parts: Vec<String> = types.iter().map(|t| format_type(t)).collect();
            parts.join(" | ")
        }
        ParsedType::Null => "null".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cdm_utils::{ResolvedField, ResolvedModel, Position, Span};
    use std::collections::HashMap;

    /// Helper to create a test schema with models and type aliases
    fn create_test_schema() -> ResolvedSchema {
        let mut models = HashMap::new();
        let mut type_aliases = HashMap::new();

        let span = Span {
            start: Position { line: 0, column: 0 },
            end: Position { line: 0, column: 0 }
        };

        // Create User model with fields
        let user_fields = vec![
            ResolvedField::new("id".to_string(), Some("number".to_string()), false, "test.cdm".to_string(), span),
            ResolvedField::new("name".to_string(), Some("string".to_string()), false, "test.cdm".to_string(), span),
            ResolvedField::new("email".to_string(), Some("string".to_string()), true, "test.cdm".to_string(), span),
            ResolvedField::new("role".to_string(), Some("\"admin\" | \"user\"".to_string()), false, "test.cdm".to_string(), span),
        ];

        models.insert(
            "User".to_string(),
            ResolvedModel {
                name: "User".to_string(),
                fields: user_fields,
                parents: vec![],
                plugin_configs: std::collections::HashMap::new(),
                source_file: "test.cdm".to_string(),
                source_span: span,
                entity_id: None,
            },
        );

        // Create Post model with reference to User
        let post_fields = vec![
            ResolvedField::new("title".to_string(), Some("string".to_string()), false, "test.cdm".to_string(), span),
            ResolvedField::new("author".to_string(), Some("User".to_string()), false, "test.cdm".to_string(), span),
            ResolvedField::new("tags".to_string(), Some("string[]".to_string()), false, "test.cdm".to_string(), span),
        ];

        models.insert(
            "Post".to_string(),
            ResolvedModel {
                name: "Post".to_string(),
                fields: post_fields,
                parents: vec![],
                plugin_configs: std::collections::HashMap::new(),
                source_file: "test.cdm".to_string(),
                source_span: span,
                entity_id: None,
            },
        );

        // Create type alias
        let id_alias = cdm::ResolvedTypeAlias::new(
            "ID".to_string(),
            "number".to_string(),
            vec![],
            "test.cdm".to_string(),
            span,
        );

        type_aliases.insert("ID".to_string(), id_alias);

        ResolvedSchema {
            models,
            type_aliases,
        }
    }

    #[test]
    fn test_validate_primitive_string() {
        let schema = create_test_schema();
        let json = serde_json::json!("hello");
        let errors = validate_value(&schema, &json, &ParsedType::Primitive(PrimitiveType::String), &[]);
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_validate_primitive_string_wrong_type() {
        let schema = create_test_schema();
        let json = serde_json::json!(123);
        let errors = validate_value(&schema, &json, &ParsedType::Primitive(PrimitiveType::String), &[]);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "Expected string, got number");
    }

    #[test]
    fn test_validate_primitive_number() {
        let schema = create_test_schema();
        let json = serde_json::json!(42);
        let errors = validate_value(&schema, &json, &ParsedType::Primitive(PrimitiveType::Number), &[]);
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_validate_primitive_boolean() {
        let schema = create_test_schema();
        let json = serde_json::json!(true);
        let errors = validate_value(&schema, &json, &ParsedType::Primitive(PrimitiveType::Boolean), &[]);
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_validate_literal_match() {
        let schema = create_test_schema();
        let json = serde_json::json!("admin");
        let errors = validate_value(&schema, &json, &ParsedType::Literal("admin".to_string()), &[]);
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_validate_literal_mismatch() {
        let schema = create_test_schema();
        let json = serde_json::json!("guest");
        let errors = validate_value(&schema, &json, &ParsedType::Literal("admin".to_string()), &[]);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "Expected literal 'admin', got 'guest'");
    }

    #[test]
    fn test_validate_null() {
        let schema = create_test_schema();
        let json = serde_json::json!(null);
        let errors = validate_value(&schema, &json, &ParsedType::Null, &[]);
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_validate_null_wrong_type() {
        let schema = create_test_schema();
        let json = serde_json::json!("not null");
        let errors = validate_value(&schema, &json, &ParsedType::Null, &[]);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "Expected null, got string");
    }

    #[test]
    fn test_validate_array_empty() {
        let schema = create_test_schema();
        let json = serde_json::json!([]);
        let array_type = ParsedType::Array(Box::new(ParsedType::Primitive(PrimitiveType::String)));
        let errors = validate_value(&schema, &json, &array_type, &[]);
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_validate_array_valid_elements() {
        let schema = create_test_schema();
        let json = serde_json::json!(["a", "b", "c"]);
        let array_type = ParsedType::Array(Box::new(ParsedType::Primitive(PrimitiveType::String)));
        let errors = validate_value(&schema, &json, &array_type, &[]);
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_validate_array_invalid_element() {
        let schema = create_test_schema();
        let json = serde_json::json!(["a", 123, "c"]);
        let array_type = ParsedType::Array(Box::new(ParsedType::Primitive(PrimitiveType::String)));
        let errors = validate_value(&schema, &json, &array_type, &[]);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].path[0].name, "1");
        assert_eq!(errors[0].message, "Expected string, got number");
    }

    #[test]
    fn test_validate_array_not_array() {
        let schema = create_test_schema();
        let json = serde_json::json!("not an array");
        let array_type = ParsedType::Array(Box::new(ParsedType::Primitive(PrimitiveType::String)));
        let errors = validate_value(&schema, &json, &array_type, &[]);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "Expected array, got string");
    }

    #[test]
    fn test_validate_union_first_type_matches() {
        let schema = create_test_schema();
        let json = serde_json::json!("hello");
        let union_type = ParsedType::Union(vec![
            ParsedType::Primitive(PrimitiveType::String),
            ParsedType::Primitive(PrimitiveType::Number),
        ]);
        let errors = validate_value(&schema, &json, &union_type, &[]);
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_validate_union_second_type_matches() {
        let schema = create_test_schema();
        let json = serde_json::json!(42);
        let union_type = ParsedType::Union(vec![
            ParsedType::Primitive(PrimitiveType::String),
            ParsedType::Primitive(PrimitiveType::Number),
        ]);
        let errors = validate_value(&schema, &json, &union_type, &[]);
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_validate_union_no_match() {
        let schema = create_test_schema();
        let json = serde_json::json!(true);
        let union_type = ParsedType::Union(vec![
            ParsedType::Primitive(PrimitiveType::String),
            ParsedType::Primitive(PrimitiveType::Number),
        ]);
        let errors = validate_value(&schema, &json, &union_type, &[]);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("Value does not match any type in union"));
    }

    #[test]
    fn test_validate_json_valid_user() {
        let schema = create_test_schema();
        let json = serde_json::json!({
            "id": 1,
            "name": "Alice",
            "role": "admin"
        });
        let errors = validate_json(&schema, &json, "User");
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_validate_json_valid_user_with_optional_field() {
        let schema = create_test_schema();
        let json = serde_json::json!({
            "id": 1,
            "name": "Alice",
            "email": "alice@example.com",
            "role": "user"
        });
        let errors = validate_json(&schema, &json, "User");
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_validate_json_missing_required_field() {
        let schema = create_test_schema();
        let json = serde_json::json!({
            "id": 1,
            "role": "admin"
        });
        let errors = validate_json(&schema, &json, "User");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].path[0].name, "name");
        assert_eq!(errors[0].message, "Required field 'name' is missing");
    }

    #[test]
    fn test_validate_json_unknown_field() {
        let schema = create_test_schema();
        let json = serde_json::json!({
            "id": 1,
            "name": "Alice",
            "role": "admin",
            "extra": "unexpected"
        });
        let errors = validate_json(&schema, &json, "User");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].path[0].name, "extra");
        assert_eq!(errors[0].message, "Unknown field 'extra'");
    }

    #[test]
    fn test_validate_json_wrong_field_type() {
        let schema = create_test_schema();
        let json = serde_json::json!({
            "id": "not a number",
            "name": "Alice",
            "role": "admin"
        });
        let errors = validate_json(&schema, &json, "User");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].path[0].name, "id");
        assert_eq!(errors[0].message, "Expected number, got string");
    }

    #[test]
    fn test_validate_json_model_not_found() {
        let schema = create_test_schema();
        let json = serde_json::json!({});
        let errors = validate_json(&schema, &json, "NonExistent");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "Model 'NonExistent' not found in schema");
    }

    #[test]
    fn test_validate_json_not_object() {
        let schema = create_test_schema();
        let json = serde_json::json!("not an object");
        let errors = validate_json(&schema, &json, "User");
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("Expected object for model 'User'"));
    }

    #[test]
    fn test_validate_reference_to_model() {
        let schema = create_test_schema();
        let json = serde_json::json!({
            "title": "My Post",
            "author": {
                "id": 1,
                "name": "Alice",
                "role": "admin"
            },
            "tags": ["rust", "cdm"]
        });
        let errors = validate_json(&schema, &json, "Post");
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_validate_reference_to_type_alias() {
        let schema = create_test_schema();
        let json = serde_json::json!(42);
        let errors = validate_value(&schema, &json, &ParsedType::Reference("ID".to_string()), &[]);
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_validate_reference_not_found() {
        let schema = create_test_schema();
        let json = serde_json::json!(42);
        let errors = validate_value(&schema, &json, &ParsedType::Reference("Unknown".to_string()), &[]);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "Type 'Unknown' not found in schema");
    }

    #[test]
    fn test_validate_nested_model_with_invalid_field() {
        let schema = create_test_schema();
        let json = serde_json::json!({
            "title": "My Post",
            "author": {
                "id": 1,
                "name": "Alice",
                "role": "invalid_role"
            },
            "tags": ["rust"]
        });
        let errors = validate_json(&schema, &json, "Post");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].path.len(), 2);
        assert_eq!(errors[0].path[0].name, "author");
        assert_eq!(errors[0].path[1].name, "role");
    }

    #[test]
    fn test_validate_array_of_numbers() {
        let schema = create_test_schema();
        let json = serde_json::json!([1, 2, 3, 4, 5]);
        let array_type = ParsedType::Array(Box::new(ParsedType::Primitive(PrimitiveType::Number)));
        let errors = validate_value(&schema, &json, &array_type, &[]);
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_validate_complex_nested_structure() {
        let schema = create_test_schema();
        let json = serde_json::json!({
            "title": "Nested Post",
            "author": {
                "id": 1,
                "name": "Bob",
                "email": "bob@example.com",
                "role": "user"
            },
            "tags": ["nested", "complex", "test"]
        });
        let errors = validate_json(&schema, &json, "Post");
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_validate_literal_union() {
        let schema = create_test_schema();
        let json = serde_json::json!("admin");
        let union_type = ParsedType::Union(vec![
            ParsedType::Literal("admin".to_string()),
            ParsedType::Literal("user".to_string()),
        ]);
        let errors = validate_value(&schema, &json, &union_type, &[]);
        assert_eq!(errors.len(), 0);
    }
}
