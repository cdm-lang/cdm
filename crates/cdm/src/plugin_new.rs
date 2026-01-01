// plugin_new.rs - Generate new CDM plugins from templates

use anyhow::{anyhow, Context, Result};
use include_dir::{include_dir, Dir};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

// Embedded templates directory - included at compile time
static RUST_TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/plugin_templates/rust");

/// Creates a new CDM plugin from a template
pub fn plugin_new(name: &str, lang: &str, output_dir: Option<&Path>) -> Result<()> {
    // Validate plugin name
    if !is_valid_plugin_name(name) {
        return Err(anyhow!(
            "Invalid plugin name '{}'. Plugin names must start with a letter and contain only lowercase letters, numbers, and hyphens.",
            name
        ));
    }

    // Validate language
    if lang != "rust" {
        return Err(anyhow!(
            "Unsupported language '{}'. Currently only 'rust' is supported.",
            lang
        ));
    }

    // Determine output directory
    let base_dir = output_dir.unwrap_or_else(|| Path::new("."));
    let plugin_dir = base_dir.join(format!("cdm-plugin-{}", name));

    // Check if directory already exists
    if plugin_dir.exists() {
        return Err(anyhow!(
            "Directory '{}' already exists. Please choose a different name or location.",
            plugin_dir.display()
        ));
    }

    println!("Creating plugin 'cdm-plugin-{}' in {}...", name, plugin_dir.display());

    // Create plugin directory structure
    create_rust_plugin(&plugin_dir, name)?;

    println!("\n✓ Plugin created successfully!");
    println!("\nNext steps:");
    println!("  1. cd {}", plugin_dir.display());
    println!("  2. ./setup.sh           # Check dependencies and run initial build");
    println!("  3. make help            # See all available commands");
    println!("\nOr build manually:");
    println!("  cargo build --release --target wasm32-wasip1");
    println!("\nUse in a CDM schema:");
    println!("  @{} from ./{}", name, plugin_dir.display());

    Ok(())
}

fn is_valid_plugin_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    let mut chars = name.chars();
    // First char must be a letter
    if let Some(first) = chars.next() {
        if !first.is_ascii_alphabetic() {
            return false;
        }
    }

    // Rest must be letters, numbers, or hyphens
    chars.all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn create_rust_plugin(plugin_dir: &Path, name: &str) -> Result<()> {
    // Create plugin directory
    fs::create_dir_all(plugin_dir).context("Failed to create plugin directory")?;

    // Prepare template variables
    let crate_name = name.replace('-', "_");
    let mut vars = HashMap::new();
    vars.insert("PLUGIN_NAME", name);
    vars.insert("CRATE_NAME", &crate_name);

    // Process all files from embedded templates directory
    process_template_dir(&RUST_TEMPLATES, plugin_dir, &vars)?;

    // Make setup.sh executable on Unix systems
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let setup_sh = plugin_dir.join("setup.sh");
        if setup_sh.exists() {
            let mut perms = fs::metadata(&setup_sh)?.permissions();
            perms.set_mode(0o755); // rwxr-xr-x
            fs::set_permissions(&setup_sh, perms)?;
        }
    }

    Ok(())
}

/// Recursively process template directory and write files
fn process_template_dir(
    template_dir: &Dir,
    output_dir: &Path,
    vars: &HashMap<&str, &str>,
) -> Result<()> {
    // Process all files in this directory
    for file in template_dir.files() {
        let file_path = file.path();
        let file_name = file_path
            .file_name()
            .ok_or_else(|| anyhow!("Invalid file path in templates"))?
            .to_str()
            .ok_or_else(|| anyhow!("Non-UTF8 filename in templates"))?;

        // Get the output filename by removing .template extension if present
        let output_name = file_name.strip_suffix(".template").unwrap_or(file_name);
        let output_path = output_dir.join(output_name);

        // Read template content
        let template_content = file
            .contents_utf8()
            .ok_or_else(|| anyhow!("Template file {} is not valid UTF-8", file_name))?;

        // Process template variables and write
        let processed_content = replace_template_vars(template_content, vars);
        fs::write(&output_path, processed_content)
            .with_context(|| format!("Failed to write {}", output_path.display()))?;
    }

    // Process subdirectories recursively
    for dir in template_dir.dirs() {
        let dir_name = dir
            .path()
            .file_name()
            .ok_or_else(|| anyhow!("Invalid directory path in templates"))?;
        let output_subdir = output_dir.join(dir_name);

        // Create subdirectory
        fs::create_dir_all(&output_subdir)
            .with_context(|| format!("Failed to create directory {}", output_subdir.display()))?;

        // Recursively process subdirectory
        process_template_dir(dir, &output_subdir, vars)?;
    }

    Ok(())
}

/// Replace template variables in content
fn replace_template_vars(content: &str, vars: &HashMap<&str, &str>) -> String {
    let mut result = content.to_string();

    for (key, value) in vars {
        let placeholder = format!("{{{{{}}}}}", key);
        result = result.replace(&placeholder, value);
    }

    result
}

#[cfg(test)]
#[path = "plugin_new/plugin_new_tests.rs"]
mod plugin_new_tests;
