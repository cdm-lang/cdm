use super::*;
use cdm_plugin_interface::{FieldDefinition, ModelDefinition, TypeAliasDefinition};
use serde_json::json;
use std::collections::HashMap;

// Helper function to create a minimal test schema
fn create_minimal_schema() -> Schema {
    Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    }
}

// Helper function to create a schema with a simple model
fn create_schema_with_model(name: &str, description: &str) -> Schema {
    let mut models = HashMap::new();
    models.insert(
        name.to_string(),
        ModelDefinition {
            name: name.to_string(),
            fields: vec![],
            parents: vec![],
            config: json!({
                "description": description
            }),
            entity_id: None,
        },
    );

    Schema {
        models,
        type_aliases: HashMap::new(),
    }
}

// Helper function to create a schema with a model with fields
fn create_schema_with_fields() -> Schema {
    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            fields: vec![
                FieldDefinition {
                    name: "id".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({
                        "description": "Unique identifier"
                    }),
                    entity_id: None,
                },
                FieldDefinition {
                    name: "email".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({
                        "description": "Email address"
                    }),
                    entity_id: None,
                },
                FieldDefinition {
                    name: "age".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "number".to_string(),
                    },
                    optional: true,
                    default: None,
                    config: json!({}),
                    entity_id: None,
                },
            ],
            parents: vec![],
            config: json!({
                "description": "A user in the system"
            }),
            entity_id: None,
        },
    );

    Schema {
        models,
        type_aliases: HashMap::new(),
    }
}

#[test]
fn test_build_default_format_is_markdown() {
    let schema = create_minimal_schema();
    let config = json!({});
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].path, "schema.md");
}

#[test]
fn test_build_markdown_format() {
    let schema = create_minimal_schema();
    let config = json!({
        "format": "markdown"
    });
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].path, "schema.md");
    assert!(outputs[0].content.contains("# Schema Documentation"));
}

#[test]
fn test_build_html_format() {
    let schema = create_minimal_schema();
    let config = json!({
        "format": "html"
    });
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].path, "schema.html");
    assert!(outputs[0].content.contains("<!DOCTYPE html>"));
    assert!(outputs[0].content.contains("</html>"));
}

#[test]
fn test_build_json_format() {
    let schema = create_minimal_schema();
    let config = json!({
        "format": "json"
    });
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].path, "schema.json");

    // Should be valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&outputs[0].content)
        .expect("Output should be valid JSON");
    assert!(parsed.is_object());
}

#[test]
fn test_build_unknown_format_returns_empty() {
    let schema = create_minimal_schema();
    let config = json!({
        "format": "unknown"
    });
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    assert_eq!(outputs.len(), 0, "Unknown format should return no outputs");
}

#[test]
fn test_build_markdown_with_custom_title() {
    let schema = create_minimal_schema();
    let config = json!({
        "format": "markdown",
        "title": "My Custom Title"
    });
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    assert!(outputs[0].content.contains("# My Custom Title"));
    assert!(!outputs[0].content.contains("# Schema Documentation"));
}

#[test]
fn test_build_markdown_default_title() {
    let schema = create_minimal_schema();
    let config = json!({
        "format": "markdown"
    });
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    assert!(outputs[0].content.contains("# Schema Documentation"));
}

#[test]
fn test_build_markdown_includes_table_of_contents() {
    let schema = create_schema_with_model("User", "A user model");
    let config = json!({
        "format": "markdown"
    });
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    assert!(outputs[0].content.contains("## Table of Contents"));
    assert!(outputs[0].content.contains("### Models"));
}

#[test]
fn test_build_markdown_includes_model_in_toc() {
    let schema = create_schema_with_model("Product", "A product");
    let config = json!({
        "format": "markdown"
    });
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    assert!(outputs[0].content.contains("- [Product](#model-product)"));
}

#[test]
fn test_build_markdown_includes_model_section() {
    let schema = create_schema_with_model("Order", "An order in the system");
    let config = json!({
        "format": "markdown"
    });
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    assert!(outputs[0].content.contains("## Models"));
    assert!(outputs[0].content.contains("### Model: Order"));
    assert!(outputs[0].content.contains("An order in the system"));
}

#[test]
fn test_build_markdown_with_fields() {
    let schema = create_schema_with_fields();
    let config = json!({
        "format": "markdown"
    });
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    let content = &outputs[0].content;

    // Should have field table
    assert!(content.contains("**Fields:**"));
    assert!(content.contains("| Field | Type | Required | Description |"));

    // Should list all fields
    assert!(content.contains("id"));
    assert!(content.contains("email"));
    assert!(content.contains("age"));

    // Should show correct required status
    assert!(content.contains("Yes")); // for id and email
    assert!(content.contains("No"));  // for age (optional)

    // Should show descriptions
    assert!(content.contains("Unique identifier"));
    assert!(content.contains("Email address"));
}

#[test]
fn test_build_markdown_field_types() {
    let schema = create_schema_with_fields();
    let config = json!({
        "format": "markdown"
    });
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    assert!(outputs[0].content.contains("`string`"));
    assert!(outputs[0].content.contains("`number`"));
}

