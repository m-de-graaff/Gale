//! Token types for the GaleX language.
//!
//! Every lexeme produced by the lexer is represented as a [`Token`] variant.
//! Tokens are paired with [`Span`] information for error reporting.

use crate::span::Span;

/// A token paired with its source location.
pub type TokenWithSpan = (Token, Span);

/// All GaleX token types.
///
/// Organized by category: keywords, operators, delimiters, literals,
/// template tokens, directives, and special tokens.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // ── Keywords — bindings (6) ────────────────────────────────────────
    /// `let` — immutable binding
    Let,
    /// `mut` — mutable binding
    Mut,
    /// `signal` — reactive state primitive
    Signal,
    /// `derive` — computed reactive value
    Derive,
    /// `frozen` — deeply immutable binding
    Frozen,
    /// `ref` — DOM element reference
    Ref,

    // ── Keywords — functions & control (6) ─────────────────────────────
    /// `fn` — function declaration
    Fn,
    /// `return` — return from function
    Return,
    /// `if` — conditional branch
    If,
    /// `else` — else branch
    Else,
    /// `for` — loop iteration
    For,
    /// `await` — await async expression
    Await,

    // ── Keywords — blocks & boundaries (3) ─────────────────────────────
    /// `server` — server-only block
    Server,
    /// `client` — client-only block
    Client,
    /// `shared` — code compiled to both Rust and JS
    Shared,

    // ── Keywords — declarations (8) ────────────────────────────────────
    /// `guard` — type + runtime validator
    Guard,
    /// `action` — server mutation callable from client
    Action,
    /// `query` — reactive data fetching
    Query,
    /// `store` — shared reactive state container
    Store,
    /// `channel` — real-time typed WebSocket stream
    Channel,
    /// `type` — type alias
    Type,
    /// `enum` — enum declaration
    Enum,
    /// `test` — inline test block
    Test,

    // ── Keywords — reactivity & effects (3) ────────────────────────────
    /// `effect` — side effect with auto-tracking
    Effect,
    /// `watch` — observe expression changes
    Watch,
    /// `bind` — two-way data binding keyword
    Bind,

    // ── Keywords — template control flow (5) ───────────────────────────
    /// `when` — conditional rendering in templates
    When,
    /// `each` — loop rendering in templates
    Each,
    /// `suspend` — async boundary with fallback
    Suspend,
    /// `slot` — content injection point
    Slot,
    /// `empty` — fallback for empty `each` lists
    Empty,

    // ── Keywords — modules & export (4) ────────────────────────────────
    /// `use` — import declaration
    Use,
    /// `out` — export declaration
    Out,
    /// `ui` — UI component marker (used with `out`)
    Ui,
    /// `api` — API route marker (used with `out`)
    Api,

    // ── Keywords — page features (6) ──────────────────────────────────
    /// `head` — document head metadata
    Head,
    /// `redirect` — server-side redirect
    Redirect,
    /// `middleware` — request/response middleware
    Middleware,
    /// `env` — typed environment variable access
    Env,
    /// `link` — client-side navigation element
    Link,
    /// `transition` — animated enter/exit directive
    Transition,

    // ── Keywords — other (2) ──────────────────────────────────────────
    /// `assert` — test assertion (only in test blocks)
    Assert,
    /// `on` — channel lifecycle handler keyword
    On,

    // ── Operators — arithmetic (5) ─────────────────────────────────────
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `%`
    Percent,

    // ── Operators — comparison (6) ─────────────────────────────────────
    /// `==`
    EqEq,
    /// `!=`
    NotEq,
    /// `<` in code context
    Less,
    /// `>` in code context
    Greater,
    /// `<=`
    LessEq,
    /// `>=`
    GreaterEq,

    // ── Operators — logical (4) ────────────────────────────────────────
    /// `&&`
    And,
    /// `||`
    Or,
    /// `!`
    Not,
    /// `??`
    NullCoalesce,

    // ── Operators — assignment (3) ─────────────────────────────────────
    /// `=`
    Eq,
    /// `+=`
    PlusEq,
    /// `-=`
    MinusEq,

    /// `?` — ternary operator
    Question,

    // ── Operators — special (7) ────────────────────────────────────────
    /// `->`
    Arrow,
    /// `=>`
    FatArrow,
    /// `<->`
    BiArrow,
    /// `...`
    Spread,
    /// `..`
    DotDot,
    /// `|>`
    Pipe,
    /// `?.`
    QuestionDot,

    // ── Delimiters (15) ───────────────────────────────────────────────
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// `<` as delimiter in generics (e.g., `Array<string>`).
    ///
    /// **Reserved for parser use.** The lexer does not produce this token —
    /// in code mode `<` yields [`Less`], in template mode `<tag` yields
    /// [`HtmlOpen`]. The parser will re-classify `Less` as `LAngle` when
    /// it detects a generic type context.
    LAngle,
    /// `>` as delimiter
    RAngle,
    /// `:`
    Colon,
    /// `;`
    Semicolon,
    /// `,`
    Comma,
    /// `.`
    Dot,
    /// `@`
    At,
    /// `#`
    Hash,
    /// `|` — type union separator
    Bar,

    // ── Literals ──────────────────────────────────────────────────────
    /// `"string content"` — double-quoted string with escapes
    StringLit(String),
    /// `42`, `0xFF`, `0b1010`, `1_000_000`
    IntLit(i64),
    /// `3.14`, `0.5`
    FloatLit(f64),
    /// `true` or `false`
    BoolLit(bool),
    /// `null`
    NullLit,
    /// `/pattern/flags`
    RegexLit { pattern: String, flags: String },

    // ── Template literal segments ─────────────────────────────────────
    /// `` `complete text` `` — template literal with no interpolation
    TemplateNoSub(String),
    /// `` `text before ${ `` — head of interpolated template literal
    TemplateHead(String),
    /// `` }text between ${ `` — middle segment
    TemplateMiddle(String),
    /// `` }text after` `` — tail segment
    TemplateTail(String),

    // ── Template tokens (template mode) ───────────────────────────────
    /// `<tagname` — opening HTML/component tag
    HtmlOpen(String),
    /// `</tagname>` — closing HTML/component tag
    HtmlClose(String),
    /// `/>`  — self-closing tag terminator
    HtmlSelfClose,
    /// `"text"` — quoted text node inside template
    HtmlText(String),
    /// `{` in template context — start expression interpolation
    ExprOpen,
    /// `}` in template context — end expression interpolation
    ExprClose,

    // ── Directives (compound tokens, template mode) ───────────────────
    /// `bind:x` — two-way binding directive
    BindDir(String),
    /// `on:event.modifier1.modifier2` — event handler directive
    OnDir {
        event: String,
        modifiers: Vec<String>,
    },
    /// `class:name` — conditional CSS class toggle
    ClassDir(String),
    /// `ref:name` — DOM element reference binding
    RefDir(String),
    /// `transition:type` — enter/exit animation
    TransDir(String),
    /// `key` — unique key for list items
    KeyDir,
    /// `into:slot` — direct content into named slot
    IntoDir(String),
    /// `form:action` — bind form submission to server action
    FormAction,
    /// `form:guard` — apply guard validation to form
    FormGuard,
    /// `form:error` — display validation error
    FormError,
    /// `prefetch` — prefetch mode for link elements
    Prefetch,

    // ── Special ───────────────────────────────────────────────────────
    /// Any identifier that isn't a keyword
    Ident(String),
    /// Significant newline (statement terminator)
    Newline,
    /// `// comment text`
    Comment(String),
    /// `/* block comment */` (nestable)
    BlockComment(String),
    /// End of file
    EOF,
}

