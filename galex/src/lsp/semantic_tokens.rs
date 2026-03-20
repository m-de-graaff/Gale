//! Semantic token provider — type-aware syntax highlighting.
//!
//! Emits semantic tokens for identifiers, declarations, types, and other
//! language constructs that benefit from type-informed highlighting beyond
//! what TextMate regexes can provide.

use lsp_types::{SemanticToken, SemanticTokenModifier, SemanticTokenType, SemanticTokensLegend};

use super::document::DocumentManager;
use crate::ast::*;
use crate::span::Span;
use crate::types::env::BindingKind;

// ── Legend ──────────────────────────────────────────────────────────────

/// Semantic token types used by the GaleX LSP.
pub const TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::NAMESPACE,      // 0  - boundary blocks
    SemanticTokenType::TYPE,           // 1  - guards, types
    SemanticTokenType::CLASS,          // 2  - components, layouts
    SemanticTokenType::ENUM,           // 3  - enums
    SemanticTokenType::INTERFACE,      // 4  - channels, queries
    SemanticTokenType::STRUCT,         // 5  - guards (usage)
    SemanticTokenType::TYPE_PARAMETER, // 6  - type params
    SemanticTokenType::PARAMETER,      // 7  - function params
    SemanticTokenType::VARIABLE,       // 8  - let, mut
    SemanticTokenType::PROPERTY,       // 9  - fields, HTML attributes
    SemanticTokenType::ENUM_MEMBER,    // 10 - enum variants
    SemanticTokenType::FUNCTION,       // 11 - functions, actions
    SemanticTokenType::METHOD,         // 12 - validators, store methods
    SemanticTokenType::MACRO,          // 13 - directives
    SemanticTokenType::KEYWORD,        // 14 - keywords (optional)
    SemanticTokenType::STRING,         // 15 - strings
    SemanticTokenType::NUMBER,         // 16 - numbers
    SemanticTokenType::OPERATOR,       // 17 - operators
    SemanticTokenType::DECORATOR,      // 18 - validator chains
];

/// Semantic token modifiers.
pub const TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[
    SemanticTokenModifier::DECLARATION,     // 0
    SemanticTokenModifier::DEFINITION,      // 1
    SemanticTokenModifier::READONLY,        // 2
    SemanticTokenModifier::STATIC,          // 3
    SemanticTokenModifier::MODIFICATION,    // 4
    SemanticTokenModifier::ASYNC,           // 5
    SemanticTokenModifier::DEFAULT_LIBRARY, // 6
];

pub fn legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: TOKEN_TYPES.to_vec(),
        token_modifiers: TOKEN_MODIFIERS.to_vec(),
    }
}

// ── Token emission ─────────────────────────────────────────────────────

/// A collected semantic token before delta encoding.
struct RawToken {
    line: u32,
    col: u32,
    length: u32,
    token_type: u32,
    modifiers: u32,
}

/// Generate semantic tokens for the entire document.
pub fn provide_semantic_tokens(docs: &DocumentManager, source: &str) -> Vec<SemanticToken> {
    let program = match docs.merged_program() {
        Some(p) => p,
        None => return vec![],
    };
    let checker = docs.cached_checker.as_ref();

    let mut raw_tokens = Vec::new();
    emit_program_tokens(program, source, checker, &mut raw_tokens);

    // Sort by position (line, then column)
    raw_tokens.sort_by(|a, b| a.line.cmp(&b.line).then(a.col.cmp(&b.col)));

    // Delta-encode
    delta_encode(&raw_tokens)
}

fn delta_encode(tokens: &[RawToken]) -> Vec<SemanticToken> {
    let mut result = Vec::with_capacity(tokens.len());
    let mut prev_line = 0u32;
    let mut prev_start = 0u32;

    for t in tokens {
        let delta_line = t.line - prev_line;
        let delta_start = if delta_line == 0 {
            t.col - prev_start
        } else {
            t.col
        };
        result.push(SemanticToken {
            delta_line,
            delta_start,
            length: t.length,
            token_type: t.token_type,
            token_modifiers_bitset: t.modifiers,
        });
        prev_line = t.line;
        prev_start = t.col;
    }

    result
}

// ── AST walkers ────────────────────────────────────────────────────────

