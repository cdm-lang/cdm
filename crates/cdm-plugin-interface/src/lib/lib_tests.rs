use super::*;

// Case conversion tests
#[test]
fn test_to_snake_case() {
    assert_eq!(to_snake_case("HelloWorld"), "hello_world");
    assert_eq!(to_snake_case("helloWorld"), "hello_world");
    assert_eq!(to_snake_case("hello"), "hello");
    assert_eq!(to_snake_case("HELLO"), "hello");  // All uppercase becomes lowercase without underscores between consecutive uppers
    assert_eq!(to_snake_case(""), "");
    assert_eq!(to_snake_case("ID"), "id");  // Consecutive uppercase letters don't get underscores between them
    assert_eq!(to_snake_case("MyHTTPServer"), "my_httpserver");  // Consecutive uppers treated as one block
}

#[test]
fn test_to_camel_case() {
    assert_eq!(to_camel_case("hello_world"), "helloWorld");
    assert_eq!(to_camel_case("hello-world"), "helloWorld");
    assert_eq!(to_camel_case("hello world"), "helloWorld");
    assert_eq!(to_camel_case("hello"), "hello");
    assert_eq!(to_camel_case("HelloWorld"), "helloWorld");
    assert_eq!(to_camel_case(""), "");
    assert_eq!(to_camel_case("one_two_three"), "oneTwoThree");
}

#[test]
fn test_to_pascal_case() {
    assert_eq!(to_pascal_case("hello_world"), "HelloWorld");
    assert_eq!(to_pascal_case("hello-world"), "HelloWorld");
    assert_eq!(to_pascal_case("hello world"), "HelloWorld");
    assert_eq!(to_pascal_case("hello"), "Hello");
    assert_eq!(to_pascal_case("HelloWorld"), "HelloWorld");
    assert_eq!(to_pascal_case(""), "");
    assert_eq!(to_pascal_case("one_two_three"), "OneTwoThree");
}

#[test]
fn test_to_kebab_case() {
    assert_eq!(to_kebab_case("HelloWorld"), "hello-world");
    assert_eq!(to_kebab_case("helloWorld"), "hello-world");
    assert_eq!(to_kebab_case("hello"), "hello");
    assert_eq!(to_kebab_case(""), "");
}

#[test]
fn test_to_constant_case() {
    assert_eq!(to_constant_case("HelloWorld"), "HELLO_WORLD");
    assert_eq!(to_constant_case("helloWorld"), "HELLO_WORLD");
    assert_eq!(to_constant_case("hello"), "HELLO");
    assert_eq!(to_constant_case(""), "");
}

#[test]
fn test_to_title_case() {
    assert_eq!(to_title_case("hello_world"), "Hello World");
    assert_eq!(to_title_case("hello"), "Hello");
    assert_eq!(to_title_case("one_two_three"), "One Two Three");
    assert_eq!(to_title_case(""), "");
}

#[test]
fn test_utils_change_case() {
    let utils = Utils;

    assert_eq!(utils.change_case("HelloWorld", CaseFormat::Snake), "hello_world");
    assert_eq!(utils.change_case("hello_world", CaseFormat::Camel), "helloWorld");
    assert_eq!(utils.change_case("hello_world", CaseFormat::Pascal), "HelloWorld");
    assert_eq!(utils.change_case("HelloWorld", CaseFormat::Kebab), "hello-world");
    assert_eq!(utils.change_case("HelloWorld", CaseFormat::Constant), "HELLO_WORLD");
    assert_eq!(utils.change_case("hello_world", CaseFormat::Title), "Hello World");
}

// Serialization tests
#[test]
fn test_config_level_serialization() {
    // Global level
    let global = ConfigLevel::Global;
    let json = serde_json::to_string(&global).unwrap();
    assert!(json.contains("\"type\":\"global\""));

    let deserialized: ConfigLevel = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, ConfigLevel::Global));

    // Model level
    let model = ConfigLevel::Model { name: "User".to_string() };
    let json = serde_json::to_string(&model).unwrap();
    assert!(json.contains("\"type\":\"model\""));
    assert!(json.contains("\"name\":\"User\""));

    // Field level
    let field = ConfigLevel::Field {
        model: "User".to_string(),
        field: "id".to_string()
    };
    let json = serde_json::to_string(&field).unwrap();
    assert!(json.contains("\"type\":\"field\""));
    assert!(json.contains("\"model\":\"User\""));
    assert!(json.contains("\"field\":\"id\""));
}

