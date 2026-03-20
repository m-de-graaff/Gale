; GaleX Tree-sitter highlight queries
;
; Zed uses these captures to colorize source code. More specific patterns
; must appear BEFORE generic ones — tree-sitter applies the first match.

; ══════════════════════════════════════════════════════════════
; DECLARATIONS — specific names get definition captures
; ══════════════════════════════════════════════════════════════

; Guard declaration: `guard UserLogin { ... }`
(guard_declaration
  "guard" @keyword.type
  name: (type_identifier) @type.definition)

; Guard fields: `email: string.email()`
(guard_field
  name: (identifier) @property.definition)

; Validator chains: `.trim().minLen(1)`
(validator_call
  "." @punctuation.delimiter
  (identifier) @function.method)

; Store: `store Counter { ... }`
(store_declaration
  "store" @keyword.type
  name: (type_identifier) @type.definition)

; Enum: `enum Status { Active, Inactive }`
(enum_declaration
  "enum" @keyword.type
  name: (type_identifier) @type.definition)

; Type alias: `type UserId = string`
(type_alias_declaration
  "type" @keyword.type
  name: (type_identifier) @type.definition)

; Component: `out ui HomePage { ... }`
(component_declaration
  "ui" @keyword.modifier
  name: (type_identifier) @type.definition)

; Layout: `out layout MainLayout { ... }`
(layout_declaration
  "layout" @keyword.modifier
  name: (type_identifier) @type.definition)

; API: `out api Users { ... }`
(api_declaration
  "api" @keyword.modifier
  name: (type_identifier) @type.definition)

; API handler: `get[id]() -> User { ... }`
(api_handler
  method: (identifier) @function.method)

; Function: `fn helper() { ... }`
(function_declaration
  "fn" @keyword.function
  name: (identifier) @function.definition)

; Action: `action login(data: LoginGuard) -> string { ... }`
(action_declaration
  "action" @keyword.function
  name: (identifier) @function.definition)

; Middleware: `middleware auth(req, next) { ... }`
(middleware_declaration
  "middleware" @keyword.function
  name: (identifier) @function.definition)

; Channel: `channel chat() <-> string { ... }`
(channel_declaration
  "channel" @keyword.type
  name: (identifier) @function.definition)

; Channel event handler: `on connect(emit) { ... }`
(channel_handler
  "on" @keyword
  event: (identifier) @property)

; Query: `query users = "/api/users" -> User[]`
(query_declaration
  "query" @keyword.type
  name: (identifier) @function.definition)

; Test: `test "user login works" { ... }`
(test_declaration
  "test" @keyword
  name: (string_literal) @string)

; Env: `env { DATABASE_URL: string.nonEmpty() }`
(env_declaration "env" @keyword.type)

; Boundary blocks: `server { }`, `client { }`, `shared { }`
(boundary_block
  boundary: _ @keyword.modifier)

; ══════════════════════════════════════════════════════════════
; STATEMENTS — bindings and control flow
; ══════════════════════════════════════════════════════════════

; Signal: `signal count = 0`
(signal_statement
  "signal" @keyword.declaration
  name: (identifier) @variable.special)

; Derive: `derive doubled = count * 2`
(derive_statement
  "derive" @keyword.declaration
  name: (identifier) @variable.special)

; Let / mut: `let x = 1`, `mut y = 2`
(let_statement
  name: (identifier) @variable.definition)

; Frozen: `frozen name = "constant"`
(frozen_statement
  "frozen" @keyword.declaration
  name: (identifier) @variable.definition)

; Ref: `ref canvas: HTMLElement`
(ref_statement
  "ref" @keyword.declaration
  name: (identifier) @variable.definition)

; Parameters
(parameter
  name: (identifier) @variable.parameter)

; For binding: `for item in list { ... }`
(for_statement
  "for" @keyword.control
  binding: (identifier) @variable.definition
  "in" @keyword.control)

; Each binding: `each item in items { ... }`
(each_block
  "each" @keyword.control
  binding: (identifier) @variable.definition
  "in" @keyword.control)

; Return
(return_statement "return" @keyword.return)

; Assert
(assert_statement "assert" @keyword)

; Assignment operators
(assignment_statement
  ["=" "+=" "-="] @operator)

; ══════════════════════════════════════════════════════════════
; EXPRESSIONS
; ══════════════════════════════════════════════════════════════