fn emit_program_tokens(
    program: &Program,
    source: &str,
    checker: Option<&crate::checker::TypeChecker>,
    tokens: &mut Vec<RawToken>,
) {
    for item in &program.items {
        emit_item_tokens(item, source, checker, tokens);
    }
}

fn emit_item_tokens(
    item: &Item,
    source: &str,
    checker: Option<&crate::checker::TypeChecker>,
    tokens: &mut Vec<RawToken>,
) {
    match item {
        Item::GuardDecl(g) => {
            emit_span(g.span, source, 1, 1, tokens); // type + declaration
            for field in &g.fields {
                emit_span(field.span, source, 9, 0, tokens); // property
                for v in &field.validators {
                    emit_span(v.span, source, 18, 0, tokens); // decorator
                }
            }
        }
        Item::ComponentDecl(c) => {
            emit_name_at_span(&c.name, &c.span, source, 2, 1, tokens); // class + declaration
            emit_component_body(&c.body, source, checker, tokens);
        }
        Item::LayoutDecl(l) => {
            emit_name_at_span(&l.name, &l.span, source, 2, 1, tokens);
            emit_component_body(&l.body, source, checker, tokens);
        }
        Item::FnDecl(f) => {
            emit_name_at_span(&f.name, &f.span, source, 11, 1, tokens); // function + declaration
            for param in &f.params {
                emit_span(param.span, source, 7, 0, tokens); // parameter
            }
            emit_block_tokens(&f.body, source, checker, tokens);
        }
        Item::ActionDecl(a) => {
            emit_name_at_span(&a.name, &a.span, source, 11, 1, tokens);
            for param in &a.params {
                emit_span(param.span, source, 7, 0, tokens);
            }
            emit_block_tokens(&a.body, source, checker, tokens);
        }
        Item::StoreDecl(s) => {
            emit_name_at_span(&s.name, &s.span, source, 0, 1, tokens); // namespace + declaration
            for member in &s.members {
                match member {
                    StoreMember::Method(f) => {
                        emit_name_at_span(&f.name, &f.span, source, 12, 1, tokens);
                    }
                    StoreMember::Signal(stmt) | StoreMember::Derive(stmt) => {
                        emit_stmt_tokens(stmt, source, checker, tokens);
                    }
                }
            }
        }
        Item::ChannelDecl(ch) => {
            emit_name_at_span(&ch.name, &ch.span, source, 4, 1, tokens); // interface + declaration
        }
        Item::EnumDecl(e) => {
            emit_name_at_span(&e.name, &e.span, source, 3, 1, tokens); // enum + declaration
        }
        Item::QueryDecl(q) => {
            emit_name_at_span(&q.name, &q.span, source, 4, 1, tokens);
        }
        Item::ApiDecl(a) => {
            emit_name_at_span(&a.name, &a.span, source, 0, 1, tokens);
        }
        Item::MiddlewareDecl(m) => {
            emit_name_at_span(&m.name, &m.span, source, 11, 1, tokens);
        }
        Item::Out(out) => emit_item_tokens(&out.inner, source, checker, tokens),
        Item::ServerBlock(b) | Item::ClientBlock(b) | Item::SharedBlock(b) => {
            for inner in &b.items {
                emit_item_tokens(inner, source, checker, tokens);
            }
        }
        Item::Stmt(stmt) => emit_stmt_tokens(stmt, source, checker, tokens),
        _ => {}
    }
}

fn emit_component_body(
    body: &ComponentBody,
    source: &str,
    checker: Option<&crate::checker::TypeChecker>,
    tokens: &mut Vec<RawToken>,
) {
    for stmt in &body.stmts {
        emit_stmt_tokens(stmt, source, checker, tokens);
    }
    for node in &body.template {
        emit_template_tokens(node, source, checker, tokens);
    }
}

