use cdm_plugin_interface::{OutputFile, Schema, Utils, JSON};
use std::collections::BTreeSet;

use crate::validate::{collect_model_references, is_array_response, strip_array_suffix};

/// Import configuration for schema or type imports
#[derive(Debug, Clone)]
struct ImportConfig {
    strategy: String,
    path: String,
}

impl ImportConfig {
    fn from_json(json: Option<&JSON>, default_path: &str) -> Self {
        match json {
            Some(config) => Self {
                strategy: config
                    .get("strategy")
                    .and_then(|v| v.as_str())
                    .unwrap_or("single")
                    .to_string(),
                path: config
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or(default_path)
                    .to_string(),
            },
            None => Self {
                strategy: "single".to_string(),
                path: default_path.to_string(),
            },
        }
    }

    fn is_per_model(&self) -> bool {
        self.strategy == "per_model"
    }
}

/// Parsed route configuration for code generation
#[derive(Debug, Clone)]
struct Route {
    name: String,
    method: String,
    path: String,
    summary: Option<String>,
    #[allow(dead_code)] // Reserved for future use (JSDoc comments, OpenAPI metadata)
    description: Option<String>,
    path_params: Option<String>,
    query: Option<String>,
    body: Option<String>,
    responses: Vec<(u16, ResponseType)>,
}

/// Response type for a status code
#[derive(Debug, Clone)]
enum ResponseType {
    /// Single model reference (e.g., "User")
    Single(String),
    /// Array of models (e.g., "User[]")
    Array(String),
    /// Union of models (e.g., ["ValidationError", "NotFoundError"])
    Union(Vec<String>),
    /// No content (null in config)
    Void,
}

/// Generates ts-rest contract from the schema
pub fn build(schema: Schema, config: JSON, _utils: &Utils) -> Vec<OutputFile> {
    // Note: build_output is handled by CDM, not by plugins.
    // Plugins return relative paths; CDM prepends the output directory.

    let base_path = config
        .get("base_path")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // Parse import configuration
    let schema_import = ImportConfig::from_json(config.get("schema_import"), "./schemas");

    let routes_config = match config.get("routes") {
        Some(r) => r,
        None => return vec![],
    };

    // Parse routes from config
    let routes = parse_routes(routes_config, base_path);
    if routes.is_empty() {
        return vec![];
    }

    // Collect all model references for imports
    let model_refs = collect_model_references(routes_config);

    // Validate model references against schema (V101-V104)
    // Note: We generate the output even if some models are missing,
    // as the TypeScript compiler will catch those errors
    let valid_models: std::collections::HashSet<String> = schema
        .models
        .keys()
        .cloned()
        .chain(schema.type_aliases.keys().cloned())
        .collect();

    // Generate the contract file
    let content = generate_contract(&routes, &model_refs, &valid_models, &schema_import);

    vec![OutputFile {
        path: "contract.ts".to_string(),
        content,
    }]
}

fn parse_routes(routes_config: &JSON, base_path: &str) -> Vec<Route> {
    let mut routes = Vec::new();

    if let Some(routes_obj) = routes_config.as_object() {
        for (route_name, route_config) in routes_obj {
            if let Some(route) = parse_route(route_name, route_config, base_path) {
                routes.push(route);
            }
        }
    }

    // Sort routes by name for consistent output
    routes.sort_by(|a, b| a.name.cmp(&b.name));
    routes
}

fn parse_route(name: &str, config: &JSON, base_path: &str) -> Option<Route> {
    let method = config.get("method")?.as_str()?.to_uppercase();
    let path = config.get("path")?.as_str()?;

    // Prepend base_path if configured
    let full_path = if base_path.is_empty() {
        path.to_string()
    } else {
        format!("{}{}", base_path, path)
    };

    let summary = config.get("summary").and_then(|v| v.as_str()).map(String::from);
    let description = config.get("description").and_then(|v| v.as_str()).map(String::from);
    let path_params = config.get("pathParams").and_then(|v| v.as_str()).map(String::from);
    let query = config.get("query").and_then(|v| v.as_str()).map(String::from);
    let body = config.get("body").and_then(|v| v.as_str()).map(String::from);

    let responses = parse_responses(config.get("responses")?)?;

    Some(Route {
        name: name.to_string(),
        method,
        path: full_path,
        summary,
        description,
        path_params,
        query,
        body,
        responses,
    })
}

fn parse_responses(responses_config: &JSON) -> Option<Vec<(u16, ResponseType)>> {
    let responses_obj = responses_config.as_object()?;
    let mut responses = Vec::new();

    for (status_code_str, response) in responses_obj {
        let status_code: u16 = status_code_str.parse().ok()?;
        let response_type = parse_response_type(response);
        responses.push((status_code, response_type));
    }

    // Sort by status code for consistent output
    responses.sort_by_key(|(code, _)| *code);
    Some(responses)
}

