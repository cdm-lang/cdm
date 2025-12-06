use super::*;

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

        let symbol_table = collect_definitions(tree.root_node(), source, &mut diagnostics);

        assert!(diagnostics.is_empty());
        assert!(symbol_table.definitions.is_empty());
    }

    #[test]
    fn test_single_type_alias() {
        let source = "Email: string";
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        let symbol_table = collect_definitions(tree.root_node(), source, &mut diagnostics);

        assert!(diagnostics.is_empty());
        assert_eq!(symbol_table.definitions.len(), 1);
        
        let def = symbol_table.get("Email").expect("Email should be defined");
        assert!(matches!(def.kind, DefinitionKind::TypeAlias));
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

        let symbol_table = collect_definitions(tree.root_node(), source, &mut diagnostics);

        assert!(diagnostics.is_empty());
        assert_eq!(symbol_table.definitions.len(), 1);

        let def = symbol_table.get("User").expect("User should be defined");
        assert!(matches!(&def.kind, DefinitionKind::Model { extends } if extends.is_empty()));
    }

    #[test]
    fn test_model_with_single_extends() {
        let source = r#"
            Timestamped {
                created_at: DateTime
            }

            Article extends Timestamped {
                title: string
            }
        "#;
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        let symbol_table = collect_definitions(tree.root_node(), source, &mut diagnostics);

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
                id: UUID
            }

            Timestamped {
                created_at: DateTime
            }

            AdminUser extends BaseUser, Timestamped {
                admin_level: number
            }
        "#;
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        let symbol_table = collect_definitions(tree.root_node(), source, &mut diagnostics);

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

        let symbol_table = collect_definitions(tree.root_node(), source, &mut diagnostics);

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

        let symbol_table = collect_definitions(tree.root_node(), source, &mut diagnostics);

        assert!(diagnostics.is_empty());
        assert_eq!(symbol_table.definitions.len(), 1);
        
        let def = symbol_table.get("Status").expect("Status should be defined");
        assert!(matches!(def.kind, DefinitionKind::TypeAlias));
    }

    #[test]
    fn test_duplicate_type_alias_error() {
        let source = r#"
            Email: string
            Email: number
        "#;
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        let symbol_table = collect_definitions(tree.root_node(), source, &mut diagnostics);

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

        collect_definitions(tree.root_node(), source, &mut diagnostics);

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

        collect_definitions(tree.root_node(), source, &mut diagnostics);

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

        let symbol_table = collect_definitions(tree.root_node(), source, &mut diagnostics);

        assert!(diagnostics.is_empty());
        assert_eq!(symbol_table.definitions.len(), 5);

        assert!(matches!(
            symbol_table.get("Email").unwrap().kind,
            DefinitionKind::TypeAlias
        ));
        assert!(matches!(
            symbol_table.get("Status").unwrap().kind,
            DefinitionKind::TypeAlias
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

        let symbol_table = collect_definitions(tree.root_node(), source, &mut diagnostics);

        // Built-ins should not be in definitions
        assert!(symbol_table.definitions.get("string").is_none());
        assert!(symbol_table.definitions.get("number").is_none());

        // But is_defined should return true for them
        assert!(symbol_table.is_defined("string"));
        assert!(symbol_table.is_defined("number"));
        assert!(symbol_table.is_defined("boolean"));
        assert!(symbol_table.is_defined("DateTime"));
    }

    #[test]
    fn test_span_tracking() {
        let source = "Email: string";
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        let symbol_table = collect_definitions(tree.root_node(), source, &mut diagnostics);

        let def = symbol_table.get("Email").expect("Email should be defined");
        assert_eq!(def.span.start.line, 0);
        assert_eq!(def.span.start.column, 0);
    }

    #[test]
    fn test_type_alias_with_plugin_block() {
        let source = r#"
            UUID: string {
                @validation { format: "uuid" }
                @sql { type: "UUID" }
            }
        "#;
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        let symbol_table = collect_definitions(tree.root_node(), source, &mut diagnostics);

        assert!(diagnostics.is_empty());
        assert_eq!(symbol_table.definitions.len(), 1);
        assert!(symbol_table.get("UUID").is_some());
    }

    #[test]
    fn test_model_with_complex_fields() {
        let source = r#"
            User {
                id: UUID
                tags: Tag[]
                status?: Status
                active: boolean = true
            }
        "#;
        let tree = parse(source);
        let mut diagnostics = Vec::new();

        let symbol_table = collect_definitions(tree.root_node(), source, &mut diagnostics);

        assert!(diagnostics.is_empty());
        assert_eq!(symbol_table.definitions.len(), 1);
        assert!(symbol_table.get("User").is_some());
    }

    #[cfg(test)]
mod validate_tests {
    use super::*;

    // =========================================================================
    // VALID FILES - NO ERRORS EXPECTED
    // =========================================================================

    #[test]
    fn test_empty_file() {
        let result = validate("");
        assert!(!result.has_errors());
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn test_comments_only() {
        let source = r#"
            // This is a comment
            // Another comment
        "#;
        let result = validate(source);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_simple_type_alias() {
        let source = "Email: string";
        let result = validate(source);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_type_alias_with_builtin_types() {
        let source = r#"
            Name: string
            Age: number
            Active: boolean
            Price: decimal
            CreatedAt: DateTime
            Metadata: JSON
            Id: UUID
        "#;
        let result = validate(source);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_union_type_alias_string_literals() {
        let source = r#"Status: "active" | "pending" | "deleted""#;
        let result = validate(source);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_union_type_alias_mixed() {
        let source = r#"
            Email: string
            Result: Email | "not_found" | "error"
        "#;
        let result = validate(source);
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
        let result = validate(source);
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
        let result = validate(source);
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
        let result = validate(source);
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
        let result = validate(source);
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
        let result = validate(source);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_model_with_single_extends() {
        let source = r#"
            Timestamped {
                created_at: DateTime
                updated_at: DateTime
            }

            Article extends Timestamped {
                title: string
                content: string
            }
        "#;
        let result = validate(source);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_model_with_multiple_extends() {
        let source = r#"
            Timestamped {
                created_at: DateTime
            }

            Identifiable {
                id: UUID
            }

            User extends Identifiable, Timestamped {
                name: string
            }
        "#;
        let result = validate(source);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_model_with_field_removal() {
        let source = r#"
            BaseUser {
                id: UUID
                name: string
                password_hash: string
            }

            PublicUser extends BaseUser {
                -password_hash
            }
        "#;
        let result = validate(source);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_model_with_plugin_config() {
        let source = r#"
            User {
                id: UUID
                email: string

                @sql { table: "users" }
            }
        "#;
        let result = validate(source);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_type_alias_with_plugin_config() {
        let source = r#"
            UUID: string {
                @validation { format: "uuid" }
                @sql { type: "UUID" }
            }
        "#;
        let result = validate(source);
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
        let result = validate(source);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_forward_reference_in_extends() {
        let source = r#"
            Article extends Timestamped {
                title: string
            }

            Timestamped {
                created_at: DateTime
            }
        "#;
        let result = validate(source);
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
        let result = validate(source);
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
                id: UUID
                status: Status
                contact: ContactInfo
            }
        "#;
        let result = validate(source);
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
        let result = validate(source);
        assert!(!result.has_errors());
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
        let result = validate(source);
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
        let result = validate(source);
        assert!(result.has_errors());
    }

    #[test]
    fn test_syntax_error_missing_colon_in_field() {
        let source = r#"
            User {
                name string
            }
        "#;
        let result = validate(source);
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
        let result = validate(source);
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
        let result = validate(source);
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
        let result = validate(source);
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
        let result = validate(source);
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
        let result = validate(source);
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
        let result = validate(source);
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
        let result = validate(source);
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
        let result = validate(source);
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
        let result = validate(source);
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
        let result = validate(source);
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
        let result = validate(source);
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
        let result = validate(source);
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
        let result = validate(source);
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
        let result = validate(source);
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
        let result = validate(source);
        
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
        let result = validate(source);
        assert!(result.tree.is_some());
    }

    #[test]
    fn test_tree_returned_even_with_semantic_errors() {
        let source = "User { email: Emaill }";
        let result = validate(source);
        assert!(result.has_errors());
        assert!(result.tree.is_some());
    }

    // =========================================================================
    // EDGE CASES
    // =========================================================================

    #[test]
    fn test_whitespace_variations() {
        let source = "Email:string";
        let result = validate(source);
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
        let result = validate(source);
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
        let result = validate(source);
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
        let result = validate(source);
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
        let result = validate(source);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_inline_union_in_field() {
        let source = r#"
            User {
                status: "active" | "pending" | "deleted"
            }
        "#;
        let result = validate(source);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_inline_union_with_default() {
        let source = r#"
            User {
                status: "active" | "pending" | "deleted" = "active"
            }
        "#;
        let result = validate(source);
        assert!(!result.has_errors());
    }
}

#[test]
fn test_parse_returns_tree_even_for_garbage() {
    // Tree-sitter is resilient - it returns a tree even for invalid input
    // The "Failed to parse file" branch is defensive, for cases like
    // allocation failure or parser cancellation
    let source = "!@#$%^&*()_+{}|:<>?~`";
    let result = validate(source);
    
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

    let symbol_table = collect_definitions(tree.root_node(), source, &mut diagnostics);

    let display = format!("{}", symbol_table);

    // Check header
    assert!(display.contains("Symbol Table"));
    assert!(display.contains("4 definitions"));

    // Check type aliases
    assert!(display.contains("Email (type alias)"));
    assert!(display.contains("Status (type alias)"));

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

    let symbol_table = collect_definitions(tree.root_node(), source, &mut diagnostics);

    let display = format!("{}", symbol_table);

    assert!(display.contains("Child (model extends Base1, Base2)"));
}