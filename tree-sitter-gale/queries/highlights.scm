; GaleX Tree-sitter highlight queries
; Aligned with the tree-sitter-gale grammar.js node types.

; ── Keywords ──────────────────────────────────────────────────

; Binding keywords
["let" "mut" "signal" "derive" "frozen" "ref"] @keyword

; Function & control keywords
["fn" "return" "if" "else" "for" "await" "in"] @keyword

; Boundary keywords
["server" "client" "shared"] @keyword

; Declaration keywords
["guard" "action" "query" "store" "channel"
 "type" "enum" "test" "middleware" "env"] @keyword

; Reactivity keywords
["effect" "watch" "bind"] @keyword

; Template control flow
["when" "each" "suspend" "slot" "empty"] @keyword

; Module keywords
["use" "out" "ui" "api" "layout" "from"] @keyword

; Other keywords
["head" "redirect" "link" "transition" "on" "assert"] @keyword

; ── Literals ──────────────────────────────────────────────────

(string_literal) @string
(string_content) @string
(string_content_single) @string
(escape_sequence) @string.escape
(number_literal) @number
(boolean_literal) @constant.builtin
(null_literal) @constant.builtin
(template_literal) @string
(template_content) @string
(template_substitution
  "${" @punctuation.special
  "}" @punctuation.special)
(regex_literal) @string.regex

; ── Comments ──────────────────────────────────────────────────

(line_comment) @comment
(block_comment) @comment

; ── Types ─────────────────────────────────────────────────────

(type_identifier) @type
(generic_type (type_identifier) @type)
(union_type "|" @operator)

; Declaration names that are types
(guard_declaration name: (type_identifier) @type)
(store_declaration name: (type_identifier) @type)
(enum_declaration name: (type_identifier) @type)
(component_declaration name: (type_identifier) @type)
(layout_declaration name: (type_identifier) @type)
(api_declaration name: (type_identifier) @type)
(type_alias_declaration name: (type_identifier) @type)

; ── Functions ─────────────────────────────────────────────────

(function_declaration name: (identifier) @function)
(action_declaration name: (identifier) @function)
(middleware_declaration name: (identifier) @function)
(call_expression callee: (identifier) @function)
(call_expression callee: (member_expression property: (identifier) @function))
(validator_call (identifier) @function)

; ── Variables ─────────────────────────────────────────────────

(signal_statement name: (identifier) @variable.special)
(derive_statement name: (identifier) @variable.special)
(let_statement name: (identifier) @variable)
(frozen_statement name: (identifier) @variable)
(ref_statement name: (identifier) @variable)
(parameter name: (identifier) @variable.parameter)
(for_statement binding: (identifier) @variable)
(each_block binding: (identifier) @variable)

; Query & channel names
(query_declaration name: (identifier) @function)
(channel_declaration name: (identifier) @function)
(channel_handler event: (identifier) @property)

; Test names
(test_declaration name: (string_literal) @string)

; ── HTML / Template ───────────────────────────────────────────

(element tag: (tag_name) @tag)
(self_closing_element tag: (tag_name) @tag)
(element (tag_name) @tag)

(attribute name: (attribute_name) @attribute)
(directive name: (directive_name) @attribute)

; Head block fields
(head_field key: (identifier) @property)

; ── Operators ─────────────────────────────────────────────────

(operator) @operator

["=>" "->" "<->" "|>" ".." "..."
 "=" "+=" "-="
 "!" "?" "?."] @operator

; ── Punctuation ───────────────────────────────────────────────

["{" "}" "(" ")" "[" "]" "<" ">" "</" "/>"] @punctuation.bracket
["," "." ":" ";"] @punctuation.delimiter

; ── Identifiers (fallback) ────────────────────────────────────

(identifier) @variable