impl Token {
    /// Returns `true` if this token can appear at the end of an expression.
    /// Used for regex vs division disambiguation.
    pub fn can_end_expression(&self) -> bool {
        matches!(
            self,
            Token::Ident(_)
                | Token::IntLit(_)
                | Token::FloatLit(_)
                | Token::StringLit(_)
                | Token::BoolLit(_)
                | Token::NullLit
                | Token::RegexLit { .. }
                | Token::TemplateNoSub(_)
                | Token::TemplateTail(_)
                | Token::RParen
                | Token::RBracket
                | Token::RBrace
        )
    }

    /// Returns `true` if this token is a keyword.
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            Token::Let
                | Token::Mut
                | Token::Signal
                | Token::Derive
                | Token::Frozen
                | Token::Ref
                | Token::Fn
                | Token::Return
                | Token::If
                | Token::Else
                | Token::For
                | Token::Await
                | Token::Server
                | Token::Client
                | Token::Shared
                | Token::Guard
                | Token::Action
                | Token::Query
                | Token::Store
                | Token::Channel
                | Token::Type
                | Token::Enum
                | Token::Test
                | Token::Effect
                | Token::Watch
                | Token::Bind
                | Token::When
                | Token::Each
                | Token::Suspend
                | Token::Slot
                | Token::Empty
                | Token::Use
                | Token::Out
                | Token::Ui
                | Token::Api
                | Token::Head
                | Token::Redirect
                | Token::Middleware
                | Token::Env
                | Token::Link
                | Token::Transition
                | Token::Assert
                | Token::On
        )
    }

    /// Returns a human-readable description of this token kind for error messages.
    pub fn kind_str(&self) -> &'static str {
        match self {
            Token::Let => "keyword `let`",
            Token::Mut => "keyword `mut`",
            Token::Signal => "keyword `signal`",
            Token::Derive => "keyword `derive`",
            Token::Frozen => "keyword `frozen`",
            Token::Ref => "keyword `ref`",
            Token::Fn => "keyword `fn`",
            Token::Return => "keyword `return`",
            Token::If => "keyword `if`",
            Token::Else => "keyword `else`",
            Token::For => "keyword `for`",
            Token::Await => "keyword `await`",
            Token::Server => "keyword `server`",
            Token::Client => "keyword `client`",
            Token::Shared => "keyword `shared`",
            Token::Guard => "keyword `guard`",
            Token::Action => "keyword `action`",
            Token::Query => "keyword `query`",
            Token::Store => "keyword `store`",
            Token::Channel => "keyword `channel`",
            Token::Type => "keyword `type`",
            Token::Enum => "keyword `enum`",
            Token::Test => "keyword `test`",
            Token::Effect => "keyword `effect`",
            Token::Watch => "keyword `watch`",
            Token::Bind => "keyword `bind`",
            Token::When => "keyword `when`",
            Token::Each => "keyword `each`",
            Token::Suspend => "keyword `suspend`",
            Token::Slot => "keyword `slot`",
            Token::Empty => "keyword `empty`",
            Token::Use => "keyword `use`",
            Token::Out => "keyword `out`",
            Token::Ui => "keyword `ui`",
            Token::Api => "keyword `api`",
            Token::Head => "keyword `head`",
            Token::Redirect => "keyword `redirect`",
            Token::Middleware => "keyword `middleware`",
            Token::Env => "keyword `env`",
            Token::Link => "keyword `link`",
            Token::Transition => "keyword `transition`",
            Token::Assert => "keyword `assert`",
            Token::On => "keyword `on`",
            Token::Plus => "`+`",
            Token::Minus => "`-`",
            Token::Star => "`*`",
            Token::Slash => "`/`",
            Token::Percent => "`%`",
            Token::EqEq => "`==`",
            Token::NotEq => "`!=`",
            Token::Less => "`<`",
            Token::Greater => "`>`",
            Token::LessEq => "`<=`",
            Token::GreaterEq => "`>=`",
            Token::And => "`&&`",
            Token::Or => "`||`",
            Token::Not => "`!`",
            Token::NullCoalesce => "`??`",
            Token::Question => "`?`",
            Token::Eq => "`=`",
            Token::PlusEq => "`+=`",
            Token::MinusEq => "`-=`",
            Token::Arrow => "`->`",
            Token::FatArrow => "`=>`",
            Token::BiArrow => "`<->`",
            Token::Spread => "`...`",
            Token::DotDot => "`..`",
            Token::Pipe => "`|>`",
            Token::QuestionDot => "`?.`",
            Token::LParen => "`(`",
            Token::RParen => "`)`",
            Token::LBrace => "`{`",
            Token::RBrace => "`}`",
            Token::LBracket => "`[`",
            Token::RBracket => "`]`",
            Token::LAngle => "`<`",
            Token::RAngle => "`>`",
            Token::Colon => "`:`",
            Token::Semicolon => "`;`",
            Token::Comma => "`,`",
            Token::Dot => "`.`",
            Token::At => "`@`",
            Token::Hash => "`#`",
            Token::Bar => "`|`",
            Token::StringLit(_) => "string literal",
            Token::IntLit(_) => "integer literal",
            Token::FloatLit(_) => "float literal",
            Token::BoolLit(_) => "boolean literal",
            Token::NullLit => "`null`",
            Token::RegexLit { .. } => "regex literal",
            Token::TemplateNoSub(_) => "template literal",
            Token::TemplateHead(_) => "template literal head",
            Token::TemplateMiddle(_) => "template literal middle",
            Token::TemplateTail(_) => "template literal tail",
            Token::HtmlOpen(_) => "HTML open tag",
            Token::HtmlClose(_) => "HTML close tag",
            Token::HtmlSelfClose => "`/>`",
            Token::HtmlText(_) => "HTML text",
            Token::ExprOpen => "`{` (expression)",
            Token::ExprClose => "`}` (expression)",
            Token::BindDir(_) => "`bind:` directive",
            Token::OnDir { .. } => "`on:` directive",
            Token::ClassDir(_) => "`class:` directive",
            Token::RefDir(_) => "`ref:` directive",
            Token::TransDir(_) => "`transition:` directive",
            Token::KeyDir => "`key` directive",
            Token::IntoDir(_) => "`into:` directive",
            Token::FormAction => "`form:action` directive",
            Token::FormGuard => "`form:guard` directive",
            Token::FormError => "`form:error` directive",
            Token::Prefetch => "`prefetch` directive",
            Token::Ident(_) => "identifier",
            Token::Newline => "newline",
            Token::Comment(_) => "comment",
            Token::BlockComment(_) => "block comment",
            Token::EOF => "end of file",
        }
    }
}

