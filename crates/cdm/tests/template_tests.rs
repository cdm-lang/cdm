//! Integration tests for CDM template functionality
//!
//! Tests the end-to-end template feature including:
//! - Grammar parsing of template imports and extends
//! - Namespace resolution
//! - Template manifest loading
//! - Qualified type references

use std::path::PathBuf;
use cdm::{
    extract_template_imports, extract_template_extends,
    validate_template_imports, validate_qualified_type_reference,
    collect_used_namespaces, check_unused_namespaces,
    TemplateSource, TemplateManifest,
    SymbolTable, Definition, DefinitionKind, ImportedNamespace,
    QualifiedName, is_type_reference_defined,
};
use cdm_utils::{Position, Span};
use std::collections::{HashMap, HashSet};

fn parse(source: &str) -> tree_sitter::Tree {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&grammar::LANGUAGE.into())
        .expect("Failed to load grammar");
    parser.parse(source, None).expect("Failed to parse")
}

fn test_span() -> Span {
    Span {
        start: Position { line: 0, column: 0 },
        end: Position { line: 0, column: 10 },
    }
}

// =============================================================================
// GRAMMAR PARSING TESTS
// =============================================================================

#[test]
fn test_parse_template_import_basic() {
    let source = "import sql from sql/postgres-types\n";
    let tree = parse(source);

    assert!(!tree.root_node().has_error());

    let imports = extract_template_imports(
        tree.root_node(),
        source,
        &PathBuf::from("test.cdm"),
    );

    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].namespace, "sql");
    match &imports[0].source {
        TemplateSource::Registry { name } => assert_eq!(name, "sql/postgres-types"),
        _ => panic!("Expected Registry source"),
    }
}

#[test]
fn test_parse_template_import_with_version() {
    let source = r#"import auth from cdm/auth { version: "^2.0.0" }
"#;
    let tree = parse(source);

    assert!(!tree.root_node().has_error());

    let imports = extract_template_imports(
        tree.root_node(),
        source,
        &PathBuf::from("test.cdm"),
    );

    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].namespace, "auth");

    let config = imports[0].config.as_ref().expect("Expected config");
    assert_eq!(config.get("version").unwrap().as_str().unwrap(), "^2.0.0");
}

#[test]
fn test_parse_template_import_git() {
    let source = r#"import custom from git:https://github.com/org/repo.git { git_ref: "v1.0.0" }
"#;
    let tree = parse(source);

    assert!(!tree.root_node().has_error());

    let imports = extract_template_imports(
        tree.root_node(),
        source,
        &PathBuf::from("test.cdm"),
    );

    assert_eq!(imports.len(), 1);
    match &imports[0].source {
        TemplateSource::Git { url } => {
            assert_eq!(url, "https://github.com/org/repo.git");
        }
        _ => panic!("Expected Git source"),
    }
}

#[test]
fn test_parse_template_import_local() {
    let source = "import local from ./templates/shared\n";
    let tree = parse(source);

    assert!(!tree.root_node().has_error());

    let imports = extract_template_imports(
        tree.root_node(),
        source,
        &PathBuf::from("test.cdm"),
    );

    assert_eq!(imports.len(), 1);
    match &imports[0].source {
        TemplateSource::Local { path } => {
            assert_eq!(path, "./templates/shared");
        }
        _ => panic!("Expected Local source"),
    }
}

#[test]
fn test_parse_template_extends_basic() {
    let source = "extends cdm/auth\n";
    let tree = parse(source);

    assert!(!tree.root_node().has_error());

    let extends = extract_template_extends(
        tree.root_node(),
        source,
        &PathBuf::from("test.cdm"),
    );

    assert_eq!(extends.len(), 1);
    match &extends[0].source {
        TemplateSource::Registry { name } => assert_eq!(name, "cdm/auth"),
        _ => panic!("Expected Registry source"),
    }
}

#[test]
fn test_parse_template_extends_with_config() {
    let source = r#"extends cdm/auth { version: "^2.1.0" }
"#;
    let tree = parse(source);

    assert!(!tree.root_node().has_error());

    let extends = extract_template_extends(
        tree.root_node(),
        source,
        &PathBuf::from("test.cdm"),
    );

    assert_eq!(extends.len(), 1);
    let config = extends[0].config.as_ref().expect("Expected config");
    assert_eq!(config.get("version").unwrap().as_str().unwrap(), "^2.1.0");
}

