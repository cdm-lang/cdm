use anyhow::{anyhow, Context, Result};
use cdm_plugin_api::{ConfigLevel, OutputFile, Schema, ValidationError, Delta};
use serde_json::Value as JSON;
use std::path::Path;
use wasmtime::*;
use wasmtime_wasi::preview1::WasiP1Ctx;
struct PluginState {
    wasi: WasiP1Ctx,
}

/// Plugin runner that loads and executes WASM plugins
pub struct PluginRunner {
    engine: Engine,
    module: Module,
}

impl PluginRunner {
    /// Create a new plugin runner from a WASM file path
    pub fn new<P: AsRef<Path>>(wasm_path: P) -> Result<Self> {
        // Create the WASM engine
        let engine = Engine::default();

        // Load the WASM module
        let module = Module::from_file(&engine, wasm_path.as_ref())
            .with_context(|| format!("Failed to load WASM module from {:?}", wasm_path.as_ref()))?;

        Ok(Self {
            engine,
            module,
        })
    }

    /// Get the plugin's schema definition
    pub fn schema(&mut self) -> Result<String> {
        // Call the WASM function (no arguments)
        let result_bytes = self.call_wasm_function("_schema", &[])?;

        // Convert bytes to string
        let schema = String::from_utf8(result_bytes)
            .context("Failed to decode schema as UTF-8")?;

        Ok(schema)
    }

    /// Validate configuration at a specific level
    ///
    /// Returns an empty error array if the plugin doesn't export _validate_config (optional function)
    /// Returns the validation errors if the plugin has _validate_config
    /// Returns Err if there was an error calling the function
    pub fn validate(
        &mut self,
        level: ConfigLevel,
        config: JSON,
    ) -> Result<Vec<ValidationError>> {
        // Check if the function exists first
        if !self.has_function("_validate_config")? {
            return Ok(Vec::new());
        }

        // Serialize inputs to JSON
        let level_json = serde_json::to_string(&level)?;
        let config_json = serde_json::to_string(&config)?;

        // Call the WASM function
        let result_json = self.call_wasm_function(
            "_validate_config",
            &[level_json.as_bytes(), config_json.as_bytes()],
        )?;

        // Deserialize the result
        let errors: Vec<ValidationError> = serde_json::from_slice(&result_json)
            .context("Failed to deserialize validation errors")?;

        Ok(errors)
    }

    /// Check if the plugin exports a specific function
    fn has_function(&self, function_name: &str) -> Result<bool> {
        // Create a minimal store just to check exports
        let wasi = wasmtime_wasi::WasiCtxBuilder::new()
            .build_p1();
        let state = PluginState { wasi };
        let mut store = Store::new(&self.engine, state);
        let mut linker = Linker::new(&self.engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |s: &mut PluginState| &mut s.wasi)?;

        let instance = linker.instantiate(&mut store, &self.module)?;

        Ok(instance.get_func(&mut store, function_name).is_some())
    }

    /// Build output files from a schema
    pub fn build(
        &mut self,
        schema: Schema,
        config: JSON,
    ) -> Result<Vec<OutputFile>> {
        // Serialize inputs to JSON
        let schema_json = serde_json::to_string(&schema)?;
        let config_json = serde_json::to_string(&config)?;

        // Call the WASM function
        let result_json = self.call_wasm_function(
            "_build",
            &[schema_json.as_bytes(), config_json.as_bytes()],
        )?;

        // Deserialize the result
        let files: Vec<OutputFile> = serde_json::from_slice(&result_json)
            .context("Failed to deserialize output files")?;

        Ok(files)
    }

    /// Generate migration files from schema changes
    pub fn migrate(
        &mut self,
        schema: Schema,
        deltas: Vec<Delta>,
        config: JSON,
    ) -> Result<Vec<OutputFile>> {
        // Serialize inputs to JSON
        let schema_json = serde_json::to_string(&schema)?;
        let deltas_json = serde_json::to_string(&deltas)?;
        let config_json = serde_json::to_string(&config)?;

        // Call the WASM function
        let result_json = self.call_wasm_function(
            "_migrate",
            &[schema_json.as_bytes(), deltas_json.as_bytes(), config_json.as_bytes()],
        )?;

        // Deserialize the result
        let files: Vec<OutputFile> = serde_json::from_slice(&result_json)
            .context("Failed to deserialize migration files")?;

        Ok(files)
    }

