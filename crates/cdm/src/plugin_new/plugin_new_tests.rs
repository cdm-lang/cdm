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
