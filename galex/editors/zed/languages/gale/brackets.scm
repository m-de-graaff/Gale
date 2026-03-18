; GaleX bracket matching queries

("{" @open "}" @close)
("(" @open ")" @close)
("[" @open "]" @close)
("<" @open ">" @close)
(("\"" @open "\"" @close) (#set! rainbow.exclude))
