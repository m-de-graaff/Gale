//! Property-based fuzz tests for the GaleX lexer.
//!
//! Uses `proptest` to generate random input and verify the lexer never panics.

use galex::{lex, LexMode, Lexer, Token};
use proptest::prelude::*;

// ── Strategy helpers ───────────────────────────────────────────────────

/// Random ASCII string (printable + whitespace).
fn arb_ascii_source() -> impl Strategy<Value = String> {
    prop::collection::vec(0x01u8..0x7F, 0..2000)
        .prop_map(|bytes| bytes.into_iter().map(|b| b as char).collect::<String>())
}

/// Random UTF-8 string (any valid Rust string).
fn arb_utf8_source() -> impl Strategy<Value = String> {
    ".*" // proptest's built-in arbitrary string regex
}

/// Generate random strings that look like plausible GaleX fragments.
fn arb_galex_fragment() -> impl Strategy<Value = String> {
    let keywords = prop::sample::select(vec![
        "let", "mut", "signal", "derive", "frozen", "ref", "fn", "return", "if", "else", "for",
        "await", "server", "client", "shared", "guard", "action", "query", "store", "channel",
        "type", "enum", "test", "effect", "watch", "bind", "when", "each", "suspend", "slot",
        "empty", "use", "out", "ui", "api", "head", "true", "false", "null",
    ]);
    let operators = prop::sample::select(vec![
        "+", "-", "*", "/", "%", "==", "!=", "<", ">", "<=", ">=", "&&", "||", "!", "=", "+=",
        "-=", "->", "=>", "<->", "..", "...", "|>", "?.", "??", "|", "?",
    ]);
    let delimiters = prop::sample::select(vec![
        "(", ")", "{", "}", "[", "]", ":", ";", ",", ".", "@", "#",
    ]);
    let literals = prop::sample::select(vec!["42", "3.14", "0xFF", "0b1010", "1_000"]);
    let strings = prop::sample::select(vec![r#""hello""#, r#""with\nnewline""#, r#""""#]);
    let whitespace = prop::sample::select(vec![" ", "\n", "  ", "\t"]);
    let identifiers = "[a-zA-Z_][a-zA-Z0-9_]{0,12}";

    let token = prop::strategy::Union::new(vec![
        keywords.prop_map(|s| s.to_string()).boxed(),
        operators.prop_map(|s| s.to_string()).boxed(),
        delimiters.prop_map(|s| s.to_string()).boxed(),
        literals.prop_map(|s| s.to_string()).boxed(),
        strings.prop_map(|s| s.to_string()).boxed(),
        whitespace.prop_map(|s| s.to_string()).boxed(),
        identifiers.boxed(),
    ]);

    prop::collection::vec(token, 0..200).prop_map(|parts| parts.join(""))
}

// ── Property tests ─────────────────────────────────────────────────────

proptest! {
    /// Random ASCII input never causes a panic.
    #[test]
    fn fuzz_random_ascii_no_panic(source in arb_ascii_source()) {
        let result = lex(&source, 0);
        // Must always end with EOF
        assert!(
            result.tokens.last().map(|(t, _)| t == &Token::EOF).unwrap_or(false),
            "token stream must end with EOF"
        );
    }

    /// Random UTF-8 input never causes a panic.
    #[test]
    fn fuzz_random_utf8_no_panic(source in arb_utf8_source()) {
        let result = lex(&source, 0);
        assert!(
            result.tokens.last().map(|(t, _)| t == &Token::EOF).unwrap_or(false),
            "token stream must end with EOF"
        );
    }

    /// Random GaleX-like fragments never cause a panic and produce reasonable output.
    #[test]
    fn fuzz_galex_fragments_no_panic(source in arb_galex_fragment()) {
        let result = lex(&source, 0);
        // Must end with EOF
        assert!(
            result.tokens.last().map(|(t, _)| t == &Token::EOF).unwrap_or(false),
            "token stream must end with EOF"
        );
        // Token count must be at least 1 (EOF)
        assert!(!result.tokens.is_empty());
        // Every span should have line >= 1
        for (_, span) in &result.tokens {
            assert!(span.line >= 1, "span line must be >= 1, got {}", span.line);
        }
    }

    /// Random input in Template mode never causes a panic.
    #[test]
    fn fuzz_template_mode_no_panic(source in arb_ascii_source()) {
        let mut lexer = Lexer::new(&source, 0);
        lexer.push_mode(LexMode::Template);
        let all = lexer.tokenize_all();
        assert!(
            all.last().map(|(t, _)| t == &Token::EOF).unwrap_or(false),
            "token stream must end with EOF"
        );
    }
}

// ── Specific adversarial inputs ────────────────────────────────────────

#[test]
fn adversarial_deeply_nested_braces() {
    let source = "{".repeat(1000) + &"}".repeat(1000);
    let result = lex(&source, 0);
    // Should not panic
    assert!(!result.tokens.is_empty());
}

#[test]
fn adversarial_many_newlines() {
    let source = "\n".repeat(10000);
    let result = lex(&source, 0);
    assert!(!result.tokens.is_empty());
}

#[test]
fn adversarial_long_identifier() {
    let source = "a".repeat(100_000);
    let result = lex(&source, 0);
    assert_eq!(result.tokens.len(), 2); // Ident + EOF
}

#[test]
fn adversarial_alternating_operators() {
    let source = "+-*/+-*/".repeat(1000);
    let result = lex(&source, 0);
    assert!(!result.tokens.is_empty());
}

#[test]
fn adversarial_null_bytes_in_string() {
    let source = "\"\\0\\0\\0\"";
    let result = lex(source, 0);
    assert!(result.is_ok());
}

#[test]
fn adversarial_empty_input() {
    let result = lex("", 0);
    assert_eq!(result.tokens.len(), 1);
    assert_eq!(result.tokens[0].0, Token::EOF);
}
