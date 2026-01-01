## Refactors

### Entity ID Extraction
Duplicated in validate.rs, format.rs, and implicitly elsewhere
Location:
crates/cdm/src/validate.rs:323-333 - extract_entity_id()
crates/cdm/src/format.rs:858-866 - extract_entity_id()
Issue: Same logic for parsing entity IDs from tree-sitter nodes. Recommendation:

// Move to: crates/cdm/src/ast_utils.rs (or cdm-utils crate)
pub fn extract_entity_id(node: tree_sitter::Node, source: &str) -> Option<u64>

### Node Text Extraction
Duplicated in validate.rs, format.rs, and elsewhere
Location:
crates/cdm/src/validate.rs:304-306 - get_node_text()
crates/cdm/src/format.rs:868-871 - get_node_text()
Multiple other locations
Issue: Trivial but repeated utility function. Recommendation:

// Move to: crates/cdm/src/ast_utils.rs
pub fn get_node_text<'a>(node: tree_sitter::Node, source: &'a str) -> &'a str


## Bugs

## Features

### Code completion on overrides
Code completion when you've extended from another model and you are about to create a new model or type alias in the extended file should suggest other models from the ancestors that you might want to override.

### Visual distinction when you're overriding an inherited model vs defining a new model in the syntax highlighter functionality

### Ability for plugins to export types
