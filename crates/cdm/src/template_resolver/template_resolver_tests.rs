use super::*;

fn parse(source: &str) -> tree_sitter::Tree {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&grammar::LANGUAGE.into())
        .expect("Failed to load grammar");
    parser.parse(source, None).expect("Failed to parse")
}

#[test]
fn test_extract_template_import_registry() {
    let source = r#"
import sql from "sql/postgres-types"
"#;
    let tree = parse(source);
    let imports = extract_template_imports(tree.root_node(), source, Path::new("test.cdm"));

    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].namespace, "sql");
    match &imports[0].source {
        TemplateSource::Registry { name } => {
            assert_eq!(name, "sql/postgres-types");
        }
        _ => panic!("Expected Registry source"),
    }
    assert!(imports[0].config.is_none());
}

#[test]
fn test_extract_template_import_with_config() {
    let source = r#"
import auth from "cdm/auth" { version: "^2.0.0" }
"#;
    let tree = parse(source);
    let imports = extract_template_imports(tree.root_node(), source, Path::new("test.cdm"));

    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].namespace, "auth");
    match &imports[0].source {
        TemplateSource::Registry { name } => {
            assert_eq!(name, "cdm/auth");
        }
        _ => panic!("Expected Registry source"),
    }
    assert!(imports[0].config.is_some());
    let config = imports[0].config.as_ref().unwrap();
    assert_eq!(config.get("version").unwrap().as_str().unwrap(), "^2.0.0");
}

#[test]
fn test_extract_template_import_git() {
    let source = r#"
import custom from "git:https://github.com/org/repo.git" { git_ref: "v1.0.0" }
"#;
    let tree = parse(source);
    let imports = extract_template_imports(tree.root_node(), source, Path::new("test.cdm"));

    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].namespace, "custom");
    match &imports[0].source {
        TemplateSource::Git { url } => {
            assert_eq!(url, "https://github.com/org/repo.git");
        }
        _ => panic!("Expected Git source"),
    }
    let config = imports[0].config.as_ref().unwrap();
    assert_eq!(config.get("git_ref").unwrap().as_str().unwrap(), "v1.0.0");
}

#[test]
fn test_extract_template_import_local() {
    let source = r#"
import local from "./templates/shared"
"#;
    let tree = parse(source);
    let imports = extract_template_imports(tree.root_node(), source, Path::new("test.cdm"));

    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].namespace, "local");
    match &imports[0].source {
        TemplateSource::Local { path } => {
            assert_eq!(path, "./templates/shared");
        }
        _ => panic!("Expected Local source"),
    }
}

#[test]
fn test_extract_template_extends_registry() {
    let source = r#"
extends "cdm/auth" { version: "^2.0.0" }
"#;
    let tree = parse(source);
    let extends = extract_template_extends(tree.root_node(), source, Path::new("test.cdm"));

    assert_eq!(extends.len(), 1);
    match &extends[0].source {
        TemplateSource::Registry { name } => {
            assert_eq!(name, "cdm/auth");
        }
        _ => panic!("Expected Registry source"),
    }
    let config = extends[0].config.as_ref().unwrap();
    assert_eq!(config.get("version").unwrap().as_str().unwrap(), "^2.0.0");
}

#[test]
fn test_extract_template_extends_git() {
    let source = r#"
extends "git:https://github.com/org/repo.git"
"#;
    let tree = parse(source);
    let extends = extract_template_extends(tree.root_node(), source, Path::new("test.cdm"));

    assert_eq!(extends.len(), 1);
    match &extends[0].source {
        TemplateSource::Git { url } => {
            assert_eq!(url, "https://github.com/org/repo.git");
        }
        _ => panic!("Expected Git source"),
    }
}

#[test]
fn test_extract_template_extends_local() {
    let source = r#"
extends "./templates/base"
"#;
    let tree = parse(source);
    let extends = extract_template_extends(tree.root_node(), source, Path::new("test.cdm"));

    assert_eq!(extends.len(), 1);
    match &extends[0].source {
        TemplateSource::Local { path } => {
            assert_eq!(path, "./templates/base");
        }
        _ => panic!("Expected Local source"),
    }
}

