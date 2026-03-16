//! Tests for literal tokenization: strings, numbers, bools, null, regex, template literals.

use galex::{lex, Token};

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

// ── String literals ────────────────────────────────────────────────────

#[test]
fn simple_string() {
    assert_eq!(tokens(r#""hello""#), vec![Token::StringLit("hello".into())]);
}

#[test]
fn empty_string() {
    assert_eq!(tokens(r#""""#), vec![Token::StringLit("".into())]);
}

#[test]
fn string_with_spaces() {
    assert_eq!(
        tokens(r#""hello world""#),
        vec![Token::StringLit("hello world".into())]
    );
}

#[test]
fn string_escape_sequences() {
    assert_eq!(
        tokens(r#""line1\nline2""#),
        vec![Token::StringLit("line1\nline2".into())]
    );
    assert_eq!(
        tokens(r#""tab\there""#),
        vec![Token::StringLit("tab\there".into())]
    );
    assert_eq!(
        tokens(r#""back\\slash""#),
        vec![Token::StringLit("back\\slash".into())]
    );
    assert_eq!(
        tokens(r#""quote\"inside""#),
        vec![Token::StringLit("quote\"inside".into())]
    );
    assert_eq!(
        tokens(r#""null\0byte""#),
        vec![Token::StringLit("null\0byte".into())]
    );
}

#[test]
fn string_carriage_return_escape() {
    assert_eq!(tokens(r#""\r""#), vec![Token::StringLit("\r".into())]);
}

#[test]
fn string_escaped_brace() {
    assert_eq!(tokens(r#""\{""#), vec![Token::StringLit("{".into())]);
}

#[test]
fn string_escaped_dollar() {
    assert_eq!(tokens(r#""\$""#), vec![Token::StringLit("$".into())]);
}

#[test]
fn template_escaped_dollar_brace() {
    // \${ should NOT trigger interpolation — produces literal "${" in the string
    assert_eq!(
        tokens(r"`\${name}`"),
        vec![Token::TemplateNoSub("${name}".into())]
    );
}

#[test]
fn string_hex_escape() {
    assert_eq!(tokens(r#""\x41""#), vec![Token::StringLit("A".into())]);
}

#[test]
fn string_unicode_escape() {
    assert_eq!(
        tokens(r#""\u{1F600}""#),
        vec![Token::StringLit("\u{1F600}".into())]
    );
}

// ── Integer literals ───────────────────────────────────────────────────

#[test]
fn simple_integers() {
    assert_eq!(tokens("0"), vec![Token::IntLit(0)]);
    assert_eq!(tokens("42"), vec![Token::IntLit(42)]);
    assert_eq!(tokens("999"), vec![Token::IntLit(999)]);
}

#[test]
fn hex_integers() {
    assert_eq!(tokens("0xFF"), vec![Token::IntLit(255)]);
    assert_eq!(tokens("0XFF"), vec![Token::IntLit(255)]);
    assert_eq!(tokens("0x0"), vec![Token::IntLit(0)]);
    assert_eq!(tokens("0xDEAD"), vec![Token::IntLit(0xDEAD)]);
}

#[test]
fn binary_integers() {
    assert_eq!(tokens("0b1010"), vec![Token::IntLit(0b1010)]);
    assert_eq!(tokens("0B1111"), vec![Token::IntLit(0b1111)]);
    assert_eq!(tokens("0b0"), vec![Token::IntLit(0)]);
}

#[test]
fn integer_separators() {
    assert_eq!(tokens("1_000_000"), vec![Token::IntLit(1_000_000)]);
    assert_eq!(tokens("0xFF_FF"), vec![Token::IntLit(0xFFFF)]);
    assert_eq!(tokens("0b1010_0101"), vec![Token::IntLit(0b1010_0101)]);
}

// ── Float literals ─────────────────────────────────────────────────────

#[test]
fn simple_floats() {
    assert_eq!(tokens("3.14"), vec![Token::FloatLit(3.14)]);
    assert_eq!(tokens("0.5"), vec![Token::FloatLit(0.5)]);
    assert_eq!(tokens("99.99"), vec![Token::FloatLit(99.99)]);
}

#[test]
fn float_with_separators() {
    assert_eq!(tokens("1_000.50"), vec![Token::FloatLit(1000.50)]);
}

#[test]
fn integer_dot_not_float() {
    // `42..` should be int(42) + DotDot, not a float
    assert_eq!(
        tokens("42..100"),
        vec![Token::IntLit(42), Token::DotDot, Token::IntLit(100)]
    );
}

#[test]
fn integer_dot_method() {
    // `42.toString` should be int + dot + ident (not float)
    assert_eq!(
        tokens("42.toString"),
        vec![
            Token::IntLit(42),
            Token::Dot,
            Token::Ident("toString".into())
        ]
    );
}

// ── Boolean and null literals ──────────────────────────────────────────

#[test]
fn bool_literals() {
    assert_eq!(tokens("true"), vec![Token::BoolLit(true)]);
    assert_eq!(tokens("false"), vec![Token::BoolLit(false)]);
}

#[test]
fn null_literal() {
    assert_eq!(tokens("null"), vec![Token::NullLit]);
}

// ── Regex literals ─────────────────────────────────────────────────────

#[test]
fn simple_regex() {
    // At start of input, / starts a regex (no previous expression)
    assert_eq!(
        tokens("/hello/"),
        vec![Token::RegexLit {
            pattern: "hello".into(),
            flags: "".into(),
        }]
    );
}

#[test]
fn regex_with_flags() {
    assert_eq!(
        tokens("/pattern/gi"),
        vec![Token::RegexLit {
            pattern: "pattern".into(),
            flags: "gi".into(),
        }]
    );
}

#[test]
fn regex_with_char_class() {
    assert_eq!(
        tokens("/[a-z]+/i"),
        vec![Token::RegexLit {
            pattern: "[a-z]+".into(),
            flags: "i".into(),
        }]
    );
}

#[test]
fn regex_with_escaped_slash() {
    assert_eq!(
        tokens(r"/a\/b/"),
        vec![Token::RegexLit {
            pattern: r"a\/b".into(),
            flags: "".into(),
        }]
    );
}

#[test]
fn regex_slash_in_char_class() {
    // `/` inside `[...]` should not terminate the regex
    assert_eq!(
        tokens(r"/[a/b]/"),
        vec![Token::RegexLit {
            pattern: "[a/b]".into(),
            flags: "".into(),
        }]
    );
}

#[test]
fn regex_after_operator() {
    // After `=`, `/` starts a regex
    assert_eq!(
        tokens("let x = /pattern/i"),
        vec![
            Token::Let,
            Token::Ident("x".into()),
            Token::Eq,
            Token::RegexLit {
                pattern: "pattern".into(),
                flags: "i".into(),
            },
        ]
    );
}

#[test]
fn division_after_identifier() {
    // After an identifier, `/` is division
    assert_eq!(
        tokens("a / b"),
        vec![
            Token::Ident("a".into()),
            Token::Slash,
            Token::Ident("b".into()),
        ]
    );
}

#[test]
fn division_after_number() {
    assert_eq!(
        tokens("10 / 2"),
        vec![Token::IntLit(10), Token::Slash, Token::IntLit(2)]
    );
}

#[test]
fn division_after_close_paren() {
    assert_eq!(
        tokens("(a) / b"),
        vec![
            Token::LParen,
            Token::Ident("a".into()),
            Token::RParen,
            Token::Slash,
            Token::Ident("b".into()),
        ]
    );
}

// ── Template literals ──────────────────────────────────────────────────

#[test]
fn template_no_interpolation() {
    assert_eq!(
        tokens("`hello world`"),
        vec![Token::TemplateNoSub("hello world".into())]
    );
}

#[test]
fn template_empty() {
    assert_eq!(tokens("``"), vec![Token::TemplateNoSub("".into())]);
}

#[test]
fn template_with_single_interpolation() {
    let toks = tokens("`hello ${name}`");
    assert_eq!(
        toks,
        vec![
            Token::TemplateHead("hello ".into()),
            Token::Ident("name".into()),
            Token::TemplateTail("".into()),
        ]
    );
}

#[test]
fn template_with_multiple_interpolations() {
    let toks = tokens("`${a} and ${b}`");
    assert_eq!(
        toks,
        vec![
            Token::TemplateHead("".into()),
            Token::Ident("a".into()),
            Token::TemplateMiddle(" and ".into()),
            Token::Ident("b".into()),
            Token::TemplateTail("".into()),
        ]
    );
}

#[test]
fn template_with_expression() {
    let toks = tokens("`total: ${a + b}`");
    assert_eq!(
        toks,
        vec![
            Token::TemplateHead("total: ".into()),
            Token::Ident("a".into()),
            Token::Plus,
            Token::Ident("b".into()),
            Token::TemplateTail("".into()),
        ]
    );
}

#[test]
fn template_with_escape() {
    assert_eq!(
        tokens(r"`line1\nline2`"),
        vec![Token::TemplateNoSub("line1\nline2".into())]
    );
}

#[test]
fn template_with_text_after_interpolation() {
    let toks = tokens("`Count: ${n} items`");
    assert_eq!(
        toks,
        vec![
            Token::TemplateHead("Count: ".into()),
            Token::Ident("n".into()),
            Token::TemplateTail(" items".into()),
        ]
    );
}
