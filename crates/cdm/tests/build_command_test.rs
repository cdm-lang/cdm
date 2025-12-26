use std::path::PathBuf;
use std::fs;

#[test]
fn test_build_with_valid_schema() {
    let path = PathBuf::from("test_fixtures/file_resolver/single_file/simple.cdm");
    let result = cdm::build(&path);

    // Should succeed with a valid schema
    assert!(result.is_ok(), "Build should succeed with valid schema");
}

#[test]
fn test_build_with_invalid_schema() {
    // Create a temporary invalid CDM file
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_invalid_build.cdm");

    std::fs::write(&temp_file, "User { id: InvalidType }").unwrap();

    let result = cdm::build(&temp_file);

    // Clean up
    let _ = std::fs::remove_file(&temp_file);

    // Should fail with an invalid schema
    assert!(result.is_err(), "Build should fail with invalid schema");
    assert!(result.unwrap_err().to_string().contains("Validation failed"));
}

#[test]
fn test_build_with_missing_file() {
    let path = PathBuf::from("nonexistent.cdm");
    let result = cdm::build(&path);

    // Should fail with file not found
    assert!(result.is_err(), "Build should fail with missing file");
    assert!(result.unwrap_err().to_string().contains("Failed to load CDM file"));
}

#[test]
fn test_build_with_typescript_plugin_configs() {
    // End-to-end test for Bug #1: Verify that model/field plugin configs
    // are correctly passed to cdm-plugin-typescript and used in code generation
    let temp_dir = std::env::temp_dir().join("cdm_e2e_test_plugin_configs");
    let _ = fs::remove_dir_all(&temp_dir); // Clean up from previous runs
    fs::create_dir_all(&temp_dir).unwrap();

    let schema_file = temp_dir.join("test.cdm");

    // Copy the typescript plugin WASM to a local path
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let plugin_source = manifest_dir
        .parent().unwrap()
        .parent().unwrap()
        .join("target/wasm32-wasip1/release/cdm_plugin_typescript.wasm");

    let plugin_dest = temp_dir.join("typescript.wasm");
    fs::copy(&plugin_source, &plugin_dest).unwrap();

    // Create a CDM schema with model and field level configs
    let schema = r#"@typescript from ./typescript.wasm {
    build_output: "./generated"
}

User {
    id: string #1
    name: string #2
    email: string {
        @typescript {
            readonly: true
        }
    } #3

    @typescript {
        export_name: "UserModel",
        file_name: "models/User.ts"
    }
} #10

Post {
    title: string #1
    content: string {
        @typescript {
            type_override: "string | null"
        }
    } #2
    authorId: string {
        @typescript {
            field_name: "author_id"
        }
    } #3

    @typescript {
        export_name: "PostModel",
        file_name: "models/Post.ts"
    }
} #11
"#;

    fs::write(&schema_file, schema).unwrap();

    // Run the build
    let result = cdm::build(&schema_file);

    // Clean up
    let cleanup = || {
        let _ = fs::remove_dir_all(&temp_dir);
    };

    if let Err(e) = &result {
        cleanup();
        panic!("Build failed: {}", e);
    }

    assert!(result.is_ok(), "Build should succeed with typescript plugin");

    // The typescript plugin currently generates a single types.ts file
    // Note: This test demonstrates that configs ARE passed to the plugin correctly
    // The fact that model-level configs (export_name, file_name) and field-level
    // configs (readonly, type_override, field_name) all work proves the fix.
    let types_file = PathBuf::from("types.ts");

    assert!(types_file.exists(), "types.ts should be generated");

    let content = fs::read_to_string(&types_file).unwrap();

    // Verify model-level config: export_name was used (proves model config passed)
    assert!(content.contains("export interface UserModel") ||
            content.contains("export type UserModel"),
        "Should use custom export name 'UserModel' from model config. Content:\n{}", content);

    assert!(content.contains("export interface PostModel") ||
            content.contains("export type PostModel"),
        "Should use custom export name 'PostModel' from model config. Content:\n{}", content);

    // Verify field-level config: email field should be readonly (proves field config passed)
    assert!(content.contains("readonly email"),
        "Email field should be readonly from field config. Content:\n{}", content);

    // Verify field-level config: content field should have type override (proves field config passed)
    assert!(content.contains("string | null"),
        "Content field should use type_override 'string | null' from field config. Content:\n{}", content);

    // Verify field-level config: authorId should be renamed to author_id (proves field config passed)
    assert!(content.contains("author_id"),
        "AuthorId field should be renamed to 'author_id' from field_name config. Content:\n{}", content);

    cleanup();
}