#[test]
fn test_parse_multiple_directives() {
    let source = r#"@extends ./base.cdm
extends cdm/auth { version: "^2.0.0" }
import sql from sql/postgres-types
@typescript { build_output: "./src/types" }

User {
  id: sql.UUID #1
  name: string #2
} #10
"#;
    let tree = parse(source);

    assert!(!tree.root_node().has_error());

    let imports = extract_template_imports(
        tree.root_node(),
        source,
        &PathBuf::from("test.cdm"),
    );
    let extends = extract_template_extends(
        tree.root_node(),
        source,
        &PathBuf::from("test.cdm"),
    );

    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].namespace, "sql");

    assert_eq!(extends.len(), 1);
}

// =============================================================================
// QUALIFIED TYPE REFERENCE TESTS
// =============================================================================

#[test]
fn test_parse_qualified_type_reference() {
    let source = r#"import sql from sql/postgres-types

User {
  id: sql.UUID #1
  name: sql.Varchar #2
  bio: sql.Text #3
} #10
"#;
    let tree = parse(source);

    assert!(!tree.root_node().has_error());

    let used = collect_used_namespaces(tree.root_node(), source);
    assert!(used.contains("sql"));
}

#[test]
fn test_qualified_name_parsing() {
    // Simple qualified name
    let simple = QualifiedName::parse("sql.UUID").unwrap();
    assert_eq!(simple.namespace_parts, vec!["sql"]);
    assert_eq!(simple.name, "UUID");

    // Nested qualified name
    let nested = QualifiedName::parse("auth.types.Email").unwrap();
    assert_eq!(nested.namespace_parts, vec!["auth", "types"]);
    assert_eq!(nested.name, "Email");

    // Not a qualified name
    assert!(QualifiedName::parse("User").is_none());
    assert!(QualifiedName::parse("string").is_none());
}

#[test]
fn test_is_type_reference_defined_with_namespaces() {
    let mut table = SymbolTable::new();

    // Add a namespace with types
    let mut ns_table = SymbolTable::new();
    ns_table.definitions.insert(
        "UUID".to_string(),
        Definition {
            kind: DefinitionKind::TypeAlias {
                references: vec!["string".to_string()],
                type_expr: "string".to_string(),
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: Some(1),
        },
    );
    ns_table.definitions.insert(
        "Varchar".to_string(),
        Definition {
            kind: DefinitionKind::TypeAlias {
                references: vec!["string".to_string()],
                type_expr: "string".to_string(),
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: Some(2),
        },
    );

    table.add_namespace(ImportedNamespace {
        name: "sql".to_string(),
        template_path: PathBuf::from("./sql"),
        symbol_table: ns_table,
        model_fields: HashMap::new(),
    });

    // Add a local type
    table.definitions.insert(
        "User".to_string(),
        Definition {
            kind: DefinitionKind::Model { extends: vec![] },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: Some(10),
        },
    );

    let ancestors = vec![];

    // Test simple types
    assert!(is_type_reference_defined("User", &table, &ancestors));
    assert!(is_type_reference_defined("string", &table, &ancestors));
    assert!(!is_type_reference_defined("NonExistent", &table, &ancestors));

    // Test qualified types
    assert!(is_type_reference_defined("sql.UUID", &table, &ancestors));
    assert!(is_type_reference_defined("sql.Varchar", &table, &ancestors));
    assert!(!is_type_reference_defined("sql.NonExistent", &table, &ancestors));
    assert!(!is_type_reference_defined("auth.UUID", &table, &ancestors));
}

// =============================================================================
// VALIDATION TESTS
// =============================================================================

#[test]
fn test_validate_duplicate_namespace_error() {
    use cdm::TemplateImport;

    let imports = vec![
        TemplateImport {
            namespace: "sql".to_string(),
            source: TemplateSource::Local { path: "./sql1".to_string() },
            config: None,
            span: test_span(),
            source_file: PathBuf::from("test.cdm"),
        },
        TemplateImport {
            namespace: "sql".to_string(),
            source: TemplateSource::Local { path: "./sql2".to_string() },
            config: None,
            span: test_span(),
            source_file: PathBuf::from("test.cdm"),
        },
    ];

    let diagnostics = validate_template_imports(&imports);

    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("E605"));
    assert!(diagnostics[0].message.contains("Duplicate namespace"));
}

#[test]
fn test_validate_unknown_namespace_error() {
    let table = SymbolTable::new();

    let diagnostics = validate_qualified_type_reference("unknown.Type", &test_span(), &table);

    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("E606"));
    assert!(diagnostics[0].message.contains("Unknown namespace"));
}

