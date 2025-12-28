use crate::{FileResolver, PluginRunner, build_cdm_schema_for_plugin};
use crate::plugin_validation::{extract_plugin_imports_from_validation_result, PluginImport};
use anyhow::{Result, Context};
use cdm_plugin_interface::{OutputFile, Schema, Delta};
use std::path::{Path, PathBuf};
use std::fs;

/// Generate migration files from schema changes
pub fn migrate(
    path: &Path,
    name: String,
    output_dir: Option<PathBuf>,
    dry_run: bool,
) -> Result<()> {
    println!("Running migrate command for: {}", path.display());
    println!("Migration name: {}", name);
    if dry_run {
        println!("Dry-run mode: no files will be written");
    }

    // Step 1: Load previous schema from .cdm/previous_schema.json
    let cdm_dir = path.parent()
        .unwrap_or_else(|| Path::new("."))
        .join(".cdm");

    let previous_schema = load_previous_schema(&cdm_dir)?;

    if previous_schema.is_none() {
        println!("No previous schema found - this is the first run");
        println!("Saving current schema for future migrations...");
    }

    // Step 2: Build current schema
    let tree = FileResolver::load(path).map_err(|diagnostics| {
        for diagnostic in &diagnostics {
            eprintln!("{}", diagnostic);
        }
        anyhow::anyhow!("Failed to load CDM file")
    })?;

    let main_path = tree.main.path.clone();
    let ancestors: Vec<_> = tree.ancestors.iter().map(|a| a.path.clone()).collect();

    // Get the source file directory for resolving relative output paths
    let source_dir = path.parent()
        .ok_or_else(|| anyhow::anyhow!("Source file has no parent directory"))?;

    let validation_result = crate::validate_tree(tree).map_err(|diagnostics| {
        for diagnostic in &diagnostics {
            eprintln!("{}", diagnostic);
        }
        anyhow::anyhow!("Validation failed")
    })?;

    let has_errors = validation_result
        .diagnostics
        .iter()
        .any(|d| d.severity == crate::Severity::Error);

    if has_errors {
        for diagnostic in &validation_result.diagnostics {
            if diagnostic.severity == crate::Severity::Error {
                eprintln!("{}", diagnostic);
            }
        }
        return Err(anyhow::anyhow!("Cannot migrate: validation errors found"));
    }

    // Extract plugin imports
    let plugin_imports = extract_plugin_imports_from_validation_result(&validation_result, &main_path)?;

    if plugin_imports.is_empty() {
        println!("No plugins configured - nothing to migrate");
        // Still save schema for first run
        if previous_schema.is_none() {
            let current_schema = build_cdm_schema_for_plugin(
                &validation_result,
                &ancestors,
                "" // No plugin filtering for storage
            )?;
            save_current_schema(&current_schema, &cdm_dir)?;
            println!("✓ Schema saved successfully");
        }
        return Ok(());
    }

    // Step 3: Compute deltas (if we have a previous schema)
    let deltas = if let Some(ref prev) = previous_schema {
        println!("Computing schema changes...");

        // For now, build current schema without plugin filtering to compare structure
        let current_schema = build_cdm_schema_for_plugin(
            &validation_result,
            &ancestors,
            ""
        )?;

        let computed_deltas = compute_deltas(prev, &current_schema)?;
        println!("Found {} change(s)", computed_deltas.len());

        if dry_run {
            println!("\nDeltas:");
            for delta in &computed_deltas {
                println!("  {:?}", delta);
            }
        }

        computed_deltas
    } else {
        Vec::new()
    };

    // Step 4 & 5: Call plugin migrate and write files
    if !deltas.is_empty() || previous_schema.is_none() {
        let mut any_success = false;

        for plugin_import in &plugin_imports {
            println!("Running plugin: {}", plugin_import.name);

            let mut runner = load_plugin(plugin_import)?;

            // Check if plugin supports migrate operation
            match runner.has_migrate() {
                Ok(false) => {
                    println!("  Skipped: Plugin '{}' does not support migrate", plugin_import.name);
                    continue;
                }
                Err(e) => {
                    eprintln!("  Warning: Failed to check migrate capability for plugin '{}': {}", plugin_import.name, e);
                    continue;
                }
                Ok(true) => {
                    // Plugin supports migrate, proceed
                }
            }

            let global_config = plugin_import.global_config.clone()
                .unwrap_or(serde_json::json!({}));

            let plugin_schema = build_cdm_schema_for_plugin(
                &validation_result,
                &ancestors,
                &plugin_import.name
            )?;

            match runner.migrate(plugin_schema, deltas.clone(), global_config.clone()) {
                Ok(migration_files) => {
                    println!("  Generated {} migration file(s)", migration_files.len());
                    any_success = true;

                    if !dry_run {
                        let output_base = resolve_migration_output_dir(
                            &output_dir,
                            &global_config,
                            &plugin_import.name,
                            source_dir
                        );
                        write_migration_files(&migration_files, &output_base)?;
                    } else {
                        for file in &migration_files {
                            println!("    Would write: {}", file.path);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("  Warning: Plugin '{}' migrate failed: {}", plugin_import.name, e);
                }
            }
        }

        // Step 6: Save current schema (only if not dry-run and at least one plugin succeeded)
        if !dry_run && (any_success || previous_schema.is_none()) {
            let current_schema = build_cdm_schema_for_plugin(
                &validation_result,
                &ancestors,
                ""
            )?;
            save_current_schema(&current_schema, &cdm_dir)?;
            println!("\n✓ Migration completed successfully");
            println!("  Schema saved to {}", cdm_dir.join("previous_schema.json").display());
        }
    } else {
        println!("No changes detected - skipping migration");
    }

    Ok(())
}

/// Load previous schema from .cdm/previous_schema.json
fn load_previous_schema(cdm_dir: &Path) -> Result<Option<Schema>> {
    let schema_path = cdm_dir.join("previous_schema.json");

    if !schema_path.exists() {
        return Ok(None);
    }

    let json = fs::read_to_string(&schema_path)
        .context("Failed to read previous_schema.json")?;

    let schema: Schema = serde_json::from_str(&json)
        .context("Failed to parse previous_schema.json")?;

    Ok(Some(schema))
}

/// Save current schema to .cdm/previous_schema.json
fn save_current_schema(schema: &Schema, cdm_dir: &Path) -> Result<()> {
    // Create .cdm directory if it doesn't exist
    fs::create_dir_all(cdm_dir)
        .context("Failed to create .cdm directory")?;

    let schema_path = cdm_dir.join("previous_schema.json");

    let json = serde_json::to_string_pretty(schema)
        .context("Failed to serialize schema")?;

    fs::write(&schema_path, json)
        .context("Failed to write previous_schema.json")?;

    Ok(())
}

/// Compute deltas between previous and current schemas
fn compute_deltas(previous: &Schema, current: &Schema) -> Result<Vec<Delta>> {
    let mut deltas = Vec::new();

    // Process in order: type aliases, models, fields
    compute_type_alias_deltas(previous, current, &mut deltas)?;
    compute_model_deltas(previous, current, &mut deltas)?;

    Ok(deltas)
}

/// Compute type alias deltas (additions, removals, and renames)
fn compute_type_alias_deltas(
    previous: &Schema,
    current: &Schema,
    deltas: &mut Vec<Delta>,
) -> Result<()> {
    use std::collections::{HashSet, HashMap};

    // Build ID maps for rename detection
    let prev_by_id: HashMap<u64, &cdm_plugin_interface::TypeAliasDefinition> = previous
        .type_aliases
        .values()
        .filter_map(|a| a.entity_id.map(|id| (id, a)))
        .collect();

    let curr_by_id: HashMap<u64, &cdm_plugin_interface::TypeAliasDefinition> = current
        .type_aliases
        .values()
        .filter_map(|a| a.entity_id.map(|id| (id, a)))
        .collect();

    let mut processed_ids = HashSet::new();
    let mut processed_names = HashSet::new();

    // Phase 1: Process type aliases with entity IDs (100% reliable rename detection)
    for (id, curr_alias) in &curr_by_id {
        processed_ids.insert(*id);
        processed_names.insert(curr_alias.name.clone());

        match prev_by_id.get(id) {
            Some(prev_alias) if prev_alias.name != curr_alias.name => {
                // RENAME: Same ID, different name
                deltas.push(Delta::TypeAliasRenamed {
                    old_name: prev_alias.name.clone(),
                    new_name: curr_alias.name.clone(),
                    id: Some(*id),
                    before: (*prev_alias).clone(),
                    after: (*curr_alias).clone(),
                });
            }
            Some(prev_alias) => {
                // Same ID, same name - check for type and config changes
                if !types_equal(&prev_alias.alias_type, &curr_alias.alias_type) {
                    deltas.push(Delta::TypeAliasTypeChanged {
                        name: curr_alias.name.clone(),
                        before: prev_alias.alias_type.clone(),
                        after: curr_alias.alias_type.clone(),
                    });
                }

                // Note: Config changes are tracked at the global level via GlobalConfigChanged
                // Type alias configs are part of the global plugin configuration
            }
            None => {
                // ADDITION: New ID
                deltas.push(Delta::TypeAliasAdded {
                    name: curr_alias.name.clone(),
                    after: (*curr_alias).clone(),
                });
            }
        }
    }

    // Phase 2: Detect removals (ID existed before, not now)
    for (id, prev_alias) in &prev_by_id {
        if !processed_ids.contains(id) {
            deltas.push(Delta::TypeAliasRemoved {
                name: prev_alias.name.clone(),
                before: (*prev_alias).clone(),
            });
        }
    }

    // Phase 3: Process type aliases WITHOUT entity IDs (treat as remove+add)
    for (name, curr_alias) in &current.type_aliases {
        if curr_alias.entity_id.is_none() && !processed_names.contains(name) {
            if !previous.type_aliases.contains_key(name) {
                // Addition
                deltas.push(Delta::TypeAliasAdded {
                    name: name.clone(),
                    after: curr_alias.clone(),
                });
            }
            processed_names.insert(name.clone());
        }
    }

    for (name, prev_alias) in &previous.type_aliases {
        if prev_alias.entity_id.is_none() && !processed_names.contains(name) {
            // Removal
            deltas.push(Delta::TypeAliasRemoved {
                name: name.clone(),
                before: prev_alias.clone(),
            });
        }
    }

    Ok(())
}

/// Compute model deltas (additions, removals, renames, and field changes)
fn compute_model_deltas(
    previous: &Schema,
    current: &Schema,
    deltas: &mut Vec<Delta>,
) -> Result<()> {
    use std::collections::{HashSet, HashMap};

    // Build ID maps for rename detection
    let prev_by_id: HashMap<u64, &cdm_plugin_interface::ModelDefinition> = previous
        .models
        .values()
        .filter_map(|m| m.entity_id.map(|id| (id, m)))
        .collect();

    let curr_by_id: HashMap<u64, &cdm_plugin_interface::ModelDefinition> = current
        .models
        .values()
        .filter_map(|m| m.entity_id.map(|id| (id, m)))
        .collect();

    let mut processed_ids = HashSet::new();
    let mut processed_names = HashSet::new();

    // Phase 1: Process models with entity IDs (100% reliable rename detection)
    for (id, curr_model) in &curr_by_id {
        processed_ids.insert(*id);
        processed_names.insert(curr_model.name.clone());

        match prev_by_id.get(id) {
            Some(prev_model) if prev_model.name != curr_model.name => {
                // RENAME: Same ID, different name
                deltas.push(Delta::ModelRenamed {
                    old_name: prev_model.name.clone(),
                    new_name: curr_model.name.clone(),
                    id: Some(*id),
                    before: (*prev_model).clone(),
                    after: (*curr_model).clone(),
                });

                // Check for field changes within renamed model
                compute_field_deltas(&curr_model.name, &prev_model.fields, &curr_model.fields, deltas)?;
                compute_inheritance_deltas(&curr_model.name, &prev_model.parents, &curr_model.parents, deltas);
            }
            Some(prev_model) => {
                // Same ID, same name - check for field/inheritance/config changes
                compute_field_deltas(&curr_model.name, &prev_model.fields, &curr_model.fields, deltas)?;
                compute_inheritance_deltas(&curr_model.name, &prev_model.parents, &curr_model.parents, deltas);

                // Check for model-level config changes
                if !configs_equal(&prev_model.config, &curr_model.config) {
                    deltas.push(Delta::ModelConfigChanged {
                        model: curr_model.name.clone(),
                        before: prev_model.config.clone(),
                        after: curr_model.config.clone(),
                    });
                }
            }
            None => {
                // ADDITION: New ID
                deltas.push(Delta::ModelAdded {
                    name: curr_model.name.clone(),
                    after: (*curr_model).clone(),
                });
            }
        }
    }

    // Phase 2: Detect removals (ID existed before, not now)
    for (id, prev_model) in &prev_by_id {
        if !processed_ids.contains(id) {
            deltas.push(Delta::ModelRemoved {
                name: prev_model.name.clone(),
                before: (*prev_model).clone(),
            });
        }
    }

    // Phase 3: Process models WITHOUT entity IDs (treat as remove+add)
    for (name, curr_model) in &current.models {
        if curr_model.entity_id.is_none() && !processed_names.contains(name) {
            match previous.models.get(name) {
                Some(prev_model) => {
                    // Same name - check for field changes
                    compute_field_deltas(name, &prev_model.fields, &curr_model.fields, deltas)?;
                    compute_inheritance_deltas(name, &prev_model.parents, &curr_model.parents, deltas);
                }
                None => {
                    // Addition
                    deltas.push(Delta::ModelAdded {
                        name: name.clone(),
                        after: curr_model.clone(),
                    });
                }
            }
            processed_names.insert(name.clone());
        }
    }

    for (name, prev_model) in &previous.models {
        if prev_model.entity_id.is_none() && !processed_names.contains(name) {
            // Removal
            deltas.push(Delta::ModelRemoved {
                name: name.clone(),
                before: prev_model.clone(),
            });
        }
    }

    Ok(())
}

/// Compute field deltas within a model
fn compute_field_deltas(
    model_name: &str,
    prev_fields: &[cdm_plugin_interface::FieldDefinition],
    curr_fields: &[cdm_plugin_interface::FieldDefinition],
    deltas: &mut Vec<Delta>,
) -> Result<()> {
    use std::collections::{HashSet, HashMap};

    // Build ID maps for rename detection
    let prev_by_id: HashMap<u64, &cdm_plugin_interface::FieldDefinition> = prev_fields
        .iter()
        .filter_map(|f| f.entity_id.map(|id| (id, f)))
        .collect();

    let curr_by_id: HashMap<u64, &cdm_plugin_interface::FieldDefinition> = curr_fields
        .iter()
        .filter_map(|f| f.entity_id.map(|id| (id, f)))
        .collect();

    let mut processed_ids = HashSet::new();
    let mut processed_names = HashSet::new();

    // Phase 1: Process fields with entity IDs (100% reliable rename detection)
    for (id, curr_field) in &curr_by_id {
        processed_ids.insert(*id);
        processed_names.insert(curr_field.name.clone());

        match prev_by_id.get(id) {
            Some(prev_field) if prev_field.name != curr_field.name => {
                // RENAME: Same ID, different name
                deltas.push(Delta::FieldRenamed {
                    model: model_name.to_string(),
                    old_name: prev_field.name.clone(),
                    new_name: curr_field.name.clone(),
                    id: Some(*id),
                    before: (*prev_field).clone(),
                    after: (*curr_field).clone(),
                });
            }
            Some(prev_field) => {
                // Same ID, same name - check for modifications
                if !types_equal(&prev_field.field_type, &curr_field.field_type) {
                    deltas.push(Delta::FieldTypeChanged {
                        model: model_name.to_string(),
                        field: curr_field.name.clone(),
                        before: prev_field.field_type.clone(),
                        after: curr_field.field_type.clone(),
                    });
                }

                if prev_field.optional != curr_field.optional {
                    deltas.push(Delta::FieldOptionalityChanged {
                        model: model_name.to_string(),
                        field: curr_field.name.clone(),
                        before: prev_field.optional,
                        after: curr_field.optional,
                    });
                }

                if !values_equal(&prev_field.default, &curr_field.default) {
                    deltas.push(Delta::FieldDefaultChanged {
                        model: model_name.to_string(),
                        field: curr_field.name.clone(),
                        before: prev_field.default.clone(),
                        after: curr_field.default.clone(),
                    });
                }

                // Check for field-level config changes
                if !configs_equal(&prev_field.config, &curr_field.config) {
                    deltas.push(Delta::FieldConfigChanged {
                        model: model_name.to_string(),
                        field: curr_field.name.clone(),
                        before: prev_field.config.clone(),
                        after: curr_field.config.clone(),
                    });
                }
            }
            None => {
                // ADDITION: New ID
                deltas.push(Delta::FieldAdded {
                    model: model_name.to_string(),
                    field: curr_field.name.clone(),
                    after: (*curr_field).clone(),
                });
            }
        }
    }

    // Phase 2: Detect removals (ID existed before, not now)
    for (id, prev_field) in &prev_by_id {
        if !processed_ids.contains(id) {
            deltas.push(Delta::FieldRemoved {
                model: model_name.to_string(),
                field: prev_field.name.clone(),
                before: (*prev_field).clone(),
            });
        }
    }

    // Phase 3: Process fields WITHOUT entity IDs
    for curr_field in curr_fields {
        if curr_field.entity_id.is_none() && !processed_names.contains(&curr_field.name) {
            // Check if field with same name exists in previous
            let prev_match = prev_fields.iter().find(|f| f.name == curr_field.name && f.entity_id.is_none());

            match prev_match {
                Some(prev_field) => {
                    // Same name, no IDs - check for modifications
                    if !types_equal(&prev_field.field_type, &curr_field.field_type) {
                        deltas.push(Delta::FieldTypeChanged {
                            model: model_name.to_string(),
                            field: curr_field.name.clone(),
                            before: prev_field.field_type.clone(),
                            after: curr_field.field_type.clone(),
                        });
                    }

                    if prev_field.optional != curr_field.optional {
                        deltas.push(Delta::FieldOptionalityChanged {
                            model: model_name.to_string(),
                            field: curr_field.name.clone(),
                            before: prev_field.optional,
                            after: curr_field.optional,
                        });
                    }

                    if !values_equal(&prev_field.default, &curr_field.default) {
                        deltas.push(Delta::FieldDefaultChanged {
                            model: model_name.to_string(),
                            field: curr_field.name.clone(),
                            before: prev_field.default.clone(),
                            after: curr_field.default.clone(),
                        });
                    }

                    // Check for field-level config changes
                    if !configs_equal(&prev_field.config, &curr_field.config) {
                        deltas.push(Delta::FieldConfigChanged {
                            model: model_name.to_string(),
                            field: curr_field.name.clone(),
                            before: prev_field.config.clone(),
                            after: curr_field.config.clone(),
                        });
                    }
                }
                None => {
                    // Addition
                    deltas.push(Delta::FieldAdded {
                        model: model_name.to_string(),
                        field: curr_field.name.clone(),
                        after: curr_field.clone(),
                    });
                }
            }
            processed_names.insert(curr_field.name.clone());
        }
    }

    for prev_field in prev_fields {
        if prev_field.entity_id.is_none() && !processed_names.contains(&prev_field.name) {
            // Removal
            deltas.push(Delta::FieldRemoved {
                model: model_name.to_string(),
                field: prev_field.name.clone(),
                before: prev_field.clone(),
            });
        }
    }

    Ok(())
}

/// Compute inheritance deltas
fn compute_inheritance_deltas(
    model_name: &str,
    prev_parents: &[String],
    curr_parents: &[String],
    deltas: &mut Vec<Delta>,
) {
    use std::collections::HashSet;

    let prev_set: HashSet<&String> = prev_parents.iter().collect();
    let curr_set: HashSet<&String> = curr_parents.iter().collect();

    // Check for added parents
    for parent in curr_parents {
        if !prev_set.contains(parent) {
            deltas.push(Delta::InheritanceAdded {
                model: model_name.to_string(),
                parent: parent.clone(),
            });
        }
    }

    // Check for removed parents
    for parent in prev_parents {
        if !curr_set.contains(parent) {
            deltas.push(Delta::InheritanceRemoved {
                model: model_name.to_string(),
                parent: parent.clone(),
            });
        }
    }
}

/// Write migration files to the output directory
fn write_migration_files(files: &[OutputFile], base_dir: &Path) -> Result<()> {
    for file in files {
        let full_path = base_dir.join(&file.path);

        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        fs::write(&full_path, &file.content)
            .with_context(|| format!("Failed to write file: {}", full_path.display()))?;

        println!("  Wrote: {}", full_path.display());
    }
    Ok(())
}

/// Resolve migration output directory based on CLI flag, plugin config, or default
fn resolve_migration_output_dir(
    cli_override: &Option<PathBuf>,
    plugin_config: &serde_json::Value,
    plugin_name: &str,
    source_dir: &Path,
) -> PathBuf {
    // Priority 1: CLI flag
    if let Some(path) = cli_override {
        return if path.is_absolute() {
            path.clone()
        } else {
            source_dir.join(path)
        };
    }

    // Priority 2: Plugin config migrations_output field
    if let Some(dir) = plugin_config.get("migrations_output")
        .and_then(|v| v.as_str()) {
        let dir_path = PathBuf::from(dir);
        return if dir_path.is_absolute() {
            dir_path
        } else {
            source_dir.join(dir_path)
        };
    }

    // Priority 3: Default (relative to source directory)
    source_dir.join("migrations").join(plugin_name)
}

/// Load a plugin from its import specification
fn load_plugin(import: &PluginImport) -> Result<PluginRunner> {
    PluginRunner::from_import(import)
}


/// Check if two type expressions are equal
fn types_equal(a: &cdm_plugin_interface::TypeExpression, b: &cdm_plugin_interface::TypeExpression) -> bool {
    use cdm_plugin_interface::TypeExpression;

    match (a, b) {
        (TypeExpression::Identifier { name: n1 }, TypeExpression::Identifier { name: n2 }) => {
            n1 == n2
        }
        (TypeExpression::Array { element_type: e1 }, TypeExpression::Array { element_type: e2 }) => {
            types_equal(e1, e2)
        }
        (TypeExpression::Union { types: t1 }, TypeExpression::Union { types: t2 }) => {
            // Union equality is order-independent
            if t1.len() != t2.len() {
                return false;
            }
            t1.iter().all(|t| t2.iter().any(|t2| types_equal(t, t2)))
        }
        (TypeExpression::StringLiteral { value: v1 }, TypeExpression::StringLiteral { value: v2 }) => {
            v1 == v2
        }
        _ => false,
    }
}

/// Check if two optional values are equal
fn values_equal(a: &Option<cdm_plugin_interface::Value>, b: &Option<cdm_plugin_interface::Value>) -> bool {
    use cdm_plugin_interface::Value;

    match (a, b) {
        (None, None) => true,
        (Some(_), None) | (None, Some(_)) => false,
        (Some(v1), Some(v2)) => match (v1, v2) {
            (Value::String(s1), Value::String(s2)) => s1 == s2,
            (Value::Number(n1), Value::Number(n2)) => (n1 - n2).abs() < f64::EPSILON,
            (Value::Boolean(b1), Value::Boolean(b2)) => b1 == b2,
            _ => false, // For now, don't compare complex types
        },
    }
}

/// Check if two JSON configs are equal
fn configs_equal(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    // Use serde_json's built-in equality
    a == b
}

#[cfg(test)]
#[path = "migrate/migrate_tests.rs"]
mod migrate_tests;

