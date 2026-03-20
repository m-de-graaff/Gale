; GaleX bracket matching queries for Zed.

("{" @open "}" @close)
("(" @open ")" @close)
("[" @open "]" @close)

; HTML tags — match open and close tags
(element
  "<" @open
  ">" @close)

(element
  "</" @open
  ">" @close)

; Template interpolation
(expression_interpolation
  "{" @open
  "}" @close)

; Template substitution in template literals
(template_substitution
  "${" @open
  "}" @close)

; Strings — exclude from rainbow brackets
(("\"" @open "\"" @close) (#set! "rainbow.exclude"))
(("'" @open "'" @close) (#set! "rainbow.exclude"))
(("`" @open "`" @close) (#set! "rainbow.exclude"))