#[test]
fn test_build_markdown_hidden_model_excluded_from_toc() {
    let mut models = HashMap::new();
    models.insert(
        "HiddenModel".to_string(),
        ModelDefinition {
            name: "HiddenModel".to_string(),
            fields: vec![],
            parents: vec![],
            config: json!({
                "hidden": true
            }),
            entity_id: None,
        },
    );
    models.insert(
        "VisibleModel".to_string(),
        ModelDefinition {
            name: "VisibleModel".to_string(),
            fields: vec![],
            parents: vec![],
            config: json!({}),
            entity_id: None,
        },
    );

    let schema = Schema {
        models,
        type_aliases: HashMap::new(),
    };

    let config = json!({
        "format": "markdown"
    });
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    // Hidden model should not appear in TOC or content
    assert!(!outputs[0].content.contains("HiddenModel"));
    assert!(outputs[0].content.contains("VisibleModel"));
}

#[test]
fn test_build_markdown_with_inheritance() {
    let mut models = HashMap::new();
    models.insert(
        "Admin".to_string(),
        ModelDefinition {
            name: "Admin".to_string(),
            fields: vec![],
            parents: vec!["User".to_string(), "Timestamped".to_string()],
            config: json!({}),
            entity_id: None,
        },
    );

    let schema = Schema {
        models,
        type_aliases: HashMap::new(),
    };

    let config = json!({
        "format": "markdown",
        "include_inheritance": true
    });
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    assert!(outputs[0].content.contains("**Extends:**"));
    assert!(outputs[0].content.contains("User"));
    assert!(outputs[0].content.contains("Timestamped"));
}

#[test]
fn test_build_markdown_without_inheritance_flag() {
    let mut models = HashMap::new();
    models.insert(
        "Admin".to_string(),
        ModelDefinition {
            name: "Admin".to_string(),
            fields: vec![],
            parents: vec!["User".to_string()],
            config: json!({}),
            entity_id: None,
        },
    );

    let schema = Schema {
        models,
        type_aliases: HashMap::new(),
    };

    let config = json!({
        "format": "markdown",
        "include_inheritance": false
    });
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    // Should not show inheritance when flag is false
    assert!(!outputs[0].content.contains("**Extends:**"));
}

#[test]
fn test_build_markdown_with_examples() {
    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            fields: vec![],
            parents: vec![],
            config: json!({
                "example": "{\"id\": \"123\", \"name\": \"John\"}"
            }),
            entity_id: None,
        },
    );

    let schema = Schema {
        models,
        type_aliases: HashMap::new(),
    };

    let config = json!({
        "format": "markdown",
        "include_examples": true
    });
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    assert!(outputs[0].content.contains("**Example:**"));
    assert!(outputs[0].content.contains("```json"));
    assert!(outputs[0].content.contains("\"id\": \"123\""));
}

#[test]
fn test_build_markdown_without_examples_flag() {
    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            fields: vec![],
            parents: vec![],
            config: json!({
                "example": "{\"id\": 1}"
            }),
            entity_id: None,
        },
    );

    let schema = Schema {
        models,
        type_aliases: HashMap::new(),
    };

    let config = json!({
        "format": "markdown",
        "include_examples": false
    });
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    // Should not show examples when flag is false
    assert!(!outputs[0].content.contains("**Example:**"));
}

#[test]
fn test_build_markdown_deprecated_field() {
    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            fields: vec![FieldDefinition {
                name: "oldField".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: json!({
                    "deprecated": true
                }),
                entity_id: None,
            }],
            parents: vec![],
            config: json!({}),
            entity_id: None,
        },
    );

    let schema = Schema {
        models,
        type_aliases: HashMap::new(),
    };

    let config = json!({
        "format": "markdown"
    });
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    // Deprecated fields should have strikethrough
    assert!(outputs[0].content.contains("~~oldField~~"));
}

#[test]
fn test_build_markdown_with_type_aliases() {
    let mut type_aliases = HashMap::new();
    type_aliases.insert(
        "Email".to_string(),
        TypeAliasDefinition {
            name: "Email".to_string(),
            alias_type: TypeExpression::Identifier {
                name: "string".to_string(),
            },
            config: json!({
                "description": "An email address"
            }),
            entity_id: None,
        },
    );

    let schema = Schema {
        models: HashMap::new(),
        type_aliases,
    };

    let config = json!({
        "format": "markdown"
    });
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    assert!(outputs[0].content.contains("## Type Aliases"));
    assert!(outputs[0].content.contains("### Type: Email"));
    assert!(outputs[0].content.contains("An email address"));
    assert!(outputs[0].content.contains("**Type:** `string`"));
}

