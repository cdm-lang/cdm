use super::*;
use crate::validate::escape_rust_keyword;
use cdm_plugin_interface::{
    EntityId, FieldDefinition, ModelDefinition, Schema, TypeAliasDefinition, TypeExpression,
};
use serde_json::json;
use std::path::PathBuf;

fn local_id(id: u64) -> Option<EntityId> {
    Some(EntityId::local(id))
}

fn fixtures_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_fixtures")
}

fn load_test_schema(fixture_name: &str) -> Schema {
    let path = fixtures_path().join(fixture_name);
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to load fixture {}: {}", path.display(), e));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse fixture {}: {}", path.display(), e))
}

fn create_test_schema() -> Schema {
    load_test_schema("basic_schema.json")
}

#[test]
fn test_default_config() {
    let config = Config::from_json(&json!({}));
    assert_eq!(config.file_strategy, "single");
    assert_eq!(config.single_file_name, "types.rs");
    assert_eq!(
        config.derive_macros,
        vec!["Debug", "Clone", "Serialize", "Deserialize"]
    );
    assert!(config.serde_support);
    assert_eq!(config.type_name_format, "preserve");
    assert_eq!(config.field_name_format, "snake");
    assert_eq!(config.number_type, "f64");
    assert_eq!(config.map_type, "HashMap");
    assert_eq!(config.visibility, "pub");
    assert!(!config.allow_unused_imports);
}

#[test]
fn test_custom_config() {
    let config = Config::from_json(&json!({
        "file_strategy": "per_model",
        "single_file_name": "models.rs",
        "derive_macros": "Debug, PartialEq",
        "serde_support": false,
        "type_name_format": "pascal",
        "field_name_format": "preserve",
        "number_type": "i64",
        "map_type": "BTreeMap",
        "visibility": "pub_crate",
        "allow_unused_imports": true
    }));

    assert_eq!(config.file_strategy, "per_model");
    assert_eq!(config.single_file_name, "models.rs");
    assert_eq!(config.derive_macros, vec!["Debug", "PartialEq"]);
    assert!(!config.serde_support);
    assert_eq!(config.type_name_format, "pascal");
    assert_eq!(config.field_name_format, "preserve");
    assert_eq!(config.number_type, "i64");
    assert_eq!(config.map_type, "BTreeMap");
    assert_eq!(config.visibility, "pub_crate");
    assert!(config.allow_unused_imports);
}

#[test]
fn test_single_file_struct_generation() {
    let schema = create_test_schema();
    let config = json!({});
    let utils = Utils;

    let output = build(schema, config, &utils);

    assert_eq!(output.len(), 1);
    assert_eq!(output[0].path, "types.rs");

    let content = &output[0].content;
    assert!(content.contains("use serde::{Serialize, Deserialize};"));
    assert!(content.contains("pub type Email = String;"));
    assert!(content.contains("#[derive(Debug, Clone, Serialize, Deserialize)]"));
    assert!(content.contains("pub enum Status {"));
    assert!(content.contains("    #[serde(rename = \"active\")]"));
    assert!(content.contains("    Active,"));
    assert!(content.contains("    #[serde(rename = \"inactive\")]"));
    assert!(content.contains("    Inactive,"));
    assert!(content.contains("pub struct Post {"));
    assert!(content.contains("pub struct User {"));
    assert!(content.contains("    pub id: String,"));
    assert!(content.contains("    pub name: String,"));
    assert!(content.contains("    pub email: Option<Email>,"));
    assert!(content.contains("    pub status: Status,"));
    assert!(content.contains("    pub tags: Vec<String>,"));
    assert!(content.contains("    pub metadata: Option<serde_json::Value>,"));
}

#[test]
fn test_optional_fields_wrapped_in_option() {
    let schema = create_test_schema();
    let config = json!({});
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    // email and age are optional
    assert!(content.contains("    pub email: Option<Email>,"));
    assert!(content.contains("    pub age: Option<f64>,"));
    // id is not optional
    assert!(content.contains("    pub id: String,"));
}

#[test]
fn test_no_serde_support() {
    let schema = create_test_schema();
    let config = json!({
        "serde_support": false,
        "derive_macros": "Debug, Clone"
    });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(!content.contains("use serde::"));
    assert!(!content.contains("#[serde("));
    assert!(content.contains("#[derive(Debug, Clone)]"));
}

