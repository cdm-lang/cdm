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

/// Built-in CDM types (matches symbol_table::is_builtin_type)
const BUILTIN_TYPES: &[&str] = &[
    "string", "number", "boolean", "JSON"
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
        | "extends_template" | "plugin_source" | "git_reference" | "plugin_path" => false,
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
        "extends" | "import" | "from" => Some((TOKEN_KEYWORD, 0)),

        // Template/plugin source paths (e.g., ../templates/sql-types/postgres.cdm)
        "local_path" | "registry_name" | "git_url" => Some((TOKEN_STRING, 0)),

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
                // Template import namespace (e.g., "pg" in "import pg from ...")
                "template_import" => {
                    Some((TOKEN_VARIABLE, 0))
                }

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

        // Extends directive
        "extends_template" => {
            // The "extends" keyword
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
#[path = "semantic_tokens/semantic_tokens_tests.rs"]
mod semantic_tokens_tests;
