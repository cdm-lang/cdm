/**
 * Tree-sitter grammar for CDM (Common Data Model) language
 *
 * Supports:
 * - Plugin imports: @sql { dialect: "postgres" }
 * - External plugins: @analytics from "git:https://github.com/myorg/cdm-analytics.git" { }
 * - Local plugins: @custom from "./plugins/my-plugin" { }
 * - Template imports: import sql from "sql/postgres-types" { version: "^1.0.0" }
 * - Template extends: extends "cdm/auth" { version: "^2.0.0" }
 * - Qualified type references: sql.UUID, auth.types.Email
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
 * - Context extensions: extends "./base.cdm"
 * - Model removal: -ModelName
 * - Entity IDs: User { name: string #1 } #10
 *
 * Note: Model members (fields, plugin configs) must be on separate lines.
 * Single-line model definitions are not supported.
 */

module.exports = grammar({
  name: "cdm",

  // Only horizontal whitespace is ignored; newlines are significant
  extras: ($) => [/[ \t]+/, $.comment],

  word: ($) => $.identifier,

  conflicts: ($) => [],

  rules: {
    // All directives (extends, imports, plugins) must appear before definitions
    // The ordering among directives is flexible
    // Newlines separate top-level items
    source_file: ($) =>
      seq(
        optional($._nls),
        repeat(seq($._directive, $._nls)),
        repeat(seq($._definition, optional($._nls)))
      ),

    // Top-level directives: extends, template imports, plugin imports
    _directive: ($) =>
      choice(
        $.extends_template,      // extends "./base.cdm", extends "cdm/auth" { version: "^2.0.0" }
        $.template_import,       // import sql from "sql/postgres-types"
        $.plugin_import          // @sql { dialect: "postgres" }
      ),

    _definition: ($) =>
      choice($.model_removal, $.type_alias, $.model_definition),

    // Comments: // single line
    comment: ($) => /\/\/[^\n]*/,

    // Newline handling
    // _nls: one or more newlines (required separator between model members)
    _nls: ($) => repeat1(/\r?\n/),

    // =========================================================================
    // PLUGIN IMPORTS (must appear before definitions)
    // =========================================================================

    // Plugin import: @name [from "source"] [{ config }]
    // Examples:
    //   @sql
    //   @sql { dialect: "postgres", schema: "public" }
    //   @analytics from "git:https://github.com/myorg/cdm-analytics.git" { endpoint: "https://..." }
    //   @custom from "./plugins/my-plugin" { debug: true }
    plugin_import: ($) =>
      seq(
        "@",
        field("name", $.identifier),
        optional(seq("from", field("source", $.string_literal))),
        optional(field("config", $.object_literal))
      ),

    // =========================================================================
    // TEMPLATE IMPORTS
    // =========================================================================

    // Template import: import <namespace> from "<source>" [{ config }]
    // Examples:
    //   import sql from "sql/postgres-types"
    //   import auth from "cdm/auth" { version: "^2.0.0" }
    //   import custom from "git:https://github.com/org/repo.git" { git_ref: "v1.0.0" }
    //   import local from "./templates/shared"
    template_import: ($) =>
      seq(
        "import",
        field("namespace", $.identifier),
        "from",
        field("source", $.string_literal),
        optional(field("config", $.object_literal))
      ),

    // Extends directive: extends "<source>" [{ config }]
    // Unified syntax for both local files and templates
    // Examples:
    //   extends "./base.cdm"                    (local file)
    //   extends "../shared/types.cdm"           (local file)
    //   extends "cdm/auth"                      (registry template)
    //   extends "cdm/auth" { version: "^2.0.0" } (with config)
    //   extends "git:https://github.com/org/repo.git" { git_ref: "main" }
    extends_template: ($) =>
      seq(
        "extends",
        field("source", $.string_literal),
        optional(field("config", $.object_literal))
      ),

    // =========================================================================
    // TOP-LEVEL DIRECTIVES
    // =========================================================================

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
    //   User {
    //     name: string
    //   }
    //   User {
    //     name: string #1
    //   } #10
    //   Article extends Timestamped {
    //     title: string
    //   } #11
    //
    // Note: Model members must be on separate lines
    model_definition: ($) =>
      seq(
        field("name", $.identifier),
        optional(field("extends", $.extends_clause)),
        field("body", $.model_body),
        optional(field("id", $.entity_id))
      ),

    extends_clause: ($) =>
      seq("extends", sep1(",", field("parent", $.identifier))),

    // Model body requires newlines between members
    // Empty models are allowed: User {}
    // Models with members require each on its own line:
    //   User {
    //     id: string #1
    //     name: string #2
    //   }
    model_body: ($) =>
      seq(
        "{",
        optional($._nls),
        optional(
          seq(
            $._model_member,
            repeat(seq($._nls, $._model_member)),
            optional($._nls)
          )
        ),
        "}"
      ),

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

    // Type identifier: simple name or qualified name (namespace.Type)
    // Examples: string, User, sql.UUID, auth.types.Email
    type_identifier: ($) =>
      choice($.qualified_identifier, $.identifier),

    // Qualified identifier for namespace access: namespace.name or namespace.nested.name
    // Examples: sql.UUID, auth.Role, auth.types.Email
    qualified_identifier: ($) =>
      seq(
        field("namespace", $.identifier),
        ".",
        field("name", $._qualified_name_rest)
      ),

    // Rest of qualified name (recursive for nested namespaces)
    _qualified_name_rest: ($) =>
      choice($.qualified_identifier, $.identifier),

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
    // Allows optional newlines for multi-line arrays
    array_literal: ($) =>
      seq(
        "[",
        optional($._nls),
        optional(
          seq(
            $._value,
            repeat(seq(",", optional($._nls), $._value)),
            optional(","),
            optional($._nls)
          )
        ),
        "]"
      ),

    // Object literal: { key: value, ... }
    // JSON/JS style with commas between entries
    // Allows optional newlines for multi-line objects
    object_literal: ($) =>
      seq(
        "{",
        optional($._nls),
        optional(
          seq(
            $.object_entry,
            repeat(seq(",", optional($._nls), $.object_entry)),
            optional(","),
            optional($._nls)
          )
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
    // Allows newlines between plugin configs
    plugin_block: ($) =>
      seq(
        "{",
        optional($._nls),
        optional(
          seq(
            $.plugin_config,
            repeat(seq(optional($._nls), $.plugin_config)),
            optional($._nls)
          )
        ),
        "}"
      ),

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
