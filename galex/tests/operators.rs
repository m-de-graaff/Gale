//! Tests for operator and delimiter tokenization.

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

// ── Arithmetic operators ───────────────────────────────────────────────

#[test]
fn arithmetic_operators() {
    assert_eq!(tokens("+"), vec![Token::Plus]);
    assert_eq!(tokens("-"), vec![Token::Minus]);
    assert_eq!(tokens("*"), vec![Token::Star]);
    assert_eq!(tokens("%"), vec![Token::Percent]);
}

#[test]
fn slash_as_division_after_expression() {
    // After an identifier, / is division
    assert_eq!(
        tokens("a / b"),
        vec![
            Token::Ident("a".into()),
            Token::Slash,
            Token::Ident("b".into()),
        ]
    );
}

// ── Comparison operators ───────────────────────────────────────────────

#[test]
fn comparison_operators() {
    assert_eq!(tokens("=="), vec![Token::EqEq]);
    assert_eq!(tokens("!="), vec![Token::NotEq]);
    assert_eq!(tokens("<"), vec![Token::Less]);
    assert_eq!(tokens(">"), vec![Token::Greater]);
    assert_eq!(tokens("<="), vec![Token::LessEq]);
    assert_eq!(tokens(">="), vec![Token::GreaterEq]);
}

// ── Logical operators ──────────────────────────────────────────────────

#[test]
fn logical_operators() {
    assert_eq!(tokens("&&"), vec![Token::And]);
    assert_eq!(tokens("||"), vec![Token::Or]);
    assert_eq!(tokens("!"), vec![Token::Not]);
    assert_eq!(tokens("??"), vec![Token::NullCoalesce]);
}

// ── Assignment operators ───────────────────────────────────────────────

#[test]
fn assignment_operators() {
    assert_eq!(tokens("="), vec![Token::Eq]);
    assert_eq!(tokens("+="), vec![Token::PlusEq]);
    assert_eq!(tokens("-="), vec![Token::MinusEq]);
}

// ── Special operators ──────────────────────────────────────────────────

#[test]
fn arrow_operators() {
    assert_eq!(tokens("->"), vec![Token::Arrow]);
    assert_eq!(tokens("=>"), vec![Token::FatArrow]);
    assert_eq!(tokens("<->"), vec![Token::BiArrow]);
}

#[test]
fn dot_operators() {
    assert_eq!(tokens(".."), vec![Token::DotDot]);
    assert_eq!(tokens("..."), vec![Token::Spread]);
    assert_eq!(tokens("."), vec![Token::Dot]);
}

#[test]
fn pipe_operator() {
    assert_eq!(tokens("|>"), vec![Token::Pipe]);
}

#[test]
fn optional_chaining() {
    assert_eq!(tokens("?."), vec![Token::QuestionDot]);
}

#[test]
fn bare_question_mark() {
    assert_eq!(tokens("?"), vec![Token::Question]);
}

#[test]
fn question_disambiguate() {
    assert_eq!(
        tokens("? ?. ??"),
        vec![Token::Question, Token::QuestionDot, Token::NullCoalesce]
    );
}

#[test]
fn bar_type_union() {
    assert_eq!(tokens("|"), vec![Token::Bar]);
}

// ── Delimiters ─────────────────────────────────────────────────────────

#[test]
fn delimiters() {
    assert_eq!(tokens("("), vec![Token::LParen]);
    assert_eq!(tokens(")"), vec![Token::RParen]);
    assert_eq!(tokens("{"), vec![Token::LBrace]);
    assert_eq!(tokens("}"), vec![Token::RBrace]);
    assert_eq!(tokens("["), vec![Token::LBracket]);
    assert_eq!(tokens("]"), vec![Token::RBracket]);
    assert_eq!(tokens(":"), vec![Token::Colon]);
    assert_eq!(tokens(";"), vec![Token::Semicolon]);
    assert_eq!(tokens(","), vec![Token::Comma]);
    assert_eq!(tokens("@"), vec![Token::At]);
    assert_eq!(tokens("#"), vec![Token::Hash]);
}

// ── Multi-char operator disambiguation ─────────────────────────────────

