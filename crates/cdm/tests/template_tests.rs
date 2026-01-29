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
use cdm_utils::{EntityId, EntityIdSource, Position, Span};
use std::collections::{HashMap, HashSet};

fn local_id(id: u64) -> Option<EntityId> {
    Some(EntityId::local(id))
}

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
    let source = "import sql from \"sql/postgres-types\"\n";
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
    let source = r#"import auth from "cdm/auth" { version: "^2.0.0" }
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
    let source = r#"import custom from "git:https://github.com/org/repo.git" { git_ref: "v1.0.0" }
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
    let source = "import local from \"./templates/shared\"\n";
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
    let source = "extends \"cdm/auth\"\n";
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
    let source = r#"extends "cdm/auth" { version: "^2.1.0" }
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
    let source = r#"extends "./base.cdm"
extends "cdm/auth" { version: "^2.0.0" }
import sql from "sql/postgres-types"
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

    // Now both local file and registry extends are captured
    assert_eq!(extends.len(), 2);
}

// =============================================================================
// QUALIFIED TYPE REFERENCE TESTS
// =============================================================================

#[test]
fn test_parse_qualified_type_reference() {
    let source = r#"import sql from "sql/postgres-types"

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
            entity_id: local_id(1),
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
            entity_id: local_id(2),
        },
    );

    table.add_namespace(ImportedNamespace {
        name: "sql".to_string(),
        template_path: PathBuf::from("./sql"),
        symbol_table: ns_table,
        model_fields: HashMap::new(),
        template_source: EntityIdSource::LocalTemplate { path: "./sql".to_string() },
    });

    // Add a local type
    table.definitions.insert(
        "User".to_string(),
        Definition {
            kind: DefinitionKind::Model { extends: vec![] },
            span: test_span(),
            plugin_configs: HashMap::new(),
            entity_id: local_id(10),
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
    assert_eq!(manifest.entry, Some("./index.cdm".to_string()));
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
    let source = r#"import sql from "sql/postgres-types" { version: "^1.0.0" }

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
            entity_id: local_id(1),
        },
    );

    let types_ns = ImportedNamespace {
        name: "types".to_string(),
        template_path: PathBuf::from("./auth/types"),
        symbol_table: types_table,
        model_fields: HashMap::new(),
        template_source: EntityIdSource::LocalTemplate { path: "./auth/types".to_string() },
    };

    let mut auth_table = SymbolTable::new();
    auth_table.add_namespace(types_ns);

    let auth_ns = ImportedNamespace {
        name: "auth".to_string(),
        template_path: PathBuf::from("./auth"),
        symbol_table: auth_table,
        model_fields: HashMap::new(),
        template_source: EntityIdSource::LocalTemplate { path: "./auth".to_string() },
    };

    table.add_namespace(auth_ns);

    let ancestors = vec![];

    // Test nested access
    assert!(is_type_reference_defined("auth.types.Email", &table, &ancestors));
    assert!(!is_type_reference_defined("auth.types.NonExistent", &table, &ancestors));
    assert!(!is_type_reference_defined("auth.Email", &table, &ancestors)); // Not at auth level
}