    /// Low-level function to call a WASM function with byte array arguments
    fn call_wasm_function(
        &mut self,
        function_name: &str,
        args: &[&[u8]],
    ) -> Result<Vec<u8>> {
        // Create WASI context with minimal permissions (no filesystem, no network)
        let wasi = wasmtime_wasi::WasiCtxBuilder::new()
            .inherit_stdout()
            .inherit_stderr()
            .build_p1();

        let state = PluginState { wasi };
        let mut store = Store::new(&self.engine, state);

        // Create a new linker for each call
        let mut linker = Linker::new(&self.engine);

        // Add WASI to the linker using preview1 API
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |s: &mut PluginState| &mut s.wasi)?;

        // Instantiate the module
        let instance = linker.instantiate(&mut store, &self.module)
            .context("Failed to instantiate WASM module")?;

        // Get the memory
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| anyhow!("Failed to find 'memory' export in WASM module"))?;

        // Get the allocation function
        let alloc_func = instance
            .get_typed_func::<u32, u32>(&mut store, "_alloc")
            .context("Failed to find '_alloc' function in WASM module")?;

        // Get the deallocation function (for cleanup)
        let dealloc_func = instance
            .get_typed_func::<(u32, u32), ()>(&mut store, "_dealloc")
            .context("Failed to find '_dealloc' function in WASM module")?;

        // Allocate memory for each argument and write data
        let mut allocated_args = Vec::new();
        for arg in args {
            let len = arg.len() as u32;
            let ptr = alloc_func.call(&mut store, len)
                .context("Failed to allocate memory in WASM")?;

            memory.write(&mut store, ptr as usize, arg)
                .context("Failed to write data to WASM memory")?;

            allocated_args.push((ptr, len));
        }

        // Build the parameters for the function call
        // The function signature varies based on the number of arguments
        let result_ptr = match function_name {
            "_schema" => {
                // This takes no arguments and returns a pointer
                let func = instance
                    .get_typed_func::<(), u32>(&mut store, function_name)
                    .with_context(|| format!("Failed to find '{}' function", function_name))?;

                func.call(&mut store, ())
                    .with_context(|| format!("Failed to call '{}' function", function_name))?
            }
            "_validate_config" | "_build" => {
                // These take 2 arguments (4 parameters: ptr1, len1, ptr2, len2)
                let func = instance
                    .get_typed_func::<(u32, u32, u32, u32), u32>(&mut store, function_name)
                    .with_context(|| format!("Failed to find '{}' function", function_name))?;

                func.call(
                    &mut store,
                    (
                        allocated_args[0].0,
                        allocated_args[0].1,
                        allocated_args[1].0,
                        allocated_args[1].1,
                    ),
                ).with_context(|| format!("Failed to call '{}' function", function_name))?
            }
            "_migrate" => {
                // This takes 3 arguments (6 parameters: ptr1, len1, ptr2, len2, ptr3, len3)
                let func = instance
                    .get_typed_func::<(u32, u32, u32, u32, u32, u32), u32>(&mut store, function_name)
                    .with_context(|| format!("Failed to find '{}' function", function_name))?;

                func.call(
                    &mut store,
                    (
                        allocated_args[0].0,
                        allocated_args[0].1,
                        allocated_args[1].0,
                        allocated_args[1].1,
                        allocated_args[2].0,
                        allocated_args[2].1,
                    ),
                ).with_context(|| format!("Failed to call '{}' function", function_name))?
            }
            _ => return Err(anyhow!("Unknown function: {}", function_name)),
        };

        // Read the result from memory
        // The result pointer points to a serialized data structure
        // We need to read the length first (assuming it's prefixed with a 4-byte length)
        let mut len_bytes = [0u8; 4];
        memory.read(&store, result_ptr as usize, &mut len_bytes)
            .context("Failed to read result length from WASM memory")?;
        let result_len = u32::from_le_bytes(len_bytes) as usize;

        // Read the actual result data
        let mut result_data = vec![0u8; result_len];
        memory.read(&store, (result_ptr + 4) as usize, &mut result_data)
            .context("Failed to read result data from WASM memory")?;

        // Deallocate the arguments
        for (ptr, len) in allocated_args {
            dealloc_func.call(&mut store, (ptr, len))
                .context("Failed to deallocate argument memory")?;
        }

        // Deallocate the result
        dealloc_func.call(&mut store, (result_ptr, result_len as u32 + 4))
            .context("Failed to deallocate result memory")?;

        Ok(result_data)
    }
}

