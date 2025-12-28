use tower_lsp::lsp_types::*;
use tree_sitter::{Node, Parser};

/// Semantic token types used by CDM
pub const LEGEND_TYPE: &[SemanticTokenType] = &[
    SemanticTokenType::COMMENT,
    SemanticTokenType::KEYWORD,
    SemanticTokenType::STRING,
    SemanticTokenType::NUMBER,
    SemanticTokenType::OPERATOR,
    SemanticTokenType::TYPE,
    SemanticTokenType::CLASS,
    SemanticTokenType::PROPERTY,
    SemanticTokenType::PARAMETER,
    SemanticTokenType::VARIABLE,
    SemanticTokenType::FUNCTION,
    SemanticTokenType::MACRO,
    SemanticTokenType::ENUM_MEMBER,
];

// Token type indices
const TOKEN_COMMENT: u32 = 0;
const TOKEN_KEYWORD: u32 = 1;
const TOKEN_STRING: u32 = 2;
const TOKEN_NUMBER: u32 = 3;
const TOKEN_OPERATOR: u32 = 4;
const TOKEN_TYPE: u32 = 5;
const TOKEN_CLASS: u32 = 6;
const TOKEN_PROPERTY: u32 = 7;
const TOKEN_PARAMETER: u32 = 8;      // Used for entity IDs
const TOKEN_VARIABLE: u32 = 9;
#[allow(dead_code)] // Used in token type display in tests
const TOKEN_FUNCTION: u32 = 10;
const TOKEN_MACRO: u32 = 11;          // Used for plugin names
const TOKEN_ENUM_MEMBER: u32 = 12;    // Used for string literals in unions

/// Semantic token modifiers used by CDM
pub const LEGEND_MODIFIER: &[SemanticTokenModifier] = &[
    SemanticTokenModifier::DECLARATION,
    SemanticTokenModifier::DEFINITION,
    SemanticTokenModifier::READONLY,
    SemanticTokenModifier::DEPRECATED,
    SemanticTokenModifier::MODIFICATION,
];

// Modifier bit flags
#[allow(dead_code)]
const MODIFIER_DECLARATION: u32 = 1 << 0;
const MODIFIER_DEFINITION: u32 = 1 << 1;
const MODIFIER_READONLY: u32 = 1 << 2;
const MODIFIER_MODIFICATION: u32 = 1 << 4;

/// Built-in CDM types
const BUILTIN_TYPES: &[&str] = &[
    "string", "number", "boolean", "decimal", "date", "datetime",
    "timestamp", "binary", "json", "JSON"
];

/// Check if a string is a built-in type
fn is_builtin_type(name: &str) -> bool {
    BUILTIN_TYPES.contains(&name)
}

/// Compute semantic tokens for the given text
pub fn compute_semantic_tokens(text: &str) -> Option<Vec<SemanticToken>> {
    let mut parser = Parser::new();
    parser
        .set_language(&grammar::LANGUAGE.into())
        .expect("Error loading CDM language");

    let tree = parser.parse(text, None)?;
    let root = tree.root_node();

    let mut tokens = Vec::new();

    // Track the previous token's position for delta encoding
    let mut prev_line = 0;
    let mut prev_start = 0;

    // Traverse the tree and collect tokens
    traverse_node(&root, text, &mut tokens, &mut prev_line, &mut prev_start);

    Some(tokens)
}

/// Traverse the syntax tree and generate semantic tokens
fn traverse_node(
    node: &Node,
    source: &str,
    tokens: &mut Vec<SemanticToken>,
    prev_line: &mut u32,
    prev_start: &mut u32,
) {
    // Process this node if it should generate a token
    if should_emit_token(node) {
        if let Some((token_type, modifiers)) = get_token_info(node, source) {
            add_token(node, token_type, modifiers, tokens, prev_line, prev_start);
        }
    }

    // Recursively process children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        traverse_node(&child, source, tokens, prev_line, prev_start);
    }
}

/// Check if a node should emit a semantic token
fn should_emit_token(node: &Node) -> bool {
    // Don't emit tokens for structural nodes that have no text content
    // or are just containers for other nodes
    match node.kind() {
        "source_file" | "model_body" | "plugin_block" | "_definition"
        | "_model_member" | "_type_expression" | "_value" | "_union_member"
        | "_default_value" | "type_alias" | "model_definition" | "field_definition"
        | "union_type" | "array_type" | "type_identifier" | "extends_clause"
        | "field_override" | "field_removal" | "model_removal" | "plugin_config"
        | "plugin_import" | "object_literal" | "array_literal" | "object_entry"
        | "extends_directive" | "plugin_source" | "git_reference" | "plugin_path" => false,
        _ => !node.is_missing() && node.start_byte() < node.end_byte(),
    }
}

