# Phase 8: GaleX Lexer/Tokenizer — Implementation Plan

## Decisions Made

| Decision | Choice |
|----------|--------|
| Crate structure | Workspace member at `galex/` |
| Indent/Dedent | Dropped — GaleX uses braces |
| Directives | Compound tokens in template mode |
| Token authority | HTML reference (~111 tokens) |
| Template literals | Segmented (Head/Middle/Tail) |
| Regex literals | Included now with disambiguation |
| Dependencies | Zero (pure Rust, no external crates) |
| Error strategy | Accumulate errors, attempt recovery, return both tokens + errors |

---

## 1. Project Structure

Convert repo into Cargo workspace. Root stays as `gale` server; `galex/` is a new workspace member.

```
Gale/
├── Cargo.toml              # Add [workspace] members = ["galex"]
│
└── galex/
    ├── Cargo.toml          # name = "galex", edition = "2021", no deps
    ├── src/
    │   ├── lib.rs          # Public API: lex(), Lexer, re-exports
    │   ├── token.rs        # Token enum (~111 variants), keyword helpers
    │   ├── span.rs         # Span struct, Display impl
    │   ├── error.rs        # LexError enum with Span
    │   └── lexer/
    │       ├── mod.rs      # Lexer struct, mode stack, next_token(), lex()
    │       ├── cursor.rs   # Low-level char iteration, peek, advance, position
    │       ├── code.rs     # Code-mode: identifiers, keywords, operators, delimiters, comments
    │       ├── number.rs   # Integer/float literals (hex, binary, separators)
    │       ├── string.rs   # String literals, template literals (segmented), escape sequences
    │       ├── regex.rs    # Regex literal lexing with division disambiguation
    │       └── template.rs # Template-mode: HTML tags, directives, HtmlText
    │
    └── tests/
        ├── keywords.rs     # All 41+ keywords lex correctly
        ├── operators.rs    # All operators including multi-char
        ├── literals.rs     # Strings, numbers, bools, null, regex
        ├── templates.rs    # HTML tags, directives, template literals
        ├── comments.rs     # Line and block comments (nestable)
        ├── errors.rs       # Error recovery, unterminated strings, etc.
        └── integration.rs  # Full .gx file tokenization (Button example)
```

---

