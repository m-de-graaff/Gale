# Phase 8.4 — Testing Plan

## Decisions
- LAngle: Keep for future parser use, add doc comment
- Fuzz: proptest (CI-friendly property-based testing)
- Benchmark: Criterion with programmatically generated 10k-line fixture

## Tasks

### 1. Cargo.toml changes
- Add `proptest = "1"` and `criterion = { version = "0.5", features = ["html_reports"] }` to [dev-dependencies]
- Add `[[bench]]` section for lexer_bench

### 2. token.rs — LAngle documentation
- Add doc comment: "Reserved for parser-driven generics (e.g., `Array<string>`). Not produced by lexer."

### 3. tests/comments.rs — strengthen error tests
- Assert specific LexError variant in each error test
- Add recovery verification for UnterminatedBlockComment, UnterminatedTemplateLiteral, UnterminatedRegex
- Add bad-digit tests: 0xZZ, 0b22

### 4. tests/spans.rs — NEW
- Single-char operators span accuracy
- Multi-char operators span accuracy
- String literal spans
- Number literal spans
- Template literal spans
- Multi-line span accuracy
- Comment spans
- Template mode: HtmlOpen, directive spans

### 5. tests/fuzz.rs — NEW
- proptest: random ASCII strings → no panic, ends with EOF
- proptest: random bytes → no panic
- proptest: random valid-looking GaleX → no panic
- proptest: template mode random input → no panic

### 6. benches/lexer_bench.rs — NEW
- Generate 10k-line .gx fixture mixing declarations, functions, strings, templates, comments
- lex_10k_lines benchmark
- lex_1k_lines benchmark
- lex_template_heavy benchmark (1k template-mode lines)

## Execution order
1. Cargo.toml deps
2. token.rs doc
3. comments.rs error tests
4. spans.rs
5. fuzz.rs
6. lexer_bench.rs
7. cargo test
8. cargo bench
9. cargo clippy
