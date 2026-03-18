//! Statement parser — bindings, control flow, reactive constructs.

use super::Parser;
use crate::ast::*;
use crate::token::Token;
use smol_str::SmolStr;

impl<'src> Parser<'src> {
    /// Parse a single statement.
    pub(crate) fn parse_stmt(&mut self) -> Stmt {
        let start = self.peek_span();
        match self.peek().clone() {
            Token::Let => self.parse_let(),
            Token::Mut => self.parse_mut(),
            Token::Frozen => self.parse_frozen(),
            Token::Signal => self.parse_signal(),
            Token::Derive => self.parse_derive(),
            Token::Ref => self.parse_ref_decl(),
            Token::Effect => self.parse_effect(),
            Token::Watch => self.parse_watch(),
            Token::If => self.parse_if(),
            Token::For => self.parse_for(),
            Token::Return => self.parse_return(),
            Token::Fn => {
                let fn_decl = self.parse_fn_decl_inner(false);
                Stmt::FnDecl(fn_decl)
            }
            Token::LBrace => Stmt::Block(self.parse_block()),
            _ => {
                // Expression statement
                let expr = self.parse_expr();
                let span = self.span_from(start);
                self.skip_terminator();
                Stmt::ExprStmt { expr, span }
            }
        }
    }

    // ── Binding statements ─────────────────────────────────────

    /// `let name: Type = expr`
    fn parse_let(&mut self) -> Stmt {
        let start = self.peek_span();
        self.advance(); // consume `let`
        let name = self.expect_ident("variable name");
        let ty_ann = if self.eat(&Token::Colon).is_some() {
            Some(self.parse_type_annotation())
        } else {
            None
        };
        self.expect(&Token::Eq, "`=`");
        let init = self.parse_expr();
        let span = self.span_from(start);
        self.skip_terminator();
        Stmt::Let {
            name: name.into(),
            ty_ann,
            init,
            span,
        }
    }

    /// `mut name: Type = expr`
    fn parse_mut(&mut self) -> Stmt {
        let start = self.peek_span();
        self.advance(); // consume `mut`
        let name = self.expect_ident("variable name");
        let ty_ann = if self.eat(&Token::Colon).is_some() {
            Some(self.parse_type_annotation())
        } else {
            None
        };
        self.expect(&Token::Eq, "`=`");
        let init = self.parse_expr();
        let span = self.span_from(start);
        self.skip_terminator();
        Stmt::Mut {
            name: name.into(),
            ty_ann,
            init,
            span,
        }
    }

    /// `frozen name = expr`
    fn parse_frozen(&mut self) -> Stmt {
        let start = self.peek_span();
        self.advance(); // consume `frozen`
        let name = self.expect_ident("variable name");
        self.expect(&Token::Eq, "`=`");
        let init = self.parse_expr();
        let span = self.span_from(start);
        self.skip_terminator();
        Stmt::Frozen {
            name: name.into(),
            init,
            span,
        }
    }

    /// `signal name: Type = expr`
    pub(crate) fn parse_signal(&mut self) -> Stmt {
        let start = self.peek_span();
        self.advance(); // consume `signal`
        let name = self.expect_ident("signal name");
        let ty_ann = if self.eat(&Token::Colon).is_some() {
            Some(self.parse_type_annotation())
        } else {
            None
        };
        self.expect(&Token::Eq, "`=`");
        let init = self.parse_expr();
        let span = self.span_from(start);
        self.skip_terminator();
        Stmt::Signal {
            name: name.into(),
            ty_ann,
            init,
            span,
        }
    }

    /// `derive name = expr`
    pub(crate) fn parse_derive(&mut self) -> Stmt {
        let start = self.peek_span();
        self.advance(); // consume `derive`
        let name = self.expect_ident("derive name");
        self.expect(&Token::Eq, "`=`");
        let init = self.parse_expr();
        let span = self.span_from(start);
        self.skip_terminator();
        Stmt::Derive {
            name: name.into(),
            init,
            span,
        }
    }

