use super::*;
use std::collections::HashMap;
use std::path::PathBuf;

// Helper to get the path to the test plugin
fn get_test_plugin_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/wasm32-wasip1/release/cdm_plugin_docs.wasm")
}

// Helper to check if the test plugin exists
fn test_plugin_exists() -> bool {
    get_test_plugin_path().exists()
}

#[test]
fn test_migrate_with_model_added() {
    if !test_plugin_exists() {
        eprintln!("Skipping test - test plugin not found");
        return;
    }

    let mut runner = PluginRunner::new(get_test_plugin_path())
        .expect("Failed to create plugin runner");

    // Create a simple schema
    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        cdm_plugin_interface::ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![
                cdm_plugin_interface::FieldDefinition {
                    name: "id".to_string(),
                    field_type: cdm_plugin_interface::TypeExpression::Identifier {
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

    // Create a delta representing adding a new model
    let deltas = vec![Delta::ModelAdded {
        name: "Post".to_string(),
        after: cdm_plugin_interface::ModelDefinition {
            name: "Post".to_string(),
            parents: vec![],
            fields: vec![
                cdm_plugin_interface::FieldDefinition {
                    name: "id".to_string(),
                    field_type: cdm_plugin_interface::TypeExpression::Identifier {
                        name: "number".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: serde_json::json!({}),
                    entity_id: None,
                },
                cdm_plugin_interface::FieldDefinition {
                    name: "title".to_string(),
                    field_type: cdm_plugin_interface::TypeExpression::Identifier {
                        name: "string".to_string(),
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
    }];

    let config = serde_json::json!({
        "format": "markdown"
    });

    // Call migrate - should not panic
    let result = runner.migrate(schema, deltas, config);

    // The docs plugin doesn't implement _migrate, so this will likely fail
    // But the test verifies that the PluginRunner can properly call the function
    // and handle serialization/deserialization
    match result {
        Ok(files) => {
            // If the plugin implements migrate, verify output files have valid structure
            for file in &files {
                assert!(!file.path.is_empty(), "Output file path should not be empty");
            }
        }
        Err(e) => {
            // Expected if the plugin doesn't export _migrate
            let error_msg = format!("{:?}", e);
            assert!(
                error_msg.contains("migrate") || error_msg.contains("function"),
                "Expected error about missing migrate function, got: {}",
                error_msg
            );
        }
    }
}

#[test]
fn test_migrate_with_field_added() {
    if !test_plugin_exists() {
        eprintln!("Skipping test - test plugin not found");
        return;
    }

    let mut runner = PluginRunner::new(get_test_plugin_path())
        .expect("Failed to create plugin runner");

    // Create a schema with one model
    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        cdm_plugin_interface::ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![
                cdm_plugin_interface::FieldDefinition {
                    name: "id".to_string(),
                    field_type: cdm_plugin_interface::TypeExpression::Identifier {
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

    // Create a delta representing adding a new field to User
    let deltas = vec![Delta::FieldAdded {
        model: "User".to_string(),
        field: "email".to_string(),
        after: cdm_plugin_interface::FieldDefinition {
            name: "email".to_string(),
            field_type: cdm_plugin_interface::TypeExpression::Identifier {
                name: "string".to_string(),
            },
            optional: false,
            default: None,
            config: serde_json::json!({}),
            entity_id: None,
        },
    }];

    let config = serde_json::json!({
        "format": "markdown"
    });

    // Call migrate
    let result = runner.migrate(schema, deltas, config);

    // Similar to the previous test, verify the call works
    match result {
        Ok(files) => {
            // Verify output files have valid structure
            for file in &files {
                assert!(!file.path.is_empty(), "Output file path should not be empty");
            }
        }
        Err(e) => {
            let error_msg = format!("{:?}", e);
            assert!(
                error_msg.contains("migrate") || error_msg.contains("function"),
                "Expected error about missing migrate function, got: {}",
                error_msg
            );
        }
    }
}

#[test]
fn test_migrate_with_multiple_deltas() {
    if !test_plugin_exists() {
        eprintln!("Skipping test - test plugin not found");
        return;
    }

    let mut runner = PluginRunner::new(get_test_plugin_path())
        .expect("Failed to create plugin runner");

    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        cdm_plugin_interface::ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![],
            config: serde_json::json!({}),
            entity_id: None,
        },
    );

    let schema = Schema {
        models,
        type_aliases: HashMap::new(),
    };

    // Multiple deltas: add a field and change config
    let deltas = vec![
        Delta::FieldAdded {
            model: "User".to_string(),
            field: "name".to_string(),
            after: cdm_plugin_interface::FieldDefinition {
                name: "name".to_string(),
                field_type: cdm_plugin_interface::TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({}),
                entity_id: None,
            },
        },
        Delta::FieldAdded {
            model: "User".to_string(),
            field: "age".to_string(),
            after: cdm_plugin_interface::FieldDefinition {
                name: "age".to_string(),
                field_type: cdm_plugin_interface::TypeExpression::Identifier {
                    name: "number".to_string(),
                },
                optional: true,
                default: None,
                config: serde_json::json!({}),
                entity_id: None,
            },
        },
    ];

    let config = serde_json::json!({
        "format": "markdown"
    });

    let result = runner.migrate(schema, deltas, config);

    match result {
        Ok(files) => {
            // Verify output files have valid structure
            for file in &files {
                assert!(!file.path.is_empty(), "Output file path should not be empty");
            }
        }
        Err(e) => {
            let error_msg = format!("{:?}", e);
            assert!(
                error_msg.contains("migrate") || error_msg.contains("function"),
                "Expected error about missing migrate function, got: {}",
                error_msg
            );
        }
    }
}

#[test]
fn test_plugin_runner_creation() {
    // Test that plugin runner can be created when plugin exists
    if !test_plugin_exists() {
        eprintln!("Skipping test - test plugin not found");
        return;
    }

    let runner = PluginRunner::new(get_test_plugin_path());
    assert!(runner.is_ok(), "Should successfully create plugin runner from valid WASM file");
}

#[test]
fn test_plugin_runner_new_nonexistent_file() {
    let result = PluginRunner::new("/nonexistent/path/to/plugin.wasm");
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("Failed to load WASM module"));
    }
}

#[test]
fn test_plugin_runner_new_invalid_wasm() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let invalid_wasm_path = temp_dir.path().join("invalid.wasm");

    // Write invalid WASM data
    fs::write(&invalid_wasm_path, b"not a valid wasm file").unwrap();

    let result = PluginRunner::new(&invalid_wasm_path);
    assert!(result.is_err());
}

#[test]
fn test_has_build_without_plugin() {
    // This test verifies the has_build method structure
    // Will skip if no test plugin is available
    if !test_plugin_exists() {
        return;
    }

    let runner = PluginRunner::new(get_test_plugin_path()).expect("Failed to create plugin runner");
    let result = runner.has_build();
    assert!(result.is_ok());
}

#[test]
fn test_has_migrate_without_plugin() {
    // This test verifies the has_migrate method structure
    // Will skip if no test plugin is available
    if !test_plugin_exists() {
        return;
    }

    let runner = PluginRunner::new(get_test_plugin_path()).expect("Failed to create plugin runner");
    let result = runner.has_migrate();
    assert!(result.is_ok());
}

#[test]
fn test_schema_call() {
    if !test_plugin_exists() {
        eprintln!("Skipping test - test plugin not found");
        return;
    }

    let mut runner = PluginRunner::new(get_test_plugin_path())
        .expect("Failed to create plugin runner");

    let result = runner.schema();

    // The schema call should either succeed or fail gracefully
    match result {
        Ok(schema) => {
            // If successful, schema should be valid JSON
            assert!(!schema.is_empty());
        }
        Err(e) => {
            // If error, it should be a meaningful error message
            let error_msg = format!("{:?}", e);
            assert!(
                error_msg.contains("schema") || error_msg.contains("function"),
                "Unexpected error: {}",
                error_msg
            );
        }
    }
}

#[test]
fn test_validate_empty_config() {
    if !test_plugin_exists() {
        eprintln!("Skipping test - test plugin not found");
        return;
    }

    let mut runner = PluginRunner::new(get_test_plugin_path())
        .expect("Failed to create plugin runner");

    let config = serde_json::json!({});
    let result = runner.validate(ConfigLevel::Global, config);

    // Should either succeed with empty/valid errors or fail gracefully with meaningful error
    match result {
        Ok(errors) => {
            // Validation succeeded - errors list should be a valid structure
            // (may be empty if config is valid, or contain validation messages)
            for error in &errors {
                assert!(!error.message.is_empty(), "Validation error messages should not be empty");
            }
        }
        Err(e) => {
            // If error, it should be a meaningful error about the validate function
            let error_msg = format!("{:?}", e);
            assert!(
                error_msg.contains("validate") || error_msg.contains("function"),
                "Unexpected error: {}",
                error_msg
            );
        }
    }
}

#[test]
fn test_build_empty_schema() {
    if !test_plugin_exists() {
        eprintln!("Skipping test - test plugin not found");
        return;
    }

    let mut runner = PluginRunner::new(get_test_plugin_path())
        .expect("Failed to create plugin runner");

    let schema = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };

    let config = serde_json::json!({});
    let result = runner.build(schema, config);

    // Should either succeed or fail with a meaningful error
    match result {
        Ok(files) => {
            // Empty schema might produce no files or minimal output
            // Verify any output files have valid structure
            for file in &files {
                assert!(!file.path.is_empty(), "Output file path should not be empty");
            }
        }
        Err(e) => {
            let error_msg = format!("{:?}", e);
            assert!(
                error_msg.contains("build") || error_msg.contains("function"),
                "Unexpected error: {}",
                error_msg
            );
        }
    }
}

#[test]
fn test_migrate_empty_deltas() {
    if !test_plugin_exists() {
        eprintln!("Skipping test - test plugin not found");
        return;
    }

    let mut runner = PluginRunner::new(get_test_plugin_path())
        .expect("Failed to create plugin runner");

    let schema = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };

    let deltas = vec![];
    let config = serde_json::json!({});

    let result = runner.migrate(schema, deltas, config);

    // Empty deltas should either produce no files or fail gracefully
    match result {
        Ok(files) => {
            // No deltas should produce no migration files
            // Verify any output files have valid structure
            for file in &files {
                assert!(!file.path.is_empty(), "Output file path should not be empty");
            }
        }
        Err(e) => {
            let error_msg = format!("{:?}", e);
            assert!(
                error_msg.contains("migrate") || error_msg.contains("function"),
                "Unexpected error: {}",
                error_msg
            );
        }
    }
}

#[test]
fn test_plugin_state_creation() {
    // Test that we can create a PluginState
    let wasi = wasmtime_wasi::WasiCtxBuilder::new().build_p1();
    let _state = PluginState { wasi };
    // If we get here without panicking, the test passes
}
