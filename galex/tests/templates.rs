//! Tests for template mode: HTML tags, directives, text nodes, expressions.

use galex::{LexMode, Lexer, Token};

/// Helper: lex in Template mode and collect tokens.
fn template_tokens(source: &str) -> Vec<Token> {
    let mut lexer = Lexer::new(source, 0);
    lexer.push_mode(LexMode::Template);
    let all = lexer.tokenize_all();
    assert!(
        lexer.errors().is_empty(),
        "unexpected lex errors: {:?}",
        lexer.errors()
    );
    all.into_iter()
        .map(|(tok, _)| tok)
        .filter(|t| !matches!(t, Token::EOF))
        .collect()
}

/// Helper: lex in HtmlTag mode (simulating inside <tag ...>).
fn tag_tokens(source: &str) -> Vec<Token> {
    let mut lexer = Lexer::new(source, 0);
    lexer.push_mode(LexMode::HtmlTag);
    let all = lexer.tokenize_all();
    assert!(
        lexer.errors().is_empty(),
        "unexpected lex errors: {:?}",
        lexer.errors()
    );
    all.into_iter()
        .map(|(tok, _)| tok)
        .filter(|t| !matches!(t, Token::EOF))
        .collect()
}

// ── HTML open tags ─────────────────────────────────────────────────────

#[test]
fn html_open_simple_tag() {
    let toks = template_tokens("<div>");
    assert!(toks.contains(&Token::HtmlOpen("div".into())));
    assert!(toks.contains(&Token::RAngle));
}

#[test]
fn html_open_component_tag() {
    let toks = template_tokens("<Button>");
    assert!(toks.contains(&Token::HtmlOpen("Button".into())));
}

#[test]
fn html_open_hyphenated_tag() {
    let toks = template_tokens("<my-component>");
    assert!(toks.contains(&Token::HtmlOpen("my-component".into())));
}

// ── HTML close tags ────────────────────────────────────────────────────

#[test]
fn html_close_tag() {
    let toks = template_tokens("</div>");
    assert_eq!(toks, vec![Token::HtmlClose("div".into())]);
}

#[test]
fn html_close_component_tag() {
    let toks = template_tokens("</Button>");
    assert_eq!(toks, vec![Token::HtmlClose("Button".into())]);
}

// ── Self-closing tags ──────────────────────────────────────────────────

#[test]
fn self_closing_in_tag_mode() {
    let toks = tag_tokens("/>");
    assert_eq!(toks, vec![Token::HtmlSelfClose]);
}

// ── Tag attributes ─────────────────────────────────────────────────────