fn emit_stmt_tokens(
    stmt: &Stmt,
    source: &str,
    checker: Option<&crate::checker::TypeChecker>,
    tokens: &mut Vec<RawToken>,
) {
    match stmt {
        Stmt::Signal { init, .. } => {
            emit_expr_tokens(init, source, checker, tokens);
        }
        Stmt::Derive { init, .. } | Stmt::Frozen { init, .. } => {
            emit_expr_tokens(init, source, checker, tokens);
        }
        Stmt::Let { init, .. } | Stmt::Mut { init, .. } => {
            emit_expr_tokens(init, source, checker, tokens);
        }
        Stmt::If {
            condition,
            then_block,
            else_branch,
            ..
        } => {
            emit_expr_tokens(condition, source, checker, tokens);
            emit_block_tokens(then_block, source, checker, tokens);
            if let Some(ElseBranch::Else(block)) = else_branch {
                emit_block_tokens(block, source, checker, tokens);
            }
            if let Some(ElseBranch::ElseIf(stmt)) = else_branch {
                emit_stmt_tokens(stmt, source, checker, tokens);
            }
        }
        Stmt::For { iterable, body, .. } => {
            emit_expr_tokens(iterable, source, checker, tokens);
            emit_block_tokens(body, source, checker, tokens);
        }
        Stmt::Effect { body, .. } | Stmt::Watch { body, .. } => {
            emit_block_tokens(body, source, checker, tokens);
        }
        Stmt::ExprStmt { expr, .. } => {
            emit_expr_tokens(expr, source, checker, tokens);
        }
        Stmt::FnDecl(f) => {
            emit_name_at_span(&f.name, &f.span, source, 11, 1, tokens);
            emit_block_tokens(&f.body, source, checker, tokens);
        }
        _ => {}
    }
}