#[test]
fn test_template_type_alias_plugin_configs_extracted() {
    // Test that @sql { type: "..." } configs in type aliases are properly extracted
    // when a template is loaded

    // Parse a template with type aliases containing @sql configs
    let template_source = r#"
@sql

UUID: string {
  @sql { type: "UUID" }
} #1

Varchar: string {
  @sql { type: "VARCHAR" }
} #2
"#;

    let tree = parse(template_source);
    assert!(!tree.root_node().has_error(), "Template should parse without errors");

    // Use extract_structured_plugin_configs to extract the configs
    let plugin_data = cdm::extract_structured_plugin_configs(tree.root_node(), template_source);

    // Verify UUID type alias has sql config
    assert!(plugin_data.type_alias_configs.contains_key("UUID"),
        "Expected UUID in type_alias_configs, got: {:?}", plugin_data.type_alias_configs.keys().collect::<Vec<_>>());

    let uuid_configs = &plugin_data.type_alias_configs["UUID"];
    assert!(uuid_configs.contains_key("sql"),
        "Expected sql config for UUID, got: {:?}", uuid_configs.keys().collect::<Vec<_>>());

    let uuid_sql_config = &uuid_configs["sql"];
    assert_eq!(uuid_sql_config["type"], "UUID",
        "Expected type: UUID, got: {:?}", uuid_sql_config);

    // Verify Varchar type alias has sql config
    assert!(plugin_data.type_alias_configs.contains_key("Varchar"),
        "Expected Varchar in type_alias_configs");

    let varchar_configs = &plugin_data.type_alias_configs["Varchar"];
    assert!(varchar_configs.contains_key("sql"),
        "Expected sql config for Varchar");

    let varchar_sql_config = &varchar_configs["sql"];
    assert_eq!(varchar_sql_config["type"], "VARCHAR",
        "Expected type: VARCHAR, got: {:?}", varchar_sql_config);
}

#[test]
fn test_collect_definitions_preserves_type_alias_plugin_configs() {
    // Verify that collect_definitions properly assigns plugin configs to type aliases
    let template_source = r#"
@sql

UUID: string {
  @sql { type: "UUID" }
} #1

Varchar: string {
  @sql { type: "VARCHAR" }
} #2
"#;

    let tree = parse(template_source);
    assert!(!tree.root_node().has_error());

    // Use the full validation which includes collect_definitions
    let ancestors: Vec<cdm::Ancestor> = vec![];
    let result = cdm::validate(template_source, &ancestors);

    // Check that type alias definitions have plugin configs
    let uuid_def = result.symbol_table.get("UUID");
    assert!(uuid_def.is_some(), "Expected UUID in symbol table");

    let uuid_def = uuid_def.unwrap();
    assert!(uuid_def.plugin_configs.contains_key("sql"),
        "Expected sql config on UUID definition, got: {:?}", uuid_def.plugin_configs);

    let uuid_sql_config = &uuid_def.plugin_configs["sql"];
    assert_eq!(uuid_sql_config["type"], "UUID",
        "Expected type: UUID on UUID definition, got: {:?}", uuid_sql_config);

    let varchar_def = result.symbol_table.get("Varchar");
    assert!(varchar_def.is_some(), "Expected Varchar in symbol table");

    let varchar_def = varchar_def.unwrap();
    assert!(varchar_def.plugin_configs.contains_key("sql"),
        "Expected sql config on Varchar definition, got: {:?}", varchar_def.plugin_configs);
}

#[test]
fn test_build_resolved_schema_includes_template_namespace_type_aliases() {
    // Test the full flow: template type aliases with plugin configs are included
    // in the resolved schema with qualified names

    // Create main file symbol table with a namespace containing template types
    let mut main_symbols = SymbolTable::new();

    // Parse template to get its symbol table
    let template_source = r#"
@sql

UUID: string {
  @sql { type: "UUID" }
} #60

Varchar: string {
  @sql { type: "VARCHAR" }
} #20
"#;

    let ancestors: Vec<cdm::Ancestor> = vec![];
    let template_result = cdm::validate(template_source, &ancestors);

    // Create namespace from template validation result
    let template_ns = ImportedNamespace {
        name: "sql".to_string(),
        template_path: PathBuf::from("./templates/sql-types/postgres.cdm"),
        symbol_table: template_result.symbol_table,
        model_fields: template_result.model_fields,
        template_source: EntityIdSource::LocalTemplate { path: "./templates/sql-types".to_string() },
    };

    // Add namespace to main symbol table
    main_symbols.add_namespace(template_ns);

    // Build resolved schema
    let current_fields = HashMap::new();
    let removals: Vec<(String, Span, &str)> = vec![];

    let resolved = cdm::build_resolved_schema(&main_symbols, &current_fields, &[], &removals);

    // Verify sql.UUID is in resolved schema with correct config
    assert!(resolved.type_aliases.contains_key("sql.UUID"),
        "Expected sql.UUID in resolved type_aliases, got: {:?}", resolved.type_aliases.keys().collect::<Vec<_>>());

    let uuid_alias = &resolved.type_aliases["sql.UUID"];
    assert!(uuid_alias.plugin_configs.contains_key("sql"),
        "Expected sql config on sql.UUID, got: {:?}", uuid_alias.plugin_configs);

    let uuid_sql_config = &uuid_alias.plugin_configs["sql"];
    assert_eq!(uuid_sql_config["type"], "UUID",
        "Expected type: UUID on sql.UUID, got: {:?}", uuid_sql_config);

    // Verify sql.Varchar is in resolved schema with correct config
    assert!(resolved.type_aliases.contains_key("sql.Varchar"),
        "Expected sql.Varchar in resolved type_aliases");

    let varchar_alias = &resolved.type_aliases["sql.Varchar"];
    assert!(varchar_alias.plugin_configs.contains_key("sql"),
        "Expected sql config on sql.Varchar, got: {:?}", varchar_alias.plugin_configs);
}