#[test]
fn simple_attribute() {
    let toks = tag_tokens(r#"class="container""#);
    assert_eq!(
        toks,
        vec![
            Token::Ident("class".into()),
            Token::Eq,
            Token::StringLit("container".into()),
        ]
    );
}

#[test]
fn attribute_with_expression() {
    let toks = tag_tokens("disabled={true}");
    assert_eq!(
        toks,
        vec![
            Token::Ident("disabled".into()),
            Token::Eq,
            Token::ExprOpen,
            Token::BoolLit(true),
            Token::ExprClose,
        ]
    );
}

// ── Directives ─────────────────────────────────────────────────────────

#[test]
fn bind_directive() {
    let toks = tag_tokens("bind:name");
    assert_eq!(toks, vec![Token::BindDir("name".into())]);
}

#[test]
fn on_directive_simple() {
    let toks = tag_tokens("on:click");
    assert_eq!(
        toks,
        vec![Token::OnDir {
            event: "click".into(),
            modifiers: vec![],
        }]
    );
}

#[test]
fn on_directive_with_modifiers() {
    let toks = tag_tokens("on:submit.prevent");
    assert_eq!(
        toks,
        vec![Token::OnDir {
            event: "submit".into(),
            modifiers: vec!["prevent".into()],
        }]
    );
}

#[test]
fn on_directive_multiple_modifiers() {
    let toks = tag_tokens("on:keydown.enter.shift");
    assert_eq!(
        toks,
        vec![Token::OnDir {
            event: "keydown".into(),
            modifiers: vec!["enter".into(), "shift".into()],
        }]
    );
}

#[test]
fn class_directive() {
    let toks = tag_tokens("class:hidden");
    assert_eq!(toks, vec![Token::ClassDir("hidden".into())]);
}

#[test]
fn class_directive_hyphenated() {
    let toks = tag_tokens("class:bg-blue-500");
    assert_eq!(toks, vec![Token::ClassDir("bg-blue-500".into())]);
}

#[test]
fn ref_directive() {
    let toks = tag_tokens("ref:canvas");
    assert_eq!(toks, vec![Token::RefDir("canvas".into())]);
}

#[test]
fn transition_directive() {
    let toks = tag_tokens("transition:fade");
    assert_eq!(toks, vec![Token::TransDir("fade".into())]);
}

#[test]
fn into_directive() {
    let toks = tag_tokens("into:header");
    assert_eq!(toks, vec![Token::IntoDir("header".into())]);
}

#[test]
fn key_directive() {
    let toks = tag_tokens("key");
    assert_eq!(toks, vec![Token::KeyDir]);
}

#[test]
fn prefetch_directive() {
    let toks = tag_tokens("prefetch");
    assert_eq!(toks, vec![Token::Prefetch]);
}

#[test]
fn form_action_directive() {
    let toks = tag_tokens("form:action");
    assert_eq!(toks, vec![Token::FormAction]);
}

#[test]
fn form_guard_directive() {
    let toks = tag_tokens("form:guard");
    assert_eq!(toks, vec![Token::FormGuard]);
}

#[test]
fn form_error_directive() {
    let toks = tag_tokens("form:error");
    assert_eq!(toks, vec![Token::FormError]);
}

// ── Directive with value ───────────────────────────────────────────────

#[test]
fn on_directive_with_expression_value() {
    let toks = tag_tokens("on:click={handler}");
    assert_eq!(
        toks,
        vec![
            Token::OnDir {
                event: "click".into(),
                modifiers: vec![],
            },
            Token::Eq,
            Token::ExprOpen,
            Token::Ident("handler".into()),
            Token::ExprClose,
        ]
    );
}

#[test]
fn bind_directive_self_closing() {
    // bind:name /> (self-closing with bind directive)
    let toks = tag_tokens("bind:name />");
    assert_eq!(
        toks,
        vec![Token::BindDir("name".into()), Token::HtmlSelfClose]
    );
}

// ── Template text nodes ────────────────────────────────────────────────

#[test]
fn quoted_text_node() {
    let toks = template_tokens(r#""Hello, world!""#);
    assert_eq!(toks, vec![Token::HtmlText("Hello, world!".into())]);
}

// ── Template expressions ───────────────────────────────────────────────

#[test]
fn expression_in_template() {
    let toks = template_tokens("{count}");
    assert_eq!(
        toks,
        vec![
            Token::ExprOpen,
            Token::Ident("count".into()),
            Token::ExprClose,
        ]
    );
}

#[test]
fn complex_expression_in_template() {
    let toks = template_tokens("{a + b}");
    assert_eq!(
        toks,
        vec![
            Token::ExprOpen,
            Token::Ident("a".into()),
            Token::Plus,
            Token::Ident("b".into()),
            Token::ExprClose,
        ]
    );
}

#[test]
fn nested_braces_in_expression() {
    // {call({key: val})} — nested braces should work
    let toks = template_tokens("{call({x: 1})}");
    assert_eq!(
        toks,
        vec![
            Token::ExprOpen,
            Token::Ident("call".into()),
            Token::LParen,
            Token::LBrace,
            Token::Ident("x".into()),
            Token::Colon,
            Token::IntLit(1),
            Token::RBrace,
            Token::RParen,
            Token::ExprClose,
        ]
    );
}

// ── Template control flow keywords ─────────────────────────────────────

#[test]
fn when_keyword_in_template() {
    let toks = template_tokens("when");
    assert_eq!(toks, vec![Token::When]);
}

#[test]
fn each_keyword_in_template() {
    let toks = template_tokens("each");
    assert_eq!(toks, vec![Token::Each]);
}

#[test]
fn slot_keyword_in_template() {
    let toks = template_tokens("slot");
    assert_eq!(toks, vec![Token::Slot]);
}

// ── Comments in template mode ──────────────────────────────────────────

#[test]
fn line_comment_in_template() {
    let toks = template_tokens("// a comment");
    assert_eq!(toks, vec![Token::Comment("a comment".into())]);
}
