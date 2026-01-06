use cdm_plugin_interface::{OutputFile, Schema, Utils, JSON};
use serde_json::{json, Map, Value};

use crate::type_mapper::{apply_field_constraints, TypeMapper};

/// Generates JSON Schema files from the CDM schema
pub fn build(schema: Schema, config: JSON, _utils: &Utils) -> Vec<OutputFile> {
    let mut files = Vec::new();

    // Extract global configuration
    let draft = config
        .get("draft")
        .and_then(|v| v.as_str())
        .unwrap_or("draft7");

    let include_schema_property = config
        .get("include_schema_property")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let include_examples = config
        .get("include_examples")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let include_descriptions = config
        .get("include_descriptions")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let output_mode = config
        .get("output_mode")
        .and_then(|v| v.as_str())
        .unwrap_or("single-file");

    let relationship_mode = config
        .get("relationship_mode")
        .and_then(|v| v.as_str())
        .unwrap_or("reference");

    let union_mode = config
        .get("union_mode")
        .and_then(|v| v.as_str())
        .unwrap_or("enum")
        .to_string();

    let schema_id = config
        .get("schema_id")
        .and_then(|v| v.as_str());

    let root_model_name = config
        .get("root_model")
        .and_then(|v| v.as_str());

    // Create type mapper
    let mut type_mapper = TypeMapper::new(union_mode);

    if output_mode == "single-file" {
        // Generate a single file with all models under $defs
        let content = generate_single_file(
            &schema,
            &mut type_mapper,
            draft,
            include_schema_property,
            include_examples,
            include_descriptions,
            relationship_mode,
            schema_id,
            root_model_name,
        );

        files.push(OutputFile {
            path: "schema.json".to_string(),
            content,
        });
    } else {
        // Generate separate files for each model
        for (model_name, model_def) in schema.models.iter() {
            // Check if model should be skipped
            let skip = model_def.config
                .get("skip")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if skip {
                continue;
            }

            let content = generate_model_file(
                model_name,
                model_def,
                &schema,
                &mut type_mapper,
                draft,
                include_schema_property,
                include_examples,
                include_descriptions,
                relationship_mode,
                &model_def.config,
            );

            files.push(OutputFile {
                path: format!("{}.schema.json", model_name),
                content,
            });
        }
    }

    files
}

fn generate_single_file(
    schema: &Schema,
    type_mapper: &mut TypeMapper,
    draft: &str,
    include_schema_property: bool,
    include_examples: bool,
    include_descriptions: bool,
    relationship_mode: &str,
    schema_id: Option<&str>,
    root_model_name: Option<&str>,
) -> String {
    let mut root = Map::new();

    // Add $schema
    if include_schema_property {
        let schema_url = get_schema_url(draft);
        root.insert("$schema".to_string(), json!(schema_url));
    }

    // Add $id if specified
    if let Some(id) = schema_id {
        root.insert("$id".to_string(), json!(id));
    }

    // Find the root model (either specified or first non-skipped model)
    let root_model = if let Some(name) = root_model_name {
        schema.models.get(name)
    } else {
        // Check for models with is_root = true
        schema.models.iter()
            .find(|(_, model_def)| {
                model_def.config.get("is_root")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            })
            .map(|(_, model_def)| model_def)
            .or_else(|| {
                // Otherwise, use the first non-skipped model
                schema.models.values().find(|model_def| {
                    !model_def.config.get("skip")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                })
            })
    };

    // Generate root schema from root model
    if let Some(model_def) = root_model {
        let model_schema = generate_model_schema(
            model_def,
            type_mapper,
            include_examples,
            include_descriptions,
            relationship_mode,
            &model_def.config,
        );

        // Merge model schema into root
        if let Value::Object(model_obj) = model_schema {
            for (key, value) in model_obj {
                root.insert(key, value);
            }
        }
    } else {
        // No root model, just make it accept any object
        root.insert("type".to_string(), json!("object"));
    }

    // Generate $defs for all models (except the root)
    let mut defs = Map::new();

    for (model_name, model_def) in schema.models.iter() {
        // Skip if this is the root model or if skip is set
        let skip = model_def.config.get("skip").and_then(|v| v.as_bool()).unwrap_or(false);
        let is_root = model_def.config.get("is_root").and_then(|v| v.as_bool()).unwrap_or(false);
        let is_named_root = Some(model_name.as_str()) == root_model_name;

        if skip || is_root || is_named_root {
            continue;
        }

        let model_schema = generate_model_schema(
            model_def,
            type_mapper,
            include_examples,
            include_descriptions,
            relationship_mode,
            &model_def.config,
        );

        defs.insert(model_name.clone(), model_schema);
    }

    // Add type aliases to $defs
    for (alias_name, alias_def) in schema.type_aliases.iter() {
        let skip = alias_def.config.get("skip").and_then(|v| v.as_bool()).unwrap_or(false);

        if skip {
            continue;
        }

        let union_mode_override = alias_def.config.get("union_mode").and_then(|v| v.as_str());
        let mut alias_schema = type_mapper.map_type(&alias_def.alias_type, union_mode_override, &alias_def.config);

        // Add description if provided and enabled
        if include_descriptions {
            if let Some(description) = alias_def.config.get("description") {
                if let Value::Object(ref mut obj) = alias_schema {
                    obj.insert("description".to_string(), description.clone());
                }
            }
        }

        defs.insert(alias_name.clone(), alias_schema);
    }

    if !defs.is_empty() {
        root.insert("$defs".to_string(), Value::Object(defs));
    }

    serde_json::to_string_pretty(&Value::Object(root)).unwrap()
}

