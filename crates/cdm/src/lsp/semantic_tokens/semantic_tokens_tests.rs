use super::*;

#[test]
fn test_semantic_tokens_simple_model() {
    let text = r#"User {
  name: string #1
} #10
"#;

    let tokens = compute_semantic_tokens(text);
    assert!(tokens.is_some());

    let tokens = tokens.unwrap();
    assert!(tokens.len() > 0, "Should generate semantic tokens");
}

#[test]
fn test_semantic_tokens_type_alias() {
    let text = r#"Email: string #1"#;

    let tokens = compute_semantic_tokens(text);
    assert!(tokens.is_some());

    let tokens = tokens.unwrap();
    assert!(tokens.len() > 0, "Should generate semantic tokens");
}

#[test]
fn test_semantic_tokens_union_type() {
    let text = r#"Status: "active" | "inactive" #1"#;

    let tokens = compute_semantic_tokens(text);
    assert!(tokens.is_some());

    let tokens = tokens.unwrap();
    assert!(tokens.len() > 0, "Should generate semantic tokens");

    // String literals in unions should be ENUM_MEMBER type
    let string_tokens: Vec<_> = tokens
        .iter()
        .filter(|t| t.token_type == TOKEN_ENUM_MEMBER)
        .collect();

    assert!(string_tokens.len() >= 2, "Should have string literal tokens in union");
}

#[test]
fn test_semantic_tokens_plugin_import() {
    let text = r#"@sql {
  dialect: "postgres"
}"#;

    let tokens = compute_semantic_tokens(text);
    assert!(tokens.is_some());

    let tokens = tokens.unwrap();
    assert!(tokens.len() > 0, "Should generate semantic tokens");
}

#[test]
fn test_semantic_tokens_with_comments() {
    let text = r#"// This is a comment
User {
  name: string #1 // Field comment
} #10"#;

    let tokens = compute_semantic_tokens(text);
    assert!(tokens.is_some());

    let tokens = tokens.unwrap();
    assert!(tokens.len() > 0, "Should generate semantic tokens");

    // Should have comment tokens
    let comment_tokens: Vec<_> = tokens
        .iter()
        .filter(|t| t.token_type == TOKEN_COMMENT)
        .collect();

    assert!(comment_tokens.len() >= 2, "Should have comment tokens");
}

#[test]
fn test_semantic_tokens_delta_encoding() {
    let text = r#"Email: string #1"#;

    let tokens = compute_semantic_tokens(text).unwrap();

    // First token should have delta_line = 0, delta_start = 0
    assert_eq!(tokens[0].delta_line, 0);
    assert_eq!(tokens[0].delta_start, 0);

    // All tokens should be on the same line
    for token in &tokens {
        assert!(token.delta_line == 0 || tokens.first() == Some(token));
    }
}

#[test]
fn test_builtin_type_detection() {
    assert!(is_builtin_type("string"));
    assert!(is_builtin_type("number"));
    assert!(is_builtin_type("boolean"));
    assert!(is_builtin_type("JSON"));
    assert!(!is_builtin_type("User"));
    assert!(!is_builtin_type("Email"));
}

#[test]
fn test_semantic_tokens_union_type_detailed() {
    let text = r#"Status: "active" | "inactive" | "pending" #2"#;

    let tokens = compute_semantic_tokens(text).expect("Should parse successfully");

    println!("\n=== Tokens for: {} ===", text);
    println!("Total tokens: {}", tokens.len());

    let mut line = 0u32;
    let mut start = 0u32;

    for (i, token) in tokens.iter().enumerate() {
        // Calculate absolute position from delta
        if token.delta_line > 0 {
            line += token.delta_line;
            start = token.delta_start;
        } else {
            start += token.delta_start;
        }

        let token_type_name = match token.token_type {
            TOKEN_COMMENT => "comment",
            TOKEN_KEYWORD => "keyword",
            TOKEN_STRING => "string",
            TOKEN_NUMBER => "number",
            TOKEN_OPERATOR => "operator",
            TOKEN_TYPE => "type",
            TOKEN_CLASS => "class",
            TOKEN_PROPERTY => "property",
            TOKEN_PARAMETER => "parameter",
            TOKEN_VARIABLE => "variable",
            TOKEN_FUNCTION => "function",
            TOKEN_MACRO => "macro",
            TOKEN_ENUM_MEMBER => "enumMember",
            _ => "unknown",
        };

        let modifiers = if token.token_modifiers_bitset & MODIFIER_DEFINITION != 0 {
            ".definition"
        } else if token.token_modifiers_bitset & MODIFIER_READONLY != 0 {
            ".readonly"
        } else if token.token_modifiers_bitset & MODIFIER_MODIFICATION != 0 {
            ".modification"
        } else {
            ""
        };

        // Extract the actual text from source
        let end = start + token.length;
        let token_text = if let Some(line_text) = text.lines().nth(line as usize) {
            let bytes = line_text.as_bytes();
            if start < bytes.len() as u32 && end <= bytes.len() as u32 {
                String::from_utf8_lossy(&bytes[start as usize..end as usize]).to_string()
            } else {
                "[out of bounds]".to_string()
            }
        } else {
            "[no line]".to_string()
        };

        println!(
            "  Token #{}: {}:{}-{} (len={}) -> '{}' = {}{}",
            i,
            line,
            start,
            end,
            token.length,
            token_text,
            token_type_name,
            modifiers
        );
    }

    // Verify we have the expected tokens:
    // 1. "Status" - type (definition)
    // 2. ":" - operator
    // 3. "active" - enumMember (in union)
    // 4. "|" - operator
    // 5. "inactive" - enumMember (in union)
    // 6. "|" - operator
    // 7. "pending" - enumMember (in union)
    // 8. "#2" - parameter (entity ID)

    assert!(tokens.len() >= 8, "Should have at least 8 tokens, got {}", tokens.len());

    // Check that we have 3 enum members
    let enum_members: Vec<_> = tokens
        .iter()
        .filter(|t| t.token_type == TOKEN_ENUM_MEMBER)
        .collect();
    assert_eq!(enum_members.len(), 3, "Should have 3 enum member tokens for union literals");

    // Check that we have the type definition
    let type_defs: Vec<_> = tokens
        .iter()
        .filter(|t| t.token_type == TOKEN_TYPE && (t.token_modifiers_bitset & MODIFIER_DEFINITION) != 0)
        .collect();
    assert_eq!(type_defs.len(), 1, "Should have 1 type definition (Status)");

    // Check that we have the entity ID
    let entity_ids: Vec<_> = tokens
        .iter()
        .filter(|t| t.token_type == TOKEN_PARAMETER)
        .collect();
    assert_eq!(entity_ids.len(), 1, "Should have 1 entity ID (#2)");
}

#[test]
fn test_semantic_tokens_template_import_no_from_keyword() {
    // Test that "from" in template imports does NOT get a semantic token
    // This allows the TextMate grammar to handle it with keyword.control.import scope
    let text = r#"import pg from ../templates/sql-types/postgres.cdm"#;

    let tokens = compute_semantic_tokens(text).expect("Should parse successfully");

    // "from" should NOT be emitted as a keyword token for template imports
    // The TextMate grammar handles it with keyword.control.import.cdm scope
    let keyword_tokens: Vec<_> = tokens
        .iter()
        .filter(|t| t.token_type == TOKEN_KEYWORD)
        .collect();

    // Should have no keyword tokens in template import
    // (import, from, and path are all handled by TextMate grammar)
    assert_eq!(
        keyword_tokens.len(),
        0,
        "Template import should not have keyword semantic tokens (let TextMate handle it)"
    );
}
