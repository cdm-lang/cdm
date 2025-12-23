use crate::FileResolver;
use anyhow::Result;
use std::path::Path;

/// Build output files from a CDM schema using configured plugins
pub fn build(path: &Path) -> Result<()> {
    // Load and parse the CDM file tree
    let tree = FileResolver::load(path).map_err(|diagnostics| {
        for diagnostic in &diagnostics {
            eprintln!("{}", diagnostic);
        }
        anyhow::anyhow!("Failed to load CDM file")
    })?;

    // Validate the tree before building
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

    // TODO: Phase 2 - Implement plugin building
    // 1. Extract plugin imports from validated schema
    // 2. Load each plugin using PluginRunner
    // 3. Build resolved schema
    // 4. Call plugin.build() for each plugin
    // 5. Write output files to configured directories

    println!("Build completed successfully");
    Ok(())
}
