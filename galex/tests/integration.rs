//! Integration test: lex a complete .gx component file.
//!
//! This tests the full lexer pipeline on realistic GaleX source code.

use galex::{lex, LexMode, Lexer, Token};

/// Helper: lex in Code mode (default) and collect non-whitespace tokens.
fn code_tokens(source: &str) -> Vec<Token> {
    let result = lex(source, 0);
    assert!(result.is_ok(), "unexpected lex errors: {:?}", result.errors);
    result
        .tokens
        .into_iter()
        .map(|(tok, _)| tok)
        .filter(|t| !matches!(t, Token::Newline | Token::EOF))
        .collect()
}

// ── Variable declarations ──────────────────────────────────────────────

#[test]
fn let_binding() {
    assert_eq!(
        code_tokens(r#"let name = "Gale""#),
        vec![
            Token::Let,
            Token::Ident("name".into()),
            Token::Eq,
            Token::StringLit("Gale".into()),
        ]
    );
}

#[test]
fn mut_binding_with_type() {
    assert_eq!(
        code_tokens("mut counter: int = 0"),
        vec![
            Token::Mut,
            Token::Ident("counter".into()),
            Token::Colon,
            Token::Ident("int".into()),
            Token::Eq,
            Token::IntLit(0),
        ]
    );
}

#[test]
fn signal_declaration() {
    assert_eq!(
        code_tokens("signal count = 0"),
        vec![
            Token::Signal,
            Token::Ident("count".into()),
            Token::Eq,
            Token::IntLit(0),
        ]
    );
}

#[test]
fn derive_expression() {
    assert_eq!(
        code_tokens("derive doubled = count * 2"),
        vec![
            Token::Derive,
            Token::Ident("doubled".into()),
            Token::Eq,
            Token::Ident("count".into()),
            Token::Star,
            Token::IntLit(2),
        ]
    );
}

#[test]
fn frozen_declaration() {
    assert_eq!(
        code_tokens(r#"frozen API = "https://api.example.com""#),
        vec![
            Token::Frozen,
            Token::Ident("API".into()),
            Token::Eq,
            Token::StringLit("https://api.example.com".into()),
        ]
    );
}

// ── Function declarations ──────────────────────────────────────────────

#[test]
fn function_declaration() {
    let toks = code_tokens("fn greet(name: string) -> string {\n  return `Hello, ${name}!`\n}");
    // Check key tokens are present in order
    assert_eq!(toks[0], Token::Fn);
    assert_eq!(toks[1], Token::Ident("greet".into()));
    assert_eq!(toks[2], Token::LParen);
    assert!(toks.contains(&Token::Arrow));
    assert!(toks.contains(&Token::Return));
    assert!(toks.iter().any(|t| matches!(t, Token::TemplateHead(_))));
    assert!(toks.iter().any(|t| matches!(t, Token::TemplateTail(_))));
}

// ── Guard declarations ─────────────────────────────────────────────────

#[test]
fn guard_declaration() {
    let toks = code_tokens("guard Email = string.email()");
    assert_eq!(
        toks,
        vec![
            Token::Guard,
            Token::Ident("Email".into()),
            Token::Eq,
            Token::Ident("string".into()),
            Token::Dot,
            Token::Ident("email".into()),
            Token::LParen,
            Token::RParen,
        ]
    );
}

// ── Channel declaration ────────────────────────────────────────────────

#[test]
fn channel_biarrow() {
    let toks = code_tokens("channel chat(room: string) <-> Message");
    assert_eq!(
        toks,
        vec![
            Token::Channel,
            Token::Ident("chat".into()),
            Token::LParen,
            Token::Ident("room".into()),
            Token::Colon,
            Token::Ident("string".into()),
            Token::RParen,
            Token::BiArrow,
            Token::Ident("Message".into()),
        ]
    );
}

// ── Use/import ─────────────────────────────────────────────────────────

#[test]
fn use_import() {
    let toks = code_tokens(r#"use Button from "lib/components/Button""#);
    assert_eq!(
        toks,
        vec![
            Token::Use,
            Token::Ident("Button".into()),
            Token::Ident("from".into()),
            Token::StringLit("lib/components/Button".into()),
        ]
    );
}

// ── Out/export ─────────────────────────────────────────────────────────

#[test]
fn out_ui_component_signature() {
    let toks = code_tokens("out ui Button(label: string)");
    assert_eq!(
        toks,
        vec![
            Token::Out,
            Token::Ui,
            Token::Ident("Button".into()),
            Token::LParen,
            Token::Ident("label".into()),
            Token::Colon,
            Token::Ident("string".into()),
            Token::RParen,
        ]
    );
}

// ── Test blocks ────────────────────────────────────────────────────────

#[test]
fn test_block() {
    let toks = code_tokens(r#"test "add works" { assert 1 + 1 == 2 }"#);
    assert_eq!(toks[0], Token::Test);
    assert_eq!(toks[1], Token::StringLit("add works".into()));
    assert_eq!(toks[2], Token::LBrace);
    assert!(toks.contains(&Token::Assert));
}

// ── Ternary-style expressions ──────────────────────────────────────────

#[test]
fn ternary_chain() {
    let source = r#"derive x = a == "sm" ? "small" : "big""#;
    let toks = code_tokens(source);
    // Contains the comparison and string literals in the right order
    assert!(toks.contains(&Token::Derive));
    assert!(toks.contains(&Token::EqEq));
    assert!(toks.contains(&Token::Colon));
    assert!(toks
        .iter()
        .any(|t| matches!(t, Token::StringLit(s) if s == "sm")));
    assert!(toks
        .iter()
        .any(|t| matches!(t, Token::StringLit(s) if s == "small")));
    assert!(toks
        .iter()
        .any(|t| matches!(t, Token::StringLit(s) if s == "big")));
}

// ── Arrow functions ────────────────────────────────────────────────────

#[test]
fn arrow_function_expression() {
    let toks = code_tokens("() => {}");
    assert_eq!(
        toks,
        vec![
            Token::LParen,
            Token::RParen,
            Token::FatArrow,
            Token::LBrace,
            Token::RBrace
        ]
    );
}

// ── Spread and rest ────────────────────────────────────────────────────

#[test]
fn spread_in_object() {
    let toks = code_tokens("{ ...user, name: x }");
    assert!(toks.contains(&Token::Spread));
}

// ── Optional chaining + null coalesce ──────────────────────────────────

#[test]
fn optional_chain_and_coalesce() {
    let toks = code_tokens(r#"user?.address?.city ?? "unknown""#);
    assert_eq!(
        toks,
        vec![
            Token::Ident("user".into()),
            Token::QuestionDot,
            Token::Ident("address".into()),
            Token::QuestionDot,
            Token::Ident("city".into()),
            Token::NullCoalesce,
            Token::StringLit("unknown".into()),
        ]
    );
}

// ── Template mode: component body ──────────────────────────────────────

#[test]
fn simple_component_template() {
    // Simulate being inside a component body (Template mode)
    let source = r#"<button type="submit">"Click"</button>"#;
    let mut lexer = Lexer::new(source, 0);
    lexer.push_mode(LexMode::Template);
    let toks: Vec<Token> = lexer
        .tokenize_all()
        .into_iter()
        .map(|(t, _)| t)
        .filter(|t| !matches!(t, Token::EOF))
        .collect();

    assert!(lexer.errors().is_empty(), "errors: {:?}", lexer.errors());
    assert_eq!(toks[0], Token::HtmlOpen("button".into()));
    assert!(toks.contains(&Token::HtmlText("Click".into())));
    assert!(toks.contains(&Token::HtmlClose("button".into())));
}

#[test]
fn self_closing_component() {
    let source = "<Spinner />";
    let mut lexer = Lexer::new(source, 0);
    lexer.push_mode(LexMode::Template);
    let toks: Vec<Token> = lexer
        .tokenize_all()
        .into_iter()
        .map(|(t, _)| t)
        .filter(|t| !matches!(t, Token::EOF))
        .collect();

    assert!(lexer.errors().is_empty(), "errors: {:?}", lexer.errors());
    assert_eq!(toks[0], Token::HtmlOpen("Spinner".into()));
    assert!(toks.contains(&Token::HtmlSelfClose));
}

#[test]
fn template_with_expression_interpolation() {
    let source = "<span>{count}</span>";
    let mut lexer = Lexer::new(source, 0);
    lexer.push_mode(LexMode::Template);
    let toks: Vec<Token> = lexer
        .tokenize_all()
        .into_iter()
        .map(|(t, _)| t)
        .filter(|t| !matches!(t, Token::EOF))
        .collect();

    assert!(lexer.errors().is_empty(), "errors: {:?}", lexer.errors());
    assert_eq!(
        toks,
        vec![
            Token::HtmlOpen("span".into()),
            Token::RAngle,
            Token::ExprOpen,
            Token::Ident("count".into()),
            Token::ExprClose,
            Token::HtmlClose("span".into()),
        ]
    );
}

#[test]
fn template_with_directive_and_expression() {
    let source = r#"<input bind:name type="text" />"#;
    let mut lexer = Lexer::new(source, 0);
    lexer.push_mode(LexMode::Template);
    let toks: Vec<Token> = lexer
        .tokenize_all()
        .into_iter()
        .map(|(t, _)| t)
        .filter(|t| !matches!(t, Token::EOF))
        .collect();

    assert!(lexer.errors().is_empty(), "errors: {:?}", lexer.errors());
    assert_eq!(toks[0], Token::HtmlOpen("input".into()));
    assert!(toks.contains(&Token::BindDir("name".into())));
    assert!(toks.contains(&Token::HtmlSelfClose));
}

#[test]
fn when_block_in_template() {
    let source = "when loading {\n  <span />\n}";
    let mut lexer = Lexer::new(source, 0);
    lexer.push_mode(LexMode::Template);
    let toks: Vec<Token> = lexer
        .tokenize_all()
        .into_iter()
        .map(|(t, _)| t)
        .filter(|t| !matches!(t, Token::EOF))
        .collect();

    assert!(lexer.errors().is_empty(), "errors: {:?}", lexer.errors());
    assert_eq!(toks[0], Token::When);
    assert_eq!(toks[1], Token::Ident("loading".into()));
}

// ── Multi-line code ────────────────────────────────────────────────────

#[test]
fn multiline_code_block() {
    let source = "signal count = 0\nderive doubled = count * 2\nlet msg = `Count: ${doubled}`";
    let toks = code_tokens(source);

    // All three declarations should be present
    assert!(toks.contains(&Token::Signal));
    assert!(toks.contains(&Token::Derive));
    assert!(toks.contains(&Token::Let));
    assert!(toks
        .iter()
        .any(|t| matches!(t, Token::TemplateHead(s) if s == "Count: ")));
}

// ── Pipe operator ──────────────────────────────────────────────────────

#[test]
fn pipe_expression() {
    let toks = code_tokens("data |> filter |> sort");
    assert_eq!(
        toks,
        vec![
            Token::Ident("data".into()),
            Token::Pipe,
            Token::Ident("filter".into()),
            Token::Pipe,
            Token::Ident("sort".into()),
        ]
    );
}

// ── Range operator ─────────────────────────────────────────────────────

#[test]
fn range_in_for() {
    let toks = code_tokens("for i in 0..10");
    assert_eq!(
        toks,
        vec![
            Token::For,
            Token::Ident("i".into()),
            Token::Ident("in".into()),
            Token::IntLit(0),
            Token::DotDot,
            Token::IntLit(10),
        ]
    );
}

// ── Server/client blocks ───────────────────────────────────────────────

#[test]
fn server_block() {
    let toks = code_tokens(r#"server { let x = 1 }"#);
    assert_eq!(toks[0], Token::Server);
    assert_eq!(toks[1], Token::LBrace);
    assert!(toks.contains(&Token::Let));
}

#[test]
fn shared_guard() {
    let toks = code_tokens("shared { guard Email = string.email() }");
    assert_eq!(toks[0], Token::Shared);
    assert!(toks.contains(&Token::Guard));
}

// ── Comprehensive: realistic component signature ───────────────────────

#[test]
fn component_param_with_default() {
    let toks = code_tokens(r#"out ui Button(label: string, disabled: bool = false)"#);
    assert_eq!(toks[0], Token::Out);
    assert_eq!(toks[1], Token::Ui);
    assert_eq!(toks[2], Token::Ident("Button".into()));
    assert!(toks.contains(&Token::BoolLit(false)));
}

#[test]
fn action_declaration() {
    let toks = code_tokens("action createUser(input: User) -> User");
    assert_eq!(toks[0], Token::Action);
    assert!(toks.contains(&Token::Arrow));
}

#[test]
fn query_declaration() {
    let toks = code_tokens("query users = /api/users -> User[]");
    assert_eq!(toks[0], Token::Query);
    assert!(toks.contains(&Token::Arrow));
}

#[test]
fn store_declaration_header() {
    let toks = code_tokens("store Cart { }");
    assert_eq!(
        toks,
        vec![
            Token::Store,
            Token::Ident("Cart".into()),
            Token::LBrace,
            Token::RBrace
        ]
    );
}

#[test]
fn effect_block() {
    let toks = code_tokens("effect { }");
    assert_eq!(toks[0], Token::Effect);
}

#[test]
fn watch_expression() {
    let toks = code_tokens("watch count");
    assert_eq!(toks, vec![Token::Watch, Token::Ident("count".into())]);
}

// ── EOF always present ─────────────────────────────────────────────────

#[test]
fn eof_on_empty_input() {
    let result = lex("", 0);
    assert_eq!(result.tokens.len(), 1);
    assert_eq!(result.tokens[0].0, Token::EOF);
}

#[test]
fn eof_after_tokens() {
    let result = lex("let x", 0);
    assert_eq!(result.tokens.last().unwrap().0, Token::EOF);
}
