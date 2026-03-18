; GaleX Tree-sitter highlight queries
; NOTE: These queries require a tree-sitter-gale grammar to be built.
; The grammar.js file is not included here — it would be in a separate
; tree-sitter-gale repository.

; Keywords
[
  "let" "mut" "signal" "derive" "frozen" "ref"
  "fn" "return" "if" "else" "for" "await"
  "guard" "action" "query" "store" "channel"
  "type" "enum" "test" "middleware" "env"
  "effect" "watch"
  "when" "each" "suspend" "slot" "empty"
  "use" "out" "from" "ui" "api" "layout"
  "server" "client" "shared"
  "head" "redirect" "link" "transition"
  "on" "bind" "assert"
] @keyword

; Literals
(string_literal) @string
(number_literal) @number
(boolean_literal) @constant.builtin
"null" @constant.builtin

; Comments
(line_comment) @comment
(block_comment) @comment

; Types
(type_annotation (identifier) @type)
(guard_declaration name: (identifier) @type)
(store_declaration name: (identifier) @type)
(enum_declaration name: (identifier) @type)
(component_declaration name: (identifier) @type)
(layout_declaration name: (identifier) @type)

; Functions
(function_declaration name: (identifier) @function)
(action_declaration name: (identifier) @function)
(function_call callee: (identifier) @function.call)

; Variables
(signal_statement name: (identifier) @variable.special)
(derive_statement name: (identifier) @variable.special)
(let_statement name: (identifier) @variable)
(parameter name: (identifier) @variable.parameter)

; HTML tags
(element tag: (tag_name) @tag)
(self_closing_element tag: (tag_name) @tag)

; Directives
(directive name: (directive_name) @attribute)

; Operators
["+" "-" "*" "/" "%" "==" "!=" "<" ">" "<=" ">="
 "&&" "||" "!" "??" "=>" "->" "<->" "|>" ".."] @operator

; Punctuation
["{" "}" "(" ")" "[" "]" "<" ">"] @punctuation.bracket
["," "." ":" ";"] @punctuation.delimiter
