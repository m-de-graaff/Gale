/// <reference types="tree-sitter-cli/dsl" />
// Tree-sitter grammar for the GaleX language.
//
// This grammar covers the core syntax needed for syntax highlighting,
// bracket matching, code outline, and indentation in Zed.

const PREC = {
  COMMENT: 0,
  ASSIGN: 1,
  TERNARY: 2,
  NULL_COALESCE: 3,
  OR: 4,
  AND: 5,
  EQUALITY: 6,
  COMPARISON: 7,
  PIPE: 8,
  ADD: 9,
  MULTIPLY: 10,
  UNARY: 11,
  CALL: 12,
  MEMBER: 13,
};

module.exports = grammar({
  name: "gale",

  extras: ($) => [/\s/, $.line_comment, $.block_comment],

  word: ($) => $.identifier,

  conflicts: ($) => [
    [$.primary_expression, $.type_identifier],
    [$._declaration, $._statement],
  ],

  rules: {
    // ── Program ──────────────────────────────────────────────
    source_file: ($) => repeat($._item),

    _item: ($) =>
      choice(
        $._declaration,
        $._statement,
      ),

    // ── Declarations ─────────────────────────────────────────
    _declaration: ($) =>
      choice(
        $.use_declaration,
        $.out_declaration,
        $.function_declaration,
        $.guard_declaration,
        $.store_declaration,
        $.action_declaration,
        $.query_declaration,
        $.channel_declaration,
        $.type_alias_declaration,
        $.enum_declaration,
        $.test_declaration,
        $.middleware_declaration,
        $.env_declaration,
        $.boundary_block,
      ),

    use_declaration: ($) =>
      seq(
        "use",
        choice(
          $.identifier,
          seq("{", commaSep1($.identifier), "}"),
          "*",
        ),
        "from",
        $.string_literal,
      ),

    out_declaration: ($) =>
      seq(
        "out",
        choice(
          $.component_declaration,
          $.layout_declaration,
          $.api_declaration,
          $._declaration,
        ),
      ),

    component_declaration: ($) =>
      seq(
        "ui",
        field("name", $.type_identifier),
        optional($.parameter_list),
        $.component_body,
      ),

    layout_declaration: ($) =>
      seq(
        "layout",
        field("name", $.type_identifier),
        optional($.parameter_list),
        $.component_body,
      ),

    api_declaration: ($) =>
      seq(
        "api",
        field("name", $.type_identifier),
        $.api_body,
      ),

    api_body: ($) =>
      seq(
        "{",
        repeat($.api_handler),
        "}",
      ),

    api_handler: ($) =>
      seq(
        field("method", $.identifier),
        optional(seq("[", commaSep1($.identifier), "]")),
        optional($.parameter_list),
        optional(seq("->", $._type)),
        $.block,
      ),

    function_declaration: ($) =>
      seq(
        "fn",
        field("name", $.identifier),
        $.parameter_list,
        optional(seq("->", $._type)),
        $.block,
      ),

    guard_declaration: ($) =>
      seq(
        "guard",
        field("name", $.type_identifier),
        "{",
        repeat($.guard_field),
        "}",
      ),

    guard_field: ($) =>
      seq(
        field("name", $.identifier),
        ":",
        $._type,
        repeat($.validator_call),
      ),

    validator_call: ($) =>
      seq(
        ".",
        $.identifier,
        "(",
        optional(commaSep1($._expression)),
        ")",
      ),

    store_declaration: ($) =>
      seq(
        "store",
        field("name", $.type_identifier),
        "{",
        repeat(choice($._statement, $.function_declaration)),
        "}",
      ),

    action_declaration: ($) =>
      seq(
        "action",
        field("name", $.identifier),
        $.parameter_list,
        optional(seq("->", $._type)),
        $.block,
      ),

    query_declaration: ($) =>
      seq(
        "query",
        field("name", $.identifier),
        "=",
        $._expression,
        optional(seq("->", $._type)),
      ),

    channel_declaration: ($) =>
      seq(
        "channel",
        field("name", $.identifier),
        optional($.parameter_list),
        choice("->", "<->"),
        $._type,
        "{",
        repeat($.channel_handler),
        "}",
      ),

    channel_handler: ($) =>
      seq(
        "on",
        field("event", $.identifier),
        $.parameter_list,
        $.block,
      ),

    type_alias_declaration: ($) =>
      seq(
        "type",
        field("name", $.type_identifier),
        "=",
        $._type,
      ),

    enum_declaration: ($) =>
      seq(
        "enum",
        field("name", $.type_identifier),
        "{",
        commaSep($.identifier),
        optional(","),
        "}",
      ),

    test_declaration: ($) =>
      seq(
        "test",
        field("name", $.string_literal),
        $.block,
      ),

    middleware_declaration: ($) =>
      seq(
        "middleware",
        field("name", $.identifier),
        $.parameter_list,
        optional(seq("->", $._type)),
        $.block,
      ),

    env_declaration: ($) =>
      seq(
        "env",
        "{",
        repeat($.env_field),
        "}",
      ),

    env_field: ($) =>
      seq(
        field("name", $.identifier),
        ":",
        $._type,
        repeat($.validator_call),
      ),

    boundary_block: ($) =>
      seq(
        field("boundary", choice("server", "client", "shared")),
        $.block,
      ),

    // ── Component body (code + template) ─────────────────────
    component_body: ($) =>
      seq(
        "{",
        repeat(choice(
          $.head_block,
          $._statement,
          $._template_node,
        )),
        "}",
      ),

    head_block: ($) =>
      seq(
        "head",
        "{",
        repeat($.head_field),
        "}",
      ),

    head_field: ($) =>
      seq(
        field("key", $.identifier),
        ":",
        $._expression,
      ),

    // ── Statements ───────────────────────────────────────────
    _statement: ($) =>
      choice(
        $.let_statement,
        $.signal_statement,
        $.derive_statement,
        $.frozen_statement,
        $.ref_statement,
        $.return_statement,
        $.if_statement,
        $.for_statement,
        $.effect_statement,
        $.watch_statement,
        $.assert_statement,
        $.expression_statement,
        $.assignment_statement,
      ),

    let_statement: ($) =>
      seq(
        choice("let", "mut"),
        field("name", $.identifier),
        optional(seq(":", $._type)),
        "=",
        $._expression,
      ),

    signal_statement: ($) =>
      seq(
        "signal",
        field("name", $.identifier),
        optional(seq(":", $._type)),
        "=",
        $._expression,
      ),

    derive_statement: ($) =>
      seq(
        "derive",
        field("name", $.identifier),
        optional(seq(":", $._type)),
        "=",
        $._expression,
      ),

    frozen_statement: ($) =>
      seq(
        "frozen",
        field("name", $.identifier),
        optional(seq(":", $._type)),
        "=",
        $._expression,
      ),

    ref_statement: ($) =>
      seq(
        "ref",
        field("name", $.identifier),
        optional(seq(":", $._type)),
      ),

    return_statement: ($) =>
      seq("return", optional($._expression)),

    if_statement: ($) =>
      seq(
        "if",
        field("condition", $._expression),
        $.block,
        optional(seq("else", choice($.if_statement, $.block))),
      ),

    for_statement: ($) =>
      seq(
        "for",
        field("binding", $.identifier),
        "in",
        field("iterable", $._expression),
        $.block,
      ),

    effect_statement: ($) =>
      seq("effect", $.block),

    watch_statement: ($) =>
      seq(
        "watch",
        $._expression,
        $.block,
      ),

    assert_statement: ($) =>
      seq(
        "assert",
        $._expression,
        optional(seq(",", $.string_literal)),
      ),

    expression_statement: ($) =>
      prec(-1, $._expression),

    assignment_statement: ($) =>
      prec(PREC.ASSIGN, seq(
        $._expression,
        choice("=", "+=", "-="),
        $._expression,
      )),

    // ── Template nodes ───────────────────────────────────────
    _template_node: ($) =>
      choice(
        $.element,
        $.self_closing_element,
        $.text_node,
        $.expression_interpolation,
        $.when_block,
        $.each_block,
        $.suspend_block,
        $.slot_node,
      ),

    element: ($) =>
      seq(
        "<",
        field("tag", $.tag_name),
        repeat($._attribute_or_directive),
        ">",
        repeat($._template_node),
        "</",
        $.tag_name,
        ">",
      ),

    self_closing_element: ($) =>
      seq(
        "<",
        field("tag", $.tag_name),
        repeat($._attribute_or_directive),
        "/>",
      ),

    text_node: ($) => $.string_literal,

    expression_interpolation: ($) =>
      seq("{", $._expression, "}"),

    _attribute_or_directive: ($) =>
      choice(
        $.attribute,
        $.directive,
      ),

    attribute: ($) =>
      seq(
        field("name", $.attribute_name),
        optional(seq("=", $._attribute_value)),
      ),

    _attribute_value: ($) =>
      choice(
        $.string_literal,
        seq("{", $._expression, "}"),
      ),

    directive: ($) =>
      seq(
        field("name", $.directive_name),
        optional(seq("=", seq("{", $._expression, "}"))),
      ),

    directive_name: ($) =>
      token(seq(
        choice("bind", "on", "class", "ref", "transition", "key", "into", "form"),
        ":",
        /[a-zA-Z][a-zA-Z0-9._]*/,
      )),

    attribute_name: ($) => /[a-zA-Z_][a-zA-Z0-9_-]*/,

    tag_name: ($) => /[a-zA-Z][a-zA-Z0-9-]*/,

    when_block: ($) =>
      seq(
        "when",
        $._expression,
        "{",
        repeat($._template_node),
        "}",
        optional(seq("else", "{", repeat($._template_node), "}")),
      ),

    each_block: ($) =>
      seq(
        "each",
        field("binding", $.identifier),
        "in",
        $._expression,
        "{",
        repeat($._template_node),
        "}",
        optional(seq("empty", "{", repeat($._template_node), "}")),
      ),

    suspend_block: ($) =>
      seq(
        "suspend",
        "{",
        repeat($._template_node),
        "}",
      ),

    slot_node: ($) =>
      seq("slot", optional(seq(":", $.identifier))),

    // ── Expressions ──────────────────────────────────────────
    _expression: ($) =>
      choice(
        $.primary_expression,
        $.binary_expression,
        $.unary_expression,
        $.ternary_expression,
        $.call_expression,
        $.member_expression,
        $.index_expression,
        $.pipe_expression,
        $.optional_chain_expression,
        $.await_expression,
        $.arrow_function,
        $.spread_expression,
        $.parenthesized_expression,
        $.array_literal,
        $.object_literal,
      ),

    primary_expression: ($) =>
      choice(
        $.identifier,
        $.string_literal,
        $.number_literal,
        $.boolean_literal,
        $.null_literal,
        $.template_literal,
        $.regex_literal,
      ),

    binary_expression: ($) =>
      choice(
        ...[
          ["+", PREC.ADD],
          ["-", PREC.ADD],
          ["*", PREC.MULTIPLY],
          ["/", PREC.MULTIPLY],
          ["%", PREC.MULTIPLY],
          ["==", PREC.EQUALITY],
          ["!=", PREC.EQUALITY],
          ["<", PREC.COMPARISON],
          [">", PREC.COMPARISON],
          ["<=", PREC.COMPARISON],
          [">=", PREC.COMPARISON],
          ["&&", PREC.AND],
          ["||", PREC.OR],
          ["??", PREC.NULL_COALESCE],
        ].map(([op, precedence]) =>
          prec.left(
            precedence,
            seq(
              field("left", $._expression),
              field("operator", alias(op, $.operator)),
              field("right", $._expression),
            ),
          ),
        ),
      ),

    unary_expression: ($) =>
      prec(PREC.UNARY, seq(
        choice("!", "-"),
        $._expression,
      )),

    ternary_expression: ($) =>
      prec.right(PREC.TERNARY, seq(
        $._expression,
        "?",
        $._expression,
        ":",
        $._expression,
      )),

    call_expression: ($) =>
      prec(PREC.CALL, seq(
        field("callee", $._expression),
        "(",
        optional(commaSep1($._expression)),
        ")",
      )),

    member_expression: ($) =>
      prec(PREC.MEMBER, seq(
        field("object", $._expression),
        ".",
        field("property", $.identifier),
      )),

    index_expression: ($) =>
      prec(PREC.MEMBER, seq(
        $._expression,
        "[",
        $._expression,
        "]",
      )),

    pipe_expression: ($) =>
      prec.left(PREC.PIPE, seq(
        $._expression,
        "|>",
        $._expression,
      )),

    optional_chain_expression: ($) =>
      prec(PREC.MEMBER, seq(
        $._expression,
        "?.",
        $.identifier,
      )),

    await_expression: ($) =>
      prec(PREC.UNARY, seq("await", $._expression)),

    arrow_function: ($) =>
      prec.right(PREC.ASSIGN, seq(
        choice(
          $.identifier,
          $.parameter_list,
        ),
        "=>",
        choice($._expression, $.block),
      )),

    spread_expression: ($) =>
      seq("...", $._expression),

    parenthesized_expression: ($) =>
      seq("(", $._expression, ")"),

    array_literal: ($) =>
      seq("[", optional(commaSep1($._expression)), "]"),

    object_literal: ($) =>
      seq("{", optional(commaSep1($.object_field)), "}"),

    object_field: ($) =>
      choice(
        seq(field("key", $.identifier), ":", field("value", $._expression)),
        $.identifier,
        $.spread_expression,
      ),

    // ── Types ────────────────────────────────────────────────
    _type: ($) =>
      choice(
        $.type_identifier,
        $.generic_type,
        $.union_type,
        $.array_type,
        $.object_type,
        $.function_type,
      ),

    type_identifier: ($) => /[A-Z][a-zA-Z0-9]*/,

    generic_type: ($) =>
      prec(1, seq(
        $.type_identifier,
        "<",
        commaSep1($._type),
        ">",
      )),

    union_type: ($) =>
      prec.left(seq(
        $._type,
        "|",
        $._type,
      )),

    array_type: ($) =>
      prec(1, seq($._type, "[", "]")),

    object_type: ($) =>
      seq(
        "{",
        optional(commaSep1($.type_field)),
        "}",
      ),

    type_field: ($) =>
      seq(
        field("name", $.identifier),
        optional("?"),
        ":",
        $._type,
      ),

    function_type: ($) =>
      seq(
        "(",
        optional(commaSep1($._type)),
        ")",
        "->",
        $._type,
      ),

    // ── Parameters ───────────────────────────────────────────
    parameter_list: ($) =>
      seq("(", optional(commaSep1($.parameter)), ")"),

    parameter: ($) =>
      seq(
        field("name", $.identifier),
        optional(seq(":", $._type)),
        optional(seq("=", $._expression)),
      ),

    // ── Block ────────────────────────────────────────────────
    block: ($) =>
      seq("{", repeat(choice($._statement, $._declaration)), "}"),

    // ── Literals ─────────────────────────────────────────────
    string_literal: ($) =>
      choice(
        seq('"', optional($.string_content), '"'),
        seq("'", optional($.string_content_single), "'"),
      ),

    string_content: ($) =>
      repeat1(choice(
        /[^"\\]+/,
        $.escape_sequence,
      )),

    string_content_single: ($) =>
      repeat1(choice(
        /[^'\\]+/,
        $.escape_sequence,
      )),

    escape_sequence: ($) =>
      token.immediate(seq("\\", /./)),

    number_literal: ($) =>
      choice(
        /\d[\d_]*\.\d[\d_]*/,
        /\d[\d_]*/,
        /0x[0-9a-fA-F][0-9a-fA-F_]*/,
        /0b[01][01_]*/,
      ),

    boolean_literal: ($) => choice("true", "false"),

    null_literal: ($) => "null",

    template_literal: ($) =>
      seq(
        "`",
        repeat(choice(
          $.template_content,
          $.template_substitution,
        )),
        "`",
      ),

    template_content: ($) => /[^`$\\]+|\\.|(\$[^{])/,

    template_substitution: ($) =>
      seq("${", $._expression, "}"),

    regex_literal: ($) =>
      seq(
        token(seq("/", /[^/\n]+/, "/")),
        optional(/[gimsuy]+/),
      ),

    // ── Comments ─────────────────────────────────────────────
    line_comment: ($) => token(seq("//", /[^\n]*/)),

    block_comment: ($) => token(seq("/*", /[^*]*\*+([^/*][^*]*\*+)*/, "/")),

    // ── Identifiers ──────────────────────────────────────────
    identifier: ($) => /[a-zA-Z_][a-zA-Z0-9_]*/,

    // ── Operator (for aliasing in binary_expression) ─────────
    operator: ($) => choice(
      "+", "-", "*", "/", "%",
      "==", "!=", "<", ">", "<=", ">=",
      "&&", "||", "??",
    ),
  },
});

// ── Helpers ────────────────────────────────────────────────────
function commaSep(rule) {
  return optional(commaSep1(rule));
}

function commaSep1(rule) {
  return seq(rule, repeat(seq(",", rule)));
}