#[test]
fn test_visibility_pub_crate() {
    let mut schema = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };
    schema.models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "id".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: json!({}),
                entity_id: local_id(1),
            }],
            config: json!({}),
            entity_id: local_id(10),
        },
    );

    let config = json!({ "visibility": "pub_crate" });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("pub(crate) struct User {"));
    assert!(content.contains("    pub(crate) id: String,"));
}

#[test]
fn test_visibility_private() {
    let mut schema = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };
    schema.models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "id".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: json!({}),
                entity_id: local_id(1),
            }],
            config: json!({}),
            entity_id: local_id(10),
        },
    );

    let config = json!({ "visibility": "private" });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("struct User {"));
    assert!(!content.contains("pub struct User {"));
    assert!(content.contains("    id: String,"));
}

#[test]
fn test_skip_model() {
    let mut schema = create_test_schema();
    schema
        .models
        .get_mut("Post")
        .unwrap()
        .config = json!({ "skip": true });

    let config = json!({});
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("pub struct User {"));
    assert!(!content.contains("pub struct Post {"));
}

#[test]
fn test_skip_field() {
    let mut schema = create_test_schema();
    schema
        .models
        .get_mut("User")
        .unwrap()
        .fields[4] // age field
        .config = json!({ "skip": true });

    let config = json!({});
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(!content.contains("age"));
}

#[test]
fn test_skip_type_alias() {
    let mut schema = create_test_schema();
    schema
        .type_aliases
        .get_mut("Email")
        .unwrap()
        .config = json!({ "skip": true });

    let config = json!({});
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(!content.contains("pub type Email"));
}

#[test]
fn test_struct_name_override() {
    let mut schema = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };
    schema.models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "id".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: json!({}),
                entity_id: local_id(1),
            }],
            config: json!({ "struct_name": "UserModel" }),
            entity_id: local_id(10),
        },
    );

    let config = json!({});
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("pub struct UserModel {"));
}

#[test]
fn test_field_name_override() {
    let mut schema = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };
    schema.models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "firstName".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: json!({ "field_name": "given_name" }),
                entity_id: local_id(1),
            }],
            config: json!({}),
            entity_id: local_id(10),
        },
    );

    let config = json!({});
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("    pub given_name: String,"));
    assert!(content.contains("#[serde(rename = \"firstName\")]"));
}

#[test]
fn test_type_override() {
    let mut schema = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };
    schema.models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "created_at".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: json!({ "type_override": "chrono::DateTime<chrono::Utc>" }),
                entity_id: local_id(1),
            }],
            config: json!({}),
            entity_id: local_id(10),
        },
    );

    let config = json!({});
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("    pub created_at: chrono::DateTime<chrono::Utc>,"));
}

#[test]
fn test_serde_rename_field() {
    let mut schema = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };
    schema.models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "user_name".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: json!({ "serde_rename": "userName" }),
                entity_id: local_id(1),
            }],
            config: json!({}),
            entity_id: local_id(10),
        },
    );

    let config = json!({});
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("#[serde(rename = \"userName\")]"));
}

#[test]
fn test_inline_union_enum_generation() {
    let mut schema = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };
    schema.models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "role".to_string(),
                field_type: TypeExpression::Union {
                    types: vec![
                        TypeExpression::StringLiteral {
                            value: "admin".to_string(),
                        },
                        TypeExpression::StringLiteral {
                            value: "member".to_string(),
                        },
                    ],
                },
                optional: false,
                default: None,
                config: json!({}),
                entity_id: local_id(1),
            }],
            config: json!({}),
            entity_id: local_id(10),
        },
    );

    let config = json!({});
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    // Should generate a named enum for the inline union
    assert!(content.contains("pub enum UserRole {"));
    assert!(content.contains("    Admin,"));
    assert!(content.contains("    Member,"));
    // The struct should reference the generated enum
    assert!(content.contains("    pub role: UserRole,"));
}

#[test]
fn test_number_type_i64() {
    let mut schema = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };
    schema.models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "age".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "number".to_string(),
                },
                optional: false,
                default: None,
                config: json!({}),
                entity_id: local_id(1),
            }],
            config: json!({}),
            entity_id: local_id(10),
        },
    );

    let config = json!({ "number_type": "i64" });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("    pub age: i64,"));
}

