use super::*;
fn validate_source(source: &str) -> ValidationResult {
    validate(source, &[])
}

fn get_errors(result: &ValidationResult) -> Vec<&Diagnostic> {
    result
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect()
}

fn has_error_containing(result: &ValidationResult, text: &str) -> bool {
    result
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error && d.message.contains(text))
}

fn parse(source: &str) -> tree_sitter::Tree {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&grammar::LANGUAGE.into())
        .expect("Failed to load grammar");
    parser.parse(source, None).expect("Failed to parse")
}

#[test]
fn test_empty_file() {
    let source = "";
    let tree = parse(source);
    let mut diagnostics = Vec::new();

    let (symbol_table, _) = collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

    assert!(diagnostics.is_empty());
    assert!(symbol_table.definitions.is_empty());
}

#[test]
fn test_single_type_alias() {
    let source = "Email: string";
    let tree = parse(source);
    let mut diagnostics = Vec::new();

    let (symbol_table, _) = collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

    assert!(diagnostics.is_empty());
    assert_eq!(symbol_table.definitions.len(), 1);
    
    let def = symbol_table.get("Email").expect("Email should be defined");
    assert!(matches!(def.kind, DefinitionKind::TypeAlias { .. }));
}

#[test]
fn test_single_model() {
    let source = r#"
        User {
            name: string
            email: string
        }
    "#;
    let tree = parse(source);
    let mut diagnostics = Vec::new();

    let (symbol_table, _) = collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

    assert!(diagnostics.is_empty());
    assert_eq!(symbol_table.definitions.len(), 1);

    let def = symbol_table.get("User").expect("User should be defined");
    assert!(matches!(&def.kind, DefinitionKind::Model { extends } if extends.is_empty()));
}

#[test]
fn test_model_with_single_extends() {
    let source = r#"
        Timestamped {
            created_at: string
        }

        Article extends Timestamped {
            title: string
        }
    "#;
    let tree = parse(source);
    let mut diagnostics = Vec::new();

    let (symbol_table, _) = collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

    assert!(diagnostics.is_empty());
    assert_eq!(symbol_table.definitions.len(), 2);

    let def = symbol_table.get("Article").expect("Article should be defined");
    match &def.kind {
        DefinitionKind::Model { extends } => {
            assert_eq!(extends, &vec!["Timestamped".to_string()]);
        }
        _ => panic!("Expected Model"),
    }
}

#[test]
fn test_model_with_multiple_extends() {
    let source = r#"
        BaseUser {
            id: number
        }

        Timestamped {
            created_at: string
        }

        AdminUser extends BaseUser, Timestamped {
            admin_level: number
        }
    "#;
    let tree = parse(source);
    let mut diagnostics = Vec::new();

    let (symbol_table, _) = collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

    assert!(diagnostics.is_empty());
    assert_eq!(symbol_table.definitions.len(), 3);

    let def = symbol_table.get("AdminUser").expect("AdminUser should be defined");
    match &def.kind {
        DefinitionKind::Model { extends } => {
            assert_eq!(extends, &vec![
                "BaseUser".to_string(),
                "Timestamped".to_string()
            ]);
        }
        _ => panic!("Expected Model"),
    }
}

#[test]
fn test_multiple_type_aliases() {
    let source = r#"
        Email: string
        Age: number
        Active: boolean
    "#;
    let tree = parse(source);
    let mut diagnostics = Vec::new();

    let (symbol_table, _) = collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

    assert!(diagnostics.is_empty());
    assert_eq!(symbol_table.definitions.len(), 3);
    assert!(symbol_table.get("Email").is_some());
    assert!(symbol_table.get("Age").is_some());
    assert!(symbol_table.get("Active").is_some());
}

#[test]
fn test_union_type_alias() {
    let source = r#"Status: "active" | "pending" | "deleted""#;
    let tree = parse(source);
    let mut diagnostics = Vec::new();

    let (symbol_table, _) = collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

    assert!(diagnostics.is_empty());
    assert_eq!(symbol_table.definitions.len(), 1);
    
    let def = symbol_table.get("Status").expect("Status should be defined");
    assert!(matches!(def.kind, DefinitionKind::TypeAlias { .. }));
}

#[test]
fn test_duplicate_type_alias_error() {
    let source = r#"
        Email: string
        Email: number
    "#;
    let tree = parse(source);
    let mut diagnostics = Vec::new();

    let (symbol_table, _) = collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].severity, Severity::Error);
    assert!(diagnostics[0].message.contains("Email"));
    assert!(diagnostics[0].message.contains("already defined"));

    // First definition should still be in the table
    assert_eq!(symbol_table.definitions.len(), 1);
    assert!(symbol_table.get("Email").is_some());
}

#[test]
fn test_duplicate_model_error() {
    let source = r#"
        User {
            name: string
        }

        User {
            email: string
        }
    "#;
    let tree = parse(source);
    let mut diagnostics = Vec::new();

    collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].severity, Severity::Error);
    assert!(diagnostics[0].message.contains("User"));
    assert!(diagnostics[0].message.contains("already defined"));
}

#[test]
fn test_duplicate_type_alias_and_model_error() {
    let source = r#"
        User: string

        User {
            name: string
        }
    "#;
    let tree = parse(source);
    let mut diagnostics = Vec::new();

    collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].severity, Severity::Error);
    assert!(diagnostics[0].message.contains("User"));
}

#[test]
fn test_mixed_definitions() {
    let source = r#"
        Email: string
        Status: "active" | "pending"

        Address {
            street: string
            city: string
        }

        User {
            email: Email
            address: Address
        }

        AdminUser extends User {
            role: string
        }
    "#;
    let tree = parse(source);
    let mut diagnostics = Vec::new();

    let (symbol_table, _) = collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

    assert!(diagnostics.is_empty());
    assert_eq!(symbol_table.definitions.len(), 5);

    assert!(matches!(
        symbol_table.get("Email").unwrap().kind,
        DefinitionKind::TypeAlias { .. }
    ));
    assert!(matches!(
        symbol_table.get("Status").unwrap().kind,
        DefinitionKind::TypeAlias { .. }
    ));
    assert!(matches!(
        symbol_table.get("Address").unwrap().kind,
        DefinitionKind::Model { ref extends } if extends.is_empty()
    ));
    assert!(matches!(
        symbol_table.get("User").unwrap().kind,
        DefinitionKind::Model { ref extends } if extends.is_empty()
    ));
    let def = symbol_table.get("AdminUser").expect("AdminUser should be defined");
    match &def.kind {
        DefinitionKind::Model { extends } => {
            assert_eq!(extends, &vec!["User".to_string()]);
        }
        _ => panic!("Expected Model"),
    }
}

#[test]
fn test_builtin_types_not_in_definitions() {
    let source = r#"
        User {
            name: string
            age: number
        }
    "#;
    let tree = parse(source);
    let mut diagnostics = Vec::new();

    let (symbol_table, _) = collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

    // Built-ins should not be in definitions
    assert!(symbol_table.definitions.get("string").is_none());
    assert!(symbol_table.definitions.get("number").is_none());

    // But is_defined should return true for them
    assert!(symbol_table.is_defined("string"));
    assert!(symbol_table.is_defined("number"));
    assert!(symbol_table.is_defined("boolean"));
    assert!(symbol_table.is_defined("string"));
}

#[test]
fn test_span_tracking() {
    let source = "Email: string";
    let tree = parse(source);
    let mut diagnostics = Vec::new();

    let (symbol_table, _) = collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

    let def = symbol_table.get("Email").expect("Email should be defined");
    assert_eq!(def.span.start.line, 0);
    assert_eq!(def.span.start.column, 0);
}

#[test]
fn test_type_alias_with_plugin_block() {
    let source = r#"
        Foo: string {
            @validation { format: "uuid" }
            @sql { type: "number" }
        }
    "#;
    let tree = parse(source);
    let mut diagnostics = Vec::new();

    let (symbol_table, _) = collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

    println!("{:?}", diagnostics);
    assert!(diagnostics.is_empty());
    assert_eq!(symbol_table.definitions.len(), 1);
    assert!(symbol_table.get("Foo").is_some());
}

#[test]
fn test_model_with_complex_fields() {
    let source = r#"
        User {
            id: number
            tags: Tag[]
            status?: Status
            active: boolean = true
        }
    "#;
    let tree = parse(source);
    let mut diagnostics = Vec::new();

    let (symbol_table, _) = collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

    assert!(diagnostics.is_empty());
    assert_eq!(symbol_table.definitions.len(), 1);
    assert!(symbol_table.get("User").is_some());
}

#[test]
fn test_type_alias_references_collected() {
    let source = r#"
        Email: string
        UserEmail: Email
        Result: Email | string | number
        Items: Email[]
    "#;
    let tree = parse(source);
    let mut diagnostics = Vec::new();

    let (symbol_table, _) = collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

    assert!(diagnostics.is_empty());

    // Email references string
    let email_def = symbol_table.get("Email").unwrap();
    match &email_def.kind {
        DefinitionKind::TypeAlias { references, type_expr: _type_expr } => {
            assert_eq!(references, &vec!["string".to_string()]);
        }
        _ => panic!("Expected TypeAlias"),
    }

    // UserEmail references Email
    let user_email_def = symbol_table.get("UserEmail").unwrap();
    match &user_email_def.kind {
        DefinitionKind::TypeAlias { references, type_expr: _type_expr } => {
            assert_eq!(references, &vec!["Email".to_string()]);
        }
        _ => panic!("Expected TypeAlias"),
    }

    // Result references Email, string, number (from union)
    let result_def = symbol_table.get("Result").unwrap();
    match &result_def.kind {
        DefinitionKind::TypeAlias { references, type_expr: _type_expr } => {
            assert!(references.contains(&"Email".to_string()));
            assert!(references.contains(&"string".to_string()));
            assert!(references.contains(&"number".to_string()));
        }
        _ => panic!("Expected TypeAlias"),
    }

    // Items references Email (from array)
    let items_def = symbol_table.get("Items").unwrap();
    match &items_def.kind {
        DefinitionKind::TypeAlias { references, type_expr: _type_expr } => {
            assert_eq!(references, &vec!["Email".to_string()]);
        }
        _ => panic!("Expected TypeAlias"),
    }
}

