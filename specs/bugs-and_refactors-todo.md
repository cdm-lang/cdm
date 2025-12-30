## Refactors

### test fixtures
I want test fixtures to be permanent and live in the repo, as opposed to being created on-demand and then removed after tests are complete. Can you review all tests and update the ones that are writing temporary files on-demand to instead save those files as permanent fixtures in the repo and load them from the file system when the test is run instead?

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

### Test consistency
Right now the structure of tests in the repo is inconsistent and I'd like to refactor to make it more consistent. Tests live in many different places:
- In the source code themselves (eg. @crates/cdm/src/build.r)
- In testing modules (eg. @crates/cdm/src/validate/tests.rs  )
- In separate test directories (eg. @crates/cdm-plugin-docs/tests/integration_test.rs)

I'd like to update testing to a common format. Tests should not live in source code files. They should instead live in a tests directory inside the crate, with a name matching the module they are testing (eg. build_command_test.rs). Test fixtures (eg. cdm files that are used by the tests) should also live in tests/fixtures. Can you make this change to the whole codebase while ensuring no tests are lost?

## Bugs