fn emit_expr_tokens(
    expr: &Expr,
    source: &str,
    checker: Option<&crate::checker::TypeChecker>,
    tokens: &mut Vec<RawToken>,
) {
    match expr {
        Expr::Ident { name, span } => {
            // Resolve the binding kind for proper coloring
            let token_type = if let Some(ch) = checker {
                if let Some(binding) = ch.env.lookup(name) {
                    match binding.kind {
                        BindingKind::Function | BindingKind::Action => 11,
                        BindingKind::Signal | BindingKind::Derived => 8,
                        BindingKind::Guard => 5,
                        BindingKind::Store => 0,
                        BindingKind::Component => 2,
                        BindingKind::Channel | BindingKind::Query => 4,
                        BindingKind::TypeAlias | BindingKind::EnumDef => 1,
                        BindingKind::Parameter => 7,
                        _ => 8,
                    }
                } else {
                    8 // default to variable
                }
            } else {
                8
            };
            emit_span(*span, source, token_type, 0, tokens);
        }
        Expr::FnCall { callee, args, .. } => {
            // The callee gets function coloring
            if let Expr::Ident { span, .. } = callee.as_ref() {
                emit_span(*span, source, 11, 0, tokens);
            } else {
                emit_expr_tokens(callee, source, checker, tokens);
            }
            for arg in args {
                emit_expr_tokens(arg, source, checker, tokens);
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            emit_expr_tokens(left, source, checker, tokens);
            emit_expr_tokens(right, source, checker, tokens);
        }
        Expr::MemberAccess { object, .. } | Expr::OptionalChain { object, .. } => {
            emit_expr_tokens(object, source, checker, tokens);
        }
        Expr::ArrayLit { elements, .. } => {
            for el in elements {
                emit_expr_tokens(el, source, checker, tokens);
            }
        }
        Expr::ObjectLit { fields, .. } => {
            for f in fields {
                emit_expr_tokens(&f.value, source, checker, tokens);
            }
        }
        Expr::Await { expr, .. } | Expr::Spread { expr, .. } | Expr::Assert { expr, .. } => {
            emit_expr_tokens(expr, source, checker, tokens);
        }
        Expr::ArrowFn { params, body, .. } => {
            for p in params {
                emit_span(p.span, source, 7, 0, tokens);
            }
            match body {
                ArrowBody::Expr(e) => emit_expr_tokens(e, source, checker, tokens),
                ArrowBody::Block(b) => emit_block_tokens(b, source, checker, tokens),
            }
        }
        Expr::Ternary {
            condition,
            then_expr,
            else_expr,
            ..
        } => {
            emit_expr_tokens(condition, source, checker, tokens);
            emit_expr_tokens(then_expr, source, checker, tokens);
            emit_expr_tokens(else_expr, source, checker, tokens);
        }
        Expr::Assign { target, value, .. } => {
            emit_expr_tokens(target, source, checker, tokens);
            emit_expr_tokens(value, source, checker, tokens);
        }
        _ => {}
    }
}

fn emit_template_tokens(
    node: &TemplateNode,
    source: &str,
    checker: Option<&crate::checker::TypeChecker>,
    tokens: &mut Vec<RawToken>,
) {
    match node {
        TemplateNode::Element {
            children,
            directives,
            attributes,
            ..
        } => {
            for dir in directives {
                emit_directive_tokens(dir, source, checker, tokens);
            }
            for attr in attributes {
                if let AttrValue::Expr(e) = &attr.value {
                    emit_expr_tokens(e, source, checker, tokens);
                }
            }
            for child in children {
                emit_template_tokens(child, source, checker, tokens);
            }
        }
        TemplateNode::SelfClosing {
            directives,
            attributes,
            ..
        } => {
            for dir in directives {
                emit_directive_tokens(dir, source, checker, tokens);
            }
            for attr in attributes {
                if let AttrValue::Expr(e) = &attr.value {
                    emit_expr_tokens(e, source, checker, tokens);
                }
            }
        }
        TemplateNode::ExprInterp { expr, .. } => {
            emit_expr_tokens(expr, source, checker, tokens);
        }
        TemplateNode::When {
            condition, body, ..
        } => {
            emit_expr_tokens(condition, source, checker, tokens);
            for child in body {
                emit_template_tokens(child, source, checker, tokens);
            }
        }
        TemplateNode::Each { iterable, body, .. } => {
            emit_expr_tokens(iterable, source, checker, tokens);
            for child in body {
                emit_template_tokens(child, source, checker, tokens);
            }
        }
        TemplateNode::Suspend { body, .. } => {
            for child in body {
                emit_template_tokens(child, source, checker, tokens);
            }
        }
        _ => {}
    }
}

fn emit_directive_tokens(
    dir: &Directive,
    source: &str,
    checker: Option<&crate::checker::TypeChecker>,
    tokens: &mut Vec<RawToken>,
) {
    match dir {
        Directive::On { handler, span, .. } => {
            emit_span(*span, source, 13, 0, tokens); // macro = directive
            emit_expr_tokens(handler, source, checker, tokens);
        }
        Directive::Class {
            condition, span, ..
        } => {
            emit_span(*span, source, 13, 0, tokens);
            emit_expr_tokens(condition, source, checker, tokens);
        }
        Directive::Key { expr, span } => {
            emit_span(*span, source, 13, 0, tokens);
            emit_expr_tokens(expr, source, checker, tokens);
        }
        Directive::FormAction { action, span } => {
            emit_span(*span, source, 13, 0, tokens);
            emit_expr_tokens(action, source, checker, tokens);
        }
        Directive::FormGuard { guard, span } => {
            emit_span(*span, source, 13, 0, tokens);
            emit_expr_tokens(guard, source, checker, tokens);
        }
        Directive::Bind { span, .. }
        | Directive::Ref { span, .. }
        | Directive::Transition { span, .. }
        | Directive::Into { span, .. }
        | Directive::FormError { span, .. }
        | Directive::Prefetch { span, .. } => {
            emit_span(*span, source, 13, 0, tokens);
        }
    }
}

fn emit_block_tokens(
    block: &Block,
    source: &str,
    checker: Option<&crate::checker::TypeChecker>,
    tokens: &mut Vec<RawToken>,
) {
    for stmt in &block.stmts {
        emit_stmt_tokens(stmt, source, checker, tokens);
    }
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Emit a token for the given span. Converts 1-based span line/col to 0-based.
fn emit_span(
    span: Span,
    _source: &str,
    token_type: u32,
    modifiers: u32,
    tokens: &mut Vec<RawToken>,
) {
    if span.is_empty() || span.line == 0 {
        return;
    }
    tokens.push(RawToken {
        line: span.line - 1,
        col: span.col - 1,
        length: span.len().min(200), // Cap length for safety
        token_type,
        modifiers,
    });
}

/// Emit a token for just the name portion of a declaration.
/// Uses the span start + offset to the name.
fn emit_name_at_span(
    name: &str,
    span: &Span,
    _source: &str,
    token_type: u32,
    modifiers: u32,
    tokens: &mut Vec<RawToken>,
) {
    if span.line == 0 {
        return;
    }
    // The name might not start at the span start (e.g., `fn name` — name is after `fn `).
    // Use the span position as an approximation; the name length is what we know.
    tokens.push(RawToken {
        line: span.line - 1,
        col: span.col - 1,
        length: name.len() as u32,
        token_type,
        modifiers,
    });
}
