use cdm::PluginRunner;
use cdm_plugin_interface::{ConfigLevel, FieldDefinition, ModelDefinition, Schema, TypeExpression};
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

const WASM_PATH: &str = "../../target/wasm32-wasip1/release/cdm_plugin_docs.wasm";

/// Ensures the docs plugin WASM is built before running tests
fn ensure_docs_plugin_built() {
    let wasm_path = PathBuf::from(WASM_PATH);

    if !wasm_path.exists() {
        // Determine the project root (2 levels up from the test binary location)
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let project_root = manifest_dir.parent().unwrap();

        let build_result = Command::new("cargo")
            .current_dir(project_root)
            .args(&[
                "build",
                "--release",
                "--target", "wasm32-wasip1",
                "-p", "cdm-plugin-docs"
            ])
            .status();

        match build_result {
            Ok(status) if status.success() => {
                // Build succeeded
            }
            Ok(status) => {
                panic!("Failed to build docs plugin WASM: exit code {:?}", status.code());
            }
            Err(e) => {
                panic!("Failed to execute cargo build for docs plugin: {}. Make sure 'wasm32-wasip1' target is installed with: rustup target add wasm32-wasip1", e);
            }
        }
    }
}

#[test]
fn test_load_docs_plugin() {
    ensure_docs_plugin_built();
    let wasm_path = "../../target/wasm32-wasip1/release/cdm_plugin_docs.wasm";
    let result = PluginRunner::new(wasm_path);
    assert!(
        result.is_ok(),
        "Failed to load plugin: {:?}",
        result.err()
    );
}

#[test]
fn test_get_schema() {
    ensure_docs_plugin_built();
    let wasm_path = WASM_PATH;
    let mut runner = PluginRunner::new(wasm_path).expect("Failed to load plugin");

    let schema = runner.schema().expect("Failed to get schema");

    // Verify that the schema contains expected sections
    assert!(
        schema.contains("GlobalSettings"),
        "Schema should contain GlobalSettings section"
    );
    assert!(
        schema.contains("ModelSettings"),
        "Schema should contain ModelSettings section"
    );
    assert!(
        schema.contains("FieldSettings"),
        "Schema should contain FieldSettings section"
    );
    assert!(
        schema.contains("format:"),
        "Schema should define format field"
    );
    assert!(
        schema.contains("description?:"),
        "Schema should define optional description field"
    );
}

#[test]
fn test_validate_global_config_valid() {
    ensure_docs_plugin_built();
    let wasm_path = WASM_PATH;
    let mut runner = PluginRunner::new(wasm_path).expect("Failed to load plugin");

    let config = json!({
        "format": "markdown",
        "include_examples": true,
        "title": "Test Documentation"
    });

    let errors = runner
        .validate(ConfigLevel::Global, config)
        .expect("Failed to validate config");

    assert_eq!(
        errors.len(),
        0,
        "Expected no validation errors, got: {:?}",
        errors
    );
}

#[test]
fn test_validate_global_config_invalid_format() {
    ensure_docs_plugin_built();
    let wasm_path = WASM_PATH;
    let mut runner = PluginRunner::new(wasm_path).expect("Failed to load plugin");

    let config = json!({
        "format": "invalid_format"
    });

    let errors = runner
        .validate(ConfigLevel::Global, config)
        .expect("Failed to validate config");

    assert!(
        errors.len() > 0,
        "Expected validation errors for invalid format"
    );
    assert!(
        errors[0].message.contains("Invalid format"),
        "Error message should mention invalid format, got: {}",
        errors[0].message
    );
}

