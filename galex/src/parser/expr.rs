//! Expression parser — Pratt (top-down operator precedence) parser.
//!
//! Binding power table (low → high):
//!   1: assignment (`=`, `+=`, `-=`)
//!   2: ternary (`? :`)
//!   3: null coalescing (`??`)
//!   4: logical OR (`||`)
//!   5: logical AND (`&&`)
//!   6: equality (`==`, `!=`)
//!   7: comparison (`<`, `>`, `<=`, `>=`)
//!   8: pipe (`|>`)
//!   9: range (`..`)
//!  10: additive (`+`, `-`)
//!  11: multiplicative (`*`, `/`, `%`)
//!  12: prefix (`-`, `!`, `await`, `...`)
//!  13: postfix (`.`, `?.`, `[`, `(`)

use crate::ast::*;
use crate::span::Span;
use crate::token::Token;
use smol_str::SmolStr;

use super::error::ParseError;
use super::Parser;

impl<'src> Parser<'src> {
    /// Parse an expression.
    pub(crate) fn parse_expr(&mut self) -> Expr {
        self.parse_expr_bp(1)
    }

    /// Pratt parser core: parse expression with minimum binding power.
    fn parse_expr_bp(&mut self, min_bp: u8) -> Expr {
        let mut left = self.parse_prefix();

        loop {
            // Check for postfix/infix operators
            let op_bp = self.infix_bp();
            if op_bp.is_none() {
                break;
            }
            let (l_bp, r_bp) = op_bp.unwrap();
            if l_bp < min_bp {
                break;
            }

            left = self.parse_infix(left, r_bp);
        }

        left
    }

