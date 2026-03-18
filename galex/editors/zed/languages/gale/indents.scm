; GaleX indentation queries

; Blocks
(block "{" @indent "}" @end)

; Component / API bodies
(component_body "{" @indent "}" @end)
(api_body "{" @indent "}" @end)

; Template elements
(element ">" @indent "</" @end)

; Declarations with braces
(guard_declaration "{" @indent "}" @end)
(store_declaration "{" @indent "}" @end)
(enum_declaration "{" @indent "}" @end)
(env_declaration "{" @indent "}" @end)

; Template control flow
(when_block "{" @indent "}" @end)
(each_block "{" @indent "}" @end)
(suspend_block "{" @indent "}" @end)

; Head block
(head_block "{" @indent "}" @end)

; Data structures
(array_literal "[" @indent "]" @end)
(object_literal "{" @indent "}" @end)
(object_type "{" @indent "}" @end)

; Parameter lists
(parameter_list "(" @indent ")" @end)