#[test]
fn test_map_type_btreemap() {
    let mut schema = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };
    schema.models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "settings".to_string(),
                field_type: TypeExpression::Map {
                    key_type: Box::new(TypeExpression::Identifier {
                        name: "string".to_string(),
                    }),
                    value_type: Box::new(TypeExpression::Identifier {
                        name: "string".to_string(),
                    }),
                },
                optional: false,
                default: None,
                config: json!({}),
                entity_id: local_id(1),
            }],
            config: json!({}),
            entity_id: local_id(10),
        },
    );

    let config = json!({ "map_type": "BTreeMap" });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("use std::collections::BTreeMap;"));
    assert!(content.contains("    pub settings: BTreeMap<String, String>,"));
}

#[test]
fn test_per_model_file_strategy() {
    let schema = create_test_schema();
    let config = json!({ "file_strategy": "per_model" });
    let utils = Utils;

    let output = build(schema, config, &utils);

    // Should have: types.rs, user.rs, post.rs, mod.rs
    let file_names: Vec<&str> = output.iter().map(|f| f.path.as_str()).collect();
    assert!(file_names.contains(&"types.rs"));
    assert!(file_names.contains(&"user.rs"));
    assert!(file_names.contains(&"post.rs"));
    assert!(file_names.contains(&"mod.rs"));

    // Check mod.rs content
    let mod_file = output.iter().find(|f| f.path == "mod.rs").unwrap();
    assert!(mod_file.content.contains("mod types;"));
    assert!(mod_file.content.contains("mod user;"));
    assert!(mod_file.content.contains("mod post;"));
    assert!(mod_file.content.contains("pub use types::*;"));
    assert!(mod_file.content.contains("pub use user::*;"));
    assert!(mod_file.content.contains("pub use post::*;"));

    // Check user.rs uses super::*
    let user_file = output.iter().find(|f| f.path == "user.rs").unwrap();
    assert!(user_file.content.contains("use super::*;"));
    assert!(user_file.content.contains("pub struct User {"));
}

#[test]
fn test_type_alias_export_name() {
    let mut schema = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };
    schema.type_aliases.insert(
        "Email".to_string(),
        TypeAliasDefinition {
            name: "Email".to_string(),
            alias_type: TypeExpression::Identifier {
                name: "string".to_string(),
            },
            config: json!({ "export_name": "EmailAddress" }),
            entity_id: local_id(1),
        },
    );

    let config = json!({});
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("pub type EmailAddress = String;"));
    assert!(!content.contains("pub type Email ="));
}

#[test]
fn test_model_derive_macros_override() {
    let mut schema = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };
    schema.models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "id".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: json!({}),
                entity_id: local_id(1),
            }],
            config: json!({ "derive_macros": "Debug, Hash, PartialEq" }),
            entity_id: local_id(10),
        },
    );

    let config = json!({});
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("#[derive(Debug, Hash, PartialEq)]"));
}

#[test]
fn test_field_name_format_snake() {
    let mut schema = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };
    schema.models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "firstName".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(1),
                },
                FieldDefinition {
                    name: "lastName".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(2),
                },
            ],
            config: json!({}),
            entity_id: local_id(10),
        },
    );

    let config = json!({ "field_name_format": "snake" });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("    pub first_name: String,"));
    assert!(content.contains("    pub last_name: String,"));
    // Should add serde rename since field name changed
    assert!(content.contains("#[serde(rename = \"firstName\")]"));
    assert!(content.contains("#[serde(rename = \"lastName\")]"));
}

#[test]
fn test_type_reference_union() {
    let mut schema = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };
    schema.type_aliases.insert(
        "Content".to_string(),
        TypeAliasDefinition {
            name: "Content".to_string(),
            alias_type: TypeExpression::Union {
                types: vec![
                    TypeExpression::Identifier {
                        name: "TextBlock".to_string(),
                    },
                    TypeExpression::Identifier {
                        name: "ImageBlock".to_string(),
                    },
                ],
            },
            config: json!({}),
            entity_id: local_id(1),
        },
    );

    let config = json!({});
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("#[serde(untagged)]"));
    assert!(content.contains("pub enum Content {"));
    assert!(content.contains("    TextBlock(TextBlock),"));
    assert!(content.contains("    ImageBlock(ImageBlock),"));
}