#[test]
fn disambiguate_eq_vs_eqeq_vs_fatarrow() {
    assert_eq!(
        tokens("= == =>"),
        vec![Token::Eq, Token::EqEq, Token::FatArrow]
    );
}

#[test]
fn disambiguate_minus_vs_minuseq_vs_arrow() {
    assert_eq!(
        tokens("- -= ->"),
        vec![Token::Minus, Token::MinusEq, Token::Arrow]
    );
}

#[test]
fn disambiguate_plus_vs_pluseq() {
    assert_eq!(tokens("+ +="), vec![Token::Plus, Token::PlusEq]);
}

#[test]
fn disambiguate_not_vs_noteq() {
    assert_eq!(tokens("! !="), vec![Token::Not, Token::NotEq]);
}

#[test]
fn disambiguate_less_vs_lesseq_vs_biarrow() {
    assert_eq!(
        tokens("< <= <->"),
        vec![Token::Less, Token::LessEq, Token::BiArrow]
    );
}

#[test]
fn disambiguate_greater_vs_greatereq() {
    assert_eq!(tokens("> >="), vec![Token::Greater, Token::GreaterEq]);
}

#[test]
fn disambiguate_bar_vs_or_vs_pipe() {
    assert_eq!(tokens("| || |>"), vec![Token::Bar, Token::Or, Token::Pipe]);
}

#[test]
fn disambiguate_question_vs_questiondot_vs_nullcoalesce() {
    // Note: bare ? is an error, but we test ?. and ?? work
    assert_eq!(
        tokens("?. ??"),
        vec![Token::QuestionDot, Token::NullCoalesce]
    );
}

#[test]
fn disambiguate_dot_vs_dotdot_vs_spread() {
    assert_eq!(
        tokens(". .. ..."),
        vec![Token::Dot, Token::DotDot, Token::Spread]
    );
}

// ── Complex expressions ────────────────────────────────────────────────

#[test]
fn complex_expression() {
    assert_eq!(
        tokens("a + b * c - d"),
        vec![
            Token::Ident("a".into()),
            Token::Plus,
            Token::Ident("b".into()),
            Token::Star,
            Token::Ident("c".into()),
            Token::Minus,
            Token::Ident("d".into()),
        ]
    );
}

#[test]
fn comparison_expression() {
    assert_eq!(
        tokens("x >= 10 && y != 0"),
        vec![
            Token::Ident("x".into()),
            Token::GreaterEq,
            Token::IntLit(10),
            Token::And,
            Token::Ident("y".into()),
            Token::NotEq,
            Token::IntLit(0),
        ]
    );
}

#[test]
fn arrow_function() {
    assert_eq!(
        tokens("(n) => n * 2"),
        vec![
            Token::LParen,
            Token::Ident("n".into()),
            Token::RParen,
            Token::FatArrow,
            Token::Ident("n".into()),
            Token::Star,
            Token::IntLit(2),
        ]
    );
}

#[test]
fn function_return_type() {
    assert_eq!(
        tokens("fn add(a: int) -> int"),
        vec![
            Token::Fn,
            Token::Ident("add".into()),
            Token::LParen,
            Token::Ident("a".into()),
            Token::Colon,
            Token::Ident("int".into()),
            Token::RParen,
            Token::Arrow,
            Token::Ident("int".into()),
        ]
    );
}

#[test]
fn spread_in_array() {
    assert_eq!(
        tokens("[...items, x]"),
        vec![
            Token::LBracket,
            Token::Spread,
            Token::Ident("items".into()),
            Token::Comma,
            Token::Ident("x".into()),
            Token::RBracket,
        ]
    );
}

#[test]
fn optional_chaining_expression() {
    assert_eq!(
        tokens("user?.name"),
        vec![
            Token::Ident("user".into()),
            Token::QuestionDot,
            Token::Ident("name".into()),
        ]
    );
}

#[test]
fn null_coalesce_expression() {
    assert_eq!(
        tokens("a ?? b"),
        vec![
            Token::Ident("a".into()),
            Token::NullCoalesce,
            Token::Ident("b".into()),
        ]
    );
}

#[test]
fn type_union_with_bar() {
    assert_eq!(
        tokens("string | null"),
        vec![Token::Ident("string".into()), Token::Bar, Token::NullLit,]
    );
}