    /// Parse a prefix expression (atoms, unary operators, grouping).
    fn parse_prefix(&mut self) -> Expr {
        let start = self.peek_span();
        match self.peek().clone() {
            // ── Literals ────────────────────────────────────────
            Token::IntLit(v) => {
                self.advance();
                Expr::IntLit {
                    value: v,
                    span: start,
                }
            }
            Token::FloatLit(v) => {
                self.advance();
                Expr::FloatLit {
                    value: v,
                    span: start,
                }
            }
            Token::StringLit(v) => {
                self.advance();
                Expr::StringLit {
                    value: v.into(),
                    span: start,
                }
            }
            Token::BoolLit(v) => {
                self.advance();
                Expr::BoolLit {
                    value: v,
                    span: start,
                }
            }
            Token::NullLit => {
                self.advance();
                Expr::NullLit { span: start }
            }
            Token::RegexLit { pattern, flags } => {
                self.advance();
                Expr::RegexLit {
                    pattern: pattern.into(),
                    flags: flags.into(),
                    span: start,
                }
            }

            // ── Template literals ───────────────────────────────
            Token::TemplateNoSub(text) => {
                self.advance();
                Expr::TemplateLit {
                    parts: vec![TemplatePart::Text(text.into())],
                    span: start,
                }
            }
            Token::TemplateHead(text) => {
                self.advance();
                self.parse_template_literal(text, start)
            }

            // ── Identifier / env access ─────────────────────────
            Token::Ident(name) => {
                self.advance();
                Expr::Ident {
                    name: name.into(),
                    span: start,
                }
            }
            Token::Env => {
                self.advance();
                self.expect(&Token::Dot, "`.`");
                let key = self.expect_ident("environment variable name");
                let span = self.span_from(start);
                Expr::EnvAccess {
                    key: key.into(),
                    span,
                }
            }

            // ── Grouping / arrow function ───────────────────────
            Token::LParen => {
                self.advance();
                // Could be grouping `(expr)` or arrow fn `(params) => body`
                // Lookahead: if we see `)` followed by `=>`, it's an arrow fn
                if self.at(&Token::RParen) {
                    // `() => ...` — zero-param arrow
                    self.advance(); // consume )
                    self.expect(&Token::FatArrow, "`=>`");
                    return self.parse_arrow_body(vec![], start);
                }
                // Try to detect arrow fn by checking if first thing is `ident :` or
                // if we see `) =>`. For now, parse as expr and check for `=>` after `)`.
                let expr = self.parse_expr();
                if self.eat(&Token::Comma).is_some() {
                    // Multi-param arrow: `(a, b) => ...`
                    let first_param = self.expr_to_param(&expr);
                    let mut params = vec![first_param];
                    self.skip_newlines();
                    while !self.at(&Token::RParen) && !self.eof() {
                        let p_start = self.peek_span();
                        let name = self.expect_ident("parameter name");
                        let ty_ann = if self.eat(&Token::Colon).is_some() {
                            Some(self.parse_type_annotation())
                        } else {
                            None
                        };
                        let default = if self.eat(&Token::Eq).is_some() {
                            Some(self.parse_expr())
                        } else {
                            None
                        };
                        params.push(Param {
                            name: name.into(),
                            ty_ann,
                            default,
                            span: self.span_from(p_start),
                        });
                        self.skip_newlines();
                        if self.eat(&Token::Comma).is_none() {
                            break;
                        }
                        self.skip_newlines();
                    }
                    self.expect(&Token::RParen, "`)`");
                    self.expect(&Token::FatArrow, "`=>`");
                    return self.parse_arrow_body(params, start);
                }
                self.expect(&Token::RParen, "`)`");
                // Check for arrow: `(x) => ...`
                if self.at(&Token::FatArrow) {
                    self.advance();
                    let param = self.expr_to_param(&expr);
                    return self.parse_arrow_body(vec![param], start);
                }
                // Just grouping
                expr
            }

            // ── Array literal ───────────────────────────────────
            Token::LBracket => {
                self.advance();
                let mut elements = Vec::new();
                self.skip_newlines();
                while !self.at(&Token::RBracket) && !self.eof() {
                    elements.push(self.parse_expr());
                    self.skip_newlines();
                    if self.eat(&Token::Comma).is_none() {
                        break;
                    }
                    self.skip_newlines();
                }
                self.expect(&Token::RBracket, "`]`");
                let span = self.span_from(start);
                Expr::ArrayLit { elements, span }
            }

            // ── Object literal ──────────────────────────────────
            Token::LBrace => {
                self.advance();
                let mut fields = Vec::new();
                self.skip_newlines();
                while !self.at(&Token::RBrace) && !self.eof() {
                    let f_start = self.peek_span();
                    let key = self.expect_ident("object key");
                    self.expect(&Token::Colon, "`:`");
                    let value = self.parse_expr();
                    let f_span = self.span_from(f_start);
                    fields.push(ObjectFieldExpr {
                        key: key.into(),
                        value,
                        span: f_span,
                    });
                    self.skip_newlines();
                    if self.eat(&Token::Comma).is_none() {
                        break;
                    }
                    self.skip_newlines();
                }
                self.expect(&Token::RBrace, "`}`");
                let span = self.span_from(start);
                Expr::ObjectLit { fields, span }
            }

            // ── Unary prefix ────────────────────────────────────
            Token::Minus => {
                self.advance();
                let operand = self.parse_expr_bp(12); // prefix binding power
                let span = self.span_from(start);
                Expr::UnaryOp {
                    op: UnaryOp::Neg,
                    operand: Box::new(operand),
                    span,
                }
            }
            Token::Not => {
                self.advance();
                let operand = self.parse_expr_bp(12);
                let span = self.span_from(start);
                Expr::UnaryOp {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                    span,
                }
            }
            Token::Await => {
                self.advance();
                let expr = self.parse_expr_bp(12);
                let span = self.span_from(start);
                Expr::Await {
                    expr: Box::new(expr),
                    span,
                }
            }
            Token::Spread => {
                self.advance();
                let expr = self.parse_expr_bp(12);
                let span = self.span_from(start);
                Expr::Spread {
                    expr: Box::new(expr),
                    span,
                }
            }
            Token::Assert => {
                self.advance();
                let expr = self.parse_expr_bp(12);
                let span = self.span_from(start);
                Expr::Assert {
                    expr: Box::new(expr),
                    span,
                }
            }

            // ── Single-param arrow fn: `x => expr` ─────────────
            // This is tricky — we see an ident and need to check if `=>` follows.
            // Already handled by Ident above + postfix check. But we also need
            // to handle it in `parse_infix` for the `FatArrow` case.
            _ => {
                let span = self.peek_span();
                let found = self.peek().clone();
                self.error(ParseError::invalid_expr(
                    &format!("unexpected {} in expression", found.kind_str()),
                    span,
                ));
                self.advance();
                Expr::NullLit { span }
            }
        }
    }

    /// Get the binding power of the current infix/postfix operator.
    /// Returns `(left_bp, right_bp)` or `None` if not an operator.
    fn infix_bp(&mut self) -> Option<(u8, u8)> {
        match self.peek() {
            // Assignment (right-associative)
            Token::Eq | Token::PlusEq | Token::MinusEq => Some((2, 1)),
            // Ternary
            Token::Question => Some((3, 3)),
            // Null coalescing
            Token::NullCoalesce => Some((5, 6)),
            // Logical OR
            Token::Or => Some((7, 8)),
            // Logical AND
            Token::And => Some((9, 10)),
            // Equality
            Token::EqEq | Token::NotEq => Some((11, 12)),
            // Comparison
            Token::Less | Token::Greater | Token::LessEq | Token::GreaterEq => Some((13, 14)),
            // Pipe
            Token::Pipe => Some((15, 16)),
            // Range
            Token::DotDot => Some((17, 18)),
            // Additive
            Token::Plus | Token::Minus => Some((19, 20)),
            // Multiplicative
            Token::Star | Token::Slash | Token::Percent => Some((21, 22)),
            // Postfix: member access, optional chain, call, index
            Token::Dot | Token::QuestionDot | Token::LParen | Token::LBracket => Some((23, 24)),
            // Arrow function: `ident => ...` (low priority)
            Token::FatArrow => Some((1, 1)),
            _ => None,
        }
    }