#[test]
fn test_validate_with_templates_adds_namespaces_to_symbol_table() {
    // Test that validate_with_templates correctly adds template namespaces
    // This simulates what happens during the build flow

    // Parse the main file that imports a template
    let main_source = r#"
import sql from "./test_template"

@sql {
  dialect: "postgresql",
  build_output: "./test_output"
}

TestUser {
  id: sql.UUID #1
} #1
"#;

    // First, validate the template to get its symbol table
    let template_source = r#"
@sql

UUID: string {
  @sql { type: "UUID" }
} #60
"#;

    let ancestors: Vec<cdm::Ancestor> = vec![];
    let template_result = cdm::validate(template_source, &ancestors);
    assert!(!template_result.has_errors(), "Template should validate without errors");

    // Create namespace from template
    let template_ns = ImportedNamespace {
        name: "sql".to_string(),
        template_path: PathBuf::from("./test_template"),
        symbol_table: template_result.symbol_table,
        model_fields: template_result.model_fields,
        template_source: EntityIdSource::LocalTemplate { path: "./test_template".to_string() },
    };

    // Validate main file with template namespace
    let main_result = cdm::validate_with_templates(main_source, &[], vec![template_ns]);

    // The symbol table should have the sql namespace
    assert!(main_result.symbol_table.has_namespace("sql"),
        "Expected sql namespace in symbol table");

    let sql_ns = main_result.symbol_table.get_namespace("sql").unwrap();

    // The namespace should have UUID definition with plugin config
    let uuid_def = sql_ns.symbol_table.get("UUID");
    assert!(uuid_def.is_some(), "Expected UUID in sql namespace");

    let uuid_def = uuid_def.unwrap();
    assert!(uuid_def.plugin_configs.contains_key("sql"),
        "Expected sql config on UUID in namespace, got: {:?}", uuid_def.plugin_configs);
}

