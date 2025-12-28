// plugin_new.rs - Generate new CDM plugins from templates

use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

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
    // Create directory structure
    fs::create_dir_all(plugin_dir).context("Failed to create plugin directory")?;
    fs::create_dir_all(plugin_dir.join("src")).context("Failed to create src directory")?;
    fs::create_dir_all(plugin_dir.join("tests")).context("Failed to create tests directory")?;

    // Prepare template variables
    let crate_name = name.replace('-', "_");
    let mut vars = HashMap::new();
    vars.insert("PLUGIN_NAME", name);
    vars.insert("CRATE_NAME", &crate_name);

    // Get template directory path (relative to this source file at compile time)
    let template_dir = get_template_dir("rust")?;

    // Template files to copy
    let templates = vec![
        ("Cargo.toml.template", "Cargo.toml"),
        ("cdm-plugin.json.template", "cdm-plugin.json"),
        ("schema.cdm.template", "schema.cdm"),
        (".gitignore.template", ".gitignore"),
        ("README.md.template", "README.md"),
        ("Makefile.template", "Makefile"),
        ("setup.sh.template", "setup.sh"),
        ("src/lib.rs.template", "src/lib.rs"),
        ("src/build.rs.template", "src/build.rs"),
        ("src/migrate.rs.template", "src/migrate.rs"),
        ("src/validate.rs.template", "src/validate.rs"),
    ];

    // Process and write each template
    for (template_name, output_name) in templates {
        let template_path = template_dir.join(template_name);
        let output_path = plugin_dir.join(output_name);

        let template_content = fs::read_to_string(&template_path)
            .with_context(|| format!("Failed to read template: {}", template_path.display()))?;

        let processed_content = replace_template_vars(&template_content, &vars);

        fs::write(&output_path, processed_content)
            .with_context(|| format!("Failed to write {}", output_path.display()))?;
    }

    // Make setup.sh executable on Unix systems
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let setup_sh = plugin_dir.join("setup.sh");
        let mut perms = fs::metadata(&setup_sh)?.permissions();
        perms.set_mode(0o755); // rwxr-xr-x
        fs::set_permissions(&setup_sh, perms)?;
    }

    Ok(())
}

/// Gets the path to the template directory for the given language
fn get_template_dir(lang: &str) -> Result<std::path::PathBuf> {
    // At runtime, templates are relative to the CDM binary location
    // We need to find the plugin_templates directory

    // First try: relative to current executable (for installed version)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let template_path = exe_dir.join("plugin_templates").join(lang);
            if template_path.exists() {
                return Ok(template_path);
            }
        }
    }

    // Second try: relative to current working directory (for development)
    let cwd_path = std::env::current_dir()?.join("crates/cdm/plugin_templates").join(lang);
    if cwd_path.exists() {
        return Ok(cwd_path);
    }

    // Third try: environment variable override
    if let Ok(template_root) = std::env::var("CDM_TEMPLATE_DIR") {
        let env_path = Path::new(&template_root).join(lang);
        if env_path.exists() {
            return Ok(env_path);
        }
    }

    // Fourth try: embedded templates (compile-time include)
    // For now, we'll use a hardcoded path relative to the cargo workspace
    // This works during development
    let workspace_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("plugin_templates")
        .join(lang);

    if workspace_path.exists() {
        return Ok(workspace_path);
    }

    Err(anyhow!(
        "Could not find plugin templates for language '{}'. Searched in multiple locations.",
        lang
    ))
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