#[test]
fn test_severity_serialization() {
    let error = Severity::Error;
    let json = serde_json::to_string(&error).unwrap();
    assert_eq!(json, "\"error\"");

    let warning = Severity::Warning;
    let json = serde_json::to_string(&warning).unwrap();
    assert_eq!(json, "\"warning\"");
}

#[test]
fn test_validation_error_serialization() {
    let error = ValidationError {
        path: vec![
            PathSegment {
                kind: "field".to_string(),
                name: "email".to_string(),
            },
        ],
        message: "Invalid email format".to_string(),
        severity: Severity::Error,
    };

    let json = serde_json::to_string(&error).unwrap();
    let deserialized: ValidationError = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.path.len(), 1);
    assert_eq!(deserialized.path[0].kind, "field");
    assert_eq!(deserialized.path[0].name, "email");
    assert_eq!(deserialized.message, "Invalid email format");
    assert_eq!(deserialized.severity, Severity::Error);
}

#[test]
fn test_output_file_serialization() {
    let file = OutputFile {
        path: "output.txt".to_string(),
        content: "Hello, world!".to_string(),
    };

    let json = serde_json::to_string(&file).unwrap();
    let deserialized: OutputFile = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.path, "output.txt");
    assert_eq!(deserialized.content, "Hello, world!");
}

#[test]
fn test_type_expression_serialization() {
    // Identifier
    let identifier = TypeExpression::Identifier {
        name: "string".to_string()
    };
    let json = serde_json::to_string(&identifier).unwrap();
    assert!(json.contains("\"type\":\"identifier\""));
    assert!(json.contains("\"name\":\"string\""));

    // Array
    let array = TypeExpression::Array {
        element_type: Box::new(TypeExpression::Identifier {
            name: "number".to_string()
        })
    };
    let json = serde_json::to_string(&array).unwrap();
    assert!(json.contains("\"type\":\"array\""));

    // Union
    let union = TypeExpression::Union {
        types: vec![
            TypeExpression::Identifier { name: "string".to_string() },
            TypeExpression::Identifier { name: "number".to_string() },
        ]
    };
    let json = serde_json::to_string(&union).unwrap();
    assert!(json.contains("\"type\":\"union\""));

    // String literal
    let literal = TypeExpression::StringLiteral {
        value: "active".to_string()
    };
    let json = serde_json::to_string(&literal).unwrap();
    assert!(json.contains("\"type\":\"string_literal\""));
}

#[test]
fn test_value_serialization() {
    // String
    let string_val = Value::String("test".to_string());
    let json = serde_json::to_string(&string_val).unwrap();
    assert_eq!(json, "\"test\"");

    // Number
    let number_val = Value::Number(42.5);
    let json = serde_json::to_string(&number_val).unwrap();
    assert_eq!(json, "42.5");

    // Boolean
    let bool_val = Value::Boolean(true);
    let json = serde_json::to_string(&bool_val).unwrap();
    assert_eq!(json, "true");

    // Null
    let null_val = Value::Null;
    let json = serde_json::to_string(&null_val).unwrap();
    assert_eq!(json, "null");
}

#[test]
fn test_delta_model_added_serialization() {
    let delta = Delta::ModelAdded {
        name: "User".to_string(),
        after: ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![],
            config: serde_json::json!({}),
            entity_id: None,
        },
    };

    let json = serde_json::to_string(&delta).unwrap();
    assert!(json.contains("\"type\":\"model_added\""));
    assert!(json.contains("\"name\":\"User\""));

    let deserialized: Delta = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, Delta::ModelAdded { .. }));
}

#[test]
fn test_delta_field_added_serialization() {
    let delta = Delta::FieldAdded {
        model: "User".to_string(),
        field: "email".to_string(),
        after: FieldDefinition {
            name: "email".to_string(),
            field_type: TypeExpression::Identifier {
                name: "string".to_string(),
            },
            optional: false,
            default: None,
            config: serde_json::json!({}),
            entity_id: None,
        },
    };

    let json = serde_json::to_string(&delta).unwrap();
    assert!(json.contains("\"type\":\"field_added\""));
    assert!(json.contains("\"model\":\"User\""));
    assert!(json.contains("\"field\":\"email\""));

    let deserialized: Delta = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, Delta::FieldAdded { .. }));
}