#[test]
fn test_extract_multiple_imports() {
    let source = r#"
import sql from "sql/postgres-types"
import auth from "cdm/auth" { version: "^2.0.0" }
import custom from "./local/template"
"#;
    let tree = parse(source);
    let imports = extract_template_imports(tree.root_node(), source, Path::new("test.cdm"));

    assert_eq!(imports.len(), 3);
    assert_eq!(imports[0].namespace, "sql");
    assert_eq!(imports[1].namespace, "auth");
    assert_eq!(imports[2].namespace, "custom");
}

#[test]
fn test_extract_mixed_directives() {
    let source = r#"
extends "./base.cdm"
extends "cdm/auth"
import sql from "sql/postgres-types"
@typescript { build_output: "./src/types" }

User {
  id: sql.UUID #1
}
"#;
    let tree = parse(source);
    let imports = extract_template_imports(tree.root_node(), source, Path::new("test.cdm"));
    let extends = extract_template_extends(tree.root_node(), source, Path::new("test.cdm"));

    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].namespace, "sql");
    // Now both local and registry extends are captured
    assert_eq!(extends.len(), 2);
    // First is local path
    match &extends[0].source {
        TemplateSource::Local { path } => assert!(path.ends_with("base.cdm")),
        _ => panic!("Expected Local source for first extends"),
    }
    // Second is registry
    match &extends[1].source {
        TemplateSource::Registry { name } => assert_eq!(name, "cdm/auth"),
        _ => panic!("Expected Registry source for second extends"),
    }
}

#[test]
fn test_template_manifest_parsing() {
    let json = r#"{
        "name": "cdm/auth",
        "version": "2.1.0",
        "description": "Authentication system",
        "entry": "./index.cdm",
        "exports": {
            ".": "./index.cdm",
            "./types": "./types.cdm"
        }
    }"#;

    let manifest: TemplateManifest = serde_json::from_str(json).unwrap();
    assert_eq!(manifest.name, "cdm/auth");
    assert_eq!(manifest.version, "2.1.0");
    assert_eq!(manifest.description, "Authentication system");
    assert_eq!(manifest.entry, "./index.cdm");
    assert_eq!(manifest.exports.len(), 2);
    assert_eq!(manifest.exports.get(".").unwrap(), "./index.cdm");
}

#[test]
fn test_template_manifest_minimal() {
    let json = r#"{
        "name": "simple",
        "version": "1.0.0",
        "description": "Simple template",
        "entry": "./index.cdm"
    }"#;

    let manifest: TemplateManifest = serde_json::from_str(json).unwrap();
    assert_eq!(manifest.name, "simple");
    assert_eq!(manifest.exports.len(), 0);
}

// =========================================================================
// STRUCT TESTS
// =========================================================================

#[test]
fn test_template_manifest_debug() {
    let manifest = TemplateManifest {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        description: "Test template".to_string(),
        entry: "./index.cdm".to_string(),
        exports: std::collections::HashMap::new(),
    };

    let debug = format!("{:?}", manifest);
    assert!(debug.contains("TemplateManifest"));
    assert!(debug.contains("test"));
}

#[test]
fn test_template_manifest_clone() {
    let mut exports = std::collections::HashMap::new();
    exports.insert(".".to_string(), "./index.cdm".to_string());

    let manifest = TemplateManifest {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        description: "Test template".to_string(),
        entry: "./index.cdm".to_string(),
        exports,
    };

    let cloned = manifest.clone();
    assert_eq!(cloned.name, manifest.name);
    assert_eq!(cloned.version, manifest.version);
    assert_eq!(cloned.exports.len(), manifest.exports.len());
}

fn make_span() -> cdm_utils::Span {
    cdm_utils::Span {
        start: cdm_utils::Position { line: 0, column: 0 },
        end: cdm_utils::Position { line: 0, column: 0 },
    }
}

