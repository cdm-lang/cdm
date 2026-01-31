use cdm_plugin_interface::{CaseFormat, OutputFile, Schema, TypeExpression, Utils, JSON};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

use crate::type_mapper::map_type_to_typescript;
use crate::zod_mapper::map_type_to_zod;

/// Tracks imports needed for a TypeScript file
#[derive(Debug, Default)]
struct ImportCollector {
    /// Model imports: model name -> file name (without .ts extension)
    model_imports: BTreeSet<String>,
    /// Type alias imports from types.ts
    type_alias_imports: BTreeSet<String>,
    /// Whether Zod import is needed
    needs_zod: bool,
    /// Whether to include Schema imports for Zod
    include_zod_schemas: bool,
}

impl ImportCollector {
    fn new() -> Self {
        Self::default()
    }

    fn add_model(&mut self, name: &str) {
        self.model_imports.insert(name.to_string());
    }

    fn add_type_alias(&mut self, name: &str) {
        self.type_alias_imports.insert(name.to_string());
    }

    fn set_needs_zod(&mut self, needs_zod: bool) {
        self.needs_zod = needs_zod;
    }

    fn set_include_zod_schemas(&mut self, include: bool) {
        self.include_zod_schemas = include;
    }

    /// Generate import statements for a file
    /// `current_model` is the model name of the current file (to avoid self-import)
    /// `model_to_file` maps model names to their file names
    /// `models_in_current_file` is the set of model names that are in the current file
    fn to_import_statements(
        &self,
        current_file: &str,
        model_to_file: &HashMap<String, String>,
        models_in_current_file: &BTreeSet<String>,
    ) -> String {
        let mut result = String::new();

        // Zod import
        if self.needs_zod {
            result.push_str("import { z } from 'zod';\n");
        }

        // Model imports - group by file
        let mut imports_by_file: HashMap<String, Vec<String>> = HashMap::new();
        for model in &self.model_imports {
            // Skip if model is in the current file
            if models_in_current_file.contains(model) {
                continue;
            }

            if let Some(file_name) = model_to_file.get(model) {
                // Skip if it's the same file
                if file_name == current_file {
                    continue;
                }

                // Get file name without .ts extension for import path
                let import_path = file_name.trim_end_matches(".ts");
                let entry = imports_by_file.entry(import_path.to_string()).or_default();

                entry.push(model.clone());
                if self.include_zod_schemas {
                    entry.push(format!("{}Schema", model));
                }
            }
        }

        // Sort and generate model imports
        let mut sorted_files: Vec<_> = imports_by_file.keys().collect();
        sorted_files.sort();
        for file in sorted_files {
            let mut imports = imports_by_file.get(file).unwrap().clone();
            imports.sort();
            imports.dedup();
            result.push_str(&format!(
                "import {{ {} }} from \"./{}\"\n",
                imports.join(", "),
                file
            ));
        }

        // Type alias imports from types.ts
        if !self.type_alias_imports.is_empty() {
            let mut type_imports: Vec<String> = self.type_alias_imports.iter().cloned().collect();
            if self.include_zod_schemas {
                let schema_imports: Vec<String> = self
                    .type_alias_imports
                    .iter()
                    .map(|t| format!("{}Schema", t))
                    .collect();
                type_imports.extend(schema_imports);
            }
            type_imports.sort();
            result.push_str(&format!(
                "import {{ {} }} from \"./types\"\n",
                type_imports.join(", ")
            ));
        }

        if !result.is_empty() {
            result.push('\n');
        }

        result
    }
}

/// Collect all type references from a TypeExpression
fn collect_type_references(type_expr: &TypeExpression, references: &mut BTreeSet<String>) {
    match type_expr {
        TypeExpression::Identifier { name } => {
            // Skip built-in types
            if !matches!(name.as_str(), "string" | "number" | "boolean" | "JSON") {
                references.insert(name.clone());
            }
        }
        TypeExpression::Array { element_type } => {
            collect_type_references(element_type, references);
        }
        TypeExpression::Union { types } => {
            for t in types {
                collect_type_references(t, references);
            }
        }
        TypeExpression::StringLiteral { .. } => {
            // String literals don't reference other types
        }
    }
}