; Function call: `fetch(url)`
(call_expression
  callee: (identifier) @function.call)

; Method call: `response.json()`
(call_expression
  callee: (member_expression
    property: (identifier) @function.method.call))

; Member access: `user.name`
(member_expression
  property: (identifier) @property)

; Optional chain: `user?.name`
(optional_chain_expression
  "?." @operator
  (identifier) @property)

; Await
(await_expression "await" @keyword.coroutine)

; Spread
(spread_expression "..." @operator)

; Arrow function: `(x) => x + 1`
(arrow_function "=>" @operator)

; Binary operators
(binary_expression
  operator: (_) @operator)

; Pipe: `value |> transform`
(pipe_expression "|>" @operator)

; Ternary: `cond ? a : b`
(ternary_expression
  "?" @operator
  ":" @operator)

; ══════════════════════════════════════════════════════════════
; TEMPLATE / HTML
; ══════════════════════════════════════════════════════════════

; HTML element tags
(element
  tag: (tag_name) @tag)
(self_closing_element
  tag: (tag_name) @tag)

; Closing tag name
(element
  (tag_name) @tag)

; Attributes: `class="container"`
(attribute
  name: (attribute_name) @attribute)

; Directives: `bind:value`, `on:click`, `form:action`
(directive
  name: (directive_name) @attribute.special)

; Interpolation braces: `{expression}`
(expression_interpolation
  "{" @punctuation.special
  "}" @punctuation.special)

; Head block fields: `title: "Page"`
(head_field
  key: (identifier) @property)

; Slot
(slot_node "slot" @tag)

; ══════════════════════════════════════════════════════════════
; IMPORTS
; ══════════════════════════════════════════════════════════════

(use_declaration
  "use" @keyword.import
  "from" @keyword.import)

(use_declaration
  (identifier) @variable)

(use_declaration
  (string_literal) @string.special.path)

; ══════════════════════════════════════════════════════════════
; TYPES
; ══════════════════════════════════════════════════════════════

; Named types (PascalCase identifiers)
(type_identifier) @type

; Generic type parameters: `Array<T>`
(generic_type
  (type_identifier) @type
  "<" @punctuation.bracket
  ">" @punctuation.bracket)

; Array type suffix: `User[]`
(array_type "[" @punctuation.bracket "]" @punctuation.bracket)

; Union type separator: `string | null`
(union_type "|" @operator)

; Function type arrow: `(string) -> bool`
(function_type "->" @operator)

; Type field in object type: `{ name: string }`
(type_field
  name: (identifier) @property
  "?" @operator)

; Object field in literal: `{ key: value }`
(object_field
  key: (identifier) @property)

; ══════════════════════════════════════════════════════════════
; KEYWORDS (generic — matched AFTER more specific patterns above)
; ══════════════════════════════════════════════════════════════

; Binding keywords
["let" "mut"] @keyword.declaration

; Control flow keywords
["if" "else" "when" "each" "suspend" "empty"] @keyword.control

; Other declaration keywords (fallback for those not captured above)
["guard" "action" "query" "store" "channel"
 "type" "enum" "test" "middleware" "env"] @keyword.type

["fn"] @keyword.function

; Reactivity
["effect" "watch"] @keyword

; Boundary fallback
["server" "client" "shared"] @keyword.modifier

; Module
["use" "out" "from"] @keyword.import

; Other
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
(template_substitution
  "${" @punctuation.special
  "}" @punctuation.special)
(regex_literal) @string.regex

; ══════════════════════════════════════════════════════════════
; COMMENTS
; ══════════════════════════════════════════════════════════════

(line_comment) @comment
(block_comment) @comment

; ══════════════════════════════════════════════════════════════
; OPERATORS (generic fallback)
; ══════════════════════════════════════════════════════════════

(operator) @operator

["=>" "->" "<->" "|>" ".." "..."
 "=" "+=" "-="
 "!" "?" "?."] @operator

; ══════════════════════════════════════════════════════════════
; PUNCTUATION
; ══════════════════════════════════════════════════════════════

["{" "}" "(" ")" "[" "]"] @punctuation.bracket
["<" ">" "</" "/>"] @punctuation.bracket
["," "." ":" ";"] @punctuation.delimiter

; ══════════════════════════════════════════════════════════════
; IDENTIFIERS (fallback — must be last)
; ══════════════════════════════════════════════════════════════

(identifier) @variable
