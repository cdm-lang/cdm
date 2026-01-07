use super::*;
use crate::symbol_table::{Definition, DefinitionKind, ImportedNamespace, SymbolTable};
use cdm_utils::{EntityIdSource, Position, Span};
use std::path::PathBuf;

fn test_span() -> Span {
    Span {
        start: Position { line: 0, column: 0 },
        end: Position { line: 0, column: 10 },
    }
}

fn parse(source: &str) -> tree_sitter::Tree {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&grammar::LANGUAGE.into())
        .expect("Failed to load grammar");
    parser.parse(source, None).expect("Failed to parse")
}

#[test]
fn test_validate_template_imports_no_duplicates() {
    let imports = vec![
        TemplateImport {
            namespace: "sql".to_string(),
            source: crate::template_resolver::TemplateSource::Local {
                path: "./sql".to_string(),
            },
            config: None,
            span: test_span(),
            source_file: PathBuf::from("test.cdm"),
        },
        TemplateImport {
            namespace: "auth".to_string(),
            source: crate::template_resolver::TemplateSource::Local {
                path: "./auth".to_string(),
            },
            config: None,
            span: test_span(),
            source_file: PathBuf::from("test.cdm"),
        },
    ];

    let diagnostics = validate_template_imports(&imports);
    assert!(diagnostics.is_empty());
}

#[test]
fn test_validate_template_imports_duplicate_namespace() {
    let imports = vec![
        TemplateImport {
            namespace: "sql".to_string(),
            source: crate::template_resolver::TemplateSource::Local {
                path: "./sql1".to_string(),
            },
            config: None,
            span: test_span(),
            source_file: PathBuf::from("test.cdm"),
        },
        TemplateImport {
            namespace: "sql".to_string(),
            source: crate::template_resolver::TemplateSource::Local {
                path: "./sql2".to_string(),
            },
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
fn test_validate_qualified_type_reference_valid() {
    let mut table = SymbolTable::new();

    let mut ns_table = SymbolTable::new();
    ns_table.definitions.insert(
        "UUID".to_string(),
        Definition {
            kind: DefinitionKind::TypeAlias {
                references: vec!["string".to_string()],
                type_expr: "string".to_string(),
            },
            span: test_span(),
            plugin_configs: std::collections::HashMap::new(),
            entity_id: None,
        },
    );

    table.add_namespace(ImportedNamespace {
        name: "sql".to_string(),
        template_path: PathBuf::from("./sql"),
        symbol_table: ns_table,
        model_fields: std::collections::HashMap::new(),
        template_source: EntityIdSource::LocalTemplate { path: "./sql".to_string() },
    });

    let diagnostics = validate_qualified_type_reference("sql.UUID", &test_span(), &table);
    assert!(diagnostics.is_empty());
}

#[test]
fn test_validate_qualified_type_reference_unknown_namespace() {
    let table = SymbolTable::new();

    let diagnostics = validate_qualified_type_reference("unknown.UUID", &test_span(), &table);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("E606"));
    assert!(diagnostics[0].message.contains("Unknown namespace"));
}

#[test]
fn test_validate_qualified_type_reference_unknown_type() {
    let mut table = SymbolTable::new();

    let ns_table = SymbolTable::new(); // Empty namespace

    table.add_namespace(ImportedNamespace {
        name: "sql".to_string(),
        template_path: PathBuf::from("./sql"),
        symbol_table: ns_table,
        model_fields: std::collections::HashMap::new(),
        template_source: EntityIdSource::LocalTemplate { path: "./sql".to_string() },
    });

    let diagnostics = validate_qualified_type_reference("sql.NonExistent", &test_span(), &table);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("E001"));
    assert!(diagnostics[0].message.contains("Unknown type"));
}

#[test]
fn test_check_unused_namespaces_all_used() {
    let imports = vec![
        TemplateImport {
            namespace: "sql".to_string(),
            source: crate::template_resolver::TemplateSource::Local {
                path: "./sql".to_string(),
            },
            config: None,
            span: test_span(),
            source_file: PathBuf::from("test.cdm"),
        },
    ];

    let mut used = HashSet::new();
    used.insert("sql".to_string());

    let diagnostics = check_unused_namespaces(&imports, &used);
    assert!(diagnostics.is_empty());
}

#[test]
fn test_check_unused_namespaces_warning() {
    let imports = vec![
        TemplateImport {
            namespace: "sql".to_string(),
            source: crate::template_resolver::TemplateSource::Local {
                path: "./sql".to_string(),
            },
            config: None,
            span: test_span(),
            source_file: PathBuf::from("test.cdm"),
        },
        TemplateImport {
            namespace: "unused".to_string(),
            source: crate::template_resolver::TemplateSource::Local {
                path: "./unused".to_string(),
            },
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

#[test]
fn test_collect_used_namespaces() {
    let source = r#"
import sql from ./sql
import auth from ./auth

User {
  id: sql.UUID #1
  role: auth.Role #2
  name: string #3
}
"#;

    let tree = parse(source);
    let used = collect_used_namespaces(tree.root_node(), source);

    assert!(used.contains("sql"));
    assert!(used.contains("auth"));
    assert!(!used.contains("unused"));
}

#[test]
fn test_extract_templates_from_source() {
    let source = r#"
import sql from sql/postgres-types
extends cdm/auth { version: "^2.0.0" }
@typescript { build_output: "./src" }

User {
  id: sql.UUID #1
}
"#;

    let tree = parse(source);
    let (imports, extends) = extract_templates_from_source(&tree, source, Path::new("test.cdm"));

    assert_eq!(imports.len(), 1);
    assert_eq!(imports[0].namespace, "sql");

    assert_eq!(extends.len(), 1);
    match &extends[0].source {
        crate::template_resolver::TemplateSource::Registry { name } => {
            assert_eq!(name, "cdm/auth");
        }
        _ => panic!("Expected Registry source"),
    }
}