    /// `ref name: Type`
    fn parse_ref_decl(&mut self) -> Stmt {
        let start = self.peek_span();
        self.advance(); // consume `ref`
        let name = self.expect_ident("ref name");
        self.expect(&Token::Colon, "`:`");
        let ty_ann = self.parse_type_annotation();
        let span = self.span_from(start);
        self.skip_terminator();
        Stmt::RefDecl {
            name: name.into(),
            ty_ann,
            span,
        }
    }

    // ── Reactive statements ────────────────────────────────────

    /// `effect { body }` or `effect { body } cleanup { cleanup_body }`
    fn parse_effect(&mut self) -> Stmt {
        let start = self.peek_span();
        self.advance(); // consume `effect`
        let body = self.parse_block();
        // TODO: cleanup block parsing (not standardized in syntax yet)
        let span = self.span_from(start);
        Stmt::Effect {
            body,
            cleanup: None,
            span,
        }
    }

    /// `watch target as (next, prev) { body }`
    fn parse_watch(&mut self) -> Stmt {
        let start = self.peek_span();
        self.advance(); // consume `watch`
        let target = self.parse_expr();
        // Expect `as`
        let as_ident = self.expect_ident("`as`");
        if as_ident != "as" {
            // Not a hard error — the ident was consumed but it should be "as"
        }
        self.expect(&Token::LParen, "`(`");
        let next_name = self.expect_ident("next value name");
        self.expect(&Token::Comma, "`,`");
        let prev_name = self.expect_ident("previous value name");
        self.expect(&Token::RParen, "`)`");
        let body = self.parse_block();
        let span = self.span_from(start);
        Stmt::Watch {
            target,
            next_name: next_name.into(),
            prev_name: prev_name.into(),
            body,
            span,
        }
    }

    // ── Control flow statements ────────────────────────────────

    /// `if condition { body } else { body }`
    fn parse_if(&mut self) -> Stmt {
        let start = self.peek_span();
        self.advance(); // consume `if`
        let condition = self.parse_expr();
        let then_block = self.parse_block();
        let else_branch = if self.eat(&Token::Else).is_some() {
            if self.at(&Token::If) {
                Some(ElseBranch::ElseIf(Box::new(self.parse_if())))
            } else {
                Some(ElseBranch::Else(self.parse_block()))
            }
        } else {
            None
        };
        let span = self.span_from(start);
        Stmt::If {
            condition,
            then_block,
            else_branch,
            span,
        }
    }

    /// `for binding, index in iterable { body }`
    fn parse_for(&mut self) -> Stmt {
        let start = self.peek_span();
        self.advance(); // consume `for`
        let binding = self.expect_ident("loop variable");
        let index = if self.eat(&Token::Comma).is_some() {
            Some(SmolStr::new(self.expect_ident("index variable")))
        } else {
            None
        };
        // Expect `in`
        let in_ident = self.expect_ident("`in`");
        if in_ident != "in" {
            // Soft error: `in` expected
        }
        let iterable = self.parse_expr();
        let body = self.parse_block();
        let span = self.span_from(start);
        Stmt::For {
            binding: binding.into(),
            index,
            iterable,
            body,
            span,
        }
    }

    /// `return expr` or `return`
    fn parse_return(&mut self) -> Stmt {
        let start = self.peek_span();
        self.advance(); // consume `return`
        let value = if matches!(
            self.peek(),
            Token::Newline | Token::RBrace | Token::Semicolon | Token::EOF
        ) {
            None
        } else {
            Some(self.parse_expr())
        };
        let span = self.span_from(start);
        self.skip_terminator();
        Stmt::Return { value, span }
    }

    // ── Shared: fn declaration (reused by decl.rs) ─────────────

    /// Parse a function declaration body (after optional `async`).
    pub(crate) fn parse_fn_decl_inner(&mut self, is_async: bool) -> FnDecl {
        let start = self.peek_span();
        self.advance(); // consume `fn`
        let name = self.expect_ident("function name");
        self.expect(&Token::LParen, "`(`");
        let params = self.parse_params();
        self.expect(&Token::RParen, "`)`");
        let ret_ty = if self.eat(&Token::Arrow).is_some() {
            Some(self.parse_type_annotation())
        } else {
            None
        };
        let body = self.parse_block();
        let span = self.span_from(start);
        FnDecl {
            name: name.into(),
            params,
            ret_ty,
            body,
            is_async,
            span,
        }
    }
}