#[test]
fn test_string_literal_union_no_references() {
    let source = r#"Status: "active" | "pending" | "deleted""#;
    let tree = parse(source);
    let mut diagnostics = Vec::new();

    let (symbol_table, _) = collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

    let def = symbol_table.get("Status").unwrap();
    match &def.kind {
        DefinitionKind::TypeAlias { references, type_expr: _type_expr } => {
            // String literals don't create type references
            assert!(references.is_empty());
        }
        _ => panic!("Expected TypeAlias"),
    }
}

#[cfg(test)]
mod validate_tests {
    use super::*;

    // =========================================================================
    // VALID FILES - NO ERRORS EXPECTED
    // =========================================================================

    #[test]
    fn test_empty_file() {
        let result = validate("", &[]);
        assert!(!result.has_errors());
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn test_comments_only() {
        let source = r#"
            // This is a comment
            // Another comment
        "#;
        let result = validate(source, &[]);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_simple_type_alias() {
        let source = "Email: string";
        let result = validate(source, &[]);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_type_alias_with_builtin_types() {
        let source = r#"
            Name: string
            Age: number
            Active: boolean
            Price: number
            CreatedAt: string
            Metadata: string
            Id: number
        "#;
        let result = validate(source, &[]);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_union_type_alias_string_literals() {
        let source = r#"Status: "active" | "pending" | "deleted""#;
        let result = validate(source, &[]);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_union_type_alias_mixed() {
        let source = r#"
            Email: string
            Result: Email | "not_found" | "error"
        "#;
        let result = validate(source, &[]);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_simple_model() {
        let source = r#"
            User {
                name: string
                email: string
            }
        "#;
        let result = validate(source, &[]);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_model_with_custom_types() {
        let source = r#"
            Email: string
            Age: number

            User {
                email: Email
                age: Age
            }
        "#;
        let result = validate(source, &[]);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_model_with_array_types() {
        let source = r#"
            Tag {
                name: string
            }

            Post {
                title: string
                tags: Tag[]
            }
        "#;
        let result = validate(source, &[]);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_model_with_optional_fields() {
        let source = r#"
            User {
                name: string
                nickname?: string
                age?: number
            }
        "#;
        let result = validate(source, &[]);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_model_with_default_values() {
        let source = r#"
            User {
                name: string
                active: boolean = true
                role: string = "user"
                score: number = 0
            }
        "#;
        let result = validate(source, &[]);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_model_with_single_extends() {
        let source = r#"
            Timestamped {
                created_at: string
                updated_at: string
            }

            Article extends Timestamped {
                title: string
                content: string
            }
        "#;
        let result = validate(source, &[]);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_model_with_multiple_extends() {
        let source = r#"
            Timestamped {
                created_at: string
            }

            Identifiable {
                id: number
            }

            User extends Identifiable, Timestamped {
                name: string
            }
        "#;
        let result = validate(source, &[]);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_model_with_field_removal() {
        let source = r#"
            BaseUser {
                id: number
                name: string
                password_hash: string
            }

            PublicUser extends BaseUser {
                -password_hash
            }
        "#;
        let result = validate(source, &[]);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_model_with_plugin_config() {
        let source = r#"
            User {
                id: number
                email: string

                @sql { table: "users" }
            }
        "#;
        let result = validate(source, &[]);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_type_alias_with_plugin_config() {
        let source = r#"
            number: string {
                @validation { format: "uuid" }
                @sql { type: "number" }
            }
        "#;
        let result = validate(source, &[]);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_forward_reference() {
        let source = r#"
            User {
                posts: Post[]
            }

            Post {
                author: User
            }
        "#;
        let result = validate(source, &[]);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_forward_reference_in_extends() {
        let source = r#"
            Article extends Timestamped {
                title: string
            }

            Timestamped {
                created_at: string
            }
        "#;
        let result = validate(source, &[]);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_self_referential_model() {
        let source = r#"
            Category {
                name: string
                parent: Category
                children: Category[]
            }
        "#;
        let result = validate(source, &[]);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_complex_nested_structure() {
        let source = r#"
            Email: string
            Status: "active" | "pending" | "deleted"

            Address {
                street: string
                city: string
                country: string
            }

            ContactInfo {
                email: Email
                address: Address
            }

            User {
                id: number
                status: Status
                contact: ContactInfo
            }
        "#;
        let result = validate(source, &[]);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_untyped_fields() {
        let source = r#"
            BasicUser {
                name
                email
                bio
            }
        "#;
        let result = validate(source, &[]);
        assert!(!result.has_errors());
    }

    // -------------------------------------------------------------------------
    // Untyped Fields with Defaults (Syntax Errors)
    // -------------------------------------------------------------------------

    #[test]
    fn untyped_field_with_string_default_is_syntax_error() {
        let source = r#"
            User {
                name = "John"
            }
        "#;
        let result = validate_source(source);
        assert!(has_error_containing(&result, "Syntax error"));
    }

    #[test]
    fn untyped_field_with_number_default_is_syntax_error() {
        let source = r#"
            User {
                count = 42
            }
        "#;
        let result = validate_source(source);
        assert!(has_error_containing(&result, "Syntax error"));
    }

    #[test]
    fn untyped_field_with_boolean_default_is_syntax_error() {
        let source = r#"
            User {
                active = true
            }
        "#;
        let result = validate_source(source);
        assert!(has_error_containing(&result, "Syntax error"));
    }

    #[test]
    fn untyped_field_with_array_default_is_syntax_error() {
        let source = r#"
            User {
                tags = ["a", "b"]
            }
        "#;
        let result = validate_source(source);
        assert!(has_error_containing(&result, "Syntax error"));
    }

    #[test]
    fn untyped_field_with_object_default_is_syntax_error() {
        let source = r#"
            User {
                config = { key: "value" }
            }
        "#;
        let result = validate_source(source);
        assert!(has_error_containing(&result, "Syntax error"));
    }

    #[test]
    fn untyped_optional_field_with_default_is_syntax_error() {
        let source = r#"
            User {
                nickname? = "none"
            }
        "#;
        let result = validate_source(source);
        assert!(has_error_containing(&result, "Syntax error"));
    }

    // -------------------------------------------------------------------------
    // Typed Fields with Defaults (Valid Syntax)
    // -------------------------------------------------------------------------

    #[test]
    fn typed_field_with_default_is_valid_syntax() {
        let source = r#"
            User {
                name: string = "John"
                count: number = 0
                active: boolean = true
            }
        "#;
        let result = validate_source(source);
        let errors = get_errors(&result);
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    }

    #[test]
    fn untyped_field_without_default_is_valid() {
        let source = r#"
            User {
                name
                email
                bio
            }
        "#;
        let result = validate_source(source);
        let errors = get_errors(&result);
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    }

    // =========================================================================
    // SYNTAX ERRORS
    // =========================================================================

    #[test]
    fn test_syntax_error_missing_brace() {
        let source = r#"
            User {
                name: string
        "#;
        let result = validate(source, &[]);
        assert!(result.has_errors());
        assert!(result.diagnostics.iter().any(|d| d.severity == Severity::Error));
    }

    #[test]
    fn test_syntax_error_invalid_token() {
        let source = r#"
            User {
                name: string
                @@invalid
            }
        "#;
        let result = validate(source, &[]);
        assert!(result.has_errors());
    }

    #[test]
    fn test_syntax_error_missing_colon_in_field() {
        let source = r#"
            User {
                name string
            }
        "#;
        let result = validate(source, &[]);
        // This might parse as untyped field "name" and model "string"
        // depending on grammar - check the actual behavior
        let _ = result;
    }

    // =========================================================================
    // UNDEFINED TYPE REFERENCES
    // =========================================================================

    #[test]
    fn test_undefined_type_in_field() {
        let source = r#"
            User {
                email: Emaill
            }
        "#;
        let result = validate(source, &[]);
        assert!(result.has_errors());
        assert!(result.diagnostics.iter().any(|d| 
            d.message.contains("Undefined type") && d.message.contains("Emaill")
        ));
    }

    #[test]
    fn test_undefined_type_in_array_field() {
        let source = r#"
            User {
                posts: Postt[]
            }
        "#;
        let result = validate(source, &[]);
        assert!(result.has_errors());
        assert!(result.diagnostics.iter().any(|d| 
            d.message.contains("Undefined type") && d.message.contains("Postt")
        ));
    }

    #[test]
    fn test_undefined_type_in_type_alias() {
        let source = r#"
            MyEmail: Emaill
        "#;
        let result = validate(source, &[]);
        assert!(result.has_errors());
        assert!(result.diagnostics.iter().any(|d| 
            d.message.contains("Undefined type") && d.message.contains("Emaill")
        ));
    }

    #[test]
    fn test_undefined_type_in_union() {
        let source = r#"
            Email: string
            Result: Email | NotFound | "error"
        "#;
        let result = validate(source, &[]);
        assert!(result.has_errors());
        assert!(result.diagnostics.iter().any(|d| 
            d.message.contains("Undefined type") && d.message.contains("NotFound")
        ));
    }

    #[test]
    fn test_multiple_undefined_types() {
        let source = r#"
            User {
                email: Emaill
                address: Addresss
                status: Statuss
            }
        "#;
        let result = validate(source, &[]);
        assert!(result.has_errors());
        assert_eq!(
            result.diagnostics.iter().filter(|d| d.message.contains("Undefined type")).count(),
            3
        );
    }

    #[test]
    fn test_undefined_type_case_sensitive() {
        let source = r#"
            Email: string

            User {
                email: email
            }
        "#;
        let result = validate(source, &[]);
        assert!(result.has_errors());
        assert!(result.diagnostics.iter().any(|d| 
            d.message.contains("Undefined type") && d.message.contains("email")
        ));
    }

    // =========================================================================
    // DUPLICATE DEFINITIONS
    // =========================================================================

    #[test]
    fn test_duplicate_type_alias() {
        let source = r#"
            Email: string
            Email: number
        "#;
        let result = validate(source, &[]);
        assert!(result.has_errors());
        assert!(result.diagnostics.iter().any(|d| 
            d.message.contains("already defined") && d.message.contains("Email")
        ));
    }

    #[test]
    fn test_duplicate_model() {
        let source = r#"
            User {
                name: string
            }

            User {
                email: string
            }
        "#;
        let result = validate(source, &[]);
        assert!(result.has_errors());
        assert!(result.diagnostics.iter().any(|d| 
            d.message.contains("already defined") && d.message.contains("User")
        ));
    }

    #[test]
    fn test_duplicate_type_alias_and_model() {
        let source = r#"
            User: string

            User {
                name: string
            }
        "#;
        let result = validate(source, &[]);
        assert!(result.has_errors());
        assert!(result.diagnostics.iter().any(|d| 
            d.message.contains("already defined") && d.message.contains("User")
        ));
    }

    #[test]
    fn test_shadowing_builtin_type() {
        let source = r#"
            string: number
        "#;
        let result = validate(source, &[]);
        // Depending on design decision - this could be an error or warning
        // For now, just verify it doesn't crash
        let _ = result;
    }

    // =========================================================================
    // EXTENDS VALIDATION
    // =========================================================================

    #[test]
    fn test_undefined_extends_target() {
        let source = r#"
            Article extends Timestampedd {
                title: string
            }
        "#;
        let result = validate(source, &[]);
        assert!(result.has_errors());
        assert!(result.diagnostics.iter().any(|d| 
            d.message.contains("Timestampedd")
        ));
    }

    #[test]
    fn test_multiple_undefined_extends_targets() {
        let source = r#"
            Article extends BaseA, BaseB, BaseC {
                title: string
            }
        "#;
        let result = validate(source, &[]);
        assert!(result.has_errors());
        // Should report all three as undefined
        assert!(result.diagnostics.len() >= 3);
    }

    #[test]
    fn test_extends_type_alias_instead_of_model() {
        let source = r#"
            Email: string

            User extends Email {
                name: string
            }
        "#;
        let result = validate(source, &[]);
        assert!(result.has_errors());
        println!("{:?}", result.diagnostics);
        assert!(result.diagnostics.iter().any(|d| 
            d.message.contains("Email") && 
            (d.message.contains("not a model") || d.message.contains("type alias"))
        ));
    }

    // =========================================================================
    // DIAGNOSTIC DETAILS
    // =========================================================================

    #[test]
    fn test_error_span_is_accurate() {
        let source = "User { email: Emaill }";
        let result = validate(source, &[]);
        assert!(result.has_errors());

        let error = result.diagnostics.iter()
            .find(|d| d.message.contains("Emaill"))
            .expect("Should have error for Emaill");

        // "Emaill" starts at column 14 (0-indexed)
        assert_eq!(error.span.start.line, 0);
        assert_eq!(error.span.start.column, 14);
    }

    #[test]
    fn test_duplicate_error_references_original_line() {
        let source = r#"Email: string
Email: number"#;
        let result = validate(source, &[]);
        
        let error = result.diagnostics.iter()
            .find(|d| d.message.contains("already defined"))
            .expect("Should have duplicate error");

        // Error should mention line 1 (where original was defined)
        assert!(error.message.contains("line 1"));
        // Error should be on line 2 (0-indexed: line 1)
        assert_eq!(error.span.start.line, 1);
    }

    // =========================================================================
    // TREE RETURNED
    // =========================================================================

    #[test]
    fn test_tree_returned_on_success() {
        let source = "Email: string";
        let result = validate(source, &[]);
        assert!(result.tree.is_some());
    }

    #[test]
    fn test_tree_returned_even_with_semantic_errors() {
        let source = "User { email: Emaill }";
        let result = validate(source, &[]);
        assert!(result.has_errors());
        assert!(result.tree.is_some());
    }

    // =========================================================================
    // EDGE CASES
    // =========================================================================

    #[test]
    fn test_whitespace_variations() {
        let source = "Email:string";
        let result = validate(source, &[]);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_comments_between_definitions() {
        let source = r#"
            // User email type
            Email: string

            // The user model
            User {
                // User's email
                email: Email
            }
        "#;
        let result = validate(source, &[]);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_deeply_nested_extends_chain() {
        let source = r#"
            A { a: string }
            B extends A { b: string }
            C extends B { c: string }
            D extends C { d: string }
            E extends D { e: string }
        "#;
        let result = validate(source, &[]);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_model_referencing_itself_in_field() {
        let source = r#"
            Node {
                value: string
                next: Node
                children: Node[]
            }
        "#;
        let result = validate(source, &[]);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_optional_array_of_custom_type() {
        let source = r#"
            Tag { name: string }

            Post {
                tags?: Tag[]
            }
        "#;
        let result = validate(source, &[]);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_inline_union_in_field() {
        let source = r#"
            User {
                status: "active" | "pending" | "deleted"
            }
        "#;
        let result = validate(source, &[]);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_inline_union_with_default() {
        let source = r#"
            User {
                status: "active" | "pending" | "deleted" = "active"
            }
        "#;
        let result = validate(source, &[]);
        assert!(!result.has_errors());
    }
}

#[test]
fn test_parse_returns_tree_even_for_garbage() {
    // Tree-sitter is resilient - it returns a tree even for invalid input
    // The "Failed to parse file" branch is defensive, for cases like
    // allocation failure or parser cancellation
    let source = "!@#$%^&*()_+{}|:<>?~`";
    let result = validate(source, &[]);
    
    // Parser still returns a tree, just with syntax errors
    assert!(result.tree.is_some());
    assert!(result.has_errors());
}

#[test]
fn test_symbol_table_display() {
    let source = r#"
Email: string
Status: "active" | "pending"

User {
    email: Email
}

AdminUser extends User {
    role: string
}
"#;
    let tree = parse(source);
    let mut diagnostics = Vec::new();

    let (symbol_table, _) = collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

    let display = format!("{}", symbol_table);

    // Check header
    assert!(display.contains("Symbol Table"));
    assert!(display.contains("4 definitions"));

    // Check type aliases (now shows references)
    assert!(display.contains("Email"));
    assert!(display.contains("type alias"));
    assert!(display.contains("Status"));

    // Check models
    assert!(display.contains("User (model)"));
    assert!(display.contains("AdminUser (model extends User)"));
}

#[test]
fn test_symbol_table_display_empty() {
    let symbol_table = SymbolTable::new();
    let display = format!("{}", symbol_table);

    assert!(display.contains("0 definitions"));
}

#[test]
fn test_symbol_table_display_multiple_extends() {
    let source = r#"
Base1 { a: string }
Base2 { b: string }
Child extends Base1, Base2 { c: string }
"#;
    let tree = parse(source);
    let mut diagnostics = Vec::new();

    let (symbol_table, _) = collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

    let display = format!("{}", symbol_table);

    assert!(display.contains("Child (model extends Base1, Base2)"));
}

#[test]
fn test_symbol_table_display_type_alias_with_references() {
    let source = r#"
Email: string
ValidatedEmail: Email
"#;
    let tree = parse(source);
    let mut diagnostics = Vec::new();

    let (symbol_table, _) = collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

    let display = format!("{}", symbol_table);

    // Should show references
    assert!(display.contains("Email (type alias -> string)"));
    assert!(display.contains("ValidatedEmail (type alias -> Email)"));
}

#[test]
fn test_get_inherited_fields_local_parent() {
    // This tests the code path where a model extends another model
    // defined in the same file (local_symbol_table), not in ancestors.
    // The field removal validation triggers get_inherited_fields.
    
    let source = r#"
        Parent {
            id: number
            secret: string
        }
        
        Child extends Parent {
            -secret
            name: string
        }
    "#;
    
    let result = validate(source, &[]);
    
    // Should be valid - secret exists in Parent (same file)
    let removal_errors: Vec<_> = result.diagnostics.iter()
        .filter(|d| d.message.contains("Cannot remove field"))
        .collect();
    
    assert!(removal_errors.is_empty(), "Errors: {:?}", removal_errors);
}

#[test]
fn test_get_inherited_fields_local_grandparent() {
    // Tests recursive inheritance within the same file:
    // Grandparent -> Parent -> Child
    
    let source = r#"
        Grandparent {
            id: number
            internal: string
        }
        
        Parent extends Grandparent {
            name: string
        }
        
        Child extends Parent {
            -internal
            email: string
        }
    "#;
    
    let result = validate(source, &[]);
    
    // Should be valid - internal exists in Grandparent (same file)
    let removal_errors: Vec<_> = result.diagnostics.iter()
        .filter(|d| d.message.contains("Cannot remove field"))
        .collect();
    
    assert!(removal_errors.is_empty(), "Errors: {:?}", removal_errors);
}

#[test]
fn test_get_inherited_fields_local_multiple_inheritance() {
    // Tests multiple inheritance within the same file
    
    let source = r#"
        Timestamped {
            created_at: string
            updated_at: string
        }
        
        Auditable {
            created_by: string
            updated_by: string
        }
        
        Document extends Timestamped, Auditable {
            -updated_by
            -updated_at
            title: string
        }
    "#;
    
    let result = validate(source, &[]);
    
    // Should be valid - fields exist in local parents
    let removal_errors: Vec<_> = result.diagnostics.iter()
        .filter(|d| d.message.contains("Cannot remove field"))
        .collect();
    
    assert!(removal_errors.is_empty(), "Errors: {:?}", removal_errors);
}

// =========================================================================
// CIRCULAR INHERITANCE TESTS
// =========================================================================

#[cfg(test)]
mod circular_inheritance_tests {
    use super::*;

    #[test]
    fn test_self_reference() {
        // Model extends itself
        let source = r#"
            A extends A {
                field: string
            }
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let cycle_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Circular inheritance"))
            .collect();
        
        assert_eq!(cycle_errors.len(), 1);
        assert!(cycle_errors[0].message.contains("A -> A"));
    }

    #[test]
    fn test_direct_cycle() {
        // A extends B, B extends A
        let source = r#"
            A extends B {
                field_a: string
            }
            
            B extends A {
                field_b: string
            }
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let cycle_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Circular inheritance"))
            .collect();
        
        // Should detect the cycle (may report from either A or B depending on iteration order)
        assert!(!cycle_errors.is_empty());
    }

    #[test]
    fn test_indirect_cycle() {
        // A -> B -> C -> A
        let source = r#"
            A extends B {
                field_a: string
            }
            
            B extends C {
                field_b: string
            }
            
            C extends A {
                field_c: string
            }
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let cycle_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Circular inheritance"))
            .collect();
        
        assert!(!cycle_errors.is_empty());
    }

    #[test]
    fn test_multiple_inheritance_cycle() {
        // A extends B, C where C extends A
        let source = r#"
            A extends B, C {
                field_a: string
            }
            
            B {
                field_b: string
            }
            
            C extends A {
                field_c: string
            }
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let cycle_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Circular inheritance"))
            .collect();
        
        assert!(!cycle_errors.is_empty());
    }

    #[test]
    fn test_no_cycle_linear_chain() {
        // A -> B -> C (no cycle)
        let source = r#"
            Base {
                id: string
            }
            
            Middle extends Base {
                name: string
            }
            
            Derived extends Middle {
                value: number
            }
        "#;
        
        let result = validate(source, &[]);
        
        let cycle_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Circular inheritance"))
            .collect();
        
        assert!(cycle_errors.is_empty());
    }

    #[test]
    fn test_no_cycle_diamond() {
        // Diamond inheritance (not a cycle)
        //      Base
        //     /    \
        //    A      B
        //     \    /
        //      Child
        let source = r#"
            Base {
                id: string
            }
            
            A extends Base {
                a: string
            }
            
            B extends Base {
                b: string
            }
            
            Child extends A, B {
                c: string
            }
        "#;
        
        let result = validate(source, &[]);
        
        let cycle_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Circular inheritance"))
            .collect();
        
        assert!(cycle_errors.is_empty());
    }

    #[test]
    fn test_cycle_in_diamond() {
        // Diamond with cycle
        //      Base
        //     /    \
        //    A      B
        //     \    /
        //      Child -> Base creates cycle
        let source = r#"
            Base extends Child {
                id: string
            }
            
            A extends Base {
                a: string
            }
            
            B extends Base {
                b: string
            }
            
            Child extends A, B {
                c: string
            }
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let cycle_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Circular inheritance"))
            .collect();
        
        assert!(!cycle_errors.is_empty());
    }

    #[test]
    fn test_multiple_separate_cycles() {
        // Two independent cycles
        let source = r#"
            A extends B {
                a: string
            }
            
            B extends A {
                b: string
            }
            
            X extends Y {
                x: string
            }
            
            Y extends X {
                y: string
            }
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let cycle_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Circular inheritance"))
            .collect();
        
        // Should detect both cycles
        assert!(cycle_errors.len() >= 2);
    }

    #[test]
    fn test_cycle_with_undefined_in_chain() {
        // A extends B, B extends Undefined
        // Should report undefined error, not crash on cycle detection
        let source = r#"
            A extends B {
                a: string
            }
            
            B extends Undefined {
                b: string
            }
        "#;
        
        let result = validate(source, &[]);
        
        // Should have undefined error but not crash
        let undefined_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Undefined"))
            .collect();
        
        assert!(!undefined_errors.is_empty());
        
        // No cycle errors since chain is broken
        let cycle_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Circular inheritance"))
            .collect();
        
        assert!(cycle_errors.is_empty());
    }
}

// =========================================================================
// CIRCULAR TYPE ALIAS TESTS
// =========================================================================

#[cfg(test)]
mod circular_type_alias_tests {
    use super::*;

    #[test]
    fn test_self_referencing_alias() {
        // Type alias references itself
        let source = r#"
            A: A
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let cycle_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Circular type reference"))
            .collect();
        
        assert_eq!(cycle_errors.len(), 1);
        assert!(cycle_errors[0].message.contains("A -> A"));
    }

    #[test]
    fn test_direct_alias_cycle() {
        // A: B and B: A
        let source = r#"
            A: B
            B: A
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let cycle_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Circular type reference"))
            .collect();
        
        assert!(!cycle_errors.is_empty());
    }

    #[test]
    fn test_indirect_alias_cycle() {
        // A -> B -> C -> A
        let source = r#"
            A: B
            B: C
            C: A
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let cycle_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Circular type reference"))
            .collect();
        
        assert!(!cycle_errors.is_empty());
    }

    #[test]
    fn test_cycle_through_array() {
        // A: B[] and B: A
        let source = r#"
            A: B[]
            B: A
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let cycle_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Circular type reference"))
            .collect();
        
        assert!(!cycle_errors.is_empty());
    }

    #[test]
    fn test_cycle_through_union() {
        // A: B | string and B: A | number
        let source = r#"
            A: B | string
            B: A | number
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let cycle_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Circular type reference"))
            .collect();
        
        assert!(!cycle_errors.is_empty());
    }

    #[test]
    fn test_cycle_in_complex_union() {
        // Cycle hidden in a larger union
        let source = r#"
            Result: Success | Failure | Pending
            Success: string
            Failure: ErrorCode
            ErrorCode: Result
            Pending: "pending"
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let cycle_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Circular type reference"))
            .collect();
        
        // Cycle: Result -> Failure -> ErrorCode -> Result
        assert!(!cycle_errors.is_empty());
    }

    #[test]
    fn test_no_cycle_to_builtin() {
        // Aliases to built-in types are not cycles
        let source = r#"
            Email: string
            Age: number
            Active: boolean
            Amount: number
            CreatedAt: string
            Metadata: string
        "#;
        
        let result = validate(source, &[]);
        
        let cycle_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Circular type reference"))
            .collect();
        
        assert!(cycle_errors.is_empty());
    }

    #[test]
    fn test_no_cycle_linear_chain() {
        // A -> B -> C -> string (no cycle)
        let source = r#"
            A: B
            B: C
            C: string
        "#;
        
        let result = validate(source, &[]);
        
        let cycle_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Circular type reference"))
            .collect();
        
        assert!(cycle_errors.is_empty());
    }

    #[test]
    fn test_no_cycle_alias_to_model() {
        // Type alias referencing a model is not a cycle
        let source = r#"
            UserRef: User
            
            User {
                name: string
            }
        "#;
        
        let result = validate(source, &[]);
        
        let cycle_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Circular type reference"))
            .collect();
        
        assert!(cycle_errors.is_empty());
    }

    #[test]
    fn test_no_cycle_string_literal_union() {
        // Union of string literals has no type references
        let source = r#"
            Status: "active" | "pending" | "deleted"
            Priority: "low" | "medium" | "high"
        "#;
        
        let result = validate(source, &[]);
        
        let cycle_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Circular type reference"))
            .collect();
        
        assert!(cycle_errors.is_empty());
    }

    #[test]
    fn test_multiple_separate_alias_cycles() {
        // Two independent cycles
        let source = r#"
            A: B
            B: A
            
            X: Y
            Y: X
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let cycle_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Circular type reference"))
            .collect();
        
        // Should detect both cycles
        assert!(cycle_errors.len() >= 2);
    }

    #[test]
    fn test_alias_cycle_with_undefined_in_chain() {
        // A: B, B: Undefined - should report undefined, not crash
        let source = r#"
            A: B
            B: Undefined
        "#;
        
        let result = validate(source, &[]);
        
        // Should have undefined error
        let undefined_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Undefined"))
            .collect();
        
        assert!(!undefined_errors.is_empty());
        
        // No cycle since chain is broken
        let cycle_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Circular type reference"))
            .collect();
        
        assert!(cycle_errors.is_empty());
    }

    #[test]
    fn test_diamond_alias_no_cycle() {
        // Diamond pattern (not a cycle)
        //      Base
        //     /    \
        //    A      B
        //     \    /
        //      Child (union)
        let source = r#"
            Base: string
            A: Base
            B: Base
            Child: A | B
        "#;
        
        let result = validate(source, &[]);
        
        let cycle_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Circular type reference"))
            .collect();
        
        assert!(cycle_errors.is_empty());
    }

    #[test]
    fn test_mixed_union_with_literals_and_types() {
        // Union mixing string literals and type references, one creates cycle
        let source = r#"
            Status: "pending" | Active | "deleted"
            Active: Status
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let cycle_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Circular type reference"))
            .collect();
        
        assert!(!cycle_errors.is_empty());
    }

    #[test]
    fn test_self_referencing_array_alias() {
        // A: A[] - array of self
        let source = r#"
            A: A[]
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let cycle_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Circular type reference"))
            .collect();
        
        assert_eq!(cycle_errors.len(), 1);
        assert!(cycle_errors[0].message.contains("A -> A"));
    }
}

// Tests for duplicate field name and field override validation
// Add this section to your existing tests.rs file

// =============================================================================
// DUPLICATE FIELD NAME TESTS
// =============================================================================

#[cfg(test)]
mod duplicate_field_tests {
    use super::*;

    #[test]
    fn test_no_duplicate_fields_valid() {
        let source = r#"
            User {
                id: number
                name: string
                email: string
                age: number
            }
        "#;
        
        let result = validate(source, &[]);
        
        let dup_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Duplicate field"))
            .collect();
        
        assert!(dup_errors.is_empty());
    }

    #[test]
    fn test_duplicate_field_simple() {
        let source = r#"
            User {
                email: string
                email: string
            }
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let dup_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Duplicate field"))
            .collect();
        
        assert_eq!(dup_errors.len(), 1);
        assert!(dup_errors[0].message.contains("email"));
        assert!(dup_errors[0].message.contains("first defined at line"));
    }

    #[test]
    fn test_duplicate_field_different_types() {
        // Same name, different types - still a duplicate
        let source = r#"
            User {
                email: string
                email: number
            }
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let dup_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Duplicate field"))
            .collect();
        
        assert_eq!(dup_errors.len(), 1);
        assert!(dup_errors[0].message.contains("email"));
    }

    #[test]
    fn test_duplicate_field_with_optional() {
        // email and email? are the same field name
        let source = r#"
            User {
                email: string
                email?: string
            }
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let dup_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Duplicate field"))
            .collect();
        
        assert_eq!(dup_errors.len(), 1);
        assert!(dup_errors[0].message.contains("email"));
    }

    #[test]
    fn test_triple_duplicate_field() {
        // Three fields with same name - should report 2 errors
        let source = r#"
            User {
                name: string
                name: number
                name: boolean
            }
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let dup_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Duplicate field"))
            .collect();
        
        // Second and third definitions are duplicates
        assert_eq!(dup_errors.len(), 2);
        for err in &dup_errors {
            assert!(err.message.contains("name"));
        }
    }

    #[test]
    fn test_multiple_different_duplicates() {
        // Two different fields duplicated
        let source = r#"
            User {
                email: string
                name: string
                email: number
                name: number
            }
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let dup_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Duplicate field"))
            .collect();
        
        assert_eq!(dup_errors.len(), 2);
        
        let messages: Vec<&str> = dup_errors.iter()
            .map(|d| d.message.as_str())
            .collect();
        
        assert!(messages.iter().any(|m| m.contains("email")));
        assert!(messages.iter().any(|m| m.contains("name")));
    }

    #[test]
    fn test_duplicate_field_with_defaults() {
        let source = r#"
            User {
                active: boolean = true
                active: boolean = false
            }
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let dup_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Duplicate field"))
            .collect();
        
        assert_eq!(dup_errors.len(), 1);
        assert!(dup_errors[0].message.contains("active"));
    }

    #[test]
    fn test_duplicate_field_untyped() {
        // Untyped fields (default to string)
        let source = r#"
            BasicUser {
                name
                email
                name
            }
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let dup_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Duplicate field"))
            .collect();
        
        assert_eq!(dup_errors.len(), 1);
        assert!(dup_errors[0].message.contains("name"));
    }

    #[test]
    fn test_duplicate_field_mixed_typed_untyped() {
        // One typed, one untyped - still duplicate
        let source = r#"
            User {
                name: string
                name
            }
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let dup_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Duplicate field"))
            .collect();
        
        assert_eq!(dup_errors.len(), 1);
    }

    #[test]
    fn test_duplicate_field_array_types() {
        let source = r#"
            User {
                tags: Tag[]
                tags: string[]
            }
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let dup_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Duplicate field"))
            .collect();
        
        assert_eq!(dup_errors.len(), 1);
        assert!(dup_errors[0].message.contains("tags"));
    }

    #[test]
    fn test_duplicate_field_in_model_with_extends() {
        // Duplicates within the child model itself (not with parent)
        let source = r#"
            BaseUser {
                id: number
            }

            AdminUser extends BaseUser {
                level: number
                level: string
            }
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let dup_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Duplicate field"))
            .collect();
        
        assert_eq!(dup_errors.len(), 1);
        assert!(dup_errors[0].message.contains("level"));
    }

    #[test]
    fn test_no_duplicate_with_same_name_in_different_models() {
        // Same field name in different models is fine
        let source = r#"
            User {
                email: string
            }

            Company {
                email: string
            }
        "#;
        
        let result = validate(source, &[]);
        
        let dup_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Duplicate field"))
            .collect();
        
        assert!(dup_errors.is_empty());
    }

    #[test]
    fn test_field_removal_not_counted_as_definition() {
        // -fieldname is removal, not definition
        let source = r#"
            BaseUser {
                password_hash: string
            }

            PublicUser extends BaseUser {
                -password_hash
                display_name: string
            }
        "#;
        
        let result = validate(source, &[]);
        
        let dup_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Duplicate field"))
            .collect();
        
        assert!(dup_errors.is_empty());
    }

    #[test]
    fn test_duplicate_field_reports_correct_line() {
        let source = "User {\n    email: string\n    name: string\n    email: number\n}";
        
        let result = validate(source, &[]);
        
        let dup_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Duplicate field"))
            .collect();
        
        assert_eq!(dup_errors.len(), 1);
        // First definition is at line 2 (0-indexed: 1)
        assert!(dup_errors[0].message.contains("first defined at line 2"));
        // Error should point to line 4 (0-indexed: 3)
        assert_eq!(dup_errors[0].span.start.line, 3);
    }

    #[test]
    fn test_duplicate_field_with_inline_plugin_block() {
        // This is the CORRECT way to add plugins to a field
        // Two definitions with inline plugins is still a duplicate
        let source = r#"
            User {
                email: string {
                    @validation { format: "email" }
                }
                email: string
            }
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let dup_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Duplicate field"))
            .collect();
        
        assert_eq!(dup_errors.len(), 1);
        assert!(dup_errors[0].message.contains("email"));
    }

    #[test]
    fn test_duplicate_field_inline_union() {
        let source = r#"
            User {
                status: "active" | "pending"
                status: "active" | "deleted"
            }
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let dup_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Duplicate field"))
            .collect();
        
        assert_eq!(dup_errors.len(), 1);
        assert!(dup_errors[0].message.contains("status"));
    }

    #[test]
    fn test_composite_type_duplicate_fields() {
        // Composite types (value objects) should also check for duplicates
        let source = r#"
            Address {
                street: string
                city: string
                street: string
            }
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let dup_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Duplicate field"))
            .collect();
        
        assert_eq!(dup_errors.len(), 1);
        assert!(dup_errors[0].message.contains("street"));
    }

    #[test]
    fn test_complex_model_with_duplicates() {
        // Real-world-ish example with various field types
        let source = r#"
            Order {
                id: UUID
                customer: User
                items: OrderItem[]
                total: Money
                status: "pending" | "shipped" | "delivered"
                created_at: DateTime = now()
                items: Product[]
            }
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let dup_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Duplicate field"))
            .collect();
        
        assert_eq!(dup_errors.len(), 1);
        assert!(dup_errors[0].message.contains("items"));
    }

    #[test]
    fn test_empty_model_no_duplicates() {
        let source = r#"
            Empty {
            }
        "#;
        
        let result = validate(source, &[]);
        
        let dup_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Duplicate field"))
            .collect();
        
        assert!(dup_errors.is_empty());
    }

    #[test]
    fn test_single_field_model_no_duplicates() {
        let source = r#"
            Single {
                only_field: string
            }
        "#;
        
        let result = validate(source, &[]);
        
        let dup_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Duplicate field"))
            .collect();
        
        assert!(dup_errors.is_empty());
    }

    #[test]
    fn test_multiple_models_each_with_duplicates() {
        let source = r#"
            User {
                email: string
                email: number
            }

            Product {
                sku: string
                sku: number
            }
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let dup_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Duplicate field"))
            .collect();
        
        assert_eq!(dup_errors.len(), 2);
        
        let messages: Vec<&str> = dup_errors.iter()
            .map(|d| d.message.as_str())
            .collect();
        
        assert!(messages.iter().any(|m| m.contains("email")));
        assert!(messages.iter().any(|m| m.contains("sku")));
    }
}

// =============================================================================
// FIELD OVERRIDE VALIDATION TESTS
// =============================================================================

#[cfg(test)]
mod field_override_tests {
    use super::*;

    #[test]
    fn test_field_override_on_same_model_is_error() {
        // This is INVALID - can't use field_override syntax for same-model fields
        let source = r#"
            Post {
                content: string
                
                content {
                    @sql { type: "TEXT" }
                }
            }
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let override_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Cannot override field"))
            .collect();
        
        assert_eq!(override_errors.len(), 1);
        assert!(override_errors[0].message.contains("content"));
        assert!(override_errors[0].message.contains("same model"));
        assert!(override_errors[0].message.contains("inline plugin syntax"));
    }

    #[test]
    fn test_inline_plugin_syntax_is_valid() {
        // This is the CORRECT way - inline plugin block
        let source = r#"
            Post {
                content: string {
                    @sql { type: "TEXT" }
                }
            }
        "#;
        
        let result = validate(source, &[]);
        
        let override_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Cannot override field"))
            .collect();
        
        assert!(override_errors.is_empty());
    }

    #[test]
    fn test_field_override_on_inherited_field_is_valid() {
        // This IS valid - overriding inherited field
        let source = r#"
            BaseContent {
                status: string
            }

            Article extends BaseContent {
                title: string
                
                status {
                    @sql { type: "ENUM", name: "article_status" }
                }
            }
        "#;
        
        let result = validate(source, &[]);
        
        let override_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Cannot override field"))
            .collect();
        
        assert!(override_errors.is_empty());
    }

    #[test]
    fn test_multiple_field_overrides_on_same_model() {
        let source = r#"
            Post {
                title: string
                content: string
                
                title {
                    @sql { type: "VARCHAR(200)" }
                }
                
                content {
                    @sql { type: "TEXT" }
                }
            }
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let override_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Cannot override field"))
            .collect();
        
        assert_eq!(override_errors.len(), 2);
        
        let messages: Vec<&str> = override_errors.iter()
            .map(|d| d.message.as_str())
            .collect();
        
        assert!(messages.iter().any(|m| m.contains("title")));
        assert!(messages.iter().any(|m| m.contains("content")));
    }

    #[test]
    fn test_field_override_reports_definition_line() {
        let source = "Post {\n    content: string\n    \n    content {\n        @sql { type: \"TEXT\" }\n    }\n}";
        
        let result = validate(source, &[]);
        
        let override_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Cannot override field"))
            .collect();
        
        assert_eq!(override_errors.len(), 1);
        // Field is defined at line 2
        assert!(override_errors[0].message.contains("line 2"));
    }

    #[test]
    fn test_field_override_for_undefined_field_is_allowed() {
        // If the field isn't defined in this model, it's presumably inherited
        // (actual inheritance validation would catch if parent doesn't have it)
        let source = r#"
            BaseModel {
                created_at: string
            }

            ChildModel extends BaseModel {
                name: string
                
                created_at {
                    @sql { type: "TIMESTAMP" }
                }
            }
        "#;
        
        let result = validate(source, &[]);
        
        let override_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Cannot override field"))
            .collect();
        
        assert!(override_errors.is_empty());
    }

    #[test]
    fn test_field_override_order_definition_after_override() {
        // Even if field_override comes before field_definition, should still error
        let source = r#"
            Post {
                content {
                    @sql { type: "TEXT" }
                }
                
                content: string
            }
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let override_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Cannot override field"))
            .collect();
        
        assert_eq!(override_errors.len(), 1);
        assert!(override_errors[0].message.contains("content"));
    }

    #[test]
    fn test_mixed_valid_and_invalid_overrides() {
        let source = r#"
            Base {
                inherited_field: string
            }

            Child extends Base {
                local_field: string
                
                // This is valid - inherited field
                inherited_field {
                    @sql { index: true }
                }
                
                // This is invalid - local field
                local_field {
                    @sql { index: true }
                }
            }
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let override_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Cannot override field"))
            .collect();
        
        assert_eq!(override_errors.len(), 1);
        assert!(override_errors[0].message.contains("local_field"));
    }

    #[test]
    fn test_field_override_with_multiple_plugins() {
        let source = r#"
            Post {
                content: string
                
                content {
                    @sql { type: "TEXT" }
                    @validation { max_length: 50000 }
                }
            }
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let override_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Cannot override field"))
            .collect();
        
        assert_eq!(override_errors.len(), 1);
    }

    #[test]
    fn test_no_error_for_model_level_plugin_config() {
        // Model-level @sql is not a field override
        let source = r#"
            User {
                id: number
                name: string
                
                @sql { table: "users" }
            }
        "#;
        
        let result = validate(source, &[]);
        
        let override_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Cannot override field"))
            .collect();
        
        assert!(override_errors.is_empty());
    }

    #[test]
    fn test_combined_duplicate_and_override_errors() {
        // Both duplicate field AND invalid override
        let source = r#"
            Post {
                content: string
                content: string
                
                content {
                    @sql { type: "TEXT" }
                }
            }
        "#;
        
        let result = validate(source, &[]);
        assert!(result.has_errors());
        
        let dup_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Duplicate field"))
            .collect();
        
        let override_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Cannot override field"))
            .collect();
        
        assert_eq!(dup_errors.len(), 1);
        assert_eq!(override_errors.len(), 1);
    }
}

// =============================================================================
// ANCESTOR & EXTENDS TESTS
// Add this module to your existing tests.rs
// =============================================================================

mod extends_tests {
    use super::*;

    fn make_ancestor(path: &str, source: &str) -> Ancestor {
        let result = validate(source, &[]);
        result.into_ancestor(path.to_string())
    }

    // -------------------------------------------------------------------------
    // extract_extends_paths tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_extract_no_extends() {
        let source = r#"
            User { name: string }
        "#;
        
        let paths = extract_extends_paths(source);
        assert!(paths.is_empty());
    }

    #[test]
    fn test_extract_single_extends() {
        let source = r#"
            @extends ./base.cdm
            
            User { name: string }
        "#;
        
        let paths = extract_extends_paths(source);
        assert_eq!(paths, vec!["./base.cdm"]);
    }

    #[test]
    fn test_extract_multiple_extends() {
        let source = r#"
            @extends ./types.cdm
            @extends ./mixins.cdm
            @extends ../shared/base.cdm
            
            User { name: string }
        "#;
        
        let paths = extract_extends_paths(source);
        assert_eq!(paths, vec![
            "./types.cdm",
            "./mixins.cdm",
            "../shared/base.cdm"
        ]);
    }

    #[test]
    fn test_extract_extends_with_plugins() {
        let source = r#"
            @sql { dialect: "postgres" }
            @extends ./types/base.cdm
            @validation { strict: true }
            
            User { name: string }
        "#;
        
        let paths = extract_extends_paths(source);
        assert_eq!(paths, vec!["./types/base.cdm"]);
    }

    #[test]
    fn test_extract_extends_preserves_order() {
        let source = r#"
            @extends ./third.cdm
            @extends ./first.cdm
            @extends ./second.cdm
            
            User { name: string }
        "#;
        
        let paths = extract_extends_paths(source);
        assert_eq!(paths, vec!["./third.cdm", "./first.cdm", "./second.cdm"]);
    }

    // -------------------------------------------------------------------------
    // ValidationResult.into_ancestor tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_validation_result_into_ancestor() {
        let source = r#"
            Email: string
            User {
                name: string
                email: Email
            }
        "#;
        
        let result = validate(source, &[]);
        assert!(!result.has_errors());
        
        assert!(result.symbol_table.is_defined("Email"));
        assert!(result.symbol_table.is_defined("User"));
        assert!(result.model_fields.contains_key("User"));
        
        let ancestor = result.into_ancestor("test.cdm".to_string());
        assert_eq!(ancestor.path, "test.cdm");
        assert!(ancestor.symbol_table.is_defined("Email"));
        assert!(ancestor.model_fields.contains_key("User"));
    }

    #[test]
    fn test_field_info_collection() {
        let source = r#"
            User {
                name: string
                age?: number
                active: boolean = true
                bio
            }
        "#;
        
        let result = validate(source, &[]);
        let fields = result.model_fields.get("User").unwrap();
        
        assert_eq!(fields.len(), 4);
        
        let name_field = fields.iter().find(|f| f.name == "name").unwrap();
        assert_eq!(name_field.type_expr, Some("string".to_string()));
        assert!(!name_field.optional);
        
        let age_field = fields.iter().find(|f| f.name == "age").unwrap();
        assert_eq!(age_field.type_expr, Some("number".to_string()));
        assert!(age_field.optional);
        
        let bio_field = fields.iter().find(|f| f.name == "bio").unwrap();
        assert_eq!(bio_field.type_expr, None);
        assert!(!bio_field.optional);
    }

    // -------------------------------------------------------------------------
    // Shadowing ancestor definitions
    // -------------------------------------------------------------------------

    #[test]
    fn test_shadow_ancestor_definition_warning() {
        let base_source = r#"
            Email: string
            User { name: string }
        "#;
        let base_ancestor = make_ancestor("base.cdm", base_source);
        
        let child_source = "Email: number";
        let result = validate(child_source, &[base_ancestor]);
        
        let warnings: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.severity == Severity::Warning && d.message.contains("shadows definition"))
            .collect();
        
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("Email"));
        assert!(warnings[0].message.contains("base.cdm"));
    }

    // -------------------------------------------------------------------------
    // Cross-file type resolution
    // -------------------------------------------------------------------------

    #[test]
    fn test_type_from_ancestor() {
        let base_source = r#"
            Email: string
            Address { street: string }
        "#;
        let base_ancestor = make_ancestor("base.cdm", base_source);
        
        let child_source = r#"
            User {
                email: Email
                address: Address
            }
        "#;
        let result = validate(child_source, &[base_ancestor]);
        
        let undefined_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Undefined type"))
            .collect();
        
        assert!(undefined_errors.is_empty());
    }

    #[test]
    fn test_extend_model_from_ancestor() {
        let base_source = r#"
            BaseUser {
                id: number
                email: string
            }
        "#;
        let base_ancestor = make_ancestor("base.cdm", base_source);
        
        let child_source = r#"
            AdminUser extends BaseUser {
                admin_level: number
            }
        "#;
        let result = validate(child_source, &[base_ancestor]);
        
        assert!(!result.has_errors());
    }

    #[test]
    fn test_multiple_ancestors() {
        let types_source = r#"
            Email: string
            UUID: string
        "#;
        let types_ancestor = make_ancestor("types.cdm", types_source);
        
        let base_source = r#"
            BaseEntity {
                id: UUID
                created_at: string
            }
        "#;
        let base_ancestor = {
            let result = validate(base_source, &[types_ancestor.clone()]);
            result.into_ancestor("base.cdm".to_string())
        };
        
        let child_source = r#"
            User extends BaseEntity {
                email: Email
            }
        "#;
        let result = validate(child_source, &[base_ancestor, types_ancestor]);
        
        assert!(!result.has_errors());
    }

    // -------------------------------------------------------------------------
    // Field removal with ancestors
    // -------------------------------------------------------------------------

    #[test]
    fn test_valid_field_removal() {
        let base_source = r#"
            BaseUser {
                id: number
                password_hash: string
                email: string
            }
        "#;
        let base_ancestor = make_ancestor("base.cdm", base_source);
        
        let child_source = r#"
            PublicUser extends BaseUser {
                -password_hash
                display_name: string
            }
        "#;
        let result = validate(child_source, &[base_ancestor]);
        
        let removal_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Cannot remove field"))
            .collect();
        
        assert!(removal_errors.is_empty());
    }

    #[test]
    fn test_invalid_field_removal_not_in_parent() {
        let base_source = r#"
            BaseUser {
                id: number
                email: string
            }
        "#;
        let base_ancestor = make_ancestor("base.cdm", base_source);
        
        let child_source = r#"
            PublicUser extends BaseUser {
                -password_hash
            }
        "#;
        let result = validate(child_source, &[base_ancestor]);
        
        assert!(result.has_errors());
        assert!(result.diagnostics.iter().any(|d| 
            d.message.contains("Cannot remove field 'password_hash'")
        ));
    }

    #[test]
    fn test_field_removal_from_grandparent() {
        let grandparent_source = r#"
            Entity {
                id: number
                internal_flags: string
            }
        "#;
        let grandparent_ancestor = make_ancestor("entity.cdm", grandparent_source);
        
        let parent_source = r#"
            BaseUser extends Entity {
                email: string
            }
        "#;
        let parent_ancestor = {
            let result = validate(parent_source, &[grandparent_ancestor.clone()]);
            result.into_ancestor("base.cdm".to_string())
        };
        
        let child_source = r#"
            PublicUser extends BaseUser {
                -internal_flags
            }
        "#;
        let result = validate(child_source, &[parent_ancestor, grandparent_ancestor]);
        
        let removal_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Cannot remove field"))
            .collect();
        
        assert!(removal_errors.is_empty());
    }

    // -------------------------------------------------------------------------
    // Field override with ancestors
    // -------------------------------------------------------------------------

    #[test]
    fn test_valid_field_override_from_parent() {
        let base_source = r#"
            BaseContent {
                status: string
            }
        "#;
        let base_ancestor = make_ancestor("base.cdm", base_source);
        
        let child_source = r#"
            Article extends BaseContent {
                title: string
                
                status {
                    @sql { type: "article_status_enum" }
                }
            }
        "#;
        let result = validate(child_source, &[base_ancestor]);
        
        let override_errors: Vec<_> = result.diagnostics.iter()
            .filter(|d| d.message.contains("Cannot override field"))
            .collect();
        
        assert!(override_errors.is_empty());
    }

    #[test]
    fn test_invalid_field_override_not_in_parent() {
        let base_source = r#"
            BaseContent {
                title: string
            }
        "#;
        let base_ancestor = make_ancestor("base.cdm", base_source);
        
        let child_source = r#"
            Article extends BaseContent {
                body: string
                
                nonexistent {
                    @sql { type: "TEXT" }
                }
            }
        "#;
        let result = validate(child_source, &[base_ancestor]);
        
        assert!(result.has_errors());
        assert!(result.diagnostics.iter().any(|d| 
            d.message.contains("Cannot override field 'nonexistent'") &&
            d.message.contains("not found in any parent")
        ));
    }

    // -------------------------------------------------------------------------
    // Complex multi-file scenarios
    // -------------------------------------------------------------------------

    #[test]
    fn test_real_world_extends_chain() {
        let types_source = r#"
            UUID: string
            Email: string
            DateTime: string
        "#;
        let types_ancestor = make_ancestor("types.cdm", types_source);
        
        let base_source = r#"
            Timestamped {
                created_at: DateTime
                updated_at: DateTime
            }
            
            BaseEntity extends Timestamped {
                id: UUID
            }
        "#;
        let base_ancestor = {
            let result = validate(base_source, &[types_ancestor.clone()]);
            assert!(!result.has_errors(), "base.cdm should be valid");
            result.into_ancestor("base.cdm".to_string())
        };
        
        let user_source = r#"
            User extends BaseEntity {
                -updated_at
                
                email: Email
                username: string
                
                created_at {
                    @sql { default: "NOW()" }
                }
            }
        "#;
        let result = validate(user_source, &[base_ancestor, types_ancestor]);
        
        assert!(!result.has_errors(), "Errors: {:?}", result.diagnostics);
    }

    #[test]
    fn test_multiple_extends_directives() {
        let types_source = r#"
            UUID: string
            Email: string
        "#;
        let types_ancestor = make_ancestor("types.cdm", types_source);
        
        let mixins_source = r#"
            Timestamped {
                created_at: string
                updated_at: string
            }
        "#;
        let mixins_ancestor = make_ancestor("mixins.cdm", mixins_source);
        
        let user_source = r#"
            @extends ./types.cdm
            @extends ./mixins.cdm
            
            User extends Timestamped {
                id: UUID
                email: Email
            }
        "#;
        
        let result = validate(user_source, &[types_ancestor, mixins_ancestor]);
        
        assert!(!result.has_errors(), "Errors: {:?}", result.diagnostics);
        
        let paths = extract_extends_paths(user_source);
        assert_eq!(paths, vec!["./types.cdm", "./mixins.cdm"]);
    }
}

mod default_value_type_checking {
    use super::*;

    // -------------------------------------------------------------------------
    // Primitive Type Defaults
    // -------------------------------------------------------------------------

    #[test]
    fn boolean_field_with_string_default_is_error() {
        let source = r#"
            User {
                active: boolean = "yes"
            }
        "#;
        let result = validate_source(source);
        assert!(has_error_containing(&result, "Type mismatch"));
        assert!(has_error_containing(&result, "boolean"));
        assert!(has_error_containing(&result, "string"));
    }

    #[test]
    fn boolean_field_with_number_default_is_error() {
        let source = r#"
            User {
                active: boolean = 1
            }
        "#;
        let result = validate_source(source);
        assert!(has_error_containing(&result, "Type mismatch"));
        assert!(has_error_containing(&result, "boolean"));
        assert!(has_error_containing(&result, "number"));
    }

    #[test]
    fn boolean_field_with_boolean_default_is_valid() {
        let source = r#"
            User {
                active: boolean = true
                inactive: boolean = false
            }
        "#;
        let result = validate_source(source);
        let errors = get_errors(&result);
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    }

    #[test]
    fn number_field_with_string_default_is_error() {
        let source = r#"
            User {
                age: number = "twenty"
            }
        "#;
        let result = validate_source(source);
        assert!(has_error_containing(&result, "Type mismatch"));
        assert!(has_error_containing(&result, "number"));
        assert!(has_error_containing(&result, "string"));
    }

    #[test]
    fn number_field_with_boolean_default_is_error() {
        let source = r#"
            User {
                count: number = true
            }
        "#;
        let result = validate_source(source);
        assert!(has_error_containing(&result, "Type mismatch"));
        assert!(has_error_containing(&result, "number"));
        assert!(has_error_containing(&result, "boolean"));
    }

    #[test]
    fn number_field_with_number_default_is_valid() {
        let source = r#"
            User {
                age: number = 25
                score: number = 99.5
                balance: number = -100
                small: number = 0
            }
        "#;
        let result = validate_source(source);
        let errors = get_errors(&result);
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    }

    #[test]
    fn string_field_with_number_default_is_error() {
        let source = r#"
            User {
                name: string = 42
            }
        "#;
        let result = validate_source(source);
        assert!(has_error_containing(&result, "Type mismatch"));
        assert!(has_error_containing(&result, "string"));
        assert!(has_error_containing(&result, "number"));
    }

    #[test]
    fn string_field_with_boolean_default_is_error() {
        let source = r#"
            User {
                name: string = false
            }
        "#;
        let result = validate_source(source);
        assert!(has_error_containing(&result, "Type mismatch"));
        assert!(has_error_containing(&result, "string"));
        assert!(has_error_containing(&result, "boolean"));
    }

    #[test]
    fn string_field_with_string_default_is_valid() {
        let source = r#"
            User {
                name: string = "John"
                empty: string = ""
            }
        "#;
        let result = validate_source(source);
        let errors = get_errors(&result);
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    }

    // -------------------------------------------------------------------------
    // Type Alias Resolution
    // -------------------------------------------------------------------------

    #[test]
    fn type_alias_to_string_with_number_default_is_error() {
        let source = r#"
            Email: string

            User {
                email: Email = 42
            }
        "#;
        let result = validate_source(source);
        assert!(has_error_containing(&result, "Type mismatch"));
        assert!(has_error_containing(&result, "string"));
    }

    #[test]
    fn type_alias_to_boolean_with_string_default_is_error() {
        let source = r#"
            Active: boolean

            User {
                active: Active = "yes"
            }
        "#;
        let result = validate_source(source);
        assert!(has_error_containing(&result, "Type mismatch"));
        assert!(has_error_containing(&result, "boolean"));
    }

    #[test]
    fn type_alias_chain_resolved_correctly() {
        let source = r#"
            BaseCount: number
            Count: BaseCount

            Stats {
                total: Count = "wrong"
            }
        "#;
        let result = validate_source(source);
        assert!(has_error_containing(&result, "Type mismatch"));
        assert!(has_error_containing(&result, "number"));
    }

    #[test]
    fn type_alias_to_primitive_with_correct_default_is_valid() {
        let source = r#"
            Email: string
            Age: number
            Active: boolean

            User {
                email: Email = "test@example.com"
                age: Age = 25
                active: Active = true
            }
        "#;
        let result = validate_source(source);
        let errors = get_errors(&result);
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    }

    // -------------------------------------------------------------------------
    // String Union Types
    // -------------------------------------------------------------------------

    #[test]
    fn string_union_with_number_default_is_error() {
        let source = r#"
            User {
                status: "active" | "pending" | "deleted" = 1
            }
        "#;
        let result = validate_source(source);
        assert!(has_error_containing(&result, "Type mismatch"));
        assert!(has_error_containing(&result, "number"));
    }

    #[test]
    fn string_union_with_invalid_string_default_is_error() {
        let source = r#"
            User {
                status: "active" | "pending" | "deleted" = "unknown"
            }
        "#;
        let result = validate_source(source);
        assert!(has_error_containing(&result, "Invalid default value"));
        assert!(has_error_containing(&result, "unknown"));
    }

    #[test]
    fn string_union_with_valid_string_default_is_valid() {
        let source = r#"
            User {
                status: "active" | "pending" | "deleted" = "active"
            }
        "#;
        let result = validate_source(source);
        let errors = get_errors(&result);
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    }

    #[test]
    fn string_union_type_alias_with_invalid_default_is_error() {
        let source = r#"
            Status: "active" | "pending" | "deleted"

            User {
                status: Status = "unknown"
            }
        "#;
        let result = validate_source(source);
        assert!(has_error_containing(&result, "Invalid default value"));
    }

    #[test]
    fn string_union_type_alias_with_valid_default_is_valid() {
        let source = r#"
            Status: "active" | "pending" | "deleted"

            User {
                status: Status = "pending"
            }
        "#;
        let result = validate_source(source);
        let errors = get_errors(&result);
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    }

    // -------------------------------------------------------------------------
    // Array Types
    // -------------------------------------------------------------------------

    #[test]
    fn string_array_with_number_array_default_is_error() {
        let source = r#"
            User {
                tags: string[] = [1, 2, 3]
            }
        "#;
        let result = validate_source(source);
        assert!(has_error_containing(&result, "Type mismatch in array"));
    }

    #[test]
    fn number_array_with_string_elements_is_error() {
        let source = r#"
            User {
                scores: number[] = ["a", "b"]
            }
        "#;
        let result = validate_source(source);
        assert!(has_error_containing(&result, "Type mismatch in array"));
    }

    #[test]
    fn array_field_with_non_array_default_is_error() {
        let source = r#"
            User {
                tags: string[] = "not-an-array"
            }
        "#;
        let result = validate_source(source);
        assert!(has_error_containing(&result, "Type mismatch"));
        assert!(has_error_containing(&result, "expected array"));
    }

    #[test]
    fn string_array_with_empty_array_default_is_valid() {
        let source = r#"
            User {
                tags: string[] = []
            }
        "#;
        let result = validate_source(source);
        let errors = get_errors(&result);
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    }

    #[test]
    fn string_array_with_string_elements_is_valid() {
        let source = r#"
            User {
                tags: string[] = ["tag1", "tag2", "tag3"]
            }
        "#;
        let result = validate_source(source);
        let errors = get_errors(&result);
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    }

    #[test]
    fn number_array_with_number_elements_is_valid() {
        let source = r#"
            User {
                scores: number[] = [100, 95.5, 87]
            }
        "#;
        let result = validate_source(source);
        let errors = get_errors(&result);
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    }

    // -------------------------------------------------------------------------
    // Model/Composite Type Defaults
    // -------------------------------------------------------------------------

    #[test]
    fn model_type_with_primitive_default_is_error() {
        let source = r#"
            Address {
                street: string
                city: string
            }

            User {
                address: Address = "123 Main St"
            }
        "#;
        let result = validate_source(source);
        assert!(has_error_containing(&result, "Type mismatch"));
        assert!(has_error_containing(&result, "object"));
    }

    #[test]
    fn model_type_with_object_default_is_valid() {
        let source = r#"
            Address {
                street: string
                city: string
            }

            User {
                address: Address = { street: "123 Main St", city: "Springfield" }
            }
        "#;
        let result = validate_source(source);
        let errors = get_errors(&result);
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    }

    // -------------------------------------------------------------------------
    // Optional Fields with Defaults
    // -------------------------------------------------------------------------

    #[test]
    fn optional_field_with_wrong_default_type_is_error() {
        let source = r#"
            User {
                age?: number = "twenty"
            }
        "#;
        let result = validate_source(source);
        assert!(has_error_containing(&result, "Type mismatch"));
    }

    #[test]
    fn optional_field_with_correct_default_type_is_valid() {
        let source = r#"
            User {
                age?: number = 0
                name?: string = "Anonymous"
                active?: boolean = false
            }
        "#;
        let result = validate_source(source);
        let errors = get_errors(&result);
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    }

    // -------------------------------------------------------------------------
    // Special Types (DateTime, JSON, etc.)
    // -------------------------------------------------------------------------

    #[test]
    fn json_type_allows_object_default() {
        let source = r#"
            User {
                metadata: JSON = { key: "value" }
            }
        "#;
        let result = validate_source(source);
        let errors = get_errors(&result);
        // JSON should accept any default
        assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    }


    // -------------------------------------------------------------------------
    // Multiple Errors in Same Model
    // -------------------------------------------------------------------------

    #[test]
    fn multiple_type_mismatches_all_reported() {
        let source = r#"
            User {
                active: boolean = "yes"
                count: number = "five"
                name: string = 42
            }
        "#;
        let result = validate_source(source);
        let errors = get_errors(&result);
        assert_eq!(errors.len(), 3, "Expected 3 errors, got {:?}", errors);
    }
}

#[cfg(test)]
mod plugin_import_tests {
    use super::*;

    // =========================================================================
    // Basic Plugin Imports
    // =========================================================================

    #[test]
    fn test_simple_plugin_import_no_source_or_config() {
        let source = "@sql";
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_plugin_import_with_config_only() {
        let source = r#"@sql { dialect: "postgres" }"#;
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_plugin_import_with_complex_config() {
        let source = r#"
            @sql {
                dialect: "postgres",
                schema: "public",
                migrations: true
            }
        "#;
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    // =========================================================================
    // Git Source Plugin Imports
    // =========================================================================

    #[test]
    fn test_plugin_import_with_git_source_no_config() {
        let source = "@analytics from git:https://github.com/myorg/cdm-analytics.git";
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_plugin_import_with_git_source_and_config() {
        let source = r#"
            @analytics from git:https://github.com/myorg/cdm-analytics.git {
                endpoint: "https://analytics.example.com",
                api_key: "secret123"
            }
        "#;
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_plugin_import_with_git_ssh_url() {
        let source = "@myPlugin from git:git@github.com:myorg/my-plugin.git";
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_plugin_import_with_git_tag() {
        let source = "@sql from git:https://github.com/cdm-lang/cdm-plugin-sql.git#v1.2.3";
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_plugin_import_with_git_branch() {
        let source = "@sql from git:https://github.com/cdm-lang/cdm-plugin-sql.git#feature/new-stuff";
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    // =========================================================================
    // Local Path Plugin Imports
    // =========================================================================

    #[test]
    fn test_plugin_import_with_local_path_current_dir() {
        let source = "@custom from ./plugins/my-plugin";
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_plugin_import_with_local_path_parent_dir() {
        let source = "@shared from ../shared-plugins/common";
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_plugin_import_with_local_path_and_config() {
        let source = r#"
            @custom from ./plugins/my-plugin {
                debug: true,
                output_dir: "./generated"
            }
        "#;
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    // =========================================================================
    // Multiple Plugin Imports
    // =========================================================================

    #[test]
    fn test_multiple_plugin_imports() {
        let source = r#"
            @sql { dialect: "postgres" }
            @typescript { strict: true }
            @validation
        "#;
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_plugin_imports_with_mixed_sources() {
        let source = r#"
            @sql { dialect: "postgres" }
            @analytics from git:https://github.com/myorg/analytics.git
            @custom from ./plugins/local
        "#;
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    // =========================================================================
    // Plugin Imports with Definitions (Ordering)
    // =========================================================================

    #[test]
    fn test_plugin_imports_before_type_alias() {
        let source = r#"
            @sql { dialect: "postgres" }

            Email: string
        "#;
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        let (symbol_table, _) = collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

        assert!(diagnostics.is_empty());
        assert_eq!(symbol_table.definitions.len(), 1);

        let def = symbol_table.get("Email").expect("Email should be defined");
        assert!(matches!(def.kind, DefinitionKind::TypeAlias { .. }));
    }

    #[test]
    fn test_plugin_imports_before_model_definition() {
        let source = r#"
            @sql { dialect: "postgres" }
            @validation

            User {
                name: string
                email: string
            }
        "#;
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        let (symbol_table, _) = collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

        assert!(diagnostics.is_empty());
        assert_eq!(symbol_table.definitions.len(), 1);

        let def = symbol_table.get("User").expect("User should be defined");
        assert!(matches!(&def.kind, DefinitionKind::Model { .. }));
    }

    // =========================================================================
    // Config Variations
    // =========================================================================

    #[test]
    fn test_plugin_import_with_nested_object_config() {
        let source = r#"
            @sql {
                connection: {
                    host: "localhost",
                    port: 5432,
                    ssl: true
                },
                pool_size: 10
            }
        "#;
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_plugin_import_with_array_config() {
        let source = r#"
            @api {
                expose: ["id", "name", "email"],
                methods: ["GET", "POST"]
            }
        "#;
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_plugin_import_with_empty_config() {
        let source = "@sql {}";
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_plugin_import_with_trailing_comma() {
        let source = r#"
            @sql {
                dialect: "postgres",
                schema: "public",
            }
        "#;
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_plugin_import_with_string_keys() {
        let source = r#"
            @sql {
                "table-name": "users",
                "primary-key": "id"
            }
        "#;
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_plugin_import_with_numeric_config_values() {
        let source = r#"
            @sql {
                pool_size: 10,
                timeout: 30.5,
                retries: -1
            }
        "#;
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[test]
    fn test_plugin_import_with_comment() {
        let source = r#"
            // Database configuration
            @sql { dialect: "postgres" }
        "#;
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_plugin_import_with_underscore_in_name() {
        let source = "@my_custom_plugin from ./plugins/custom";
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_plugin_import_with_numbers_in_name() {
        let source = "@sql2 { version: 2 }";
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_plugin_import_deeply_nested_path() {
        let source = "@custom from ./a/b/c/d/e/plugin";
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

        assert!(diagnostics.is_empty());
    }

    // =========================================================================
    // Full Integration Test
    // =========================================================================

    #[test]
    fn test_full_example_with_imports_and_definitions() {
        let source = r#"
            // Plugin imports
            @sql from git:https://github.com/cdm-lang/cdm-plugin-sql.git {
                dialect: "postgres",
                schema: "app"
            }
            @typescript { strict: true }
            @validation from ./plugins/validation

            // Type definitions
            Email: string {
                @validation { format: "email" }
            }

            User {
                id: UUID
                email: Email
            }
        "#;
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        let (symbol_table, _) = collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

        assert!(diagnostics.is_empty());
        assert_eq!(symbol_table.definitions.len(), 2);

        let email_def = symbol_table.get("Email").expect("Email should be defined");
        assert!(matches!(email_def.kind, DefinitionKind::TypeAlias { .. }));

        let user_def = symbol_table.get("User").expect("User should be defined");
        assert!(matches!(user_def.kind, DefinitionKind::Model { .. }));
    }

    // =========================================================================
    // Error Cases
    // =========================================================================

    #[test]
    fn test_definition_before_import_produces_error() {
        let source = r#"
            Email: string
            @sql { dialect: "postgres" }
        "#;
        let result = validate_source(source);

        assert!(
            result.has_errors(),
            "Expected error when import comes after definition"
        );
    }

    #[test]
    fn test_model_before_import_produces_error() {
        let source = r#"
            User { name: string }
            @sql { dialect: "postgres" }
        "#;
        let result = validate_source(source);

        assert!(
            result.has_errors(),
            "Expected error when import comes after model"
        );
    }
}

#[test]
fn test_array_with_valid_string_union_elements() {
    let source = r#"
        Status: "active" | "pending" | "deleted"

        User {
            statuses: Status[] = ["active", "pending"]
        }
    "#;
    let tree = parse(source);
    let mut diagnostics = Vec::new();

    let (symbol_table, _) = collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

    assert!(diagnostics.is_empty());
}

#[test]
fn test_array_with_invalid_string_union_element() {
    let source = r#"
        Status: "active" | "pending" | "deleted"

        User {
            statuses: Status[] = ["active", "invalid", "pending"]
        }
    "#;
    let result = validate_source(source);

    assert!(result.has_errors());
    assert!(has_error_containing(&result, "Invalid array element \"invalid\""));
    assert!(has_error_containing(&result, "expected one of"));
}

#[test]
fn test_array_with_multiple_invalid_string_union_elements() {
    let source = r#"
        Priority: "low" | "medium" | "high"

        Task {
            priorities: Priority[] = ["low", "unknown", "high", "critical"]
        }
    "#;
    let result = validate_source(source);

    let errors = get_errors(&result);
    assert_eq!(errors.len(), 2);
    assert!(has_error_containing(&result, "\"unknown\""));
    assert!(has_error_containing(&result, "\"critical\""));
}

#[test]
fn test_array_with_all_invalid_string_union_elements() {
    let source = r#"
        Color: "red" | "green" | "blue"

        Palette {
            colors: Color[] = ["yellow", "purple"]
        }
    "#;
    let result = validate_source(source);

    let errors = get_errors(&result);
    assert_eq!(errors.len(), 2);
    assert!(has_error_containing(&result, "\"yellow\""));
    assert!(has_error_containing(&result, "\"purple\""));
}

#[test]
fn test_string_union_array_with_invalid_element_via_alias() {
    let source = r#"
        Mode: "debug" | "release"

        Config {
            modes: Mode[] = ["debug", "profile"]
        }
    "#;
    let result = validate_source(source);

    assert!(result.has_errors());
    assert!(has_error_containing(&result, "Invalid array element \"profile\""));
}

#[test]
fn test_empty_array_for_string_union_type() {
    let source = r#"
        Status: "active" | "pending"

        User {
            statuses: Status[] = []
        }
    "#;
    let tree = parse(source);
    let mut diagnostics = Vec::new();

    let (symbol_table, _) = collect_definitions(tree.root_node(), source, &[], &mut diagnostics);

    assert!(diagnostics.is_empty());
}