/// Add a semantic token with delta encoding
fn add_token(
    node: &Node,
    token_type: u32,
    modifiers: u32,
    tokens: &mut Vec<SemanticToken>,
    prev_line: &mut u32,
    prev_start: &mut u32,
) {
    let start_pos = node.start_position();
    let length = node.end_byte() - node.start_byte();

    // Calculate delta encoding
    let delta_line = start_pos.row as u32 - *prev_line;
    let delta_start = if delta_line == 0 {
        start_pos.column as u32 - *prev_start
    } else {
        start_pos.column as u32
    };

    tokens.push(SemanticToken {
        delta_line,
        delta_start,
        length: length as u32,
        token_type,
        token_modifiers_bitset: modifiers,
    });

    *prev_line = start_pos.row as u32;
    *prev_start = start_pos.column as u32;
}

/// Get the semantic token type and modifiers for a node
fn get_token_info(node: &Node, source: &str) -> Option<(u32, u32)> {
    let kind = node.kind();

    match kind {
        // Comments
        "comment" => Some((TOKEN_COMMENT, 0)),

        // Keywords
        "extends" => Some((TOKEN_KEYWORD, 0)),
        "from" => Some((TOKEN_KEYWORD, 0)),

        // String literals
        "string_literal" => {
            // Check if this is in a union type - if so, it's an enum member
            if is_in_union_type(node) {
                Some((TOKEN_ENUM_MEMBER, MODIFIER_READONLY))
            } else {
                Some((TOKEN_STRING, 0))
            }
        }

        // Numbers
        "number_literal" => Some((TOKEN_NUMBER, 0)),

        // Booleans
        "boolean_literal" => Some((TOKEN_NUMBER, 0)),

        // Operators and punctuation
        "?" | "|" | "=" | "-" | ":" | "[" | "]" | "{" | "}" | "(" | ")" | "," | "@" => {
            Some((TOKEN_OPERATOR, 0))
        }

        // Model name in definition
        "model_definition" => {
            // The first child named "name" is the model name
            if let Some(_name_node) = node.child_by_field_name("name") {
                Some((TOKEN_CLASS, MODIFIER_DEFINITION))
            } else {
                None
            }
        }

        // Type alias name in definition
        "type_alias" => {
            // The first child named "name" is the type alias name
            if let Some(_name_node) = node.child_by_field_name("name") {
                Some((TOKEN_TYPE, MODIFIER_DEFINITION))
            } else {
                None
            }
        }

        // Field name in definition
        "field_definition" => {
            if let Some(_name_node) = node.child_by_field_name("name") {
                Some((TOKEN_PROPERTY, MODIFIER_DEFINITION))
            } else {
                None
            }
        }

        // Identifiers - need to determine context
        "identifier" => {
            let parent_kind = node.parent().map(|p| p.kind()).unwrap_or("");
            let text = node.utf8_text(source.as_bytes()).unwrap_or("");

            match parent_kind {
                // Plugin name
                "plugin_import" | "plugin_config" => {
                    Some((TOKEN_MACRO, 0))
                }

                // Type reference
                "type_identifier" => {
                    if is_builtin_type(text) {
                        Some((TOKEN_TYPE, MODIFIER_READONLY))
                    } else {
                        Some((TOKEN_TYPE, 0))
                    }
                }

                // Array type
                "array_type" => {
                    if is_builtin_type(text) {
                        Some((TOKEN_TYPE, MODIFIER_READONLY))
                    } else {
                        Some((TOKEN_CLASS, 0))
                    }
                }

                // Model name in definition (handled above, but kept for safety)
                "model_definition" => {
                    Some((TOKEN_CLASS, MODIFIER_DEFINITION))
                }

                // Type alias name (handled above)
                "type_alias" => {
                    Some((TOKEN_TYPE, MODIFIER_DEFINITION))
                }

                // Field name (handled above)
                "field_definition" => {
                    Some((TOKEN_PROPERTY, MODIFIER_DEFINITION))
                }

                // Field removal or model removal
                "field_removal" | "model_removal" => {
                    Some((TOKEN_VARIABLE, MODIFIER_MODIFICATION))
                }

                // Extends clause - parent model reference
                "extends_clause" => {
                    Some((TOKEN_CLASS, 0))
                }

                // Field override
                "field_override" => {
                    Some((TOKEN_PROPERTY, MODIFIER_MODIFICATION))
                }

                // Object entry key
                "object_entry" => {
                    Some((TOKEN_PROPERTY, 0))
                }

                _ => None,
            }
        }

        // Type identifier (type reference)
        "type_identifier" => {
            let text = node.utf8_text(source.as_bytes()).unwrap_or("");
            if is_builtin_type(text) {
                Some((TOKEN_TYPE, MODIFIER_READONLY))
            } else {
                Some((TOKEN_TYPE, 0))
            }
        }

        // Entity IDs (#1, #2, etc.)
        "entity_id" => Some((TOKEN_PARAMETER, MODIFIER_READONLY)),

        // Plugin directives
        "extends_directive" => {
            // The @ symbol and "extends" keyword
            Some((TOKEN_KEYWORD, 0))
        }

        _ => None,
    }
}

/// Check if a node is inside a union type
fn is_in_union_type(node: &Node) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "union_type" {
            return true;
        }
        current = parent.parent();
    }
    false
}

#[cfg(test)]
mod tests {
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
}