    /// Parse an infix/postfix expression with the given right binding power.
    fn parse_infix(&mut self, left: Expr, r_bp: u8) -> Expr {
        let start = left.span();
        let op_tok = self.advance();

        match op_tok.0 {
            // ── Binary operators ─────────────────────────────────
            Token::Plus => self.make_binary(left, BinOp::Add, r_bp, start),
            Token::Minus => self.make_binary(left, BinOp::Sub, r_bp, start),
            Token::Star => self.make_binary(left, BinOp::Mul, r_bp, start),
            Token::Slash => self.make_binary(left, BinOp::Div, r_bp, start),
            Token::Percent => self.make_binary(left, BinOp::Mod, r_bp, start),
            Token::EqEq => self.make_binary(left, BinOp::Eq, r_bp, start),
            Token::NotEq => self.make_binary(left, BinOp::NotEq, r_bp, start),
            Token::Less => self.make_binary(left, BinOp::Lt, r_bp, start),
            Token::Greater => self.make_binary(left, BinOp::Gt, r_bp, start),
            Token::LessEq => self.make_binary(left, BinOp::LtEq, r_bp, start),
            Token::GreaterEq => self.make_binary(left, BinOp::GtEq, r_bp, start),
            Token::And => self.make_binary(left, BinOp::And, r_bp, start),
            Token::Or => self.make_binary(left, BinOp::Or, r_bp, start),
            Token::DotDot => self.make_binary(left, BinOp::DotDot, r_bp, start),

            // ── Pipe ────────────────────────────────────────────
            Token::Pipe => {
                let right = self.parse_expr_bp(r_bp);
                let span = Self::span_between(start, right.span());
                Expr::Pipe {
                    left: Box::new(left),
                    right: Box::new(right),
                    span,
                }
            }

            // ── Null coalescing ─────────────────────────────────
            Token::NullCoalesce => {
                let right = self.parse_expr_bp(r_bp);
                let span = Self::span_between(start, right.span());
                Expr::NullCoalesce {
                    left: Box::new(left),
                    right: Box::new(right),
                    span,
                }
            }

            // ── Ternary ─────────────────────────────────────────
            Token::Question => {
                let then_expr = self.parse_expr();
                self.expect(&Token::Colon, "`:`");
                let else_expr = self.parse_expr_bp(r_bp);
                let span = Self::span_between(start, else_expr.span());
                Expr::Ternary {
                    condition: Box::new(left),
                    then_expr: Box::new(then_expr),
                    else_expr: Box::new(else_expr),
                    span,
                }
            }

            // ── Assignment ──────────────────────────────────────
            Token::Eq => {
                let value = self.parse_expr_bp(r_bp);
                let span = Self::span_between(start, value.span());
                Expr::Assign {
                    target: Box::new(left),
                    op: AssignOp::Assign,
                    value: Box::new(value),
                    span,
                }
            }
            Token::PlusEq => {
                let value = self.parse_expr_bp(r_bp);
                let span = Self::span_between(start, value.span());
                Expr::Assign {
                    target: Box::new(left),
                    op: AssignOp::AddAssign,
                    value: Box::new(value),
                    span,
                }
            }
            Token::MinusEq => {
                let value = self.parse_expr_bp(r_bp);
                let span = Self::span_between(start, value.span());
                Expr::Assign {
                    target: Box::new(left),
                    op: AssignOp::SubAssign,
                    value: Box::new(value),
                    span,
                }
            }

            // ── Member access ───────────────────────────────────
            Token::Dot => {
                let field = self.expect_ident("field name");
                let span = self.span_from(start);
                Expr::MemberAccess {
                    object: Box::new(left),
                    field: field.into(),
                    span,
                }
            }

            // ── Optional chain ──────────────────────────────────
            Token::QuestionDot => {
                let field = self.expect_ident("field name");
                let span = self.span_from(start);
                Expr::OptionalChain {
                    object: Box::new(left),
                    field: field.into(),
                    span,
                }
            }

            // ── Function call ───────────────────────────────────
            Token::LParen => {
                let mut args = Vec::new();
                self.skip_newlines();
                while !self.at(&Token::RParen) && !self.eof() {
                    args.push(self.parse_expr());
                    self.skip_newlines();
                    if self.eat(&Token::Comma).is_none() {
                        break;
                    }
                    self.skip_newlines();
                }
                self.expect(&Token::RParen, "`)`");
                let span = self.span_from(start);
                Expr::FnCall {
                    callee: Box::new(left),
                    args,
                    span,
                }
            }

            // ── Index access ────────────────────────────────────
            Token::LBracket => {
                let index = self.parse_expr();
                self.expect(&Token::RBracket, "`]`");
                let span = self.span_from(start);
                Expr::IndexAccess {
                    object: Box::new(left),
                    index: Box::new(index),
                    span,
                }
            }

            // ── Single-param arrow: `x => expr` ────────────────
            Token::FatArrow => {
                let param = self.expr_to_param(&left);
                self.parse_arrow_body(vec![param], start)
            }

            _ => left,
        }
    }

