use cdm_plugin_interface::{OutputFile, Schema, Utils, JSON};
use std::collections::BTreeSet;

use crate::validate::{collect_model_references, is_array_output, is_void_output, strip_array_suffix};

/// Import configuration for schema imports
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

/// Parsed procedure configuration for code generation
#[derive(Debug, Clone)]
struct Procedure {
    name: String,
    procedure_type: String,
    input: Option<String>,
    output: String,
    #[allow(dead_code)] // Reserved for future error handling features
    error: Option<String>,
}

/// Output type for a procedure
#[derive(Debug, Clone)]
enum OutputType {
    /// Single model reference (e.g., "User")
    Single(String),
    /// Array of models (e.g., "User[]")
    Array(String),
    /// No output (void)
    Void,
}

/// Generates tRPC router contract from the schema
pub fn build(schema: Schema, config: JSON, _utils: &Utils) -> Vec<OutputFile> {
    // Note: build_output is handled by CDM, not by plugins.
    // Plugins return relative paths; CDM prepends the output directory.

    // Parse import configuration
    let schema_import = ImportConfig::from_json(config.get("schema_import"), "./types");

    let procedures_config = match config.get("procedures") {
        Some(p) => p,
        None => return vec![],
    };

    // Parse procedures from config
    let procedures = parse_procedures(procedures_config);
    if procedures.is_empty() {
        return vec![];
    }

    // Collect all model references for imports
    let model_refs = collect_model_references(procedures_config);

    // Get valid models from schema (models + type aliases)
    let valid_models: std::collections::HashSet<String> = schema
        .models
        .keys()
        .cloned()
        .chain(schema.type_aliases.keys().cloned())
        .collect();

    // Generate the contract file
    let content = generate_contract(&procedures, &model_refs, &valid_models, &schema_import);

    vec![OutputFile {
        path: "contract.ts".to_string(),
        content,
    }]
}

fn parse_procedures(procedures_config: &JSON) -> Vec<Procedure> {
    let mut procedures = Vec::new();

    if let Some(procedures_obj) = procedures_config.as_object() {
        for (procedure_name, procedure_config) in procedures_obj {
            if let Some(procedure) = parse_procedure(procedure_name, procedure_config) {
                procedures.push(procedure);
            }
        }
    }

    // Sort procedures by name for consistent output
    procedures.sort_by(|a, b| a.name.cmp(&b.name));
    procedures
}

fn parse_procedure(name: &str, config: &JSON) -> Option<Procedure> {
    let procedure_type = config.get("type")?.as_str()?.to_lowercase();
    let output = config.get("output")?.as_str()?.to_string();

    let input = config
        .get("input")
        .and_then(|v| v.as_str())
        .map(String::from);
    let error = config
        .get("error")
        .and_then(|v| v.as_str())
        .map(String::from);

    Some(Procedure {
        name: name.to_string(),
        procedure_type,
        input,
        output,
        error,
    })
}

fn parse_output_type(output: &str) -> OutputType {
    if is_void_output(output) {
        OutputType::Void
    } else if is_array_output(output) {
        OutputType::Array(strip_array_suffix(output).to_string())
    } else {
        OutputType::Single(output.to_string())
    }
}

/// Represents a tree node for nested router generation
#[derive(Debug, Default)]
struct RouterNode {
    /// Child namespaces
    children: std::collections::BTreeMap<String, RouterNode>,
    /// Procedures directly at this level
    procedures: Vec<Procedure>,
}

impl RouterNode {
    fn insert(&mut self, path: &[&str], procedure: Procedure) {
        if path.is_empty() {
            self.procedures.push(procedure);
        } else {
            self.children
                .entry(path[0].to_string())
                .or_default()
                .insert(&path[1..], procedure);
        }
    }
}

/// Build a tree of routers from the flat procedure list
fn build_router_tree(procedures: &[Procedure]) -> RouterNode {
    let mut root = RouterNode::default();

    for procedure in procedures {
        let parts: Vec<&str> = procedure.name.split('.').collect();
        if parts.len() == 1 {
            // Flat procedure (no namespace)
            root.procedures.push(procedure.clone());
        } else {
            // Namespaced procedure - use all but the last part as the path
            let namespace_parts = &parts[..parts.len() - 1];
            let procedure_name = parts[parts.len() - 1];
            let mut namespaced_procedure = procedure.clone();
            namespaced_procedure.name = procedure_name.to_string();
            root.insert(namespace_parts, namespaced_procedure);
        }
    }

    root
}