#[test]
fn test_generate_markdown() {
    ensure_docs_plugin_built();
    let wasm_path = WASM_PATH;
    let mut runner = PluginRunner::new(wasm_path).expect("Failed to load plugin");

    let schema = Schema {
        models: HashMap::from([(
            "User".to_string(),
            ModelDefinition {
                name: "User".to_string(),
                parents: vec![],
                fields: vec![FieldDefinition {
                    name: "id".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: None,
                }],
                config: json!({}),
                entity_id: None,
            },
        )]),
        type_aliases: HashMap::new(),
    };

    let config = json!({
        "format": "markdown"
    });

    let files = runner
        .build(schema, config)
        .expect("Failed to generate files");

    assert_eq!(files.len(), 1, "Expected 1 output file");
    assert_eq!(files[0].path, "schema.md");
    assert!(
        files[0].content.contains("User"),
        "Output should contain 'User' model"
    );
    assert!(
        files[0].content.contains("id"),
        "Output should contain 'id' field"
    );
}

#[test]
fn test_generate_html() {
    ensure_docs_plugin_built();
    let wasm_path = WASM_PATH;
    let mut runner = PluginRunner::new(wasm_path).expect("Failed to load plugin");

    let schema = Schema {
        models: HashMap::from([(
            "Product".to_string(),
            ModelDefinition {
                name: "Product".to_string(),
                parents: vec![],
                fields: vec![
                    FieldDefinition {
                        name: "name".to_string(),
                        field_type: TypeExpression::Identifier {
                            name: "string".to_string(),
                        },
                        optional: false,
                        default: None,
                        config: json!({ "description": "Product name" }),
                        entity_id: None,
                    },
                    FieldDefinition {
                        name: "price".to_string(),
                        field_type: TypeExpression::Identifier {
                            name: "number".to_string(),
                        },
                        optional: false,
                        default: None,
                        config: json!({}),
                        entity_id: None,
                    },
                ],
                config: json!({ "description": "Represents a product" }),
                entity_id: None,
            },
        )]),
        type_aliases: HashMap::new(),
    };

    let config = json!({
        "format": "html"
    });

    let files = runner
        .build(schema, config)
        .expect("Failed to generate files");

    assert_eq!(files.len(), 1, "Expected 1 output file");
    assert_eq!(files[0].path, "schema.html");
    assert!(
        files[0].content.contains("Product"),
        "Output should contain 'Product' model"
    );
}

#[test]
fn test_validate_model_config() {
    ensure_docs_plugin_built();
    let wasm_path = WASM_PATH;
    let mut runner = PluginRunner::new(wasm_path).expect("Failed to load plugin");

    let config = json!({
        "description": "A user model",
        "hidden": false
    });

    let errors = runner
        .validate(
            ConfigLevel::Model {
                name: "User".to_string(),
            },
            config,
        )
        .expect("Failed to validate config");

    assert_eq!(
        errors.len(),
        0,
        "Expected no validation errors, got: {:?}",
        errors
    );
}

#[test]
fn test_validate_field_config() {
    ensure_docs_plugin_built();
    let wasm_path = WASM_PATH;
    let mut runner = PluginRunner::new(wasm_path).expect("Failed to load plugin");

    let config = json!({
        "description": "User ID field",
        "deprecated": true
    });

    let errors = runner
        .validate(
            ConfigLevel::Field {
                model: "User".to_string(),
                field: "id".to_string(),
            },
            config,
        )
        .expect("Failed to validate config");

    assert_eq!(
        errors.len(),
        0,
        "Expected no validation errors, got: {:?}",
        errors
    );
}

#[test]
fn test_validate_config_is_optional() {
    ensure_docs_plugin_built();
    // This test verifies that if a plugin doesn't have validate_config,
    // the validate() method returns an empty error array instead of failing.
    // Since cdm-plugin-docs DOES have validate_config, this test
    // serves as documentation of the expected behavior.
    let wasm_path = WASM_PATH;
    let mut runner = PluginRunner::new(wasm_path).expect("Failed to load plugin");

    let config = json!({
        "format": "markdown"
    });

    let result = runner.validate(ConfigLevel::Global, config);

    assert!(
        result.is_ok(),
        "validate() should not fail even if plugin doesn't have validate_config"
    );

    // For cdm-plugin-docs, it should return errors (or empty array)
    // We just verify we got a valid result
    let _ = result.unwrap();
}
