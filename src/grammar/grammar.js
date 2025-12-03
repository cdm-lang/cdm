// grammar.js
module.exports = grammar({
  name: "datamodel",

  extras: ($) => [/\s/, $.comment],

  rules: {
    source_file: ($) => seq(repeat($.file_directive), repeat($._definition)),

    // File-level directives
    file_directive: ($) => choice($.extends_directive, $.import_directive),

    extends_directive: ($) => seq("@extends", $.path_list),

    import_directive: ($) =>
      seq("@import", $.path, optional(seq("{", commaSep1($.identifier), "}"))),

    path_list: ($) => commaSep1($.path),

    path: ($) => /[\.\/][^\s,]*/,

    // Top-level definitions
    _definition: ($) => choice($.type_alias, $.composite_type, $.model_removal),

    // Type alias: Email: string { ... }
    type_alias: ($) =>
      seq($.identifier, ":", $.type_expression, optional($.plugin_blocks)),

    // Composite type/model: User { ... }
    composite_type: ($) =>
      seq(
        $.identifier,
        optional(seq("extends", $.identifier_list)),
        $.type_body
      ),

    // Remove model: -ModelName
    model_removal: ($) => seq("-", $.identifier),

    type_body: ($) =>
      seq(
        "{",
        repeat(
          choice(
            $.field_definition,
            $.field_removal,
            $.field_override,
            $.plugin_block
          )
        ),
        "}"
      ),

    // Field definitions
    field_definition: ($) =>
      seq(
        $.identifier,
        ":",
        $.type_expression,
        optional("?"), // Optional marker
        optional(seq("=", $.default_value)), // Default value
        optional($.inline_plugin_blocks)
      ),

    // Field removal: -fieldname
    field_removal: ($) => seq("-", $.identifier),

    // Field override block
    field_override: ($) => seq($.identifier, $.plugin_blocks),

    // Type expressions
    type_expression: ($) =>
      choice($.primitive_type, $.type_reference, $.array_type, $.union_type),

    primitive_type: ($) =>
      choice(
        "string",
        "number",
        "decimal",
        "boolean",
        "Date",
        "DateTime",
        "JSON",
        "UUID"
      ),

    type_reference: ($) => $.identifier,

    array_type: ($) => prec.left(2, seq($.type_expression, "[", "]")),

    union_type: ($) =>
      prec.left(1, seq($.union_member, repeat1(seq("|", $.union_member)))),

    union_member: ($) =>
      choice($.string_literal, $.type_reference, $.primitive_type),

    // Default values
    default_value: ($) =>
      choice(
        $.string_literal,
        $.number_literal,
        $.boolean_literal,
        $.null_literal,
        $.function_call,
        $.object_literal,
        $.array_literal
      ),

    function_call: ($) =>
      seq($.identifier, "(", optional(commaSep($.default_value)), ")"),

    object_literal: ($) => seq("{", commaSep($.object_field), "}"),

    object_field: ($) => seq($.identifier, ":", $.default_value),

    array_literal: ($) => seq("[", commaSep($.default_value), "]"),

    // Plugin blocks
    plugin_blocks: ($) =>
      seq("{", repeat(choice($.plugin_block, $.field_override)), "}"),

    inline_plugin_blocks: ($) => seq("{", repeat($.plugin_block), "}"),

    plugin_block: ($) => seq("@", $.identifier, $.plugin_body),

    plugin_body: ($) => seq("{", repeat($.plugin_property), "}"),

    plugin_property: ($) =>
      seq(
        $.property_path,
        ":",
        $.plugin_value,
        optional(choice(",", ";")) // Optional separator
      ),

    property_path: ($) => seq($.identifier, repeat(seq(".", $.identifier))),

    plugin_value: ($) =>
      choice(
        $.string_literal,
        $.number_literal,
        $.boolean_literal,
        $.array_literal,
        $.object_literal,
        $.identifier
      ),

    // Utilities
    identifier_list: ($) => commaSep1($.identifier),

    // Literals
    string_literal: ($) =>
      choice(
        seq('"', repeat(choice(/[^"\\]/, /\\./)), '"'),
        seq("'", repeat(choice(/[^'\\]/, /\\./)), "'")
      ),

    number_literal: ($) => /\-?\d+(\.\d+)?/,

    boolean_literal: ($) => choice("true", "false"),

    null_literal: ($) => "null",

    identifier: ($) => /[a-zA-Z_][a-zA-Z0-9_]*/,

    comment: ($) =>
      choice(seq("//", /.*/), seq("/*", /[^*]*\*+([^/*][^*]*\*+)*/, "/")),
  },
});

// Helper function for comma-separated lists
function commaSep1(rule) {
  return seq(rule, repeat(seq(",", rule)), optional(","));
}

function commaSep(rule) {
  return optional(commaSep1(rule));
}
