; GaleX injection queries for Zed.
;
; Injections tell Zed to use a different grammar for embedded content.
; Currently GaleX doesn't embed other languages inline, but this file
; is ready for future extensions (e.g., CSS in style blocks, SQL in
; tagged template literals).

; Regex literals could be highlighted with the regex grammar
(regex_literal) @content
(#set! "language" "regex")
