; GaleX semantic token overrides for Zed.
;
; These queries identify scopes where Zed should use the LSP's semantic
; tokens instead of the tree-sitter highlights. This improves accuracy
; for type-aware coloring (e.g., distinguishing a signal from a parameter).

; Inside function/action bodies — prefer semantic tokens for identifiers
(block
  (identifier) @_semantic)

; Inside expression interpolation — prefer semantic tokens
(expression_interpolation
  (identifier) @_semantic)

; Inside component bodies — signals/derives benefit from semantic coloring
(component_body
  (identifier) @_semantic)