fn generate_model_file(
    _model_name: &str,
    model_def: &cdm_plugin_interface::ModelDefinition,
    _schema: &Schema,
    type_mapper: &mut TypeMapper,
    draft: &str,
    include_schema_property: bool,
    include_examples: bool,
    include_descriptions: bool,
    relationship_mode: &str,
    model_config: &JSON,
) -> String {
    let mut root = Map::new();

    // Add $schema
    if include_schema_property {
        let schema_url = get_schema_url(draft);
        root.insert("$schema".to_string(), json!(schema_url));
    }

    // Add model schema
    let model_schema = generate_model_schema(
        model_def,
        type_mapper,
        include_examples,
        include_descriptions,
        relationship_mode,
        model_config,
    );

    // Merge model schema into root
    if let Value::Object(model_obj) = model_schema {
        for (key, value) in model_obj {
            root.insert(key, value);
        }
    }

    serde_json::to_string_pretty(&Value::Object(root)).unwrap()
}

fn generate_model_schema(
    model_def: &cdm_plugin_interface::ModelDefinition,
    type_mapper: &mut TypeMapper,
    include_examples: bool,
    include_descriptions: bool,
    _relationship_mode: &str,
    model_config: &JSON,
) -> Value {
    let mut schema = Map::new();

    schema.insert("type".to_string(), json!("object"));

    // Add title if specified
    if let Some(title) = model_config.get("title") {
        schema.insert("title".to_string(), title.clone());
    }

    // Add description if specified and enabled
    if include_descriptions {
        if let Some(description) = model_config.get("description") {
            schema.insert("description".to_string(), description.clone());
        }
    }

    // Generate properties
    let mut properties = Map::new();
    let mut required = Vec::new();

    for field in &model_def.fields {
        // Check if field should be skipped
        let skip = field.config.get("skip").and_then(|v| v.as_bool()).unwrap_or(false);
        if skip {
            continue;
        }

        // Get union mode override for type aliases
        let union_mode_override = field.config.get("union_mode").and_then(|v| v.as_str());

        // Map field type
        let field_schema = type_mapper.map_type(&field.field_type, union_mode_override, &field.config);

        // Convert field schema to Map for modifications
        if let Value::Object(mut field_obj) = field_schema {
            // Apply field constraints
            field_obj = apply_field_constraints(field_obj, &field.config);

            // Add examples if enabled
            if include_examples {
                if let Some(examples) = field.config.get("examples") {
                    field_obj.insert("examples".to_string(), examples.clone());
                }
            }

            // Add description if enabled
            if include_descriptions {
                if let Some(description) = field.config.get("description") {
                    field_obj.insert("description".to_string(), description.clone());
                }
            }

            properties.insert(field.name.clone(), Value::Object(field_obj));
        } else {
            properties.insert(field.name.clone(), field_schema);
        }

        // Track required fields
        if !field.optional {
            required.push(field.name.clone());
        }
    }

    schema.insert("properties".to_string(), Value::Object(properties));

    if !required.is_empty() {
        schema.insert("required".to_string(), Value::Array(
            required.into_iter().map(|s| json!(s)).collect()
        ));
    }

    // Additional properties
    if let Some(additional_props) = model_config.get("additional_properties") {
        schema.insert("additionalProperties".to_string(), additional_props.clone());
    } else {
        // Default to false (strict mode)
        schema.insert("additionalProperties".to_string(), json!(false));
    }

    Value::Object(schema)
}

fn get_schema_url(draft: &str) -> &'static str {
    match draft {
        "draft4" => "http://json-schema.org/draft-04/schema#",
        "draft6" => "http://json-schema.org/draft-06/schema#",
        "draft7" => "http://json-schema.org/draft-07/schema#",
        "draft2019-09" => "https://json-schema.org/draft/2019-09/schema",
        "draft2020-12" => "https://json-schema.org/draft/2020-12/schema",
        _ => "http://json-schema.org/draft-07/schema#", // Default
    }
}

#[cfg(test)]
#[path = "build/build_tests.rs"]
mod build_tests;
