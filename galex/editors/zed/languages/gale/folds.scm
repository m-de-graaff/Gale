; GaleX folding queries
; Every brace-delimited and tag-delimited construct can be folded.

; ── Declarations ──────────────────────────────────────────────
(guard_declaration) @fold
(store_declaration) @fold
(enum_declaration) @fold
(env_declaration) @fold
(action_declaration) @fold
(function_declaration) @fold
(middleware_declaration) @fold
(test_declaration) @fold
(channel_declaration) @fold
(query_declaration) @fold
(type_alias_declaration) @fold

; ── Components / Layouts / APIs ───────────────────────────────
(component_declaration) @fold
(layout_declaration) @fold
(api_declaration) @fold
(component_body) @fold
(api_body) @fold

; ── Boundary blocks ───────────────────────────────────────────
(boundary_block) @fold

; ── Code blocks ───────────────────────────────────────────────
(block) @fold
(head_block) @fold

; ── Template control flow ─────────────────────────────────────
(when_block) @fold
(each_block) @fold
(suspend_block) @fold

; ── HTML elements (multi-line) ────────────────────────────────
(element) @fold

; ── Data structures ───────────────────────────────────────────
(object_literal) @fold
(array_literal) @fold
(object_type) @fold

; ── Comments ──────────────────────────────────────────────────
(block_comment) @fold