#[test]
fn test_actual_postgres_template_has_sql_configs() {
    // Load the actual postgres.cdm template and verify it has @sql { type: "..." } configs
    let template_source = include_str!("../../../templates/sql-types/postgres.cdm");

    let ancestors: Vec<cdm::Ancestor> = vec![];
    let result = cdm::validate(template_source, &ancestors);

    // Template should validate without errors
    let errors: Vec<_> = result.diagnostics.iter()
        .filter(|d| d.severity == cdm::Severity::Error)
        .collect();
    assert!(errors.is_empty(), "Template has errors: {:?}", errors);

    // Check that UUID has sql config
    let uuid_def = result.symbol_table.get("UUID");
    assert!(uuid_def.is_some(), "Expected UUID in template symbol table");

    let uuid_def = uuid_def.unwrap();
    assert!(uuid_def.plugin_configs.contains_key("sql"),
        "Expected sql config on UUID, got plugin_configs: {:?}", uuid_def.plugin_configs);

    let uuid_sql_config = &uuid_def.plugin_configs["sql"];
    assert_eq!(uuid_sql_config["type"], "UUID",
        "Expected type: UUID, got: {:?}", uuid_sql_config);

    // Check that Varchar has sql config
    let varchar_def = result.symbol_table.get("Varchar");
    assert!(varchar_def.is_some(), "Expected Varchar in template symbol table");

    let varchar_def = varchar_def.unwrap();
    assert!(varchar_def.plugin_configs.contains_key("sql"),
        "Expected sql config on Varchar, got plugin_configs: {:?}", varchar_def.plugin_configs);

    let varchar_sql_config = &varchar_def.plugin_configs["sql"];
    assert_eq!(varchar_sql_config["type"], "VARCHAR",
        "Expected type: VARCHAR, got: {:?}", varchar_sql_config);
}

#[test]
fn test_build_cdm_schema_for_plugin_includes_template_type_configs() {
    // Test that build_cdm_schema_for_plugin correctly includes template type aliases
    // with their plugin configs in the schema sent to plugins

    // Simulate what validate_tree does: create a validation result with namespaces
    let template_source = r#"
@sql

UUID: string {
  @sql { type: "UUID" }
} #60

Varchar: string {
  @sql { type: "VARCHAR" }
} #20
"#;

    let ancestors: Vec<cdm::Ancestor> = vec![];
    let template_result = cdm::validate(template_source, &ancestors);

    // Create namespace from template
    let template_ns = ImportedNamespace {
        name: "sql".to_string(),
        template_path: PathBuf::from("./templates/sql-types/postgres.cdm"),
        symbol_table: template_result.symbol_table,
        model_fields: template_result.model_fields,
        template_source: EntityIdSource::LocalTemplate { path: "./templates/sql-types".to_string() },
    };

    // Create main file validation result with the namespace
    let main_source = r#"
import sql from "./templates/sql-types/postgres.cdm"

@sql {
  dialect: "postgresql",
  build_output: "./test_output"
}

TestUser {
  id: sql.UUID #1
  name: sql.Varchar #2
} #1
"#;

    let main_result = cdm::validate_with_templates(main_source, &[], vec![template_ns]);
    assert!(!main_result.has_errors(), "Main file should validate without errors");

    // Verify the namespace is in the symbol table
    assert!(main_result.symbol_table.has_namespace("sql"),
        "Expected sql namespace in main validation result");

    // Build schema for plugin
    let plugin_schema = cdm::build_cdm_schema_for_plugin(&main_result, &[], "sql")
        .expect("build_cdm_schema_for_plugin should succeed");

    // The type_aliases HashMap should contain sql.UUID and sql.Varchar
    println!("Type aliases in schema: {:?}", plugin_schema.type_aliases.keys().collect::<Vec<_>>());

    assert!(plugin_schema.type_aliases.contains_key("sql.UUID"),
        "Expected sql.UUID in plugin schema type_aliases, got: {:?}", plugin_schema.type_aliases.keys().collect::<Vec<_>>());

    let uuid_alias = &plugin_schema.type_aliases["sql.UUID"];
    println!("sql.UUID config: {:?}", uuid_alias.config);

    // The config should have type: "UUID"
    assert_eq!(uuid_alias.config["type"], "UUID",
        "Expected config.type = UUID for sql.UUID, got: {:?}", uuid_alias.config);

    // Check sql.Varchar
    assert!(plugin_schema.type_aliases.contains_key("sql.Varchar"),
        "Expected sql.Varchar in plugin schema type_aliases");

    let varchar_alias = &plugin_schema.type_aliases["sql.Varchar"];
    assert_eq!(varchar_alias.config["type"], "VARCHAR",
        "Expected config.type = VARCHAR for sql.Varchar, got: {:?}", varchar_alias.config);
}