fn generate_contract(
    procedures: &[Procedure],
    _model_refs: &std::collections::HashSet<String>,
    valid_models: &std::collections::HashSet<String>,
    schema_import: &ImportConfig,
) -> String {
    let mut output = String::new();

    // Header comment
    output.push_str("/**\n");
    output.push_str(" * Generated by CDM @trpc plugin\n");
    output.push_str(" * DO NOT EDIT - changes will be overwritten\n");
    output.push_str(" */\n\n");

    // Import tRPC
    let has_subscriptions = procedures
        .iter()
        .any(|p| p.procedure_type == "subscription");
    if has_subscriptions {
        // Import TRPCError for Observable error type
        output.push_str("import { initTRPC, TRPCError } from '@trpc/server';\n");
        // Import both the function and the type for explicit return type annotations
        output.push_str("import { observable, type Observable } from '@trpc/server/observable';\n");
    } else {
        output.push_str("import { initTRPC } from '@trpc/server';\n");
    }

    // Check if we need zod import (for void, array, or unknown types)
    let needs_zod = procedures.iter().any(|p| {
        // Need z.void() for void outputs
        is_void_output(&p.output)
            // Need z.array() for array outputs
            || is_array_output(&p.output)
            // Need z.unknown() if input model not found
            || p.input.as_ref().is_some_and(|i| !valid_models.contains(strip_array_suffix(i)))
            // Need z.unknown() if output model not found (and not void)
            || (!is_void_output(&p.output) && !valid_models.contains(strip_array_suffix(&p.output)))
    });
    if needs_zod {
        output.push_str("import { z } from 'zod';\n");
    }

    // Generate schema imports
    let schema_imports_str = generate_schema_imports(procedures, valid_models, schema_import);
    if !schema_imports_str.is_empty() {
        output.push_str(&schema_imports_str);
    }

    output.push('\n');

    // Initialize tRPC with context placeholder
    output.push_str("// Initialize tRPC - replace TContext with your context type\n");
    output.push_str("type TContext = Record<string, unknown>;\n");
    output.push_str("const t = initTRPC.context<TContext>().create();\n\n");

    // Procedure builders
    output.push_str("// Procedure builders\n");
    output.push_str("const router = t.router;\n");
    output.push_str("const publicProcedure = t.procedure;\n\n");

    // Build router tree from procedures
    let router_tree = build_router_tree(procedures);

    // Generate router definition
    output.push_str("// Router definition - implement handlers in your server code\n");
    output.push_str("export const appRouter = router({\n");

    output.push_str(&generate_router_content(&router_tree, valid_models, 1));

    output.push_str("});\n\n");

    // Export router type
    output.push_str("// Export router type for client usage\n");
    output.push_str("export type AppRouter = typeof appRouter;\n");

    output
}

/// Generate the content of a router (procedures and nested routers)
fn generate_router_content(
    node: &RouterNode,
    valid_models: &std::collections::HashSet<String>,
    indent_level: usize,
) -> String {
    let mut output = String::new();
    let indent = "  ".repeat(indent_level);

    // Collect all items (both procedures and child routers) for proper comma handling
    let mut items: Vec<String> = Vec::new();

    // Generate procedures at this level
    for procedure in &node.procedures {
        items.push(generate_procedure(procedure, valid_models, indent_level));
    }

    // Generate child routers (sorted for consistent output)
    for (name, child_node) in &node.children {
        let child_content = generate_router_content(child_node, valid_models, indent_level + 1);
        items.push(format!(
            "{}{}: router({{\n{}{}}})",
            indent, name, child_content, indent
        ));
    }

    // Join items with commas and newlines
    for (i, item) in items.iter().enumerate() {
        output.push_str(item);
        if i < items.len() - 1 {
            output.push_str(",\n\n");
        } else {
            output.push_str(",\n");
        }
    }

    output
}

fn generate_schema_imports(
    procedures: &[Procedure],
    valid_models: &std::collections::HashSet<String>,
    schema_import: &ImportConfig,
) -> String {
    // Collect all model names that need schema imports
    let mut models: BTreeSet<String> = BTreeSet::new();
    // Collect model names that need TypeScript type imports (for subscription observable<T>)
    let mut subscription_output_types: BTreeSet<String> = BTreeSet::new();

    for procedure in procedures {
        if let Some(ref input) = procedure.input {
            let model = strip_array_suffix(input);
            if valid_models.contains(model) {
                models.insert(model.to_string());
            }
        }

        if !is_void_output(&procedure.output) {
            let model = strip_array_suffix(&procedure.output);
            if valid_models.contains(model) {
                models.insert(model.to_string());
                // Subscription procedures use observable<T> which requires the TypeScript type
                if procedure.procedure_type == "subscription" {
                    subscription_output_types.insert(model.to_string());
                }
            }
        }

        if let Some(ref error) = procedure.error {
            let model = strip_array_suffix(error);
            if valid_models.contains(model) {
                models.insert(model.to_string());
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
                let type_import = if subscription_output_types.contains(model) {
                    format!("type {}, ", model)
                } else {
                    String::new()
                };
                format!(
                    "import {{ {}{}Schema }} from '{}/{}';\n",
                    type_import, model, schema_import.path, model
                )
            })
            .collect::<Vec<_>>()
            .join("")
    } else {
        // Single file strategy: generate one import with all schemas
        let mut import_items: Vec<String> = Vec::new();

        // Add TypeScript type imports for subscription output types
        for type_name in &subscription_output_types {
            import_items.push(format!("  type {},", type_name));
        }

        // Add schema imports
        for model in &models {
            import_items.push(format!("  {}Schema,", model));
        }

        format!(
            "import {{\n{}\n}} from '{}';\n",
            import_items.join("\n"),
            schema_import.path
        )
    }
}

