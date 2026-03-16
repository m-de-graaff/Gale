//! Tests for keyword and identifier tokenization.

use galex::{lex, Token};

/// Helper: lex source and return just the token types (no spans), filtering out Newline/EOF.
fn tokens(source: &str) -> Vec<Token> {
    let result = lex(source, 0);
    assert!(result.is_ok(), "unexpected lex errors: {:?}", result.errors);
    result
        .tokens
        .into_iter()
        .map(|(tok, _)| tok)
        .filter(|t| !matches!(t, Token::Newline | Token::EOF))
        .collect()
}

// ── All 43 keywords ────────────────────────────────────────────────────

#[test]
fn all_keywords_lex_correctly() {
    let cases: Vec<(&str, Token)> = vec![
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
    ];

    for (source, expected) in cases {
        let toks = tokens(source);
        assert_eq!(
            toks,
            vec![expected.clone()],
            "failed for keyword: {}",
            source
        );
    }
}

#[test]
fn bool_literals_are_keywords() {
    assert_eq!(tokens("true"), vec![Token::BoolLit(true)]);
    assert_eq!(tokens("false"), vec![Token::BoolLit(false)]);
}

#[test]
fn null_literal_is_keyword() {
    assert_eq!(tokens("null"), vec![Token::NullLit]);
}

// ── Identifiers ────────────────────────────────────────────────────────

#[test]
fn simple_identifiers() {
    assert_eq!(tokens("foo"), vec![Token::Ident("foo".into())]);
    assert_eq!(tokens("myVar"), vec![Token::Ident("myVar".into())]);
    assert_eq!(tokens("_private"), vec![Token::Ident("_private".into())]);
    assert_eq!(tokens("x1"), vec![Token::Ident("x1".into())]);
}

#[test]
fn identifiers_are_case_sensitive() {
    assert_eq!(tokens("Let"), vec![Token::Ident("Let".into())]);
    assert_eq!(tokens("LET"), vec![Token::Ident("LET".into())]);
    assert_eq!(tokens("Signal"), vec![Token::Ident("Signal".into())]);
    assert_eq!(tokens("TRUE"), vec![Token::Ident("TRUE".into())]);
}

#[test]
fn keyword_prefix_identifiers() {
    // Identifiers that start with a keyword but are longer
    assert_eq!(tokens("letting"), vec![Token::Ident("letting".into())]);
    assert_eq!(tokens("fns"), vec![Token::Ident("fns".into())]);
    assert_eq!(tokens("iffy"), vec![Token::Ident("iffy".into())]);
    assert_eq!(tokens("format"), vec![Token::Ident("format".into())]);
    assert_eq!(tokens("returned"), vec![Token::Ident("returned".into())]);
}

#[test]
fn underscored_identifiers() {
    assert_eq!(tokens("__init__"), vec![Token::Ident("__init__".into())]);
    assert_eq!(tokens("_"), vec![Token::Ident("_".into())]);
    assert_eq!(tokens("a_b_c"), vec![Token::Ident("a_b_c".into())]);
}

// ── Unicode identifiers ────────────────────────────────────────────────

#[test]
fn unicode_identifiers() {
    assert_eq!(tokens("café"), vec![Token::Ident("café".into())]);
    assert_eq!(tokens("名前"), vec![Token::Ident("名前".into())]);
    assert_eq!(tokens("_привет"), vec![Token::Ident("_привет".into())]);
    assert_eq!(tokens("Ωmega"), vec![Token::Ident("Ωmega".into())]);
    assert_eq!(tokens("über"), vec![Token::Ident("über".into())]);
}

#[test]
fn unicode_identifier_not_keyword() {
    // Unicode identifiers should never match ASCII keywords
    assert_eq!(tokens("lét"), vec![Token::Ident("lét".into())]);
    assert_eq!(tokens("ïf"), vec![Token::Ident("ïf".into())]);
}

#[test]
fn mixed_ascii_and_unicode_identifiers() {
    assert_eq!(
        tokens("let café = 1"),
        vec![
            Token::Let,
            Token::Ident("café".into()),
            Token::Eq,
            Token::IntLit(1),
        ]
    );
}

// ── Multiple tokens on one line ────────────────────────────────────────

#[test]
fn keyword_sequence() {
    assert_eq!(
        tokens("let mut signal"),
        vec![Token::Let, Token::Mut, Token::Signal]
    );
}

#[test]
fn mixed_keywords_and_identifiers() {
    assert_eq!(
        tokens("let x = 42"),
        vec![
            Token::Let,
            Token::Ident("x".into()),
            Token::Eq,
            Token::IntLit(42),
        ]
    );
}

// ── Span accuracy ──────────────────────────────────────────────────────

#[test]
fn keyword_spans_are_accurate() {
    let result = lex("let x", 0);
    let spans: Vec<_> = result
        .tokens
        .iter()
        .map(|(_, s)| (s.line, s.col, s.start, s.end))
        .collect();
    // "let" at line 1, col 1, bytes 0..3
    assert_eq!(spans[0], (1, 1, 0, 3));
    // "x" at line 1, col 5, bytes 4..5
    assert_eq!(spans[1], (1, 5, 4, 5));
}

#[test]
fn multiline_spans() {
    let result = lex("let\nmut", 0);
    let toks: Vec<_> = result
        .tokens
        .iter()
        .map(|(t, s)| (t.clone(), s.line))
        .collect();
    assert_eq!(toks[0], (Token::Let, 1));
    assert_eq!(toks[1], (Token::Newline, 1));
    assert_eq!(toks[2], (Token::Mut, 2));
}