    // ── Helpers ─────────────────────────────────────────────────

    fn make_binary(&mut self, left: Expr, op: BinOp, r_bp: u8, start: Span) -> Expr {
        let right = self.parse_expr_bp(r_bp);
        let span = Self::span_between(start, right.span());
        Expr::BinaryOp {
            left: Box::new(left),
            op,
            right: Box::new(right),
            span,
        }
    }

    /// Parse a template literal after consuming the head segment.
    fn parse_template_literal(&mut self, head: String, start: Span) -> Expr {
        let mut parts = Vec::new();
        if !head.is_empty() {
            parts.push(TemplatePart::Text(head.into()));
        }
        // Parse interpolated expression
        parts.push(TemplatePart::Expr(self.parse_expr()));

        // Continue with middle/tail segments
        loop {
            match self.peek().clone() {
                Token::TemplateMiddle(text) => {
                    self.advance();
                    if !text.is_empty() {
                        parts.push(TemplatePart::Text(text.into()));
                    }
                    parts.push(TemplatePart::Expr(self.parse_expr()));
                }
                Token::TemplateTail(text) => {
                    self.advance();
                    if !text.is_empty() {
                        parts.push(TemplatePart::Text(text.into()));
                    }
                    break;
                }
                _ => {
                    // Error recovery — expected template continuation
                    break;
                }
            }
        }
        let span = self.span_from(start);
        Expr::TemplateLit { parts, span }
    }

    /// Convert an expression to a parameter (for arrow function parsing).
    fn expr_to_param(&mut self, expr: &Expr) -> Param {
        match expr {
            Expr::Ident { name, span } => Param {
                name: name.clone(),
                ty_ann: None,
                default: None,
                span: *span,
            },
            _ => {
                let span = expr.span();
                self.error(ParseError::invalid_expr("expected parameter name", span));
                Param {
                    name: SmolStr::new("_"),
                    ty_ann: None,
                    default: None,
                    span,
                }
            }
        }
    }

    /// Parse the body of an arrow function (expression or block).
    fn parse_arrow_body(&mut self, params: Vec<Param>, start: Span) -> Expr {
        let body = if self.at(&Token::LBrace) {
            ArrowBody::Block(self.parse_block())
        } else {
            ArrowBody::Expr(Box::new(self.parse_expr()))
        };
        let span = self.span_from(start);
        Expr::ArrowFn {
            params,
            ret_ty: None,
            body,
            span,
        }
    }
}

/// Helper: get span from an expression.
impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::IntLit { span, .. }
            | Expr::FloatLit { span, .. }
            | Expr::StringLit { span, .. }
            | Expr::BoolLit { span, .. }
            | Expr::NullLit { span }
            | Expr::RegexLit { span, .. }
            | Expr::TemplateLit { span, .. }
            | Expr::Ident { span, .. }
            | Expr::BinaryOp { span, .. }
            | Expr::UnaryOp { span, .. }
            | Expr::Ternary { span, .. }
            | Expr::NullCoalesce { span, .. }
            | Expr::FnCall { span, .. }
            | Expr::MemberAccess { span, .. }
            | Expr::OptionalChain { span, .. }
            | Expr::IndexAccess { span, .. }
            | Expr::ArrayLit { span, .. }
            | Expr::ObjectLit { span, .. }
            | Expr::ArrowFn { span, .. }
            | Expr::Spread { span, .. }
            | Expr::Range { span, .. }
            | Expr::Pipe { span, .. }
            | Expr::Await { span, .. }
            | Expr::Assign { span, .. }
            | Expr::Assert { span, .. }
            | Expr::EnvAccess { span, .. } => *span,
        }
    }
}
