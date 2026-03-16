# Phase 8.3 — Error Recovery & Diagnostics Plan

## Already done (from 8.1/8.2)
- LexError enum with 7 variants, all with Span
- Error recovery (skip/continue on all error types)
- Vec<LexError> accumulation in Lexer and LexResult

## To implement: Rich diagnostic formatter

### New file: `galex/src/diagnostic.rs` (~200 lines)

DiagnosticRenderer struct:
- `new(source, file_table, color)` — precomputes line_starts index
- `render(&LexError) -> String` — full formatted error with source context
- `render_all(&[LexError]) -> String` — all errors joined
- `extract_line(line_num) -> &str` — source line by 1-based number
- `render_caret(col, len) -> String` — underline/caret

Internal Style helper for optional ANSI:
- `red_bold(s)`, `blue(s)`, `red(s)` — wrap with escape codes when color=true

### Modify: `galex/src/error.rs` (~40 lines added)

Add to LexError:
- `error_code() -> &'static str` — GX0001-GX0007
- `message() -> String` — short description
- `hint() -> &'static str` — contextual help after caret

Error codes:
- GX0001: UnexpectedCharacter
- GX0002: UnterminatedString
- GX0003: UnterminatedTemplateLiteral
- GX0004: UnterminatedBlockComment
- GX0005: UnterminatedRegex
- GX0006: InvalidEscapeSequence
- GX0007: InvalidNumberLiteral

### Modify: `galex/src/lib.rs` (~2 lines)

Add `pub mod diagnostic;` and re-export `DiagnosticRenderer`.

### New file: `galex/tests/diagnostics.rs` (~120 lines)

8 tests:
1. Single error renders with file path, line, source, caret, code
2. Multi-line source — error on line 3
3. Multi-error — two errors both rendered
4. Caret width — multi-char span
5. Color mode — ANSI codes present/absent
6. All error codes unique
7. Edge cases — line 1 col 1, EOF, empty line
8. Unicode source — caret alignment