#[test]
fn test_build_markdown_type_alias_with_example() {
    let mut type_aliases = HashMap::new();
    type_aliases.insert(
        "UUID".to_string(),
        TypeAliasDefinition {
            name: "UUID".to_string(),
            alias_type: TypeExpression::Identifier {
                name: "string".to_string(),
            },
            config: json!({
                "example": "550e8400-e29b-41d4-a716-446655440000"
            }),
            entity_id: None,
        },
    );

    let schema = Schema {
        models: HashMap::new(),
        type_aliases,
    };

    let config = json!({
        "format": "markdown",
        "include_examples": true
    });
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    assert!(outputs[0].content.contains("**Example:**"));
    assert!(outputs[0].content.contains("550e8400-e29b-41d4-a716-446655440000"));
}

#[test]
fn test_build_markdown_hidden_type_alias_excluded() {
    let mut type_aliases = HashMap::new();
    type_aliases.insert(
        "HiddenType".to_string(),
        TypeAliasDefinition {
            name: "HiddenType".to_string(),
            alias_type: TypeExpression::Identifier {
                name: "string".to_string(),
            },
            config: json!({
                "hidden": true
            }),
            entity_id: None,
        },
    );
    type_aliases.insert(
        "VisibleType".to_string(),
        TypeAliasDefinition {
            name: "VisibleType".to_string(),
            alias_type: TypeExpression::Identifier {
                name: "number".to_string(),
            },
            config: json!({}),
            entity_id: None,
        },
    );

    let schema = Schema {
        models: HashMap::new(),
        type_aliases,
    };

    let config = json!({
        "format": "markdown"
    });
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    assert!(!outputs[0].content.contains("HiddenType"));
    assert!(outputs[0].content.contains("VisibleType"));
}

#[test]
fn test_format_type_expression_identifier() {
    let type_expr = TypeExpression::Identifier {
        name: "string".to_string(),
    };

    let result = format_type_expression(&type_expr);
    assert_eq!(result, "string");
}

#[test]
fn test_format_type_expression_array() {
    let type_expr = TypeExpression::Array {
        element_type: Box::new(TypeExpression::Identifier {
            name: "number".to_string(),
        }),
    };

    let result = format_type_expression(&type_expr);
    assert_eq!(result, "number[]");
}

#[test]
fn test_format_type_expression_nested_array() {
    let type_expr = TypeExpression::Array {
        element_type: Box::new(TypeExpression::Array {
            element_type: Box::new(TypeExpression::Identifier {
                name: "string".to_string(),
            }),
        }),
    };

    let result = format_type_expression(&type_expr);
    assert_eq!(result, "string[][]");
}

#[test]
fn test_format_type_expression_union() {
    let type_expr = TypeExpression::Union {
        types: vec![
            TypeExpression::Identifier {
                name: "string".to_string(),
            },
            TypeExpression::Identifier {
                name: "number".to_string(),
            },
        ],
    };

    let result = format_type_expression(&type_expr);
    assert_eq!(result, "string | number");
}

#[test]
fn test_format_type_expression_string_literal() {
    let type_expr = TypeExpression::StringLiteral {
        value: "active".to_string(),
    };

    let result = format_type_expression(&type_expr);
    assert_eq!(result, "\"active\"");
}

#[test]
fn test_format_type_expression_union_with_literals() {
    let type_expr = TypeExpression::Union {
        types: vec![
            TypeExpression::StringLiteral {
                value: "active".to_string(),
            },
            TypeExpression::StringLiteral {
                value: "inactive".to_string(),
            },
        ],
    };

    let result = format_type_expression(&type_expr);
    assert_eq!(result, "\"active\" | \"inactive\"");
}

#[test]
fn test_build_json_serializes_schema() {
    let schema = create_schema_with_model("User", "A user model");
    let config = json!({
        "format": "json"
    });
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    let parsed: serde_json::Value = serde_json::from_str(&outputs[0].content)
        .expect("Should be valid JSON");

    assert!(parsed["models"].is_object());
    assert!(parsed["models"]["User"].is_object());
}

#[test]
fn test_build_html_contains_required_structure() {
    let schema = create_minimal_schema();
    let config = json!({
        "format": "html"
    });
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    let content = &outputs[0].content;

    assert!(content.contains("<!DOCTYPE html>"));
    assert!(content.contains("<html>"));
    assert!(content.contains("<head>"));
    assert!(content.contains("<meta charset=\"utf-8\">"));
    assert!(content.contains("<title>Schema Documentation</title>"));
    assert!(content.contains("<style>"));
    assert!(content.contains("<body>"));
    assert!(content.contains("</body>"));
    assert!(content.contains("</html>"));
}

#[test]
fn test_build_markdown_multiple_models() {
    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            fields: vec![],
            parents: vec![],
            config: json!({}),
            entity_id: None,
        },
    );
    models.insert(
        "Post".to_string(),
        ModelDefinition {
            name: "Post".to_string(),
            fields: vec![],
            parents: vec![],
            config: json!({}),
            entity_id: None,
        },
    );

    let schema = Schema {
        models,
        type_aliases: HashMap::new(),
    };

    let config = json!({
        "format": "markdown"
    });
    let utils = Utils {};

    let outputs = build(schema, config, &utils);

    assert!(outputs[0].content.contains("### Model: User"));
    assert!(outputs[0].content.contains("### Model: Post"));
}