## 2. Token Enum (~111 variants)

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // ── Keywords — bindings (6) ────────────────
    Let, Mut, Signal, Derive, Frozen, Ref,

    // ── Keywords — functions & control (6) ─────
    Fn, Return, If, Else, For, Await,

    // ── Keywords — blocks & boundaries (3) ─────
    Server, Client, Shared,

    // ── Keywords — declarations (8) ────────────
    Guard, Action, Query, Store, Channel, Type, Enum, Test,

    // ── Keywords — reactivity & effects (3) ────
    Effect, Watch, Bind,

    // ── Keywords — template control flow (5) ───
    When, Each, Suspend, Slot, Empty,

    // ── Keywords — modules & export (4) ────────
    Use, Out, Ui, Api,

    // ── Keywords — page features (6) ──────────
    Head, Redirect, Middleware, Env, Link, Transition,

    // ── Keywords — other (2) ──────────────────
    Assert, On,

    // ── Operators — arithmetic (5) ─────────────
    Plus,       // +
    Minus,      // -
    Star,       // *
    Slash,      // /
    Percent,    // %

    // ── Operators — comparison (6) ─────────────
    EqEq,       // ==
    NotEq,      // !=
    Less,       // <        (in Code mode only)
    Greater,    // >        (in Code mode only)
    LessEq,     // <=
    GreaterEq,  // >=

    // ── Operators — logical (4) ────────────────
    And,            // &&
    Or,             // ||
    Not,            // !
    NullCoalesce,   // ??

    // ── Operators — assignment (3) ─────────────
    Eq,         // =
    PlusEq,     // +=
    MinusEq,    // -=

    // ── Operators — special (7) ────────────────
    Arrow,      // ->
    FatArrow,   // =>
    BiArrow,    // <->
    Spread,     // ...
    DotDot,     // ..
    Pipe,       // |>
    QuestionDot,// ?.

    // ── Delimiters (14) ───────────────────────
    LParen,     // (
    RParen,     // )
    LBrace,     // {
    RBrace,     // }
    LBracket,   // [
    RBracket,   // ]
    LAngle,     // <        (template mode: only when not starting a tag)
    RAngle,     // >
    Colon,      // :
    Semicolon,  // ;
    Comma,      // ,
    Dot,        // .
    At,         // @
    Hash,       // #

    // ── Literals ──────────────────────────────
    StringLit(String),
    IntLit(i64),
    FloatLit(f64),
    BoolLit(bool),
    NullLit,
    RegexLit { pattern: String, flags: String },

    // ── Template literal segments ─────────────
    TemplateNoSub(String),   // `text`        (no interpolation)
    TemplateHead(String),    // `text${       (before first interpolation)
    TemplateMiddle(String),  // }text${       (between interpolations)
    TemplateTail(String),    // }text`        (after last interpolation)

    // ── Template tokens (template mode) ───────
    HtmlOpen(String),        // <tagname
    HtmlClose(String),       // </tagname>
    HtmlSelfClose,           // />
    HtmlText(String),        // "text" inside template
    ExprOpen,                // { in template context
    ExprClose,               // } in template context

    // ── Directives (compound, template mode) ──
    BindDir(String),         // bind:x
    OnDir { event: String, modifiers: Vec<String> },  // on:click.prevent.once
    ClassDir(String),        // class:name
    RefDir(String),          // ref:name
    TransDir(String),        // transition:type
    KeyDir,                  // key (= is separate)
    IntoDir(String),         // into:slot
    FormAction,              // form:action
    FormGuard,               // form:guard
    FormError,               // form:error
    Prefetch,                // prefetch

    // ── Special ───────────────────────────────
    Ident(String),
    Newline,
    Comment(String),
    BlockComment(String),
    EOF,
}
```

---

## 3. Span Struct

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub file_id: u32,   // Index into a shared FileTable (avoids cloning PathBuf per token)
    pub start: u32,     // Byte offset in source
    pub end: u32,       // Byte offset (exclusive)
    pub line: u32,      // 1-based line number
    pub col: u32,       // 1-based column (byte offset from line start)
}

pub type TokenWithSpan = (Token, Span);

/// Shared file table — maps file_id to PathBuf
pub struct FileTable {
    files: Vec<std::path::PathBuf>,
}
```

---

## 4. Error Types

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum LexError {
    UnterminatedString { span: Span },
    UnterminatedTemplateLiteral { span: Span },
    UnterminatedBlockComment { span: Span },
    UnterminatedRegex { span: Span },
    InvalidEscapeSequence { span: Span, sequence: char },
    InvalidNumberLiteral { span: Span, reason: String },
    UnexpectedCharacter { span: Span, ch: char },
    NestedTemplateLiteralDepthExceeded { span: Span },
}

pub struct LexResult {
    pub tokens: Vec<TokenWithSpan>,
    pub errors: Vec<LexError>,
}
```

---

## 5. Lexer Architecture

### Cursor

Low-level character stream with position tracking:

```rust
pub(crate) struct Cursor<'src> {
    source: &'src str,
    chars: std::str::CharIndices<'src>,
    pos: usize,          // current byte offset
    line: u32,           // current line (1-based)
    col: u32,            // current column (1-based)
    peeked: Option<(usize, char)>,
}

impl Cursor {
    fn peek(&self) -> Option<char>;
    fn peek_second(&self) -> Option<char>;   // 2-char lookahead
    fn peek_third(&self) -> Option<char>;    // 3-char lookahead (for ..., <->)
    fn advance(&mut self) -> Option<char>;
    fn eat_if(&mut self, predicate: fn(char) -> bool) -> bool;
    fn eat_while(&mut self, predicate: fn(char) -> bool) -> &str;
    fn pos(&self) -> usize;
    fn line(&self) -> u32;
    fn col(&self) -> u32;
    fn is_eof(&self) -> bool;
}
```

### Mode Stack

```rust
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum LexMode {
    Code,
    Template,
    HtmlTag { tag_name: String },
    TemplateLiteral,
    TemplateExpr { depth: u32 },
}
```

### Lexer Main Loop

```rust
pub struct Lexer<'src> {
    source: &'src str,
    cursor: Cursor<'src>,
    mode_stack: Vec<LexMode>,
    prev_token_kind: Option<TokenKind>,  // for regex disambiguation
    errors: Vec<LexError>,
    file_id: u32,
}

