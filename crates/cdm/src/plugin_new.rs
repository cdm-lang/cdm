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
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_is_valid_plugin_name() {
        // Valid names
        assert!(is_valid_plugin_name("my-plugin"));
        assert!(is_valid_plugin_name("plugin123"));
        assert!(is_valid_plugin_name("MyPlugin"));
        assert!(is_valid_plugin_name("a"));
        assert!(is_valid_plugin_name("my-plugin-name"));
        assert!(is_valid_plugin_name("plugin-123-test"));
        assert!(is_valid_plugin_name("x1y2z3"));

        // Invalid names
        assert!(!is_valid_plugin_name(""));
        assert!(!is_valid_plugin_name("123plugin"));
        assert!(!is_valid_plugin_name("my_plugin"));
        assert!(!is_valid_plugin_name("my.plugin"));
        assert!(!is_valid_plugin_name("-starts-with-hyphen"));
        assert!(!is_valid_plugin_name("has space"));
        assert!(!is_valid_plugin_name("has@symbol"));
    }

    #[test]
    fn test_is_valid_plugin_name_special_cases() {
        // Single character edge cases
        assert!(is_valid_plugin_name("a"));
        assert!(is_valid_plugin_name("Z"));
        assert!(!is_valid_plugin_name("1"));
        assert!(!is_valid_plugin_name("-"));

        // Mixed case
        assert!(is_valid_plugin_name("MixedCase"));
        assert!(is_valid_plugin_name("camelCase"));
        assert!(is_valid_plugin_name("PascalCase"));

        // Numbers in various positions
        assert!(is_valid_plugin_name("plugin1"));
        assert!(is_valid_plugin_name("p1lugin"));
        assert!(!is_valid_plugin_name("1plugin"));
    }

    #[test]
    fn test_replace_template_vars() {
        let template = "name: {{PLUGIN_NAME}}, crate: {{CRATE_NAME}}";
        let mut vars = HashMap::new();
        vars.insert("PLUGIN_NAME", "my-plugin");
        vars.insert("CRATE_NAME", "my_plugin");

        let result = replace_template_vars(template, &vars);
        assert_eq!(result, "name: my-plugin, crate: my_plugin");
    }

    #[test]
    fn test_replace_multiple_occurrences() {
        let template = "{{NAME}} and {{NAME}} again";
        let mut vars = HashMap::new();
        vars.insert("NAME", "test");

        let result = replace_template_vars(template, &vars);
        assert_eq!(result, "test and test again");
    }

    #[test]
    fn test_replace_template_vars_no_vars() {
        let template = "no variables here";
        let vars = HashMap::new();

        let result = replace_template_vars(template, &vars);
        assert_eq!(result, "no variables here");
    }

    #[test]
    fn test_replace_template_vars_missing_var() {
        let template = "name: {{PLUGIN_NAME}}, crate: {{CRATE_NAME}}";
        let mut vars = HashMap::new();
        vars.insert("PLUGIN_NAME", "my-plugin");
        // CRATE_NAME is missing

        let result = replace_template_vars(template, &vars);
        // Missing variables are left as-is
        assert_eq!(result, "name: my-plugin, crate: {{CRATE_NAME}}");
    }

    #[test]
    fn test_replace_template_vars_multiline() {
        let template = r#"
[package]
name = "cdm-plugin-{{PLUGIN_NAME}}"
version = "1.0.0"

[lib]
name = "cdm_plugin_{{CRATE_NAME}}"
"#;
        let mut vars = HashMap::new();
        vars.insert("PLUGIN_NAME", "test-plugin");
        vars.insert("CRATE_NAME", "test_plugin");

        let result = replace_template_vars(template, &vars);
        assert!(result.contains("name = \"cdm-plugin-test-plugin\""));
        assert!(result.contains("name = \"cdm_plugin_test_plugin\""));
    }

    #[test]
    fn test_replace_template_vars_adjacent_vars() {
        let template = "{{VAR1}}{{VAR2}}";
        let mut vars = HashMap::new();
        vars.insert("VAR1", "hello");
        vars.insert("VAR2", "world");

        let result = replace_template_vars(template, &vars);
        assert_eq!(result, "helloworld");
    }

    #[test]
    fn test_get_template_dir_exists() {
        // This should succeed in the workspace since we have templates
        let result = get_template_dir("rust");
        assert!(result.is_ok(), "Should find rust templates");

        let template_path = result.unwrap();
        assert!(template_path.exists());
        assert!(template_path.join("Cargo.toml.template").exists());
    }

    #[test]
    fn test_get_template_dir_invalid_lang() {
        let result = get_template_dir("nonexistent-language");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Could not find plugin templates"));
    }

    #[test]
    fn test_create_rust_plugin_creates_all_files() {
        let temp_dir = TempDir::new().unwrap();
        let plugin_dir = temp_dir.path().join("test-plugin");

        let result = create_rust_plugin(&plugin_dir, "test-plugin");
        assert!(result.is_ok(), "Should create plugin successfully: {:?}", result.err());

        // Verify directory structure
        assert!(plugin_dir.exists());
        assert!(plugin_dir.join("src").exists());
        assert!(plugin_dir.join("tests").exists());

        // Verify all files were created
        assert!(plugin_dir.join("Cargo.toml").exists());
        assert!(plugin_dir.join("cdm-plugin.json").exists());
        assert!(plugin_dir.join("schema.cdm").exists());
        assert!(plugin_dir.join(".gitignore").exists());
        assert!(plugin_dir.join("README.md").exists());
        assert!(plugin_dir.join("Makefile").exists());
        assert!(plugin_dir.join("setup.sh").exists());
        assert!(plugin_dir.join("src/lib.rs").exists());
        assert!(plugin_dir.join("src/build.rs").exists());
        assert!(plugin_dir.join("src/migrate.rs").exists());
        assert!(plugin_dir.join("src/validate.rs").exists());
    }

    #[test]
    fn test_create_rust_plugin_substitutes_variables() {
        let temp_dir = TempDir::new().unwrap();
        let plugin_dir = temp_dir.path().join("my-test-plugin");

        let result = create_rust_plugin(&plugin_dir, "my-test");
        assert!(result.is_ok());

        // Read Cargo.toml and verify substitution
        let cargo_toml = fs::read_to_string(plugin_dir.join("Cargo.toml")).unwrap();
        assert!(cargo_toml.contains("cdm-plugin-my-test"));
        assert!(!cargo_toml.contains("{{PLUGIN_NAME}}"));
        assert!(!cargo_toml.contains("{{CRATE_NAME}}"));

        // Read cdm-plugin.json and verify substitution
        let plugin_json = fs::read_to_string(plugin_dir.join("cdm-plugin.json")).unwrap();
        assert!(plugin_json.contains("\"name\": \"my-test\""));
        assert!(plugin_json.contains("cdm_plugin_my_test.wasm"));

        // Read README.md and verify substitution
        let readme = fs::read_to_string(plugin_dir.join("README.md")).unwrap();
        assert!(readme.contains("# cdm-plugin-my-test"));
        assert!(readme.contains("@my-test"));
    }

    #[test]
    fn test_create_rust_plugin_with_hyphens() {
        let temp_dir = TempDir::new().unwrap();
        let plugin_dir = temp_dir.path().join("plugin");

        let result = create_rust_plugin(&plugin_dir, "my-complex-name");
        assert!(result.is_ok());

        // Check Cargo.toml has the plugin name with hyphens
        let cargo_toml = fs::read_to_string(plugin_dir.join("Cargo.toml")).unwrap();
        assert!(cargo_toml.contains("cdm-plugin-my-complex-name"));

        // Check cdm-plugin.json has the WASM file with underscores in crate name
        let plugin_json = fs::read_to_string(plugin_dir.join("cdm-plugin.json")).unwrap();
        assert!(plugin_json.contains("cdm_plugin_my_complex_name.wasm"));
    }

    #[test]
    fn test_plugin_new_invalid_name() {
        let temp_dir = TempDir::new().unwrap();

        let result = plugin_new("123invalid", "rust", Some(temp_dir.path()));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid plugin name"));
    }

    #[test]
    fn test_plugin_new_invalid_language() {
        let temp_dir = TempDir::new().unwrap();

        let result = plugin_new("valid-name", "python", Some(temp_dir.path()));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported language"));
    }

    #[test]
    fn test_plugin_new_directory_already_exists() {
        let temp_dir = TempDir::new().unwrap();
        let plugin_path = temp_dir.path().join("cdm-plugin-existing");

        // Create the directory first
        fs::create_dir(&plugin_path).unwrap();

        let result = plugin_new("existing", "rust", Some(temp_dir.path()));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_plugin_new_success() {
        let temp_dir = TempDir::new().unwrap();

        let result = plugin_new("test-success", "rust", Some(temp_dir.path()));
        assert!(result.is_ok(), "Failed: {:?}", result.err());

        // Verify the plugin was created
        let plugin_dir = temp_dir.path().join("cdm-plugin-test-success");
        assert!(plugin_dir.exists());
        assert!(plugin_dir.join("Cargo.toml").exists());
        assert!(plugin_dir.join("src/lib.rs").exists());
    }

    #[test]
    fn test_plugin_new_default_output_dir() {
        // This test verifies that when output_dir is None, it uses current directory
        // We save and restore the current directory to avoid affecting other tests

        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();

        std::env::set_current_dir(&temp_dir).unwrap();

        // Create a subdirectory that would conflict
        fs::create_dir(temp_dir.path().join("cdm-plugin-default-test")).unwrap();

        let result = plugin_new("default-test", "rust", None);

        // Restore original directory BEFORE asserting
        std::env::set_current_dir(&original_dir).unwrap();

        // Should error because directory exists in current dir
        assert!(result.is_err());
    }

    #[test]
    fn test_generated_plugin_has_valid_json() {
        let temp_dir = TempDir::new().unwrap();
        let plugin_dir = temp_dir.path().join("json-test");

        create_rust_plugin(&plugin_dir, "json-test").unwrap();

        // Verify cdm-plugin.json is valid JSON
        let json_content = fs::read_to_string(plugin_dir.join("cdm-plugin.json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_content).unwrap();

        assert_eq!(parsed["name"], "json-test");
        assert_eq!(parsed["version"], "1.0.0");
        assert!(parsed["capabilities"].is_array());
    }

    #[test]
    fn test_generated_plugin_has_valid_toml() {
        let temp_dir = TempDir::new().unwrap();
        let plugin_dir = temp_dir.path().join("toml-test");

        create_rust_plugin(&plugin_dir, "toml-test").unwrap();

        // Verify Cargo.toml is valid TOML
        let toml_content = fs::read_to_string(plugin_dir.join("Cargo.toml")).unwrap();
        let parsed: toml::Value = toml::from_str(&toml_content).unwrap();

        assert_eq!(parsed["package"]["name"].as_str().unwrap(), "cdm-plugin-toml-test");
        assert!(parsed["lib"].is_table());
        assert!(parsed["dependencies"].is_table());
    }

    #[test]
    fn test_template_vars_case_sensitivity() {
        let template = "{{PLUGIN_NAME}} {{plugin_name}} {{PlUgIn_NaMe}}";
        let mut vars = HashMap::new();
        vars.insert("PLUGIN_NAME", "test");

        let result = replace_template_vars(template, &vars);
        // Only exact match should be replaced
        assert_eq!(result, "test {{plugin_name}} {{PlUgIn_NaMe}}");
    }

    #[test]
    fn test_create_rust_plugin_empty_name() {
        let temp_dir = TempDir::new().unwrap();
        let plugin_dir = temp_dir.path().join("empty");

        // Even though validation happens in plugin_new, test the create function directly
        let result = create_rust_plugin(&plugin_dir, "");
        // Should still create files, validation is done at higher level
        assert!(result.is_ok());
    }
}