/// Look up a keyword from an identifier string.
/// Returns `None` if the string is not a keyword (it's a plain identifier).
pub fn lookup_keyword(ident: &str) -> Option<Token> {
    match ident {
        // Bindings
        "let" => Some(Token::Let),
        "mut" => Some(Token::Mut),
        "signal" => Some(Token::Signal),
        "derive" => Some(Token::Derive),
        "frozen" => Some(Token::Frozen),
        "ref" => Some(Token::Ref),
        // Functions & control
        "fn" => Some(Token::Fn),
        "return" => Some(Token::Return),
        "if" => Some(Token::If),
        "else" => Some(Token::Else),
        "for" => Some(Token::For),
        "await" => Some(Token::Await),
        // Blocks & boundaries
        "server" => Some(Token::Server),
        "client" => Some(Token::Client),
        "shared" => Some(Token::Shared),
        // Declarations
        "guard" => Some(Token::Guard),
        "action" => Some(Token::Action),
        "query" => Some(Token::Query),
        "store" => Some(Token::Store),
        "channel" => Some(Token::Channel),
        "type" => Some(Token::Type),
        "enum" => Some(Token::Enum),
        "test" => Some(Token::Test),
        // Reactivity & effects
        "effect" => Some(Token::Effect),
        "watch" => Some(Token::Watch),
        "bind" => Some(Token::Bind),
        // Template control flow
        "when" => Some(Token::When),
        "each" => Some(Token::Each),
        "suspend" => Some(Token::Suspend),
        "slot" => Some(Token::Slot),
        "empty" => Some(Token::Empty),
        // Modules & export
        "use" => Some(Token::Use),
        "out" => Some(Token::Out),
        "ui" => Some(Token::Ui),
        "api" => Some(Token::Api),
        // Page features
        "head" => Some(Token::Head),
        "redirect" => Some(Token::Redirect),
        "middleware" => Some(Token::Middleware),
        "env" => Some(Token::Env),
        "link" => Some(Token::Link),
        "transition" => Some(Token::Transition),
        // Other keywords
        "assert" => Some(Token::Assert),
        "on" => Some(Token::On),
        // Literal keywords
        "true" => Some(Token::BoolLit(true)),
        "false" => Some(Token::BoolLit(false)),
        "null" => Some(Token::NullLit),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_lookup_finds_all_keywords() {
        let keywords = [
            ("let", Token::Let),
            ("mut", Token::Mut),
            ("signal", Token::Signal),
            ("derive", Token::Derive),
            ("frozen", Token::Frozen),
            ("ref", Token::Ref),
            ("fn", Token::Fn),
            ("return", Token::Return),
            ("if", Token::If),
            ("else", Token::Else),
            ("for", Token::For),
            ("await", Token::Await),
            ("server", Token::Server),
            ("client", Token::Client),
            ("shared", Token::Shared),
            ("guard", Token::Guard),
            ("action", Token::Action),
            ("query", Token::Query),
            ("store", Token::Store),
            ("channel", Token::Channel),
            ("type", Token::Type),
            ("enum", Token::Enum),
            ("test", Token::Test),
            ("effect", Token::Effect),
            ("watch", Token::Watch),
            ("bind", Token::Bind),
            ("when", Token::When),
            ("each", Token::Each),
            ("suspend", Token::Suspend),
            ("slot", Token::Slot),
            ("empty", Token::Empty),
            ("use", Token::Use),
            ("out", Token::Out),
            ("ui", Token::Ui),
            ("api", Token::Api),
            ("head", Token::Head),
            ("redirect", Token::Redirect),
            ("middleware", Token::Middleware),
            ("env", Token::Env),
            ("link", Token::Link),
            ("transition", Token::Transition),
            ("assert", Token::Assert),
            ("on", Token::On),
            ("true", Token::BoolLit(true)),
            ("false", Token::BoolLit(false)),
            ("null", Token::NullLit),
        ];
        for (text, expected) in &keywords {
            assert_eq!(
                lookup_keyword(text).as_ref(),
                Some(expected),
                "keyword lookup failed for {:?}",
                text
            );
        }
    }

    #[test]
    fn non_keywords_return_none() {
        assert_eq!(lookup_keyword("foo"), None);
        assert_eq!(lookup_keyword("myVar"), None);
        assert_eq!(lookup_keyword("Signal"), None); // case-sensitive
        assert_eq!(lookup_keyword("LET"), None);
    }

    #[test]
    fn expression_ending_tokens() {
        assert!(Token::Ident("x".into()).can_end_expression());
        assert!(Token::IntLit(42).can_end_expression());
        assert!(Token::RParen.can_end_expression());
        assert!(Token::RBracket.can_end_expression());
        assert!(Token::RBrace.can_end_expression());
        assert!(Token::BoolLit(true).can_end_expression());
        assert!(Token::NullLit.can_end_expression());

        assert!(!Token::Plus.can_end_expression());
        assert!(!Token::Eq.can_end_expression());
        assert!(!Token::LParen.can_end_expression());
        assert!(!Token::Let.can_end_expression());
        assert!(!Token::Comma.can_end_expression());
    }
}