fn generate_procedure(
    procedure: &Procedure,
    valid_models: &std::collections::HashSet<String>,
    indent_level: usize,
) -> String {
    let mut output = String::new();
    let indent = "  ".repeat(indent_level);
    let inner_indent = "  ".repeat(indent_level + 1);

    output.push_str(&format!("{}{}: publicProcedure\n", indent, procedure.name));

    // Input schema
    if let Some(ref input) = procedure.input {
        let input_schema = format_schema(input, valid_models);
        output.push_str(&format!("{}.input({})\n", inner_indent, input_schema));
    }

    // Output schema
    let output_type = parse_output_type(&procedure.output);
    let output_schema = format_output_schema(&output_type, valid_models);
    output.push_str(&format!("{}.output({})\n", inner_indent, output_schema));

    // Procedure type with handler
    match procedure.procedure_type.as_str() {
        "query" => {
            output.push_str(&generate_query_handler(procedure, &output_type, indent_level));
        }
        "mutation" => {
            output.push_str(&generate_mutation_handler(procedure, &output_type, indent_level));
        }
        "subscription" => {
            output.push_str(&generate_subscription_handler(procedure, &output_type, indent_level));
        }
        _ => {
            // Fallback to query for unknown types
            output.push_str(&generate_query_handler(procedure, &output_type, indent_level));
        }
    }

    output
}

fn generate_query_handler(_procedure: &Procedure, output_type: &OutputType, indent_level: usize) -> String {
    let return_comment = generate_return_comment(output_type, &_procedure.output);
    let inner_indent = "  ".repeat(indent_level + 1);
    let body_indent = "  ".repeat(indent_level + 2);

    // Explicit `: never` return type prevents TS2742 portability errors with Yarn PnP
    // No parameters needed for stub that just throws
    format!(
        "{}.query((): never => {{\n{}// TODO: Implement - return {}\n{}throw new Error('Not implemented');\n{}}})",
        inner_indent, body_indent, return_comment, body_indent, inner_indent
    )
}

fn generate_mutation_handler(_procedure: &Procedure, output_type: &OutputType, indent_level: usize) -> String {
    let return_comment = generate_return_comment(output_type, &_procedure.output);
    let inner_indent = "  ".repeat(indent_level + 1);
    let body_indent = "  ".repeat(indent_level + 2);

    // Explicit `: never` return type prevents TS2742 portability errors with Yarn PnP
    // No parameters needed for stub that just throws
    format!(
        "{}.mutation((): never => {{\n{}// TODO: Implement - return {}\n{}throw new Error('Not implemented');\n{}}})",
        inner_indent, body_indent, return_comment, body_indent, inner_indent
    )
}

fn generate_subscription_handler(_procedure: &Procedure, output_type: &OutputType, indent_level: usize) -> String {
    let emit_type = match output_type {
        OutputType::Single(model) => model.clone(),
        OutputType::Array(model) => format!("{}[]", model),
        OutputType::Void => "void".to_string(),
    };
    let inner_indent = "  ".repeat(indent_level + 1);
    let body_indent = "  ".repeat(indent_level + 2);
    let deep_indent = "  ".repeat(indent_level + 3);

    // Explicit return type `Observable<TValue, TRPCError>` prevents TS2742 portability errors with Yarn PnP
    // No parameters needed for stub
    format!(
        "{}.subscription((): Observable<{}, TRPCError> => {{\n{}return observable<{}>(_emit => {{\n{}// TODO: Implement - call _emit.next(value) when data is available\n{}return () => {{ /* cleanup */ }};\n{}}});\n{}}})",
        inner_indent, emit_type, body_indent, emit_type, deep_indent, deep_indent, body_indent, inner_indent
    )
}

fn generate_return_comment(output_type: &OutputType, original_output: &str) -> String {
    match output_type {
        OutputType::Void => "void".to_string(),
        OutputType::Single(_) | OutputType::Array(_) => original_output.to_string(),
    }
}

fn format_schema(model_name: &str, valid_models: &std::collections::HashSet<String>) -> String {
    let is_array = is_array_output(model_name);
    let base_model = strip_array_suffix(model_name);

    let schema = if valid_models.contains(base_model) {
        format!("{}Schema", base_model)
    } else {
        // If model not found, use z.unknown() as fallback
        "z.unknown()".to_string()
    };

    if is_array {
        format!("z.array({})", schema)
    } else {
        schema
    }
}

fn format_output_schema(
    output_type: &OutputType,
    valid_models: &std::collections::HashSet<String>,
) -> String {
    match output_type {
        OutputType::Void => "z.void()".to_string(),
        OutputType::Single(model) => {
            if valid_models.contains(model) {
                format!("{}Schema", model)
            } else {
                "z.unknown()".to_string()
            }
        }
        OutputType::Array(model) => {
            if valid_models.contains(model) {
                format!("z.array({}Schema)", model)
            } else {
                "z.array(z.unknown())".to_string()
            }
        }
    }
}

#[cfg(test)]
#[path = "build/build_tests.rs"]
mod build_tests;
