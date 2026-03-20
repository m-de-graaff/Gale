; GaleX indentation queries for Zed.
;
; @indent marks where the indent level should increase.
; @end marks where it should decrease.
; @outdent marks tokens that should be outdented on the line they appear.

; ── Code blocks ───────────────────────────────────────────────
(block "{" @indent "}" @outdent)

; ── Component / API bodies ────────────────────────────────────
(component_body "{" @indent "}" @outdent)
(api_body "{" @indent "}" @outdent)

; ── Declaration blocks ────────────────────────────────────────
(guard_declaration "{" @indent "}" @outdent)
(store_declaration "{" @indent "}" @outdent)
(enum_declaration "{" @indent "}" @outdent)
(env_declaration "{" @indent "}" @outdent)
(channel_declaration "{" @indent "}" @outdent)

; ── Template elements ─────────────────────────────────────────
; Indent after open tag, outdent at close tag
(element ">" @indent "</" @outdent)

; ── Template control flow ─────────────────────────────────────
(when_block "{" @indent "}" @outdent)
(each_block "{" @indent "}" @outdent)
(suspend_block "{" @indent "}" @outdent)

; ── Head block ────────────────────────────────────────────────
(head_block "{" @indent "}" @outdent)

; ── Data structures ───────────────────────────────────────────
(array_literal "[" @indent "]" @outdent)
(object_literal "{" @indent "}" @outdent)
(object_type "{" @indent "}" @outdent)

; ── Parameter lists (multi-line) ──────────────────────────────
(parameter_list "(" @indent ")" @outdent)

; ── If/else chains ────────────────────────────────────────────
(if_statement "if" @indent)
