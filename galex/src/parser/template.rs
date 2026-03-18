//! Template parser — HTML elements, directives, control flow.
//!
//! Parses the template portion of component/layout bodies. The lexer
//! is expected to be in Template mode, producing `HtmlOpen`, `HtmlClose`,
//! `HtmlSelfClose`, `HtmlText`, `ExprOpen`, etc.

use super::error::ParseError;
use super::Parser;
use crate::ast::*;
use crate::token::Token;
use smol_str::SmolStr;

impl<'src> Parser<'src> {
    /// Parse template nodes until a closing `}` or EOF.
    pub(crate) fn parse_template_nodes(&mut self) -> Vec<TemplateNode> {
        let mut nodes = Vec::new();
        loop {
            self.skip_newlines();
            match self.peek().clone() {
                Token::RBrace | Token::EOF => break,
                Token::HtmlClose(_) => break, // Parent will consume
                _ => {
                    if let Some(node) = self.parse_template_node() {
                        nodes.push(node);
                    } else {
                        break;
                    }
                }
            }
        }
        nodes
    }

    /// Parse a single template node.
    fn parse_template_node(&mut self) -> Option<TemplateNode> {
        match self.peek().clone() {
            Token::HtmlOpen(tag) => Some(self.parse_element(tag)),
            Token::HtmlText(text) => {
                let span = self.peek_span();
                self.advance();
                Some(TemplateNode::Text {
                    value: text.into(),
                    span,
                })
            }
            Token::ExprOpen => {
                let start = self.peek_span();
                self.advance(); // consume `{`
                let expr = self.parse_expr();
                self.expect(&Token::ExprClose, "`}`");
                let span = self.span_from(start);
                Some(TemplateNode::ExprInterp { expr, span })
            }
            Token::When => Some(self.parse_when()),
            Token::Each => Some(self.parse_each()),
            Token::Suspend => Some(self.parse_suspend()),
            Token::Slot => Some(self.parse_slot_node()),
            _ => None,
        }
    }

    // ── HTML elements ──────────────────────────────────────────

    /// Parse an element: `<tag attrs directives>children</tag>` or `<tag ... />`
    fn parse_element(&mut self, tag: String) -> TemplateNode {
        let start = self.peek_span();
        self.advance(); // consume HtmlOpen(tag)

        // Parse attributes and directives (lexer is now in HtmlTag mode)
        let mut attributes = Vec::new();
        let mut directives = Vec::new();
        self.parse_attrs_and_directives(&mut attributes, &mut directives);

        // Check for self-closing
        if self.eat(&Token::HtmlSelfClose).is_some() {
            let span = self.span_from(start);
            return TemplateNode::SelfClosing {
                tag: tag.into(),
                attributes,
                directives,
                span,
            };
        }

        // Expect `>` — the HtmlTag-mode lexer produces RAngle for `>`
        if self.eat(&Token::RAngle).is_none() {
            self.eat(&Token::Greater); // fallback for Code-mode `>`
        }

        // Parse children (lexer should be back in Template mode)
        let children = self.parse_template_nodes();

        // Expect closing tag
        match self.peek().clone() {
            Token::HtmlClose(close_tag) if close_tag == tag => {
                self.advance();
            }
            Token::HtmlClose(_) => {
                let span = self.peek_span();
                let found = self.peek().clone();
                self.error(ParseError::unexpected(&format!("`</{tag}>`"), &found, span));
                self.advance(); // consume mismatched close tag
            }
            _ => {
                // Missing close tag — error recovery
            }
        }

        let span = self.span_from(start);
        TemplateNode::Element {
            tag: tag.into(),
            attributes,
            directives,
            children,
            span,
        }
    }

