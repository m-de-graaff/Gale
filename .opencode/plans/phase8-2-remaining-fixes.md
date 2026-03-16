# Phase 8.2 — Remaining Fixes Plan

## Fix 1: `\{` escape sequence support

**File:** `galex/src/lexer/string.rs:192-206`

Add `Some('{') => Ok('{')` to `lex_escape_sequence()` match arms, after the existing `Some('`') => Ok('`')` arm.

**Tests to add (in `galex/tests/literals.rs`):**
- `string_escaped_brace`: `"\{"` → `StringLit("{")`
- `template_escaped_dollar_brace`: `` `\${name}` `` → `TemplateNoSub("${name}")`

## Fix 2: Unicode identifiers via `unicode-xid`

**Dependency:** Add `unicode-xid = "0.2"` to `galex/Cargo.toml` under `[dependencies]`.

**Files to modify:**

### `galex/src/lexer/code.rs`
- Line 42: Change `'a'..='z' | 'A'..='Z' | '_'` to `ch if UnicodeXID::is_xid_start(ch) || ch == '_'`
- Line 69: Change `ch.is_ascii_alphanumeric() || ch == '_'` to `UnicodeXID::is_xid_continue(ch) || ch == '_'`
- Add `use unicode_xid::UnicodeXID;` at top

### `galex/src/lexer/template.rs`
- Line 70 (template dispatch): Change `ch.is_ascii_alphabetic() || ch == '_'` to `UnicodeXID::is_xid_start(ch) || ch == '_'`
- Line 186 (template identifier eat_while): Change `ch.is_ascii_alphanumeric() || ch == '_'` to `UnicodeXID::is_xid_continue(ch) || ch == '_'`
- Add `use unicode_xid::UnicodeXID;` at top

**Tests to add (in `galex/tests/keywords.rs`):**
- `unicode_identifiers`: Test `café`, `名前`, `_привет` as valid `Ident` tokens
- `unicode_identifier_not_keyword`: Ensure Unicode identifiers don't match keywords
- `emoji_not_identifier`: Ensure `🎉` is rejected (not XID_Start)

## Fix 3: Test gap coverage

### `galex/tests/operators.rs` — add:
- `bare_question_mark`: `?` → `Token::Question`
- `question_disambiguate`: `? ?. ??` → `Question, QuestionDot, NullCoalesce`

### `galex/tests/literals.rs` — add:
- `string_carriage_return_escape`: `"\r"` → `StringLit("\r")`
- `regex_slash_in_char_class`: `/[a/b]/` → `RegexLit { pattern: "[a/b]" ... }`

### `galex/tests/comments.rs` — add:
- `block_comment_deeply_nested`: `/* /* /* deep */ mid */ outer */` → single `BlockComment`

## Execution Order

1. Add `unicode-xid` dependency to `galex/Cargo.toml`
2. Fix `\{` escape in `string.rs`
3. Update identifier scanning in `code.rs` and `template.rs` for Unicode
4. Add all new tests
5. `cargo test -p galex`
6. `cargo clippy -p galex -- -D warnings`
7. Verify full workspace: `cargo test`
