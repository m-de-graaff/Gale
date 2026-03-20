; GaleX Tree-sitter highlight queries
;
; Canonical copy lives at: galex/editors/zed/languages/gale/highlights.scm
; This file is kept in sync for tree-sitter CLI testing (tree-sitter highlight).
;
; More specific patterns must appear BEFORE generic ones — tree-sitter
; applies the first match.

; ══════════════════════════════════════════════════════════════
; DECLARATIONS — specific names get definition captures
; ══════════════════════════════════════════════════════════════

(guard_declaration
  "guard" @keyword.type
  name: (type_identifier) @type.definition)

(guard_field
  name: (identifier) @property.definition)

(validator_call
  "." @punctuation.delimiter
  (identifier) @function.method)

(store_declaration
  "store" @keyword.type
  name: (type_identifier) @type.definition)

(enum_declaration
  "enum" @keyword.type
  name: (type_identifier) @type.definition)

(type_alias_declaration
  "type" @keyword.type
  name: (type_identifier) @type.definition)

(component_declaration
  "ui" @keyword.modifier
  name: (type_identifier) @type.definition)

(layout_declaration
  "layout" @keyword.modifier
  name: (type_identifier) @type.definition)

(api_declaration
  "api" @keyword.modifier
  name: (type_identifier) @type.definition)

(api_handler
  method: (identifier) @function.method)

(function_declaration
  "fn" @keyword.function
  name: (identifier) @function.definition)

(action_declaration
  "action" @keyword.function
  name: (identifier) @function.definition)

(middleware_declaration
  "middleware" @keyword.function
  name: (identifier) @function.definition)

(channel_declaration
  "channel" @keyword.type
  name: (identifier) @function.definition)

(channel_handler
  "on" @keyword
  event: (identifier) @property)

(query_declaration
  "query" @keyword.type
  name: (identifier) @function.definition)

(test_declaration
  "test" @keyword
  name: (string_literal) @string)

(env_declaration "env" @keyword.type)

(boundary_block
  boundary: _ @keyword.modifier)

; ══════════════════════════════════════════════════════════════
; STATEMENTS
; ══════════════════════════════════════════════════════════════

(signal_statement
  "signal" @keyword.declaration
  name: (identifier) @variable.special)

(derive_statement
  "derive" @keyword.declaration
  name: (identifier) @variable.special)

(let_statement
  name: (identifier) @variable.definition)

(frozen_statement
  "frozen" @keyword.declaration
  name: (identifier) @variable.definition)

(ref_statement
  "ref" @keyword.declaration
  name: (identifier) @variable.definition)

(parameter
  name: (identifier) @variable.parameter)

(for_statement
  "for" @keyword.control
  binding: (identifier) @variable.definition
  "in" @keyword.control)

(each_block
  "each" @keyword.control
  binding: (identifier) @variable.definition
  "in" @keyword.control)

(return_statement "return" @keyword.return)

(assert_statement "assert" @keyword)

(assignment_statement
  ["=" "+=" "-="] @operator)

; ══════════════════════════════════════════════════════════════
; EXPRESSIONS
; ══════════════════════════════════════════════════════════════

(call_expression
  callee: (identifier) @function.call)

(call_expression
  callee: (member_expression
    property: (identifier) @function.method.call))

(member_expression
  property: (identifier) @property)

(optional_chain_expression
  "?." @operator
  (identifier) @property)

(await_expression "await" @keyword.coroutine)
(spread_expression "..." @operator)
(arrow_function "=>" @operator)
(binary_expression operator: (_) @operator)
(pipe_expression "|>" @operator)
(ternary_expression "?" @operator ":" @operator)

; ══════════════════════════════════════════════════════════════
; TEMPLATE / HTML
; ══════════════════════════════════════════════════════════════

(element tag: (tag_name) @tag)
(self_closing_element tag: (tag_name) @tag)
(element (tag_name) @tag)

(attribute name: (attribute_name) @attribute)
(directive name: (directive_name) @attribute.special)

(expression_interpolation
  "{" @punctuation.special
  "}" @punctuation.special)

(head_field key: (identifier) @property)
(slot_node "slot" @tag)

; ══════════════════════════════════════════════════════════════
; IMPORTS
; ══════════════════════════════════════════════════════════════

(use_declaration "use" @keyword.import "from" @keyword.import)
(use_declaration (identifier) @variable)
(use_declaration (string_literal) @string.special.path)

; ══════════════════════════════════════════════════════════════
; TYPES
; ══════════════════════════════════════════════════════════════

(type_identifier) @type
(generic_type (type_identifier) @type "<" @punctuation.bracket ">" @punctuation.bracket)
(array_type "[" @punctuation.bracket "]" @punctuation.bracket)
(union_type "|" @operator)
(function_type "->" @operator)
(type_field name: (identifier) @property "?" @operator)
(object_field key: (identifier) @property)

; ══════════════════════════════════════════════════════════════
; KEYWORDS (generic fallback)
; ══════════════════════════════════════════════════════════════

["let" "mut"] @keyword.declaration
["if" "else" "when" "each" "suspend" "empty"] @keyword.control
["guard" "action" "query" "store" "channel" "type" "enum" "test" "middleware" "env"] @keyword.type
["fn"] @keyword.function
["effect" "watch"] @keyword
["server" "client" "shared"] @keyword.modifier
["use" "out" "from"] @keyword.import
["head" "redirect" "link" "transition" "on" "bind"
 "signal" "derive" "frozen" "ref"
 "ui" "api" "layout"
 "slot" "await" "assert"] @keyword

; ══════════════════════════════════════════════════════════════
; LITERALS
; ══════════════════════════════════════════════════════════════

(string_literal) @string
(string_content) @string
(string_content_single) @string
(escape_sequence) @string.escape
(number_literal) @number
(boolean_literal) @constant.builtin
(null_literal) @constant.builtin
(template_literal) @string
(template_content) @string
(template_substitution "${" @punctuation.special "}" @punctuation.special)
(regex_literal) @string.regex

; ══════════════════════════════════════════════════════════════
; COMMENTS
; ══════════════════════════════════════════════════════════════

(line_comment) @comment
(block_comment) @comment

; ══════════════════════════════════════════════════════════════
; OPERATORS & PUNCTUATION
; ══════════════════════════════════════════════════════════════

(operator) @operator
["=>" "->" "<->" "|>" ".." "..." "=" "+=" "-=" "!" "?" "?."] @operator

["{" "}" "(" ")" "[" "]"] @punctuation.bracket
["<" ">" "</" "/>"] @punctuation.bracket
["," "." ":" ";"] @punctuation.delimiter

; ══════════════════════════════════════════════════════════════
; IDENTIFIERS (fallback — must be last)
; ══════════════════════════════════════════════════════════════

(identifier) @variable