impl Lexer {
    pub fn next_token(&mut self) -> TokenWithSpan {
        match self.current_mode() {
            LexMode::Code => self.lex_code(),
            LexMode::Template => self.lex_template(),
            LexMode::HtmlTag { .. } => self.lex_html_tag(),
            LexMode::TemplateLiteral => self.lex_template_literal_continuation(),
            LexMode::TemplateExpr { .. } => self.lex_code(), // same as code, but tracks braces
        }
    }
}
```

---

## 6. Mode Transitions

| From | Trigger | To | Notes |
|------|---------|----|-------|
| Code | `` ` `` | push TemplateLiteral | Lex template string content |
| TemplateLiteral | `${` | push TemplateExpr { depth: 0 } | Lex expression inside template string |
| TemplateExpr | `{` | depth += 1 | Nested brace in expression |
| TemplateExpr | `}` at depth 0 | pop → TemplateLiteral | Return to template string |
| TemplateExpr | `}` at depth > 0 | depth -= 1 | Close nested brace |
| Template | `<` + letter | push HtmlTag | Start HTML open tag |
| Template | `</` | push HtmlTag | Start HTML close tag |
| HtmlTag | `>` | pop → Template | End tag, back to template body |
| HtmlTag | `/>` | pop → Template | Self-close, back to template body |
| Template | `{` | push TemplateExpr { depth: 0 } | Expression interpolation in template |
| TemplateExpr | `}` at depth 0 | pop → Template | Back to template body |

The parser drives Code → Template transitions by calling `push_mode(Template)` when entering a component body.

---

## 7. Regex Disambiguation

Track the previous token's "kind" (ignoring data). After tokens that can end an expression, `/` is Slash (division). Otherwise `/` starts a regex.

**Tokens that end an expression** (/ is division after these):
- `Ident`, `IntLit`, `FloatLit`, `StringLit`, `BoolLit`, `NullLit`, `RegexLit`
- `RParen`, `RBracket`, `RBrace`
- `TemplateNoSub`, `TemplateTail`

**All other tokens** (/ starts regex after these):
- `Eq`, `PlusEq`, `MinusEq`, operators, `LParen`, `LBracket`, `Comma`, `Semicolon`, keywords, etc.

---

## 8. Keyword Lookup

Simple match on &str after lexing an identifier. 43 keywords total:

```rust
fn lookup_keyword(ident: &str) -> Option<Token> {
    match ident {
        "let" => Some(Token::Let),
        "mut" => Some(Token::Mut),
        "signal" => Some(Token::Signal),
        // ... 40 more
        "true" => Some(Token::BoolLit(true)),
        "false" => Some(Token::BoolLit(false)),
        "null" => Some(Token::NullLit),
        _ => None,
    }
}
```

---

## 9. Number Literal Lexing

```
IntLit:   42, 0xFF, 0b1010, 1_000_000
FloatLit: 3.14, 0.5, 99.99
```

Strategy:
1. Starts with digit → enter number lexing
2. If `0x` → hex integer
3. If `0b` → binary integer
4. Otherwise decimal; if `.` followed by digit → float
5. Underscores are allowed as visual separators (stripped before parsing)
6. No leading-dot floats (`.5` is not valid — use `0.5`)

---

## 10. String Literal Escapes

Standard escapes in double-quoted strings:
- `\\`, `\"`, `\n`, `\r`, `\t`, `\0`
- `\x{HH}` — hex byte
- `\u{HHHH}` — Unicode codepoint

---

## 11. Template Literal Segmentation

Input: `` `Hello, ${name}! Count: ${count * 2}` ``

Produces:
1. `TemplateHead("Hello, ")`
2. `Ident("name")`
3. `TemplateMiddle("! Count: ")`
4. `Ident("count")`, `Star`, `IntLit(2)`
5. `TemplateTail("")`

Input: `` `no interpolation` ``

Produces:
1. `TemplateNoSub("no interpolation")`

---

## 12. Template Mode — HTML Tags & Directives

### HTML Tag Lexing (HtmlTag mode)

