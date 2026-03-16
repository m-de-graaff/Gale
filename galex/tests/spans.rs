//! Tests for span accuracy — every token has correct (line, col, start, end).

use galex::{lex, LexMode, Lexer, Span, Token};

/// Helper: lex and return (token, span) pairs, filtering comments/newlines.
fn token_spans(source: &str) -> Vec<(Token, Span)> {
    let result = lex(source, 0);
    assert!(result.is_ok(), "unexpected errors: {:?}", result.errors);
    result
        .tokens
        .into_iter()
        .filter(|(t, _)| !matches!(t, Token::Newline | Token::EOF | Token::Comment(_)))
        .collect()
}

/// Assert a span matches expected values.
fn assert_span(span: &Span, line: u32, col: u32, start: u32, end: u32) {
    assert_eq!(
        (span.line, span.col, span.start, span.end),
        (line, col, start, end),
        "span mismatch"
    );
}

// ── Single-character operators ─────────────────────────────────────────

#[test]
fn span_single_char_operators() {
    let ts = token_spans("+ - * %");
    // +  at col 1, bytes 0..1
    assert_span(&ts[0].1, 1, 1, 0, 1);
    // -  at col 3, bytes 2..3
    assert_span(&ts[1].1, 1, 3, 2, 3);
    // *  at col 5, bytes 4..5
    assert_span(&ts[2].1, 1, 5, 4, 5);
    // %  at col 7, bytes 6..7
    assert_span(&ts[3].1, 1, 7, 6, 7);
}

#[test]
fn span_delimiters() {
    let ts = token_spans("( ) { }");
    assert_span(&ts[0].1, 1, 1, 0, 1); // (
    assert_span(&ts[1].1, 1, 3, 2, 3); // )
    assert_span(&ts[2].1, 1, 5, 4, 5); // {
    assert_span(&ts[3].1, 1, 7, 6, 7); // }
}

// ── Multi-character operators ──────────────────────────────────────────

#[test]
fn span_two_char_operators() {
    let ts = token_spans("== => !=");
    // == at col 1, bytes 0..2
    assert_span(&ts[0].1, 1, 1, 0, 2);
    assert_eq!(ts[0].0, Token::EqEq);
    // => at col 4, bytes 3..5
    assert_span(&ts[1].1, 1, 4, 3, 5);
    assert_eq!(ts[1].0, Token::FatArrow);
    // != at col 7, bytes 6..8
    assert_span(&ts[2].1, 1, 7, 6, 8);
    assert_eq!(ts[2].0, Token::NotEq);
}

#[test]
fn span_three_char_operators() {
    let ts = token_spans("<-> ...");
    // <-> at col 1, bytes 0..3
    assert_span(&ts[0].1, 1, 1, 0, 3);
    assert_eq!(ts[0].0, Token::BiArrow);
    // ... at col 5, bytes 4..7
    assert_span(&ts[1].1, 1, 5, 4, 7);
    assert_eq!(ts[1].0, Token::Spread);
}

// ── Keywords and identifiers ───────────────────────────────────────────

#[test]
fn span_keywords() {
    let ts = token_spans("let mut");
    assert_span(&ts[0].1, 1, 1, 0, 3); // "let" = 3 bytes
    assert_span(&ts[1].1, 1, 5, 4, 7); // "mut" = 3 bytes
}

#[test]
fn span_identifier() {
    let ts = token_spans("myVariable");
    assert_span(&ts[0].1, 1, 1, 0, 10); // 10 chars
    assert_eq!(ts[0].0, Token::Ident("myVariable".into()));
}

// ── String literals ────────────────────────────────────────────────────

