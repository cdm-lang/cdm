use crate::{FileResolver, PluginRunner, build_cdm_schema_for_plugin};
use crate::plugin_validation::{extract_plugin_imports_from_validation_result, PluginImport};
use anyhow::{Result, Context};
use cdm_plugin_interface::OutputFile;
use std::path::{Path, PathBuf};
use std::fs;

/// Build output files from a CDM schema using configured plugins
pub fn build(path: &Path) -> Result<()> {
    // Load and parse the CDM file tree
    let tree = FileResolver::load(path).map_err(|diagnostics| {
        for diagnostic in &diagnostics {
            eprintln!("{}", diagnostic);
        }
        anyhow::anyhow!("Failed to load CDM file")
    })?;

    // Extract data we need before consuming tree
    let main_path = tree.main.path.clone();
    let ancestors: Vec<_> = tree.ancestors.iter().map(|a| a.path.clone()).collect();

    // Validate the tree (consumes tree)
    let validation_result = crate::validate_tree(tree).map_err(|diagnostics| {
        for diagnostic in &diagnostics {
            eprintln!("{}", diagnostic);
        }
        anyhow::anyhow!("Validation failed")
    })?;

    // Check for validation errors
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
        return Err(anyhow::anyhow!("Cannot build: validation errors found"));
    }

    // Step 1: Extract plugin imports
    let plugin_imports = extract_plugin_imports_from_validation_result(&validation_result, &main_path)?;

    if plugin_imports.is_empty() {
        println!("No plugins configured - nothing to build");
        return Ok(());
    }

    // Step 2: Process each plugin
    let mut all_output_files = Vec::new();

    // Get the source file directory for resolving relative output paths
    let source_dir = path.parent()
        .ok_or_else(|| anyhow::anyhow!("Source file has no parent directory"))?;

    for plugin_import in &plugin_imports {
        println!("Running plugin: {}", plugin_import.name);

        // Load the plugin
        let mut runner = load_plugin(plugin_import)?;

        // Check if plugin supports build operation
        match runner.has_build() {
            Ok(false) => {
                println!("  Skipped: Plugin '{}' does not support build", plugin_import.name);
                continue;
            }
            Err(e) => {
                eprintln!("  Warning: Failed to check build capability for plugin '{}': {}", plugin_import.name, e);
                continue;
            }
            Ok(true) => {
                // Plugin supports build, proceed
            }
        }

        // Get the plugin's global config (or empty JSON object)
        let global_config = plugin_import.global_config.clone()
            .unwrap_or(serde_json::json!({}));

        // Build schema with this plugin's configs extracted
        let plugin_schema = build_cdm_schema_for_plugin(
            &validation_result,
            &ancestors,
            &plugin_import.name
        )?;

        // Extract build_output from config (if specified)
        let build_output = global_config
            .get("build_output")
            .and_then(|v| v.as_str())
            .map(|s| PathBuf::from(s));

        // Call the plugin's build function
        match runner.build(plugin_schema, global_config) {
            Ok(mut output_files) => {
                println!("  Generated {} file(s)", output_files.len());

                // If build_output is specified, prepend it to all output file paths
                if let Some(ref build_dir) = build_output {
                    for file in &mut output_files {
                        let file_path = Path::new(&file.path);
                        // Only prepend if the path is relative
                        if file_path.is_relative() {
                            file.path = build_dir.join(file_path).to_string_lossy().to_string();
                        }
                    }
                }

                all_output_files.extend(output_files);
            }
            Err(e) => {
                eprintln!("  Warning: Plugin '{}' build failed: {}", plugin_import.name, e);
            }
        }
    }

    // Step 4: Write all output files (resolve relative paths from source directory)
    write_output_files(&all_output_files, source_dir)?;

    println!("\n✓ Build completed successfully");
    println!("  {} plugin(s) executed", plugin_imports.len());
    println!("  {} file(s) generated", all_output_files.len());

    Ok(())
}

/// Load a plugin from its import specification
fn load_plugin(import: &PluginImport) -> Result<PluginRunner> {
    PluginRunner::from_import(import)
}

/// Write output files to disk, resolving paths relative to source_dir
fn write_output_files(files: &[OutputFile], source_dir: &Path) -> Result<()> {
    for file in files {
        let file_path = Path::new(&file.path);

        // Resolve the path relative to the source directory
        let resolved_path = if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            source_dir.join(file_path)
        };

        // Create parent directories if needed
        if let Some(parent) = resolved_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        // Write the file
        fs::write(&resolved_path, &file.content)
            .with_context(|| format!("Failed to write file: {}", resolved_path.display()))?;

        println!("  Wrote: {}", resolved_path.display());
    }

    Ok(())
}

#[cfg(test)]
#[path = "build/build_tests.rs"]
mod build_tests;