After `<tagname` (entering HtmlTag mode), lex attributes:
- Plain attributes: `Ident("class")`, `Eq`, `StringLit("value")` or `ExprOpen`, expr, `ExprClose`
- Directives recognized by prefix:
  - `bind:x` → `BindDir("x")`
  - `on:click` → `OnDir { event: "click", modifiers: [] }`
  - `on:click.prevent.once` → `OnDir { event: "click", modifiers: ["prevent", "once"] }`
  - `class:name` → `ClassDir("name")`
  - `ref:name` → `RefDir("name")`
  - `transition:fade` → `TransDir("fade")`
  - `into:header` → `IntoDir("header")`
  - `form:action` → `FormAction`
  - `form:guard` → `FormGuard`
  - `form:error` → `FormError`
  - `key` → `KeyDir`
  - `prefetch` → `Prefetch`
- Tag ends with `>` (pop to Template) or `/>` (emit HtmlSelfClose, pop to Template)

### Template Body

In Template mode:
- `<tag` → push HtmlTag, emit `HtmlOpen("tag")`
- `</tag>` → emit `HtmlClose("tag")`
- `"text"` → emit `HtmlText("text")`
- `{` → push TemplateExpr, emit `ExprOpen`
- Code keywords (`when`, `each`, `suspend`, `slot`, `derive`, `let`, `signal`, etc.) → lex as keywords
- Whitespace between elements → skip (not significant in template)

---

## 13. Implementation Order

| Step | Task | Files | Est. Lines |
|------|------|-------|-----------|
| 1 | Workspace setup | Root `Cargo.toml`, `galex/Cargo.toml` | ~20 |
| 2 | Token enum + Span | `token.rs`, `span.rs` | ~350 |
| 3 | Error types | `error.rs` | ~60 |
| 4 | Cursor (char stream) | `lexer/cursor.rs` | ~120 |
| 5 | Lexer skeleton + mode stack | `lexer/mod.rs` | ~150 |
| 6 | Code mode — identifiers + keywords | `lexer/code.rs` | ~200 |
| 7 | Code mode — operators + delimiters | `lexer/code.rs` (extend) | ~200 |
| 8 | Number literals | `lexer/number.rs` | ~150 |
| 9 | String literals + escapes | `lexer/string.rs` | ~150 |
| 10 | Template literals (segmented) | `lexer/string.rs` (extend) | ~100 |
| 11 | Regex literals + disambiguation | `lexer/regex.rs` | ~100 |
| 12 | Comments (line + nestable block) | `lexer/code.rs` (extend) | ~60 |
| 13 | Template mode — HTML tags | `lexer/template.rs` | ~200 |
| 14 | Template mode — directives | `lexer/template.rs` (extend) | ~150 |
| 15 | Lib re-exports + lex() | `lib.rs` | ~30 |
| 16 | Tests — keywords + operators | `tests/keywords.rs`, `tests/operators.rs` | ~200 |
| 17 | Tests — literals + comments | `tests/literals.rs`, `tests/comments.rs` | ~200 |
| 18 | Tests — templates + directives | `tests/templates.rs` | ~200 |
| 19 | Tests — error recovery | `tests/errors.rs` | ~100 |
| 20 | Integration test — Button.gx | `tests/integration.rs` | ~150 |
| 21 | Clippy + final polish | All files | — |

**Estimated total: ~2,400 lines** across ~15 files.

---

## 14. Open Considerations

- **Unicode identifiers**: Start ASCII-only (`[a-zA-Z_][a-zA-Z0-9_]*`). Add `unicode-xid` later if needed.
- **`<` ambiguity**: In Code mode → always `LAngle`/`Less`. In Template mode → `<` + letter starts HtmlTag. Parser calls `push_mode(Template)` for component bodies.
- **Newline handling**: Emit `Newline` tokens (significant as statement terminators). Consecutive newlines collapse to one. Whether newlines inside `()`, `[]`, `{}` are suppressed is deferred to the parser.
- **`|` token**: The HTML reference uses `|` in type unions (`"primary" | "ghost"`) but doesn't list it as a distinct token. It appears within string-union type annotations. We should add a `BitOr` / `TypeUnion` token for `|` if needed, or handle it in the parser. For now, lex `|` as a standalone token (maybe `Pipe` single-char? currently `|>` is `Pipe`). **Decision needed**: add `Bar` token for single `|`.