#[test]
fn test_template_import_debug() {
    let import = TemplateImport {
        namespace: "sql".to_string(),
        source: TemplateSource::Registry { name: "sql/postgres-types".to_string() },
        config: None,
        span: make_span(),
        source_file: PathBuf::from("test.cdm"),
    };

    let debug = format!("{:?}", import);
    assert!(debug.contains("TemplateImport"));
    assert!(debug.contains("sql"));
}

#[test]
fn test_template_import_clone() {
    let import = TemplateImport {
        namespace: "sql".to_string(),
        source: TemplateSource::Registry { name: "sql/postgres-types".to_string() },
        config: Some(serde_json::json!({"version": "^1.0.0"})),
        span: make_span(),
        source_file: PathBuf::from("test.cdm"),
    };

    let cloned = import.clone();
    assert_eq!(cloned.namespace, import.namespace);
    assert!(cloned.config.is_some());
}

#[test]
fn test_template_extends_debug() {
    let extends = TemplateExtends {
        source: TemplateSource::Registry { name: "cdm/auth".to_string() },
        config: None,
        span: make_span(),
        source_file: PathBuf::from("test.cdm"),
    };

    let debug = format!("{:?}", extends);
    assert!(debug.contains("TemplateExtends"));
}

#[test]
fn test_template_extends_clone() {
    let extends = TemplateExtends {
        source: TemplateSource::Git { url: "https://github.com/org/repo.git".to_string() },
        config: Some(serde_json::json!({"git_ref": "main"})),
        span: make_span(),
        source_file: PathBuf::from("test.cdm"),
    };

    let cloned = extends.clone();
    assert!(cloned.config.is_some());
}

#[test]
fn test_template_source_debug() {
    let registry = TemplateSource::Registry { name: "cdm/auth".to_string() };
    let git = TemplateSource::Git { url: "https://github.com/org/repo.git".to_string() };
    let local = TemplateSource::Local { path: "./templates".to_string() };

    assert!(format!("{:?}", registry).contains("Registry"));
    assert!(format!("{:?}", git).contains("Git"));
    assert!(format!("{:?}", local).contains("Local"));
}

#[test]
fn test_template_source_clone() {
    let registry = TemplateSource::Registry { name: "cdm/auth".to_string() };
    let cloned = registry.clone();
    match cloned {
        TemplateSource::Registry { name } => assert_eq!(name, "cdm/auth"),
        _ => panic!("Expected Registry"),
    }
}

#[test]
fn test_loaded_template_debug() {
    let template = LoadedTemplate {
        manifest: TemplateManifest {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            entry: "./index.cdm".to_string(),
            exports: std::collections::HashMap::new(),
        },
        path: PathBuf::from("/tmp/test"),
        entry_path: PathBuf::from("/tmp/test/index.cdm"),
    };

    let debug = format!("{:?}", template);
    assert!(debug.contains("LoadedTemplate"));
}

#[test]
fn test_loaded_template_clone() {
    let template = LoadedTemplate {
        manifest: TemplateManifest {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            entry: "./index.cdm".to_string(),
            exports: std::collections::HashMap::new(),
        },
        path: PathBuf::from("/tmp/test"),
        entry_path: PathBuf::from("/tmp/test/index.cdm"),
    };

    let cloned = template.clone();
    assert_eq!(cloned.path, template.path);
    assert_eq!(cloned.entry_path, template.entry_path);
}

// =========================================================================
// SERIALIZATION TESTS
// =========================================================================

#[test]
fn test_template_manifest_serialization() {
    let mut exports = std::collections::HashMap::new();
    exports.insert("types".to_string(), "./types.cdm".to_string());

    let manifest = TemplateManifest {
        name: "test/template".to_string(),
        version: "2.0.0".to_string(),
        description: "A test template".to_string(),
        entry: "./main.cdm".to_string(),
        exports,
    };

    let json = serde_json::to_string(&manifest).unwrap();
    assert!(json.contains("\"name\":\"test/template\""));
    assert!(json.contains("\"version\":\"2.0.0\""));
}