#[test]
fn test_empty_derive_macros() {
    let mut schema = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };
    schema.models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "id".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: false,
                default: None,
                config: json!({}),
                entity_id: local_id(1),
            }],
            config: json!({}),
            entity_id: local_id(10),
        },
    );

    let config = json!({
        "derive_macros": "",
        "serde_support": false
    });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(!content.contains("#[derive("));
}

#[test]
fn test_field_visibility_override() {
    let mut schema = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };
    schema.models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "id".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(1),
                },
                FieldDefinition {
                    name: "secret".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({ "visibility": "private" }),
                    entity_id: local_id(2),
                },
            ],
            config: json!({}),
            entity_id: local_id(10),
        },
    );

    let config = json!({});
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("    pub id: String,"));
    // "secret" should not have "pub" prefix
    assert!(content.contains("    secret: String,"));
}

#[test]
fn test_allow_unused_imports_single_file() {
    let schema = create_test_schema();
    let config = json!({ "allow_unused_imports": true });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("#[allow(unused_imports)]\nuse serde::{Serialize, Deserialize};"));
}

#[test]
fn test_allow_unused_imports_default_off() {
    let schema = create_test_schema();
    let config = json!({});
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(!content.contains("#[allow(unused_imports)]"));
}

#[test]
fn test_allow_unused_imports_per_model() {
    let schema = create_test_schema();
    let config = json!({
        "file_strategy": "per_model",
        "allow_unused_imports": true
    });
    let utils = Utils;

    let output = build(schema, config, &utils);

    // Check mod.rs has allow attribute on use statements
    let mod_file = output.iter().find(|f| f.path == "mod.rs").unwrap();
    assert!(mod_file
        .content
        .contains("#[allow(unused_imports)]\nuse serde::{Serialize, Deserialize};"));

    // Check mod.rs has allow attribute on pub use re-exports
    assert!(mod_file
        .content
        .contains("#[allow(unused_imports)]\npub use types::*;"));
    assert!(mod_file
        .content
        .contains("#[allow(unused_imports)]\npub use user::*;"));

    // Check model files have allow attribute on use super::*
    let user_file = output.iter().find(|f| f.path == "user.rs").unwrap();
    assert!(user_file
        .content
        .contains("#[allow(unused_imports)]\nuse super::*;"));
}

#[test]
fn test_escape_rust_keyword_field_name() {
    let mut schema = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };
    schema.models.insert(
        "Task".to_string(),
        ModelDefinition {
            name: "Task".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "id".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(1),
                },
                FieldDefinition {
                    name: "type".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "string".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: json!({}),
                    entity_id: local_id(2),
                },
            ],
            config: json!({}),
            entity_id: local_id(10),
        },
    );

    let config = json!({ "field_name_format": "preserve" });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    // "type" is a Rust keyword and should be escaped with r#
    assert!(content.contains("    pub r#type: String,"));
    // Should add serde rename since the Rust field name differs
    assert!(content.contains("#[serde(rename = \"type\")]"));
    // Non-keyword field should not be escaped
    assert!(content.contains("    pub id: String,"));
}

#[test]
fn test_escape_rust_keyword_function() {
    assert_eq!(escape_rust_keyword("type"), "r#type");
    assert_eq!(escape_rust_keyword("self"), "r#self");
    assert_eq!(escape_rust_keyword("fn"), "r#fn");
    assert_eq!(escape_rust_keyword("name"), "name");
    assert_eq!(escape_rust_keyword("id"), "id");
}

#[test]
fn test_escape_rust_keyword_optional_field() {
    let mut schema = Schema {
        models: HashMap::new(),
        type_aliases: HashMap::new(),
    };
    schema.models.insert(
        "Task".to_string(),
        ModelDefinition {
            name: "Task".to_string(),
            parents: vec![],
            fields: vec![FieldDefinition {
                name: "type".to_string(),
                field_type: TypeExpression::Identifier {
                    name: "string".to_string(),
                },
                optional: true,
                default: None,
                config: json!({}),
                entity_id: local_id(1),
            }],
            config: json!({}),
            entity_id: local_id(10),
        },
    );

    let config = json!({ "field_name_format": "preserve" });
    let utils = Utils;

    let output = build(schema, config, &utils);
    let content = &output[0].content;

    assert!(content.contains("    pub r#type: Option<String>,"));
}
