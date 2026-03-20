; GaleX outline queries — breadcrumbs and symbol outline panel.
;
; Each @item rule produces an entry in Zed's outline. The @name capture
; is displayed as the symbol name, and @context provides the keyword prefix.

; ── Guards ────────────────────────────────────────────────────
(guard_declaration
  "guard" @context
  name: (_) @name) @item

; Guard fields appear nested under their parent guard
(guard_field
  name: (identifier) @name) @item

; ── Components ────────────────────────────────────────────────
(out_declaration
  "out" @context
  (component_declaration
    "ui" @context
    name: (_) @name)) @item

; ── Layouts ───────────────────────────────────────────────────
(out_declaration
  "out" @context
  (layout_declaration
    "layout" @context
    name: (_) @name)) @item

; ── API resources ─────────────────────────────────────────────
(out_declaration
  "out" @context
  (api_declaration
    "api" @context
    name: (_) @name)) @item

; API handlers appear nested
(api_handler
  method: (identifier) @name) @item

; ── Functions ─────────────────────────────────────────────────
(function_declaration
  "fn" @context
  name: (_) @name) @item

; ── Actions ───────────────────────────────────────────────────
(action_declaration
  "action" @context
  name: (_) @name) @item

; ── Stores ────────────────────────────────────────────────────
(store_declaration
  "store" @context
  name: (_) @name) @item

; ── Channels ──────────────────────────────────────────────────
(channel_declaration
  "channel" @context
  name: (_) @name) @item

; Channel event handlers appear nested
(channel_handler
  "on" @context
  event: (identifier) @name) @item

; ── Queries ───────────────────────────────────────────────────
(query_declaration
  "query" @context
  name: (_) @name) @item

; ── Enums ─────────────────────────────────────────────────────
(enum_declaration
  "enum" @context
  name: (_) @name) @item

; ── Type aliases ──────────────────────────────────────────────
(type_alias_declaration
  "type" @context
  name: (_) @name) @item

; ── Middleware ─────────────────────────────────────────────────
(middleware_declaration
  "middleware" @context
  name: (_) @name) @item

; ── Tests ─────────────────────────────────────────────────────
(test_declaration
  "test" @context
  name: (string_literal) @name) @item

; ── Signals (inside components) ───────────────────────────────
(signal_statement
  "signal" @context
  name: (identifier) @name) @item

; ── Derives (inside components/stores) ────────────────────────
(derive_statement
  "derive" @context
  name: (identifier) @name) @item

; ── Boundary blocks ───────────────────────────────────────────
(boundary_block
  boundary: (_) @name) @item

; ── Environment declarations ──────────────────────────────────
(env_declaration
  "env" @context) @item

; ── Head block (inside components) ────────────────────────────
(head_block
  "head" @context) @item