/// Represents an entity (model or type alias) for topological sorting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntityKind {
    Model,
    TypeAlias,
}

/// Topologically sorts entities (models and type aliases) so that dependencies come before dependents.
/// Returns a list of (name, kind) pairs in sorted order.
/// Uses Kahn's algorithm for topological sorting.
fn topological_sort_entities(schema: &Schema) -> Vec<(String, EntityKind)> {
    // Build dependency graph
    // dependencies[name] = set of names that 'name' depends on
    let mut dependencies: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    // in_degree[name] = number of entities that must come before 'name'
    let mut in_degree: BTreeMap<String, usize> = BTreeMap::new();
    // All entity names
    let mut all_entities: BTreeSet<String> = BTreeSet::new();
    // Track entity kinds
    let mut entity_kinds: BTreeMap<String, EntityKind> = BTreeMap::new();

    // Collect all entity names and their kinds
    for name in schema.models.keys() {
        all_entities.insert(name.clone());
        entity_kinds.insert(name.clone(), EntityKind::Model);
        in_degree.insert(name.clone(), 0);
    }
    for name in schema.type_aliases.keys() {
        all_entities.insert(name.clone());
        entity_kinds.insert(name.clone(), EntityKind::TypeAlias);
        in_degree.insert(name.clone(), 0);
    }

    // Build dependencies for models
    for (name, model) in &schema.models {
        let mut refs = BTreeSet::new();
        for field in &model.fields {
            collect_type_references(&field.field_type, &mut refs);
        }
        // Filter to only entities that exist in our schema
        let valid_refs: BTreeSet<String> = refs
            .into_iter()
            .filter(|r| all_entities.contains(r))
            .collect();
        dependencies.insert(name.clone(), valid_refs);
    }

    // Build dependencies for type aliases
    for (name, alias) in &schema.type_aliases {
        let mut refs = BTreeSet::new();
        collect_type_references(&alias.alias_type, &mut refs);
        // Filter to only entities that exist in our schema
        let valid_refs: BTreeSet<String> = refs
            .into_iter()
            .filter(|r| all_entities.contains(r))
            .collect();
        dependencies.insert(name.clone(), valid_refs);
    }

    // Calculate in-degrees
    for (_, deps) in &dependencies {
        for dep in deps {
            if let Some(degree) = in_degree.get_mut(dep) {
                *degree += 1;
            }
        }
    }

    // Kahn's algorithm
    // Start with nodes that have no dependencies
    let mut queue: VecDeque<String> = all_entities
        .iter()
        .filter(|name| {
            dependencies
                .get(*name)
                .map(|d| d.is_empty())
                .unwrap_or(true)
        })
        .cloned()
        .collect();

    let mut sorted: Vec<(String, EntityKind)> = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();

    while let Some(name) = queue.pop_front() {
        if visited.contains(&name) {
            continue;
        }
        visited.insert(name.clone());

        let kind = entity_kinds.get(&name).copied().unwrap_or(EntityKind::Model);
        sorted.push((name.clone(), kind));

        // For each entity that depends on this one, decrement its effective in-degree
        for (dependent, deps) in &dependencies {
            if deps.contains(&name) && !visited.contains(dependent) {
                // Check if all dependencies of 'dependent' have been visited
                let all_deps_visited = deps.iter().all(|d| visited.contains(d) || d == &name);
                if all_deps_visited {
                    queue.push_back(dependent.clone());
                }
            }
        }
    }

    // Handle any remaining entities (circular dependencies)
    // Add them in alphabetical order
    for name in &all_entities {
        if !visited.contains(name) {
            let kind = entity_kinds.get(name).copied().unwrap_or(EntityKind::Model);
            sorted.push((name.clone(), kind));
        }
    }

    sorted
}

