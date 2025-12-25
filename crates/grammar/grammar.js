/**
 * Tree-sitter grammar for CDM (Common Data Model) language
 *
 * Supports:
 * - Plugin imports: @sql { dialect: "postgres" }
 * - External plugins: @analytics from git:https://github.com/myorg/cdm-analytics.git { }
 * - Local plugins: @custom from ./plugins/my-plugin { }
 * - Simple type aliases: Email: string
 * - Union types: Status: "active" | "pending" | "deleted"
 * - Composite types / models: User { name: string }
 * - Model inheritance: Article extends Timestamped { }
 * - Multiple inheritance: AdminUser extends BaseUser, Timestamped { }
 * - Field removal: -password_hash
 * - Optional fields: name?: string
 * - Array types: Post[]
 * - Plugin configurations: @sql { table: "users" }
 * - Field-level plugin overrides
 * - Context extensions: @extends ./base.cdm
 * - Model removal: -ModelName
 * - Entity IDs: User { name: string #1 } #10
 */

module.exports = grammar({
  name: "cdm",

  extras: ($) => [/\s/, $.comment],

  word: ($) => $.identifier,

  conflicts: ($) => [],

  rules: {
    // Enforce ordering: @extends directives → plugin imports → definitions
    source_file: ($) =>
      seq(
        repeat($.extends_directive),
        repeat($.plugin_import),
        repeat($._definition)
      ),

    _definition: ($) =>
      choice(
        $.model_removal,
        $.type_alias,
        $.model_definition
      ),

    // Comments: // single line
    comment: ($) => /\/\/[^\n]*/,

    // =========================================================================
    // PLUGIN IMPORTS (must appear before definitions)
    // =========================================================================

    // Plugin import: @name [from source] [{ config }]
    // Examples:
    //   @sql
    //   @sql { dialect: "postgres", schema: "public" }
    //   @analytics from git:https://github.com/myorg/cdm-analytics.git { endpoint: "https://..." }
    //   @custom from ./plugins/my-plugin { debug: true }
    plugin_import: ($) =>
      seq(
        "@",
        field("name", $.identifier),
        optional(seq("from", field("source", $.plugin_source))),
        optional(field("config", $.object_literal))
      ),

    // Plugin source: git URL or local path
    plugin_source: ($) => choice($.git_reference, $.plugin_path),

    // Git reference: git:<url>
    git_reference: ($) => seq("git:", field("url", $.git_url)),

    // Flexible git URL pattern
    git_url: ($) => /[^\s\n{}]+/,

    // Local plugin path: ./path or ../path
    plugin_path: ($) => /\.\.?\/[^\s\n{}]+/,

    // =========================================================================
    // TOP-LEVEL DIRECTIVES
    // =========================================================================

    // Context extension: @extends ./path/to/base.cdm
    extends_directive: ($) => seq("@extends", field("path", $.path)),

    // File path for extends
    path: ($) => /[^\s\n]+/,

    // Model removal at file level: -ModelName
    model_removal: ($) => seq("-", field("name", $.identifier)),

    // =========================================================================
    // ENTITY IDs
    // =========================================================================

    // Entity ID: #N where N is a positive integer
    // Examples: #1, #42, #1000
    // Used for stable identity tracking across schema versions
    entity_id: ($) => seq("#", /[1-9][0-9]*/),

    // =========================================================================
    // TYPE ALIASES
    // =========================================================================

    // Type alias: Name: type [{ plugins }] [#id]
    // Examples:
    //   Email: string
    //   Email: string #1
    //   Status: "active" | "pending" | "deleted" #2
    //   UUID: string { @validation { format: uuid } } #3
    //   AccountType: "free" | "premium" | "enterprise" { @sql { type: "ENUM" } } #4
    type_alias: ($) =>
      seq(
        field("name", $.identifier),
        ":",
        field("type", $._type_expression),
        optional(field("plugins", $.plugin_block)),
        optional(field("id", $.entity_id))
      ),

    // =========================================================================
    // MODEL DEFINITIONS
    // =========================================================================

    // Model: Name [extends Parents] { members } [#id]
    // Examples:
    //   User { name: string }
    //   User { name: string } #10
    //   Article extends Timestamped { title: string } #11
    //   AdminUser extends BaseUser, Timestamped { level: number } #12
    model_definition: ($) =>
      seq(
        field("name", $.identifier),
        optional(field("extends", $.extends_clause)),
        field("body", $.model_body),
        optional(field("id", $.entity_id))
      ),

    extends_clause: ($) =>
      seq("extends", sep1(",", field("parent", $.identifier))),

    model_body: ($) => seq("{", repeat($._model_member), "}"),

    _model_member: ($) =>
      choice(
        $.field_removal,
        $.plugin_config,
        $.field_override,
        $.field_definition
      ),

    // =========================================================================
    // FIELD DEFINITIONS
    // =========================================================================

    // Field removal: -field_name
    field_removal: ($) => seq("-", field("name", $.identifier)),

    // Field-specific override: field_name { @plugins }
    // Used to add/override plugins on a field INHERITED from a parent model.
    // NOT for fields defined in the same model - use inline plugin blocks instead.
    //
    // VALID (overriding inherited field):
    //   AdminUser extends User {
    //     status { @sql { type: "admin_status_enum" } }
    //   }
    //
    // INVALID (use inline syntax instead):
    //   Post {
    //     content: string
    //     content { @sql { type: "TEXT" } }  // ERROR!
    //   }
    //
    // CORRECT inline syntax:
    //   Post {
    //     content: string { @sql { type: "TEXT" } }
    //   }
    field_override: ($) =>
      prec(
        2,
        seq(field("name", $.identifier), field("plugins", $.plugin_block))
      ),

    // Field definition: name[?] [: type [= default] [{ plugins }]] [#id]
    // Examples:
    //   name                                    (untyped, defaults to string)
    //   name #1                                 (untyped with ID)
    //   email: string #2                        (typed with ID)
    //   active: boolean = true #3               (with default and ID)
    //   age?: Age #4                            (optional with ID)
    //   bio? #5                                 (optional, untyped with ID)
    //   posts: Post[] #6                        (array with ID)
    //   tags?: Tag[] #7                         (optional array with ID)
    //   status: "draft" | "published" = "draft" #8 (inline union with default and ID)
    //   content: string { @sql { type: "TEXT" } } #9  (with plugins and ID)
    //   average_rating: decimal { @computed { from: "AVG(reviews.rating)" } } #10
    field_definition: ($) =>
      prec(
        1,
        seq(
          field("name", $.identifier),
          field("optional", optional("?")),
          optional(
            seq(
              ":",
              field("type", $._type_expression),
              optional(seq("=", field("default", $._default_value))),
              optional(field("plugins", $.plugin_block))
            )
          ),
          optional(field("id", $.entity_id))
        )
      ),

    // Default values can include function calls like now()
    _default_value: ($) =>
      choice(
        $.string_literal,
        $.number_literal,
        $.boolean_literal,
        $.array_literal,
        $.object_literal
      ),

    // Function call for defaults: now()
    function_call: ($) => seq(field("name", $.identifier), "(", ")"),

    // =========================================================================
    // TYPE EXPRESSIONS
    // =========================================================================

    _type_expression: ($) =>
      choice($.union_type, $.array_type, $.type_identifier),

    // Union type: "a" | "b" | "c" or Type1 | Type2 | "literal"
    // Supports both string literals and type references
    union_type: ($) =>
      prec.left(1, seq($._union_member, repeat1(seq("|", $._union_member)))),

    _union_member: ($) =>
      choice($.string_literal, $.array_type, $.type_identifier),

    // Basic type reference
    type_identifier: ($) => $.identifier,

    // Array type: Type[]
    array_type: ($) => prec(2, seq($.type_identifier, "[", "]")),

    // =========================================================================
    // VALUES (for defaults and config)
    // =========================================================================

    _value: ($) =>
      choice(
        $.string_literal,
        $.number_literal,
        $.boolean_literal,
        $.array_literal,
        $.object_literal
      ),

    // Array literal: [value, value, ...]
    array_literal: ($) =>
      seq(
        "[",
        optional(seq($._value, repeat(seq(",", $._value)), optional(","))),
        "]"
      ),

    // Object literal: { key: value, ... }
    // JSON/JS style with commas between entries
    object_literal: ($) =>
      seq(
        "{",
        optional(
          seq($.object_entry, repeat(seq(",", $.object_entry)), optional(","))
        ),
        "}"
      ),

    object_entry: ($) =>
      seq(
        field("key", choice($.identifier, $.string_literal)),
        ":",
        field("value", $._value)
      ),

    // =========================================================================
    // PLUGIN SYSTEM
    // =========================================================================

    // Plugin block: { @plugin1 {} @plugin2 {} }
    plugin_block: ($) => seq("{", repeat($.plugin_config), "}"),

    // Plugin config: @name { json_config }
    // Examples:
    //   @sql { "table": "users" }
    //   @validation { format: "email" }
    //   @api { expose: ["id", "name"] }
    plugin_config: ($) =>
      seq("@", field("name", $.identifier), field("config", $.object_literal)),

    // =========================================================================
    // LITERALS
    // =========================================================================

    // String: "content" with escape support
    string_literal: ($) =>
      seq('"', repeat(choice($.string_content, $.escape_sequence)), '"'),

    string_content: ($) => token.immediate(prec(1, /[^"\\]+/)),

    escape_sequence: ($) =>
      token.immediate(seq("\\", choice(/["\\/bfnrt]/, /u[0-9a-fA-F]{4}/))),

    // Number: integers and decimals, optionally negative
    number_literal: ($) =>
      token(seq(optional("-"), /\d+/, optional(seq(".", /\d+/)))),

    // Boolean
    boolean_literal: ($) => choice("true", "false"),

    // Identifier: starts with letter or underscore
    identifier: ($) => /[a-zA-Z_][a-zA-Z0-9_]*/,
  },
});

/**
 * Creates a rule for one or more occurrences separated by a delimiter
 * @param {string} separator - The separator between elements
 * @param {Rule} rule - The rule to repeat
 * @returns {SeqRule}
 */
function sep1(separator, rule) {
  return seq(rule, repeat(seq(separator, rule)));
}