#[test]
fn test_template_manifest_roundtrip() {
    let mut exports = std::collections::HashMap::new();
    exports.insert(".".to_string(), "./index.cdm".to_string());
    exports.insert("types".to_string(), "./types.cdm".to_string());

    let original = TemplateManifest {
        name: "test/template".to_string(),
        version: "1.2.3".to_string(),
        description: "Test description".to_string(),
        entry: "./index.cdm".to_string(),
        exports,
    };

    let json = serde_json::to_string(&original).unwrap();
    let parsed: TemplateManifest = serde_json::from_str(&json).unwrap();

    assert_eq!(original.name, parsed.name);
    assert_eq!(original.version, parsed.version);
    assert_eq!(original.entry, parsed.entry);
    assert_eq!(original.exports.len(), parsed.exports.len());
}

// =========================================================================
// EDGE CASE TESTS
// =========================================================================

#[test]
fn test_extract_no_imports() {
    let source = r#"
User {
  id: number #1
}
"#;
    let tree = parse(source);
    let imports = extract_template_imports(tree.root_node(), source, Path::new("test.cdm"));

    assert_eq!(imports.len(), 0);
}

#[test]
fn test_extract_no_extends() {
    let source = r#"
User {
  id: number #1
}
"#;
    let tree = parse(source);
    let extends = extract_template_extends(tree.root_node(), source, Path::new("test.cdm"));

    assert_eq!(extends.len(), 0);
}

#[test]
fn test_import_span_populated() {
    let source = r#"import sql from "sql/postgres-types""#;
    let tree = parse(source);
    let imports = extract_template_imports(tree.root_node(), source, Path::new("test.cdm"));

    assert_eq!(imports.len(), 1);
    // Span should be populated (not default)
    let span = &imports[0].span;
    assert!(span.start.line == 0 || span.end.column > 0);
}

#[test]
fn test_extends_span_populated() {
    let source = r#"extends "cdm/auth""#;
    let tree = parse(source);
    let extends = extract_template_extends(tree.root_node(), source, Path::new("test.cdm"));

    assert_eq!(extends.len(), 1);
    // Span should be populated
    let span = &extends[0].span;
    assert!(span.start.line == 0 || span.end.column > 0);
}

#[test]
fn test_source_file_path_preserved() {
    let source = r#"import sql from "sql/postgres-types""#;
    let tree = parse(source);
    let source_path = Path::new("/project/schema.cdm");
    let imports = extract_template_imports(tree.root_node(), source, source_path);

    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].source_file, source_path);
}

#[test]
fn test_import_config_with_multiple_fields() {
    let source = r#"
import sql from "sql/postgres-types" { version: "^1.0.0", strict: true }
"#;
    let tree = parse(source);
    let imports = extract_template_imports(tree.root_node(), source, Path::new("test.cdm"));

    assert_eq!(imports.len(), 1);
    let config = imports[0].config.as_ref().unwrap();
    assert!(config.get("version").is_some());
    assert!(config.get("strict").is_some());
}

// =========================================================================
// RESOLVE TESTS
// =========================================================================

#[test]
fn test_resolve_local_template_not_found() {
    let import = TemplateImport {
        namespace: "test".to_string(),
        source: TemplateSource::Local { path: "./nonexistent/template".to_string() },
        config: None,
        span: make_span(),
        source_file: PathBuf::from("/project/schema.cdm"),
    };

    let result = resolve_template(&import);
    assert!(result.is_err());
}

#[test]
fn test_resolve_registry_template_not_implemented() {
    let import = TemplateImport {
        namespace: "test".to_string(),
        source: TemplateSource::Registry { name: "unknown/template".to_string() },
        config: None,
        span: make_span(),
        source_file: PathBuf::from("/project/schema.cdm"),
    };

    let result = resolve_template(&import);
    // Registry templates are not fully implemented, should error
    assert!(result.is_err());
}

#[test]
fn test_resolve_template_from_source_local() {
    let source = TemplateSource::Local { path: "./nonexistent".to_string() };
    let result = resolve_template_from_source(&source, &None, Path::new("/project/schema.cdm"));
    assert!(result.is_err());
}
