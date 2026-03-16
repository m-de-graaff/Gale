//! Tests for comments and error recovery.

use galex::error::LexError;
use galex::{lex, Token};

fn tokens(source: &str) -> Vec<Token> {
    let result = lex(source, 0);
    assert!(result.is_ok(), "unexpected lex errors: {:?}", result.errors);
    result
        .tokens
        .into_iter()
        .map(|(tok, _)| tok)
        .filter(|t| !matches!(t, Token::EOF))
        .collect()
}

fn tokens_with_errors(source: &str) -> (Vec<Token>, Vec<LexError>) {
    let result = lex(source, 0);
    let toks = result
        .tokens
        .into_iter()
        .map(|(tok, _)| tok)
        .filter(|t| !matches!(t, Token::EOF))
        .collect();
    (toks, result.errors)
}

// ── Line comments ──────────────────────────────────────────────────────

#[test]
fn line_comment_standalone() {
    let toks = tokens("// this is a comment");
    assert_eq!(toks, vec![Token::Comment("this is a comment".into())]);
}

#[test]
fn line_comment_after_code() {
    let toks = tokens("let x = 1 // assign x");
    assert_eq!(
        toks,
        vec![
            Token::Let,
            Token::Ident("x".into()),
            Token::Eq,
            Token::IntLit(1),
            Token::Comment("assign x".into()),
        ]
    );
}

#[test]
fn line_comment_does_not_consume_newline() {
    let toks = tokens("// comment\nlet x");
    assert_eq!(
        toks,
        vec![
            Token::Comment("comment".into()),
            Token::Newline,
            Token::Let,
            Token::Ident("x".into()),
        ]
    );
}

#[test]
fn empty_line_comment() {
    let toks = tokens("//");
    assert_eq!(toks, vec![Token::Comment("".into())]);
}

// ── Block comments ─────────────────────────────────────────────────────

#[test]
fn block_comment_single_line() {
    let toks = tokens("/* hello */");
    assert_eq!(toks, vec![Token::BlockComment("hello".into())]);
}

#[test]
fn block_comment_multiline() {
    let toks = tokens("/* line1\n   line2 */");
    assert_eq!(toks, vec![Token::BlockComment("line1\n   line2".into())]);
}

#[test]
fn block_comment_nested() {
    let toks = tokens("/* outer /* inner */ end */");
    assert_eq!(
        toks,
        vec![Token::BlockComment("outer /* inner */ end".into())]
    );
}

#[test]
fn block_comment_deeply_nested() {
    let toks = tokens("/* L1 /* L2 /* L3 */ L2 */ L1 */");
    assert_eq!(
        toks,
        vec![Token::BlockComment("L1 /* L2 /* L3 */ L2 */ L1".into())]
    );
}

#[test]
fn block_comment_between_code() {
    let toks = tokens("let /* skip */ x");
    assert_eq!(
        toks,
        vec![
            Token::Let,
            Token::BlockComment("skip".into()),
            Token::Ident("x".into()),
        ]
    );
}

// ── Error recovery — assert specific variants + recovery ───────────────

#[test]
fn unterminated_string_recovers_with_correct_variant() {
    let (toks, errors) = tokens_with_errors("\"hello\nlet x");
    assert_eq!(errors.len(), 1);
    assert!(
        matches!(&errors[0], LexError::UnterminatedString { .. }),
        "expected UnterminatedString, got {:?}",
        errors[0]
    );
    assert_eq!(errors[0].error_code(), "GX0002");
    // Recovery: lexer continues and finds `let x` on the next line
    assert!(toks.iter().any(|t| matches!(t, Token::Let)));
    assert!(toks
        .iter()
        .any(|t| matches!(t, Token::Ident(s) if s == "x")));
}

#[test]
fn unterminated_block_comment_recovers_with_correct_variant() {
    let (toks, errors) = tokens_with_errors("let a = 1\n/* never closed");
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, LexError::UnterminatedBlockComment { .. })),
        "expected UnterminatedBlockComment, got {:?}",
        errors
    );
    // Recovery: tokens before the bad comment should still be present
    assert!(toks.iter().any(|t| matches!(t, Token::Let)));
    assert!(toks.iter().any(|t| matches!(t, Token::IntLit(1))));
}

