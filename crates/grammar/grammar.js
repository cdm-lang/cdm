/**
 * @file Contextual Data Models
 * @author Aaron Larner <aaron@larner.dev>
 * @license MIT
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

module.exports = grammar({
  name: "cdm",

  rules: {
    // TODO: add the actual grammar rules
    source_file: $ => "hello"
  }
});