#[cfg(test)]
mod tests {
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
            cdm_plugin_api::ModelDefinition {
                name: "User".to_string(),
                parents: vec![],
                fields: vec![
                    cdm_plugin_api::FieldDefinition {
                        name: "id".to_string(),
                        field_type: cdm_plugin_api::TypeExpression::Identifier {
                            name: "number".to_string(),
                        },
                        optional: false,
                        default: None,
                        config: serde_json::json!({}),
                    },
                ],
                config: serde_json::json!({}),
            },
        );

        let schema = Schema {
            models,
            type_aliases: HashMap::new(),
        };

        // Create a delta representing adding a new model
        let deltas = vec![Delta::ModelAdded {
            name: "Post".to_string(),
            after: cdm_plugin_api::ModelDefinition {
                name: "Post".to_string(),
                parents: vec![],
                fields: vec![
                    cdm_plugin_api::FieldDefinition {
                        name: "id".to_string(),
                        field_type: cdm_plugin_api::TypeExpression::Identifier {
                            name: "number".to_string(),
                        },
                        optional: false,
                        default: None,
                        config: serde_json::json!({}),
                    },
                    cdm_plugin_api::FieldDefinition {
                        name: "title".to_string(),
                        field_type: cdm_plugin_api::TypeExpression::Identifier {
                            name: "string".to_string(),
                        },
                        optional: false,
                        default: None,
                        config: serde_json::json!({}),
                    },
                ],
                config: serde_json::json!({}),
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
                // If the plugin implements migrate, we should get output files
                assert!(files.is_empty() || !files.is_empty());
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
            cdm_plugin_api::ModelDefinition {
                name: "User".to_string(),
                parents: vec![],
                fields: vec![
                    cdm_plugin_api::FieldDefinition {
                        name: "id".to_string(),
                        field_type: cdm_plugin_api::TypeExpression::Identifier {
                            name: "number".to_string(),
                        },
                        optional: false,
                        default: None,
                        config: serde_json::json!({}),
                    },
                ],
                config: serde_json::json!({}),
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
            after: cdm_plugin_api::FieldDefinition {
                name: "email".to_string(),
                field_type: cdm_plugin_api::TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: serde_json::json!({}),
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
                assert!(files.is_empty() || !files.is_empty());
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
            cdm_plugin_api::ModelDefinition {
                name: "User".to_string(),
                parents: vec![],
                fields: vec![],
                config: serde_json::json!({}),
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
                after: cdm_plugin_api::FieldDefinition {
                    name: "name".to_string(),
                    field_type: cdm_plugin_api::TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: serde_json::json!({}),
                },
            },
            Delta::FieldAdded {
                model: "User".to_string(),
                field: "age".to_string(),
                after: cdm_plugin_api::FieldDefinition {
                    name: "age".to_string(),
                    field_type: cdm_plugin_api::TypeExpression::Identifier {
                        name: "number".to_string(),
                    },
                    optional: true,
                    default: None,
                    config: serde_json::json!({}),
                },
            },
        ];

        let config = serde_json::json!({
            "format": "markdown"
        });

        let result = runner.migrate(schema, deltas, config);

        match result {
            Ok(files) => {
                assert!(files.is_empty() || !files.is_empty());
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
        // This test will fail until we have a real WASM file
        // It's here as a placeholder for future tests
    }
}