#[test]
fn span_string_literal() {
    // "hello" — the span covers the opening " through closing "
    let ts = token_spans(r#""hello""#);
    assert_span(&ts[0].1, 1, 1, 0, 7); // 7 bytes: " h e l l o "
    assert_eq!(ts[0].0, Token::StringLit("hello".into()));
}

#[test]
fn span_empty_string() {
    let ts = token_spans(r#""""#);
    assert_span(&ts[0].1, 1, 1, 0, 2); // 2 bytes: " "
}

// ── Number literals ────────────────────────────────────────────────────

#[test]
fn span_integer() {
    let ts = token_spans("42");
    assert_span(&ts[0].1, 1, 1, 0, 2);
    assert_eq!(ts[0].0, Token::IntLit(42));
}

#[test]
fn span_float() {
    let ts = token_spans("3.14");
    assert_span(&ts[0].1, 1, 1, 0, 4);
    assert_eq!(ts[0].0, Token::FloatLit(3.14));
}

#[test]
fn span_hex_literal() {
    let ts = token_spans("0xFF");
    assert_span(&ts[0].1, 1, 1, 0, 4);
    assert_eq!(ts[0].0, Token::IntLit(255));
}

// ── Template literals ──────────────────────────────────────────────────

#[test]
fn span_template_no_interpolation() {
    let ts = token_spans("`hello`");
    assert_span(&ts[0].1, 1, 1, 0, 7);
    assert_eq!(ts[0].0, Token::TemplateNoSub("hello".into()));
}

#[test]
fn span_template_head() {
    let ts = token_spans("`hi ${x}`");
    // TemplateHead covers from ` through ${
    assert_span(&ts[0].1, 1, 1, 0, 6); // `hi ${
    assert_eq!(ts[0].0, Token::TemplateHead("hi ".into()));
    // "x" identifier
    assert_eq!(ts[1].0, Token::Ident("x".into()));
    assert_span(&ts[1].1, 1, 7, 6, 7); // x at byte 6
}

// ── Multi-line spans ───────────────────────────────────────────────────

#[test]
fn span_multiline() {
    let source = "let a = 1\nlet b = 2\nlet c = 3";
    let ts = token_spans(source);
    // Line 1: let(0..3) a(4..5) =(6..7) 1(8..9)
    assert_span(&ts[0].1, 1, 1, 0, 3);
    assert_span(&ts[3].1, 1, 9, 8, 9);
    // Line 2: let(10..13) b(14..15) =(16..17) 2(18..19)
    assert_span(&ts[4].1, 2, 1, 10, 13);
    assert_span(&ts[7].1, 2, 9, 18, 19);
    // Line 3: let(20..23) c(24..25) =(26..27) 3(28..29)
    assert_span(&ts[8].1, 3, 1, 20, 23);
    assert_span(&ts[11].1, 3, 9, 28, 29);
}

// ── Comments ───────────────────────────────────────────────────────────

#[test]
fn span_line_comment() {
    let result = lex("// hello", 0);
    let comments: Vec<_> = result
        .tokens
        .iter()
        .filter(|(t, _)| matches!(t, Token::Comment(_)))
        .collect();
    assert_eq!(comments.len(), 1);
    assert_span(&comments[0].1, 1, 1, 0, 8);
}

#[test]
fn span_block_comment() {
    let result = lex("/* hi */", 0);
    let comments: Vec<_> = result
        .tokens
        .iter()
        .filter(|(t, _)| matches!(t, Token::BlockComment(_)))
        .collect();
    assert_eq!(comments.len(), 1);
    assert_span(&comments[0].1, 1, 1, 0, 8);
}

// ── Template mode spans ───────────────────────────────────────────────

#[test]
fn span_html_open_tag() {
    let mut lexer = Lexer::new("<div>", 0);
    lexer.push_mode(LexMode::Template);
    let all = lexer.tokenize_all();
    // HtmlOpen("div") should span from < through div (bytes 0..4)
    let open = &all[0];
    assert_eq!(open.0, Token::HtmlOpen("div".into()));
    assert_span(&open.1, 1, 1, 0, 4);
}

#[test]
fn span_html_close_tag() {
    let mut lexer = Lexer::new("</div>", 0);
    lexer.push_mode(LexMode::Template);
    let all = lexer.tokenize_all();
    let close = &all[0];
    assert_eq!(close.0, Token::HtmlClose("div".into()));
    assert_span(&close.1, 1, 1, 0, 6);
}

#[test]
fn span_directive_in_tag() {
    let mut lexer = Lexer::new("on:click.prevent", 0);
    lexer.push_mode(LexMode::HtmlTag);
    let all = lexer.tokenize_all();
    let dir = &all[0];
    assert_eq!(
        dir.0,
        Token::OnDir {
            event: "click".into(),
            modifiers: vec!["prevent".into()]
        }
    );
    assert_span(&dir.1, 1, 1, 0, 16);
}

// ── Boolean and null literal spans ─────────────────────────────────────

#[test]
fn span_bool_and_null() {
    let ts = token_spans("true false null");
    assert_span(&ts[0].1, 1, 1, 0, 4); // true
    assert_span(&ts[1].1, 1, 6, 5, 10); // false
    assert_span(&ts[2].1, 1, 12, 11, 15); // null
}

// ── Regex literal span ────────────────────────────────────────────────

#[test]
fn span_regex_literal() {
    let ts = token_spans("/abc/gi");
    assert_span(&ts[0].1, 1, 1, 0, 7); // /abc/gi = 7 bytes
    assert!(matches!(ts[0].0, Token::RegexLit { .. }));
}

// ── Complex expression spans ───────────────────────────────────────────

#[test]
fn span_complex_expression() {
    let ts = token_spans("a + b");
    assert_span(&ts[0].1, 1, 1, 0, 1); // a
    assert_span(&ts[1].1, 1, 3, 2, 3); // +
    assert_span(&ts[2].1, 1, 5, 4, 5); // b
}