#[derive(Debug, Clone)]
struct Config {
    output_format: String,
    file_strategy: String,
    single_file_name: String,
    optional_strategy: String,
    strict_nulls: bool,
    export_all: bool,
    type_name_format: String,
    field_name_format: String,
    generate_zod: bool,
}

impl Config {
    fn from_json(json: &JSON) -> Self {
        Self {
            output_format: json
                .get("output_format")
                .and_then(|v| v.as_str())
                .unwrap_or("interface")
                .to_string(),
            file_strategy: json
                .get("file_strategy")
                .and_then(|v| v.as_str())
                .unwrap_or("single")
                .to_string(),
            single_file_name: json
                .get("single_file_name")
                .and_then(|v| v.as_str())
                .unwrap_or("types.ts")
                .to_string(),
            optional_strategy: json
                .get("optional_strategy")
                .and_then(|v| v.as_str())
                .unwrap_or("native")
                .to_string(),
            strict_nulls: json
                .get("strict_nulls")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            export_all: json
                .get("export_all")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            type_name_format: json
                .get("type_name_format")
                .and_then(|v| v.as_str())
                .unwrap_or("preserve")
                .to_string(),
            field_name_format: json
                .get("field_name_format")
                .and_then(|v| v.as_str())
                .unwrap_or("preserve")
                .to_string(),
            generate_zod: json
                .get("generate_zod")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        }
    }
}

pub fn build(schema: Schema, config: JSON, utils: &Utils) -> Vec<OutputFile> {
    let cfg = Config::from_json(&config);

    match cfg.file_strategy.as_str() {
        "single" => build_single_file(schema, cfg, utils),
        "per_model" => build_per_model_files(schema, cfg, utils),
        _ => vec![],
    }
}

fn build_single_file(schema: Schema, cfg: Config, utils: &Utils) -> Vec<OutputFile> {
    let mut content = String::new();
    let mut zod_content = String::new();

    // Check if any models need Zod schemas
    let needs_zod = schema.models.iter().any(|(_, model)| {
        !should_skip_model(&model.config) && should_generate_zod(&model.config, cfg.generate_zod)
    });

    // Add Zod import if needed
    if needs_zod {
        content.push_str("import { z } from 'zod';\n\n");
    }

    // Get topologically sorted entities for proper Zod schema ordering
    let sorted_entities = topological_sort_entities(&schema);

    // Generate type aliases first (using alphabetical order for type definitions)
    for (name, alias) in &schema.type_aliases {
        if should_skip_type_alias(alias) {
            continue;
        }

        let formatted_name = format_name(name, &cfg.type_name_format, utils);
        let type_str = map_type_to_typescript(&alias.alias_type, cfg.strict_nulls);

        let export = if cfg.export_all { "export " } else { "" };
        content.push_str(&format!(
            "{}type {} = {};\n\n",
            export, formatted_name, type_str
        ));
    }

    // Generate models (using alphabetical order for type definitions)
    for (name, model) in &schema.models {
        // Config is already filtered to this plugin by CDM core
        let model_config = &model.config;

        if should_skip_model(model_config) {
            continue;
        }

        let model_output_format = get_model_output_format(model_config, &cfg.output_format);
        let formatted_name = get_export_name(model_config, name, &cfg.type_name_format, utils);

        match model_output_format.as_str() {
            "interface" => {
                content.push_str(&generate_interface(&formatted_name, model, &cfg, utils));
            }
            "class" => {
                content.push_str(&generate_class(&formatted_name, model, &cfg, utils));
            }
            "type" => {
                content.push_str(&generate_type_alias(&formatted_name, model, &cfg, utils));
            }
            _ => {}
        }
        content.push('\n');
    }

    // Generate Zod schemas in topologically sorted order (dependencies before dependents)
    if needs_zod {
        for (name, kind) in &sorted_entities {
            match kind {
                EntityKind::TypeAlias => {
                    if let Some(alias) = schema.type_aliases.get(name) {
                        if should_skip_type_alias(alias) {
                            continue;
                        }
                        let formatted_name = format_name(name, &cfg.type_name_format, utils);
                        zod_content
                            .push_str(&generate_type_alias_zod_schema(&formatted_name, alias, &cfg));
                        zod_content.push_str("\n\n");
                    }
                }
                EntityKind::Model => {
                    if let Some(model) = schema.models.get(name) {
                        let model_config = &model.config;
                        if should_skip_model(model_config) {
                            continue;
                        }
                        if should_generate_zod(model_config, cfg.generate_zod) {
                            let formatted_name =
                                get_export_name(model_config, name, &cfg.type_name_format, utils);
                            zod_content
                                .push_str(&generate_zod_schema(&formatted_name, model, &cfg, utils));
                            zod_content.push_str("\n\n");
                        }
                    }
                }
            }
        }
    }

    // Append Zod schemas after type definitions
    if !zod_content.is_empty() {
        content.push('\n');
        content.push_str(&zod_content);
    }

    vec![OutputFile {
        path: cfg.single_file_name.clone(),
        content,
    }]
}