fn parse_response_type(response: &JSON) -> ResponseType {
    match response {
        JSON::Null => ResponseType::Void,
        JSON::String(model_name) => {
            if is_array_response(model_name) {
                ResponseType::Array(strip_array_suffix(model_name).to_string())
            } else {
                ResponseType::Single(model_name.clone())
            }
        }
        JSON::Array(arr) => {
            let models: Vec<String> = arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
            ResponseType::Union(models)
        }
        _ => ResponseType::Void,
    }
}

fn generate_contract(
    routes: &[Route],
    _model_refs: &std::collections::HashSet<String>,
    valid_models: &std::collections::HashSet<String>,
    schema_import: &ImportConfig,
) -> String {
    let mut output = String::new();

    // Header comment
    output.push_str("/**\n");
    output.push_str(" * Generated by CDM @tsRest plugin\n");
    output.push_str(" * DO NOT EDIT - changes will be overwritten\n");
    output.push_str(" */\n\n");

    // Import ts-rest
    output.push_str("import { initContract } from '@ts-rest/core';\n");
    output.push_str("import { z } from 'zod';\n");

    // Generate schema imports
    let schema_imports_str = generate_schema_imports(routes, valid_models, schema_import);
    if !schema_imports_str.is_empty() {
        output.push_str(&schema_imports_str);
    }

    output.push('\n');

    // Initialize contract
    output.push_str("const c = initContract();\n\n");

    // Generate contract router
    output.push_str("export const contract = c.router({\n");

    for (i, route) in routes.iter().enumerate() {
        output.push_str(&generate_route(route));
        if i < routes.len() - 1 {
            output.push('\n');
        }
    }

    output.push_str("});\n");

    output
}

fn generate_schema_imports(
    routes: &[Route],
    valid_models: &std::collections::HashSet<String>,
    schema_import: &ImportConfig,
) -> String {
    // Collect all model names that need schema imports
    let mut models: BTreeSet<String> = BTreeSet::new();

    for route in routes {
        if let Some(ref path_params) = route.path_params {
            if valid_models.contains(path_params) {
                models.insert(path_params.clone());
            }
        }
        if let Some(ref query) = route.query {
            if valid_models.contains(query) {
                models.insert(query.clone());
            }
        }
        if let Some(ref body) = route.body {
            if valid_models.contains(body) {
                models.insert(body.clone());
            }
        }
        for (_status_code, response) in &route.responses {
            match response {
                ResponseType::Single(model) | ResponseType::Array(model) => {
                    if valid_models.contains(model) {
                        models.insert(model.clone());
                    }
                }
                ResponseType::Union(union_models) => {
                    for model in union_models {
                        if valid_models.contains(model) {
                            models.insert(model.clone());
                        }
                    }
                }
                ResponseType::Void => {}
            }
        }
    }

    if models.is_empty() {
        return String::new();
    }

    if schema_import.is_per_model() {
        // Per-model strategy: generate separate imports for each model
        models
            .iter()
            .map(|model| {
                format!(
                    "import {{ {}Schema }} from '{}/{}';\n",
                    model, schema_import.path, model
                )
            })
            .collect::<Vec<_>>()
            .join("")
    } else {
        // Single file strategy: generate one import with all schemas
        let schema_names: Vec<String> = models
            .iter()
            .map(|m| format!("  {}Schema,", m))
            .collect();
        format!(
            "import {{\n{}\n}} from '{}';\n",
            schema_names.join("\n"),
            schema_import.path
        )
    }
}

fn generate_route(route: &Route) -> String {
    let mut output = String::new();

    output.push_str(&format!("  {}: {{\n", route.name));
    output.push_str(&format!("    method: '{}',\n", route.method));
    output.push_str(&format!("    path: '{}',\n", route.path));

    // Path params
    if let Some(ref path_params) = route.path_params {
        output.push_str(&format!("    pathParams: {}Schema,\n", path_params));
    }

    // Query params
    if let Some(ref query) = route.query {
        output.push_str(&format!("    query: {}Schema,\n", query));
    }

    // Body
    if let Some(ref body) = route.body {
        output.push_str(&format!("    body: {}Schema,\n", body));
    }

    // Responses
    output.push_str("    responses: {\n");
    for (status_code, response) in &route.responses {
        let response_schema = format_response_schema(response);
        output.push_str(&format!("      {}: {},\n", status_code, response_schema));
    }
    output.push_str("    },\n");

    // Summary
    if let Some(ref summary) = route.summary {
        output.push_str(&format!("    summary: '{}',\n", escape_string(summary)));
    }

    output.push_str("  },\n");
    output
}

fn format_response_schema(response: &ResponseType) -> String {
    match response {
        ResponseType::Single(model) => format!("{}Schema", model),
        ResponseType::Array(model) => format!("z.array({}Schema)", model),
        ResponseType::Union(models) => {
            if models.len() == 1 {
                format!("{}Schema", models[0])
            } else {
                let schemas: Vec<String> = models.iter().map(|m| format!("{}Schema", m)).collect();
                format!("z.union([{}])", schemas.join(", "))
            }
        }
        ResponseType::Void => "z.void()".to_string(),
    }
}

fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

#[cfg(test)]
#[path = "build/build_tests.rs"]
mod build_tests;