#[test]
fn unterminated_template_literal_recovers_with_correct_variant() {
    let (toks, errors) = tokens_with_errors("let a = 1\n`unclosed template");
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, LexError::UnterminatedTemplateLiteral { .. })),
        "expected UnterminatedTemplateLiteral, got {:?}",
        errors
    );
    // Recovery: tokens before the bad template literal should still be present
    assert!(toks.iter().any(|t| matches!(t, Token::Let)));
    assert!(toks.iter().any(|t| matches!(t, Token::IntLit(1))));
}

#[test]
fn unterminated_regex_recovers_with_correct_variant() {
    let (toks, errors) = tokens_with_errors("let x = /unclosed\nlet y = 2");
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, LexError::UnterminatedRegex { .. })),
        "expected UnterminatedRegex, got {:?}",
        errors
    );
    // Recovery: tokens after the bad regex should still be present
    assert!(toks.iter().any(|t| matches!(t, Token::IntLit(2))));
}

#[test]
fn unexpected_character_correct_variant_and_recovery() {
    let (toks, errors) = tokens_with_errors("let ~ x");
    assert_eq!(errors.len(), 1);
    assert!(
        matches!(&errors[0], LexError::UnexpectedCharacter { ch: '~', .. }),
        "expected UnexpectedCharacter('~'), got {:?}",
        errors[0]
    );
    assert_eq!(errors[0].error_code(), "GX0001");
    assert!(toks.contains(&Token::Let));
    assert!(toks.contains(&Token::Ident("x".into())));
}

#[test]
fn invalid_escape_correct_variant_and_recovery() {
    let (toks, errors) = tokens_with_errors("\"\\q\" let y");
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, LexError::InvalidEscapeSequence { sequence: 'q', .. })),
        "expected InvalidEscapeSequence('q'), got {:?}",
        errors
    );
    // Recovery: string token still produced, and code after it is lexed
    assert!(toks.iter().any(|t| matches!(t, Token::StringLit(_))));
    assert!(toks.iter().any(|t| matches!(t, Token::Let)));
}

#[test]
fn invalid_hex_literal_correct_variant() {
    let (toks, errors) = tokens_with_errors("0x let y");
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, LexError::InvalidNumberLiteral { .. })),
        "expected InvalidNumberLiteral, got {:?}",
        errors
    );
    assert_eq!(errors[0].error_code(), "GX0007");
    // Recovery: produces IntLit(0) and continues
    assert!(toks.iter().any(|t| matches!(t, Token::IntLit(0))));
    assert!(toks.iter().any(|t| matches!(t, Token::Let)));
}

#[test]
fn invalid_binary_literal_correct_variant() {
    let (toks, errors) = tokens_with_errors("0b let y");
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, LexError::InvalidNumberLiteral { .. })),
        "expected InvalidNumberLiteral, got {:?}",
        errors
    );
    assert!(toks.iter().any(|t| matches!(t, Token::IntLit(0))));
    assert!(toks.iter().any(|t| matches!(t, Token::Let)));
}

// ── Bad digit sequences ────────────────────────────────────────────────

#[test]
fn hex_with_invalid_digits() {
    // 0xZZ — 'Z' is not a hex digit, so 0x has no valid digits
    let (_, errors) = tokens_with_errors("0xZZ");
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, LexError::InvalidNumberLiteral { .. })),
        "0xZZ should produce InvalidNumberLiteral: {:?}",
        errors
    );
}

#[test]
fn binary_with_invalid_digits() {
    // 0b22 — '2' is not a binary digit, so 0b has no valid digits
    let (_, errors) = tokens_with_errors("0b22");
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, LexError::InvalidNumberLiteral { .. })),
        "0b22 should produce InvalidNumberLiteral: {:?}",
        errors
    );
}

// ── Multiple errors in one source ──────────────────────────────────────

#[test]
fn multiple_errors_accumulated() {
    let (_, errors) = tokens_with_errors("~ ^ `unterminated");
    assert!(
        errors.len() >= 2,
        "should accumulate multiple errors, got {}",
        errors.len()
    );
}