fn build_per_model_files(schema: Schema, cfg: Config, utils: &Utils) -> Vec<OutputFile> {
    let mut files: HashMap<String, String> = HashMap::new();
    let mut model_to_file: HashMap<String, String> = HashMap::new();
    // Track which files need Zod import
    let mut files_needing_zod: std::collections::HashSet<String> = std::collections::HashSet::new();
    // Track which models are in each file
    let mut file_to_models: HashMap<String, BTreeSet<String>> = HashMap::new();

    // Create a set of all model names for reference detection
    let model_names: BTreeSet<String> = schema.models.keys().cloned().collect();
    // Create a set of all type alias names
    let type_alias_names: BTreeSet<String> = schema.type_aliases.keys().cloned().collect();

    // First pass: determine which models go to which files and which need Zod
    for (name, model) in &schema.models {
        // Config is already filtered to this plugin by CDM core
        let model_config = &model.config;

        if should_skip_model(model_config) {
            continue;
        }

        let file_name = get_file_name(model_config, name, utils);
        model_to_file.insert(name.clone(), file_name.clone());

        if !files.contains_key(&file_name) {
            files.insert(file_name.clone(), String::new());
        }

        // Track which models are in each file
        file_to_models
            .entry(file_name.clone())
            .or_default()
            .insert(name.clone());

        // Track if this file needs Zod import
        if should_generate_zod(model_config, cfg.generate_zod) {
            files_needing_zod.insert(file_name);
        }
    }

    // Check if any models need Zod schemas
    let needs_zod = schema.models.iter().any(|(_, model)| {
        !should_skip_model(&model.config) && should_generate_zod(&model.config, cfg.generate_zod)
    });

    // Get topologically sorted entities for proper Zod schema ordering (for types.ts)
    let sorted_entities = topological_sort_entities(&schema);

    // Second pass: generate type aliases in a shared file
    if !schema.type_aliases.is_empty() {
        let mut types_content = String::new();
        let mut types_zod_content = String::new();

        // Add Zod import if needed
        if needs_zod {
            types_content.push_str("import { z } from 'zod';\n\n");
        }

        for (name, alias) in &schema.type_aliases {
            if should_skip_type_alias(alias) {
                continue;
            }

            let formatted_name = format_name(name, &cfg.type_name_format, utils);
            let type_str = map_type_to_typescript(&alias.alias_type, cfg.strict_nulls);

            let export = if cfg.export_all { "export " } else { "" };
            types_content.push_str(&format!(
                "{}type {} = {};\n\n",
                export, formatted_name, type_str
            ));
        }

        // Generate Zod schemas for type aliases in topologically sorted order
        if needs_zod {
            for (name, kind) in &sorted_entities {
                if *kind == EntityKind::TypeAlias {
                    if let Some(alias) = schema.type_aliases.get(name) {
                        if should_skip_type_alias(alias) {
                            continue;
                        }
                        let formatted_name = format_name(name, &cfg.type_name_format, utils);
                        types_zod_content.push_str(&generate_type_alias_zod_schema(
                            &formatted_name,
                            alias,
                            &cfg,
                        ));
                        types_zod_content.push_str("\n\n");
                    }
                }
            }
        }

        // Append Zod schemas after type definitions
        if !types_zod_content.is_empty() {
            types_content.push_str(&types_zod_content);
        }

        if !types_content.is_empty() {
            files.insert("types.ts".to_string(), types_content);
        }
    }

    // Third pass: collect imports and generate models grouped by file
    // We need to process each file separately to generate proper imports
    let mut file_contents: HashMap<String, (ImportCollector, String)> = HashMap::new();

    for (file_name, models_in_file) in &file_to_models {
        let mut imports = ImportCollector::new();
        let mut content = String::new();
        let mut zod_content = String::new();

        // Check if this file needs Zod
        let file_needs_zod = files_needing_zod.contains(file_name);
        imports.set_needs_zod(file_needs_zod);
        imports.set_include_zod_schemas(file_needs_zod);

        // Collect all type references from all models in this file
        for model_name in models_in_file {
            if let Some(model) = schema.models.get(model_name) {
                for field in &model.fields {
                    if should_skip_field(&field.config) {
                        continue;
                    }

                    let mut references = BTreeSet::new();
                    collect_type_references(&field.field_type, &mut references);

                    for ref_name in references {
                        if model_names.contains(&ref_name) {
                            imports.add_model(&ref_name);
                        } else if type_alias_names.contains(&ref_name) {
                            imports.add_type_alias(&ref_name);
                        }
                    }
                }
            }
        }

        // Generate model type definitions for all models in this file (alphabetical order)
        for model_name in models_in_file {
            if let Some(model) = schema.models.get(model_name) {
                let model_config = &model.config;
                let model_output_format = get_model_output_format(model_config, &cfg.output_format);
                let formatted_name =
                    get_export_name(model_config, model_name, &cfg.type_name_format, utils);

                match model_output_format.as_str() {
                    "interface" => {
                        content.push_str(&generate_interface(&formatted_name, model, &cfg, utils));
                    }
                    "class" => {
                        content.push_str(&generate_class(&formatted_name, model, &cfg, utils));
                    }
                    "type" => {
                        content.push_str(&generate_type_alias(&formatted_name, model, &cfg, utils));
                    }
                    _ => {}
                }
                content.push('\n');
            }
        }

        // Generate Zod schemas in topologically sorted order for models in this file
        if file_needs_zod {
            for (entity_name, kind) in &sorted_entities {
                if *kind == EntityKind::Model && models_in_file.contains(entity_name) {
                    if let Some(model) = schema.models.get(entity_name) {
                        let model_config = &model.config;
                        if should_generate_zod(model_config, cfg.generate_zod) {
                            let formatted_name = get_export_name(
                                model_config,
                                entity_name,
                                &cfg.type_name_format,
                                utils,
                            );
                            zod_content.push('\n');
                            zod_content
                                .push_str(&generate_zod_schema(&formatted_name, model, &cfg, utils));
                            zod_content.push('\n');
                        }
                    }
                }
            }
        }

        // Append Zod schemas after type definitions
        content.push_str(&zod_content);

        file_contents.insert(file_name.clone(), (imports, content));
    }

    // Fourth pass: generate final file contents with imports
    let mut result_files: Vec<OutputFile> = Vec::new();

    for (file_name, (imports, model_content)) in file_contents {
        let models_in_file = file_to_models.get(&file_name).cloned().unwrap_or_default();
        let import_statements =
            imports.to_import_statements(&file_name, &model_to_file, &models_in_file);

        let content = format!("{}{}", import_statements, model_content);
        result_files.push(OutputFile {
            path: file_name,
            content,
        });
    }

    // Add the types.ts file if it exists
    if let Some(types_content) = files.remove("types.ts") {
        result_files.push(OutputFile {
            path: "types.ts".to_string(),
            content: types_content,
        });
    }

    result_files
}