#[test]
fn test_unused_namespace_warning() {
    use cdm::TemplateImport;

    let imports = vec![
        TemplateImport {
            namespace: "sql".to_string(),
            source: TemplateSource::Local { path: "./sql".to_string() },
            config: None,
            span: test_span(),
            source_file: PathBuf::from("test.cdm"),
        },
        TemplateImport {
            namespace: "unused".to_string(),
            source: TemplateSource::Local { path: "./unused".to_string() },
            config: None,
            span: test_span(),
            source_file: PathBuf::from("test.cdm"),
        },
    ];

    let mut used = HashSet::new();
    used.insert("sql".to_string());

    let diagnostics = check_unused_namespaces(&imports, &used);

    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("W101"));
    assert!(diagnostics[0].message.contains("unused"));
}

// =============================================================================
// TEMPLATE MANIFEST TESTS
// =============================================================================

#[test]
fn test_template_manifest_deserialization() {
    let json = r#"{
        "name": "sql/postgres-types",
        "version": "1.0.0",
        "description": "PostgreSQL type aliases",
        "entry": "./index.cdm",
        "exports": {
            ".": "./index.cdm",
            "./types": "./types.cdm"
        }
    }"#;

    let manifest: TemplateManifest = serde_json::from_str(json).unwrap();

    assert_eq!(manifest.name, "sql/postgres-types");
    assert_eq!(manifest.version, "1.0.0");
    assert_eq!(manifest.description, "PostgreSQL type aliases");
    assert_eq!(manifest.entry, "./index.cdm");
    assert_eq!(manifest.exports.len(), 2);
    assert_eq!(manifest.exports.get(".").unwrap(), "./index.cdm");
    assert_eq!(manifest.exports.get("./types").unwrap(), "./types.cdm");
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
    assert!(manifest.exports.is_empty());
}

// =============================================================================
// COMBINED USAGE TESTS
// =============================================================================

#[test]
fn test_full_template_usage_scenario() {
    // Simulate a file that imports SQL types and uses them
    let source = r#"import sql from sql/postgres-types { version: "^1.0.0" }

User {
  id: sql.UUID #1
  email: sql.Varchar #2
  created_at: sql.Timestamp #3
  name: string #4
} #10

Post {
  id: sql.UUID #1
  author: User #2
  title: sql.Varchar #3
} #11
"#;

    let tree = parse(source);
    assert!(!tree.root_node().has_error());

    // Extract imports
    let imports = extract_template_imports(
        tree.root_node(),
        source,
        &PathBuf::from("test.cdm"),
    );
    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].namespace, "sql");

    // Check for duplicate namespaces (should be none)
    let duplicate_errors = validate_template_imports(&imports);
    assert!(duplicate_errors.is_empty());

    // Collect used namespaces
    let used = collect_used_namespaces(tree.root_node(), source);
    assert!(used.contains("sql"));

    // Check for unused namespaces (should be none since sql is used)
    let unused_warnings = check_unused_namespaces(&imports, &used);
    assert!(unused_warnings.is_empty());
}

#[test]
fn test_nested_namespace_access() {
    // Create a nested namespace structure: auth.types.Email
    let mut table = SymbolTable::new();

    let mut types_table = SymbolTable::new();
    types_table.definitions.insert(
        "Email".to_string(),
        Definition {
            kind: DefinitionKind::TypeAlias {
                references: vec!["string".to_string()],
                type_expr: "string".to_string(),
            },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: Some(1),
        },
    );

    let types_ns = ImportedNamespace {
        name: "types".to_string(),
        template_path: PathBuf::from("./auth/types"),
        symbol_table: types_table,
        model_fields: HashMap::new(),
    };

    let mut auth_table = SymbolTable::new();
    auth_table.add_namespace(types_ns);

    let auth_ns = ImportedNamespace {
        name: "auth".to_string(),
        template_path: PathBuf::from("./auth"),
        symbol_table: auth_table,
        model_fields: HashMap::new(),
    };

    table.add_namespace(auth_ns);

    let ancestors = vec![];

    // Test nested access
    assert!(is_type_reference_defined("auth.types.Email", &table, &ancestors));
    assert!(!is_type_reference_defined("auth.types.NonExistent", &table, &ancestors));
    assert!(!is_type_reference_defined("auth.Email", &table, &ancestors)); // Not at auth level
}