    /// Parse attributes and directives inside an open tag.
    fn parse_attrs_and_directives(
        &mut self,
        attributes: &mut Vec<Attribute>,
        directives: &mut Vec<Directive>,
    ) {
        loop {
            match self.peek().clone() {
                Token::Greater | Token::RAngle | Token::HtmlSelfClose | Token::EOF => break,
                Token::Newline => {
                    self.advance();
                    continue;
                }
                // ── Directives (compound tokens from lexer) ────
                Token::BindDir(field) => {
                    let span = self.peek_span();
                    self.advance();
                    directives.push(Directive::Bind {
                        field: field.into(),
                        span,
                    });
                }
                Token::OnDir { event, modifiers } => {
                    let span = self.peek_span();
                    self.advance();
                    // Value: `={handler}`
                    self.eat(&Token::Eq);
                    let handler = if self.eat(&Token::ExprOpen).is_some() {
                        let e = self.parse_expr();
                        self.expect(&Token::ExprClose, "`}`");
                        e
                    } else {
                        Expr::NullLit { span }
                    };
                    directives.push(Directive::On {
                        event: event.into(),
                        modifiers: modifiers.into_iter().map(SmolStr::new).collect(),
                        handler,
                        span,
                    });
                }
                Token::ClassDir(name) => {
                    let span = self.peek_span();
                    self.advance();
                    self.eat(&Token::Eq);
                    let condition = if self.eat(&Token::ExprOpen).is_some() {
                        let e = self.parse_expr();
                        self.expect(&Token::ExprClose, "`}`");
                        e
                    } else {
                        Expr::BoolLit { value: true, span }
                    };
                    directives.push(Directive::Class {
                        name: name.into(),
                        condition,
                        span,
                    });
                }
                Token::RefDir(name) => {
                    let span = self.peek_span();
                    self.advance();
                    directives.push(Directive::Ref {
                        name: name.into(),
                        span,
                    });
                }
                Token::TransDir(kind) => {
                    let span = self.peek_span();
                    self.advance();
                    let config = if self.eat(&Token::Eq).is_some() {
                        if self.eat(&Token::ExprOpen).is_some() {
                            let e = self.parse_expr();
                            self.expect(&Token::ExprClose, "`}`");
                            Some(e)
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    directives.push(Directive::Transition {
                        kind: kind.into(),
                        config,
                        span,
                    });
                }
                Token::KeyDir => {
                    let span = self.peek_span();
                    self.advance();
                    self.eat(&Token::Eq);
                    let expr = if self.eat(&Token::ExprOpen).is_some() {
                        let e = self.parse_expr();
                        self.expect(&Token::ExprClose, "`}`");
                        e
                    } else {
                        Expr::NullLit { span }
                    };
                    directives.push(Directive::Key { expr, span });
                }
                Token::IntoDir(slot) => {
                    let span = self.peek_span();
                    self.advance();
                    directives.push(Directive::Into {
                        slot: slot.into(),
                        span,
                    });
                }
                Token::FormAction => {
                    let span = self.peek_span();
                    self.advance();
                    self.eat(&Token::Eq);
                    let action = if self.eat(&Token::ExprOpen).is_some() {
                        let e = self.parse_expr();
                        self.expect(&Token::ExprClose, "`}`");
                        e
                    } else {
                        Expr::NullLit { span }
                    };
                    directives.push(Directive::FormAction { action, span });
                }
                Token::FormGuard => {
                    let span = self.peek_span();
                    self.advance();
                    self.eat(&Token::Eq);
                    let guard = if self.eat(&Token::ExprOpen).is_some() {
                        let e = self.parse_expr();
                        self.expect(&Token::ExprClose, "`}`");
                        e
                    } else {
                        Expr::NullLit { span }
                    };
                    directives.push(Directive::FormGuard { guard, span });
                }
                Token::FormError => {
                    let span = self.peek_span();
                    self.advance();
                    // Parse field="name"
                    let field = if let Some(_) = self.eat(&Token::Eq) {
                        match self.peek().clone() {
                            Token::StringLit(s) => {
                                self.advance();
                                SmolStr::new(s)
                            }
                            _ => SmolStr::new(self.expect_ident("field name")),
                        }
                    } else {
                        SmolStr::new("")
                    };
                    directives.push(Directive::FormError { field, span });
                }
                Token::Prefetch => {
                    let span = self.peek_span();
                    self.advance();
                    let mode = if self.eat(&Token::Eq).is_some() {
                        match self.peek().clone() {
                            Token::StringLit(s) => {
                                self.advance();
                                SmolStr::new(s)
                            }
                            _ => SmolStr::new("hover"),
                        }
                    } else {
                        SmolStr::new("hover")
                    };
                    directives.push(Directive::Prefetch { mode, span });
                }

                // ── Regular attributes ─────────────────────────
                Token::Ident(name) => {
                    let span = self.peek_span();
                    self.advance();
                    let value = if self.eat(&Token::Eq).is_some() {
                        match self.peek().clone() {
                            Token::StringLit(s) => {
                                self.advance();
                                AttrValue::String(s.into())
                            }
                            Token::ExprOpen => {
                                self.advance();
                                let e = self.parse_expr();
                                self.expect(&Token::ExprClose, "`}`");
                                AttrValue::Expr(e)
                            }
                            _ => AttrValue::Bool,
                        }
                    } else {
                        AttrValue::Bool
                    };
                    attributes.push(Attribute {
                        name: name.into(),
                        value,
                        span,
                    });
                }
                _ => {
                    // Skip unexpected token in attribute position
                    self.advance();
                }
            }
        }
    }

    // ── Template control flow ──────────────────────────────────

    /// `when condition { body } else { body }`
    fn parse_when(&mut self) -> TemplateNode {
        let start = self.peek_span();
        self.advance(); // consume `when`
        let condition = self.parse_expr();
        self.expect(&Token::LBrace, "`{`");
        let body = self.parse_template_nodes();
        self.expect(&Token::RBrace, "`}`");
        let else_branch = if self.eat(&Token::Else).is_some() {
            if self.at(&Token::When) {
                Some(WhenElse::ElseWhen(Box::new(self.parse_when())))
            } else {
                self.expect(&Token::LBrace, "`{`");
                let nodes = self.parse_template_nodes();
                self.expect(&Token::RBrace, "`}`");
                Some(WhenElse::Else(nodes))
            }
        } else {
            None
        };
        let span = self.span_from(start);
        TemplateNode::When {
            condition,
            body,
            else_branch,
            span,
        }
    }

    /// `each item, index in list { body } empty { fallback }`
    fn parse_each(&mut self) -> TemplateNode {
        let start = self.peek_span();
        self.advance(); // consume `each`
        let binding = self.expect_ident("binding name");
        let index = if self.eat(&Token::Comma).is_some() {
            Some(SmolStr::new(self.expect_ident("index variable")))
        } else {
            None
        };
        // Expect `in`
        let in_ident = self.expect_ident("`in`");
        if in_ident != "in" {
            // Soft error
        }
        let iterable = self.parse_expr();
        self.expect(&Token::LBrace, "`{`");
        let body = self.parse_template_nodes();
        self.expect(&Token::RBrace, "`}`");
        let empty = if self.eat(&Token::Empty).is_some() {
            self.expect(&Token::LBrace, "`{`");
            let nodes = self.parse_template_nodes();
            self.expect(&Token::RBrace, "`}`");
            Some(nodes)
        } else {
            None
        };
        let span = self.span_from(start);
        TemplateNode::Each {
            binding: binding.into(),
            index,
            iterable,
            body,
            empty,
            span,
        }
    }

    /// `suspend { body } fallback { node }`
    fn parse_suspend(&mut self) -> TemplateNode {
        let start = self.peek_span();
        self.advance(); // consume `suspend`
        self.expect(&Token::LBrace, "`{`");
        let body = self.parse_template_nodes();
        self.expect(&Token::RBrace, "`}`");
        // TODO: parse fallback
        let span = self.span_from(start);
        TemplateNode::Suspend {
            fallback: None,
            body,
            span,
        }
    }

    /// `<slot/>` or `<slot name="x">default</slot>`
    fn parse_slot_node(&mut self) -> TemplateNode {
        let start = self.peek_span();
        self.advance(); // consume `slot` keyword
                        // In template mode, slot might appear as `<slot/>` (HtmlOpen)
                        // or as the `slot` keyword. Handle both.
        let span = self.span_from(start);
        TemplateNode::Slot {
            name: None,
            default: None,
            span,
        }
    }
}