fn generate_interface(
    name: &str,
    model: &cdm_plugin_interface::ModelDefinition,
    cfg: &Config,
    utils: &Utils,
) -> String {
    let mut result = String::new();

    let export = if cfg.export_all { "export " } else { "" };
    result.push_str(&format!("{}interface {} {{\n", export, name));

    for field in &model.fields {
        // Config is already filtered to this plugin by CDM core
        let field_config = &field.config;

        if should_skip_field(field_config) {
            continue;
        }

        let field_name = get_field_name(field_config, &field.name, &cfg.field_name_format, utils);
        let readonly = if is_readonly_field(field_config) || is_readonly_model(&model.config) {
            "readonly "
        } else {
            ""
        };

        let type_str = get_field_type(field_config, &field.field_type, cfg.strict_nulls);
        let optional_marker = format_optional(field.optional, &cfg.optional_strategy);

        result.push_str(&format!("  {}{}{}: {};\n", readonly, field_name, optional_marker, type_str));
    }

    result.push('}');
    result
}

fn generate_class(
    name: &str,
    model: &cdm_plugin_interface::ModelDefinition,
    cfg: &Config,
    utils: &Utils,
) -> String {
    let mut result = String::new();

    let export = if cfg.export_all { "export " } else { "" };
    result.push_str(&format!("{}class {} {{\n", export, name));

    // Properties
    for field in &model.fields {
        // Config is already filtered to this plugin by CDM core
        let field_config = &field.config;

        if should_skip_field(field_config) {
            continue;
        }

        let field_name = get_field_name(field_config, &field.name, &cfg.field_name_format, utils);
        let readonly = if is_readonly_field(field_config) || is_readonly_model(&model.config) {
            "readonly "
        } else {
            ""
        };

        let type_str = get_field_type(field_config, &field.field_type, cfg.strict_nulls);
        let optional_marker = format_optional(field.optional, &cfg.optional_strategy);

        result.push_str(&format!("  {}{}{}: {};\n", readonly, field_name, optional_marker, type_str));
    }

    // Constructor
    result.push_str(&format!("\n  constructor(data: Partial<{}>) {{\n", name));
    result.push_str("    Object.assign(this, data);\n");
    result.push_str("  }\n");

    result.push('}');
    result
}