#[test]
fn test_schema_serialization() {
    let mut models = HashMap::new();
    models.insert(
        "User".to_string(),
        ModelDefinition {
            name: "User".to_string(),
            parents: vec![],
            fields: vec![
                FieldDefinition {
                    name: "id".to_string(),
                    field_type: TypeExpression::Identifier {
                        name: "number".to_string(),
                    },
                    optional: false,
                    default: None,
                    config: serde_json::json!({}),
                    entity_id: None,
                },
            ],
            config: serde_json::json!({}),
            entity_id: None,
        },
    );

    let schema = Schema {
        models,
        type_aliases: HashMap::new(),
    };

    let json = serde_json::to_string(&schema).unwrap();
    let deserialized: Schema = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.models.len(), 1);
    assert!(deserialized.models.contains_key("User"));
    assert_eq!(deserialized.type_aliases.len(), 0);
}

#[test]
fn test_case_format_serialization() {
    assert_eq!(
        serde_json::to_string(&CaseFormat::Snake).unwrap(),
        "\"snake\""
    );
    assert_eq!(
        serde_json::to_string(&CaseFormat::Camel).unwrap(),
        "\"camel\""
    );
    assert_eq!(
        serde_json::to_string(&CaseFormat::Pascal).unwrap(),
        "\"pascal\""
    );
    assert_eq!(
        serde_json::to_string(&CaseFormat::Kebab).unwrap(),
        "\"kebab\""
    );
    assert_eq!(
        serde_json::to_string(&CaseFormat::Constant).unwrap(),
        "\"constant\""
    );
    assert_eq!(
        serde_json::to_string(&CaseFormat::Title).unwrap(),
        "\"title\""
    );
}

// Pluralization tests
#[test]
fn test_pluralize_regular() {
    assert_eq!(pluralize("cat"), "cats");
    assert_eq!(pluralize("dog"), "dogs");
    assert_eq!(pluralize("table"), "tables");
    assert_eq!(pluralize(""), "");
}

#[test]
fn test_pluralize_s_x_z_ch_sh() {
    assert_eq!(pluralize("bus"), "buses");
    assert_eq!(pluralize("box"), "boxes");
    assert_eq!(pluralize("buzz"), "buzzes");
    assert_eq!(pluralize("church"), "churches");
    assert_eq!(pluralize("dish"), "dishes");
}

#[test]
fn test_pluralize_consonant_y() {
    assert_eq!(pluralize("city"), "cities");
    assert_eq!(pluralize("baby"), "babies");
    assert_eq!(pluralize("lady"), "ladies");
}

#[test]
fn test_pluralize_vowel_y() {
    assert_eq!(pluralize("boy"), "boys");
    assert_eq!(pluralize("key"), "keys");
    assert_eq!(pluralize("toy"), "toys");
}

#[test]
fn test_pluralize_f_fe() {
    assert_eq!(pluralize("leaf"), "leaves");
    assert_eq!(pluralize("knife"), "knives");
    assert_eq!(pluralize("wife"), "wives");
}

#[test]
fn test_pluralize_consonant_o() {
    assert_eq!(pluralize("hero"), "heroes");
    assert_eq!(pluralize("potato"), "potatoes");
    assert_eq!(pluralize("tomato"), "tomatoes");
}

#[test]
fn test_pluralize_irregular() {
    assert_eq!(pluralize("person"), "people");
    assert_eq!(pluralize("child"), "children");
    assert_eq!(pluralize("man"), "men");
    assert_eq!(pluralize("woman"), "women");
    assert_eq!(pluralize("tooth"), "teeth");
    assert_eq!(pluralize("foot"), "feet");
    assert_eq!(pluralize("mouse"), "mice");
    assert_eq!(pluralize("goose"), "geese");
}

#[test]
fn test_pluralize_irregular_capitalized() {
    assert_eq!(pluralize("Person"), "People");
    assert_eq!(pluralize("Child"), "Children");
    assert_eq!(pluralize("Man"), "Men");
}

#[test]
fn test_pluralize_unchanging() {
    assert_eq!(pluralize("sheep"), "sheep");
    assert_eq!(pluralize("fish"), "fish");
    assert_eq!(pluralize("deer"), "deer");
    assert_eq!(pluralize("species"), "species");
    assert_eq!(pluralize("series"), "series");
}

#[test]
fn test_utils_pluralize() {
    let utils = Utils;
    assert_eq!(utils.pluralize("user"), "users");
    assert_eq!(utils.pluralize("category"), "categories");
    assert_eq!(utils.pluralize("person"), "people");
}