fn generate_type_alias(
    name: &str,
    model: &cdm_plugin_interface::ModelDefinition,
    cfg: &Config,
    utils: &Utils,
) -> String {
    let mut result = String::new();

    let export = if cfg.export_all { "export " } else { "" };
    result.push_str(&format!("{}type {} = {{\n", export, name));

    for field in &model.fields {
        // Config is already filtered to this plugin by CDM core
        let field_config = &field.config;

        if should_skip_field(field_config) {
            continue;
        }

        let field_name = get_field_name(field_config, &field.name, &cfg.field_name_format, utils);
        let readonly = if is_readonly_field(field_config) || is_readonly_model(&model.config) {
            "readonly "
        } else {
            ""
        };

        let type_str = get_field_type(field_config, &field.field_type, cfg.strict_nulls);
        let optional_marker = format_optional(field.optional, &cfg.optional_strategy);

        result.push_str(&format!("  {}{}{}: {};\n", readonly, field_name, optional_marker, type_str));
    }

    result.push_str("};");
    result
}

// Helper functions

fn should_skip_type_alias(alias: &cdm_plugin_interface::TypeAliasDefinition) -> bool {
    alias.config
        .get("skip")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn should_skip_model(model_config: &serde_json::Value) -> bool {
    model_config
        .get("skip")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn should_skip_field(field_config: &serde_json::Value) -> bool {
    field_config
        .get("skip")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn is_readonly_model(model_config: &serde_json::Value) -> bool {
    model_config
        .get("readonly")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn is_readonly_field(field_config: &serde_json::Value) -> bool {
    field_config
        .get("readonly")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn get_model_output_format(model_config: &serde_json::Value, default: &str) -> String {
    model_config
        .get("output_format")
        .and_then(|v| v.as_str())
        .unwrap_or(default)
        .to_string()
}

fn get_export_name(model_config: &serde_json::Value, default_name: &str, format: &str, utils: &Utils) -> String {
    model_config
        .get("export_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format_name(default_name, format, utils))
}

fn get_file_name(model_config: &serde_json::Value, model_name: &str, utils: &Utils) -> String {
    model_config
        .get("file_name")
        .and_then(|v| v.as_str())
        .map(|s| {
            if s.ends_with(".ts") {
                s.to_string()
            } else {
                format!("{}.ts", s)
            }
        })
        .unwrap_or_else(|| format!("{}.ts", utils.change_case(model_name, CaseFormat::Pascal)))
}

fn get_field_name(field_config: &serde_json::Value, default_name: &str, format: &str, utils: &Utils) -> String {
    field_config
        .get("field_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format_name(default_name, format, utils))
}

fn get_field_type(field_config: &serde_json::Value, default_type: &cdm_plugin_interface::TypeExpression, strict_nulls: bool) -> String {
    field_config
        .get("type_override")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| map_type_to_typescript(default_type, strict_nulls))
}

fn format_name(name: &str, format: &str, utils: &Utils) -> String {
    match format {
        "preserve" => name.to_string(),
        "pascal" => utils.change_case(name, CaseFormat::Pascal),
        "camel" => utils.change_case(name, CaseFormat::Camel),
        "snake" => utils.change_case(name, CaseFormat::Snake),
        "kebab" => utils.change_case(name, CaseFormat::Kebab),
        "constant" => utils.change_case(name, CaseFormat::Constant),
        _ => name.to_string(),
    }
}

fn format_optional(is_optional: bool, strategy: &str) -> String {
    if !is_optional {
        return String::new();
    }

    match strategy {
        "native" => "?".to_string(),
        "union_undefined" => String::new(),
        _ => "?".to_string(),
    }
}

/// Determines if a model should have a Zod schema generated.
/// Model-level setting overrides global setting.
fn should_generate_zod(model_config: &serde_json::Value, global_generate_zod: bool) -> bool {
    model_config
        .get("generate_zod")
        .and_then(|v| v.as_bool())
        .unwrap_or(global_generate_zod)
}

/// Generates a Zod schema for a model
fn generate_zod_schema(
    name: &str,
    model: &cdm_plugin_interface::ModelDefinition,
    cfg: &Config,
    utils: &Utils,
) -> String {
    let mut result = String::new();

    let export = if cfg.export_all { "export " } else { "" };
    result.push_str(&format!(
        "{}const {}Schema: z.ZodType<{}> = z.object({{\n",
        export, name, name
    ));

    for field in &model.fields {
        let field_config = &field.config;

        if should_skip_field(field_config) {
            continue;
        }

        let field_name = get_field_name(field_config, &field.name, &cfg.field_name_format, utils);
        let zod_type = get_field_zod_type(field_config, &field.field_type, cfg.strict_nulls);

        // Handle optional fields
        let final_type = if field.optional {
            format!("{}.optional()", zod_type)
        } else {
            zod_type
        };

        result.push_str(&format!("  {}: {},\n", field_name, final_type));
    }

    result.push_str("});");
    result
}

/// Generates a Zod schema for a type alias
fn generate_type_alias_zod_schema(
    name: &str,
    alias: &cdm_plugin_interface::TypeAliasDefinition,
    cfg: &Config,
) -> String {
    let export = if cfg.export_all { "export " } else { "" };
    let zod_type = map_type_to_zod(&alias.alias_type, cfg.strict_nulls);
    format!("{}const {}Schema = {};", export, name, zod_type)
}

/// Gets the Zod type for a field, respecting type_override if present
fn get_field_zod_type(
    field_config: &serde_json::Value,
    default_type: &cdm_plugin_interface::TypeExpression,
    strict_nulls: bool,
) -> String {
    // If there's a type_override, we can't generate accurate Zod - use z.any()
    if field_config.get("type_override").is_some() {
        return "z.any()".to_string();
    }
    map_type_to_zod(default_type, strict_nulls)
}


#[cfg(test)]
#[path = "build/build_tests.rs"]
mod build_tests;
