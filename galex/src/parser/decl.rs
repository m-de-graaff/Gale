//! Declaration parser — top-level items.

use super::error::ParseError;
use super::Parser;
use crate::ast::*;
use crate::token::Token;
use smol_str::SmolStr;

impl<'src> Parser<'src> {
    /// Parse a top-level item. Returns `None` on skip/error recovery.
    pub(crate) fn parse_item(&mut self) -> Option<Item> {
        self.skip_newlines();
        if self.eof() {
            return None;
        }

        let result = match self.peek().clone() {
            Token::Use => Some(self.parse_use()),
            Token::Out => Some(self.parse_out()),
            Token::Fn => Some(Item::FnDecl(self.parse_fn_decl_inner(false))),
            Token::Guard => Some(self.parse_guard()),
            Token::Store => Some(self.parse_store()),
            Token::Action => Some(self.parse_action()),
            Token::Query => Some(self.parse_query()),
            Token::Channel => Some(self.parse_channel()),
            Token::Type => Some(self.parse_type_alias()),
            Token::Enum => Some(self.parse_enum()),
            Token::Test => Some(self.parse_test()),
            Token::Middleware => Some(self.parse_middleware()),
            Token::Env => Some(self.parse_env()),
            Token::Server => Some(self.parse_boundary_block(BoundaryKind::Server)),
            Token::Client => Some(self.parse_boundary_block(BoundaryKind::Client)),
            Token::Shared => Some(self.parse_boundary_block(BoundaryKind::Shared)),
            // Statements at top level
            _ => {
                let stmt = self.parse_stmt();
                Some(Item::Stmt(stmt))
            }
        };

        result
    }

    // ── use declaration ────────────────────────────────────────

    /// `use Name from "path"` or `use { A, B } from "path"` or `use * from "path"`
    fn parse_use(&mut self) -> Item {
        let start = self.peek_span();
        self.advance(); // consume `use`

        let imports = match self.peek().clone() {
            Token::Star => {
                self.advance();
                ImportKind::Star
            }
            Token::LBrace => {
                self.advance();
                let mut names = Vec::new();
                self.skip_newlines();
                while !self.at(&Token::RBrace) && !self.eof() {
                    names.push(SmolStr::new(self.expect_ident("import name")));
                    self.skip_newlines();
                    if self.eat(&Token::Comma).is_none() {
                        break;
                    }
                    self.skip_newlines();
                }
                self.expect(&Token::RBrace, "`}`");
                ImportKind::Named(names)
            }
            _ => {
                let name = self.expect_ident("import name");
                ImportKind::Default(name.into())
            }
        };

        // Expect `from`
        let from_ident = self.expect_ident("`from`");
        if from_ident != "from" {
            // Soft error
        }

        let path = match self.peek().clone() {
            Token::StringLit(p) => {
                self.advance();
                SmolStr::new(p)
            }
            _ => {
                self.expect(&Token::StringLit(String::new()), "string path");
                SmolStr::new("")
            }
        };

        let span = self.span_from(start);
        self.skip_terminator();
        Item::Use(UseDecl {
            imports,
            path,
            span,
        })
    }

    // ── out declaration ────────────────────────────────────────

    /// `out ui Name(...) { ... }` or `out api Name { ... }` etc.
    fn parse_out(&mut self) -> Item {
        let start = self.peek_span();
        self.advance(); // consume `out`

        match self.peek().clone() {
            Token::Ui => {
                self.advance();
                let inner = self.parse_component_decl();
                let span = self.span_from(start);
                Item::Out(OutDecl {
                    inner: Box::new(inner),
                    span,
                })
            }
            Token::Layout => {
                self.advance();
                let inner = self.parse_layout_decl();
                let span = self.span_from(start);
                Item::Out(OutDecl {
                    inner: Box::new(inner),
                    span,
                })
            }
            Token::Api => {
                self.advance();
                let inner = self.parse_api_decl();
                let span = self.span_from(start);
                Item::Out(OutDecl {
                    inner: Box::new(inner),
                    span,
                })
            }
            _ => {
                // Generic out: `out fn ...`, `out guard ...`, etc.
                if let Some(inner) = self.parse_item() {
                    let span = self.span_from(start);
                    Item::Out(OutDecl {
                        inner: Box::new(inner),
                        span,
                    })
                } else {
                    let span = self.peek_span();
                    let found = self.peek().clone();
                    self.error(ParseError::unexpected(
                        "declaration after `out`",
                        &found,
                        span,
                    ));
                    self.synchronize();
                    Item::Stmt(Stmt::ExprStmt {
                        expr: Expr::NullLit { span },
                        span,
                    })
                }
            }
        }
    }

    // ── guard declaration ──────────────────────────────────────

    fn parse_guard(&mut self) -> Item {
        let start = self.peek_span();
        self.advance(); // consume `guard`
        let name = self.expect_ident("guard name");
        self.expect(&Token::LBrace, "`{`");
        let mut fields = Vec::new();
        self.skip_newlines();
        while !self.at(&Token::RBrace) && !self.eof() {
            fields.push(self.parse_guard_field());
            self.skip_newlines();
        }
        self.expect(&Token::RBrace, "`}`");
        let span = self.span_from(start);
        Item::GuardDecl(GuardDecl {
            name: name.into(),
            fields,
            span,
        })
    }

    fn parse_guard_field(&mut self) -> GuardFieldDecl {
        let start = self.peek_span();
        let name = self.expect_ident("field name");
        self.expect(&Token::Colon, "`:`");
        let ty = self.parse_type_annotation();
        let validators = self.parse_validator_chain();
        let span = self.span_from(start);
        self.skip_terminator();
        GuardFieldDecl {
            name: name.into(),
            ty,
            validators,
            span,
        }
    }

    fn parse_validator_chain(&mut self) -> Vec<ValidatorCall> {
        let mut validators = Vec::new();
        // Accept both `@name(args)` and `.name(args)` syntax for validators.
        while self.eat(&Token::Dot).is_some() || self.eat(&Token::At).is_some() {
            let start = self.peek_span();
            let name = self.expect_ident("validator name");
            let args = if self.eat(&Token::LParen).is_some() {
                let mut a = Vec::new();
                self.skip_newlines();
                while !self.at(&Token::RParen) && !self.eof() {
                    a.push(self.parse_expr());
                    self.skip_newlines();
                    if self.eat(&Token::Comma).is_none() {
                        break;
                    }
                    self.skip_newlines();
                }
                self.expect(&Token::RParen, "`)`");
                a
            } else {
                vec![]
            };
            let span = self.span_from(start);
            validators.push(ValidatorCall {
                name: name.into(),
                args,
                span,
            });
        }
        validators
    }

    // ── store declaration ──────────────────────────────────────

    fn parse_store(&mut self) -> Item {
        let start = self.peek_span();
        self.advance(); // consume `store`
        let name = self.expect_ident("store name");
        self.expect(&Token::LBrace, "`{`");
        let mut members = Vec::new();
        self.skip_newlines();
        while !self.at(&Token::RBrace) && !self.eof() {
            match self.peek().clone() {
                Token::Signal => members.push(StoreMember::Signal(self.parse_signal())),
                Token::Derive => members.push(StoreMember::Derive(self.parse_derive())),
                Token::Fn => {
                    let fn_decl = self.parse_fn_decl_inner(false);
                    members.push(StoreMember::Method(fn_decl));
                }
                _ => {
                    let span = self.peek_span();
                    let found = self.peek().clone();
                    self.error(ParseError::unexpected(
                        "`signal`, `derive`, or `fn`",
                        &found,
                        span,
                    ));
                    self.synchronize();
                }
            }
            self.skip_newlines();
        }
        self.expect(&Token::RBrace, "`}`");
        let span = self.span_from(start);
        Item::StoreDecl(StoreDecl {
            name: name.into(),
            members,
            span,
        })
    }

    // ── action declaration ─────────────────────────────────────

    fn parse_action(&mut self) -> Item {
        let start = self.peek_span();
        self.advance(); // consume `action`
        let name = self.expect_ident("action name");
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
        Item::ActionDecl(ActionDecl {
            name: name.into(),
            params,
            ret_ty,
            body,
            span,
        })
    }

    // ── query declaration ──────────────────────────────────────

    fn parse_query(&mut self) -> Item {
        let start = self.peek_span();
        self.advance(); // consume `query`
        let name = self.expect_ident("query name");
        self.expect(&Token::Eq, "`=`");
        let url_pattern = self.parse_expr();
        let ret_ty = if self.eat(&Token::Arrow).is_some() {
            Some(self.parse_type_annotation())
        } else {
            None
        };
        let span = self.span_from(start);
        self.skip_terminator();
        Item::QueryDecl(QueryDecl {
            name: name.into(),
            url_pattern,
            ret_ty,
            span,
        })
    }

    // ── channel declaration ────────────────────────────────────

    fn parse_channel(&mut self) -> Item {
        let start = self.peek_span();
        self.advance(); // consume `channel`
        let name = self.expect_ident("channel name");

        // Optional params
        let params = if self.eat(&Token::LParen).is_some() {
            let p = self.parse_params();
            self.expect(&Token::RParen, "`)`");
            p
        } else {
            vec![]
        };

        // Direction: `->`, `<-`, or `<->`
        let direction = match self.peek().clone() {
            Token::Arrow => {
                self.advance();
                ChannelDirection::ServerToClient
            }
            Token::BiArrow => {
                self.advance();
                ChannelDirection::Bidirectional
            }
            Token::Less => {
                self.advance();
                self.expect(&Token::Minus, "`-`");
                ChannelDirection::ClientToServer
            }
            _ => {
                let span = self.peek_span();
                let found = self.peek().clone();
                self.error(ParseError::unexpected("`->`, `<-`, or `<->`", &found, span));
                ChannelDirection::Bidirectional
            }
        };

        let msg_ty = self.parse_type_annotation();

        // Optional handler block
        let handlers = if self.at(&Token::LBrace) {
            self.advance();
            let mut h = Vec::new();
            self.skip_newlines();
            while !self.at(&Token::RBrace) && !self.eof() {
                h.push(self.parse_channel_handler());
                self.skip_newlines();
            }
            self.expect(&Token::RBrace, "`}`");
            h
        } else {
            vec![]
        };

        let span = self.span_from(start);
        Item::ChannelDecl(ChannelDecl {
            name: name.into(),
            params,
            direction,
            msg_ty,
            handlers,
            span,
        })
    }

    fn parse_channel_handler(&mut self) -> ChannelHandler {
        let start = self.peek_span();
        self.expect(&Token::On, "`on`");
        let event = self.expect_ident("event name");
        let params = if self.eat(&Token::LParen).is_some() {
            let p = self.parse_params();
            self.expect(&Token::RParen, "`)`");
            p
        } else {
            vec![]
        };
        let body = self.parse_block();
        let span = self.span_from(start);
        ChannelHandler {
            event: event.into(),
            params,
            body,
            span,
        }
    }

    // ── type alias ─────────────────────────────────────────────

    fn parse_type_alias(&mut self) -> Item {
        let start = self.peek_span();
        self.advance(); // consume `type`
        let name = self.expect_ident("type name");
        self.expect(&Token::Eq, "`=`");
        let ty = self.parse_type_annotation();
        let span = self.span_from(start);
        self.skip_terminator();
        Item::TypeAlias(TypeAliasDecl {
            name: name.into(),
            ty,
            span,
        })
    }

    // ── enum declaration ───────────────────────────────────────

    fn parse_enum(&mut self) -> Item {
        let start = self.peek_span();
        self.advance(); // consume `enum`
        let name = self.expect_ident("enum name");
        self.expect(&Token::LBrace, "`{`");
        let mut variants = Vec::new();
        self.skip_newlines();
        while !self.at(&Token::RBrace) && !self.eof() {
            variants.push(SmolStr::new(self.expect_ident("variant name")));
            self.skip_newlines();
            if self.eat(&Token::Comma).is_none() {
                break;
            }
            self.skip_newlines();
        }
        self.expect(&Token::RBrace, "`}`");
        let span = self.span_from(start);
        Item::EnumDecl(EnumDecl {
            name: name.into(),
            variants,
            span,
        })
    }

    // ── test declaration ───────────────────────────────────────

    fn parse_test(&mut self) -> Item {
        let start = self.peek_span();
        self.advance(); // consume `test`
        let name = match self.peek().clone() {
            Token::StringLit(s) => {
                self.advance();
                SmolStr::new(s)
            }
            _ => SmolStr::new(self.expect_ident("test name")),
        };
        let body = self.parse_block();
        let span = self.span_from(start);
        Item::TestDecl(TestDecl { name, body, span })
    }

    // ── middleware declaration ──────────────────────────────────

    fn parse_middleware(&mut self) -> Item {
        let start = self.peek_span();
        self.advance(); // consume `middleware`

        // Parse target: none (global), `for "path"`, `for Name`
        let target = if self.peek().clone() == Token::For {
            self.advance();
            match self.peek().clone() {
                Token::StringLit(path) => {
                    self.advance();
                    MiddlewareTarget::PathPrefix(path.into())
                }
                _ => {
                    let name = self.expect_ident("resource name");
                    MiddlewareTarget::Resource(name.into())
                }
            }
        } else {
            MiddlewareTarget::Global
        };

        let name = self.expect_ident("middleware name");
        self.expect(&Token::LParen, "`(`");
        let params = self.parse_params();
        self.expect(&Token::RParen, "`)`");
        let body = self.parse_block();
        let span = self.span_from(start);
        Item::MiddlewareDecl(MiddlewareDecl {
            name: name.into(),
            target,
            params,
            body,
            span,
        })
    }

    // ── env declaration ────────────────────────────────────────

    fn parse_env(&mut self) -> Item {
        let start = self.peek_span();
        self.advance(); // consume `env`
        self.expect(&Token::LBrace, "`{`");
        let mut vars = Vec::new();
        self.skip_newlines();
        while !self.at(&Token::RBrace) && !self.eof() {
            let v_start = self.peek_span();
            let key = self.expect_ident("env variable name");
            self.expect(&Token::Colon, "`:`");
            let ty = self.parse_type_annotation();
            let validators = self.parse_validator_chain();
            let default = if self.eat(&Token::Eq).is_some() {
                Some(self.parse_expr())
            } else {
                None
            };
            let v_span = self.span_from(v_start);
            vars.push(EnvVarDef {
                key: key.into(),
                ty,
                validators,
                default,
                span: v_span,
            });
            self.skip_newlines();
            if self.eat(&Token::Comma).is_none() {
                // Allow newline-separated entries too
            }
            self.skip_newlines();
        }
        self.expect(&Token::RBrace, "`}`");
        let span = self.span_from(start);
        Item::EnvDecl(EnvDecl { vars, span })
    }

    // ── boundary blocks ────────────────────────────────────────

    fn parse_boundary_block(&mut self, kind: BoundaryKind) -> Item {
        let start = self.peek_span();
        self.advance(); // consume `server`/`client`/`shared`
        self.expect(&Token::LBrace, "`{`");
        let mut items = Vec::new();
        self.skip_newlines();
        while !self.at(&Token::RBrace) && !self.eof() {
            if let Some(item) = self.parse_item() {
                items.push(item);
            }
            self.skip_newlines();
        }
        self.expect(&Token::RBrace, "`}`");
        let span = self.span_from(start);
        let block = BoundaryBlock { items, span };
        match kind {
            BoundaryKind::Server => Item::ServerBlock(block),
            BoundaryKind::Client => Item::ClientBlock(block),
            BoundaryKind::Shared => Item::SharedBlock(block),
        }
    }

    // ── component declaration ──────────────────────────────────

    /// Parse a component body (used by `out ui` and standalone).
    pub(crate) fn parse_component_decl(&mut self) -> Item {
        let start = self.peek_span();
        let name = self.expect_ident("component name");

        // Optional props
        let props = if self.eat(&Token::LParen).is_some() {
            let p = self.parse_params();
            self.expect(&Token::RParen, "`)`");
            p
        } else {
            vec![]
        };

        let body = self.parse_component_body();
        let span = self.span_from(start);
        Item::ComponentDecl(ComponentDecl {
            name: name.into(),
            props,
            body,
            span,
        })
    }

    /// Parse a layout body (used by `out layout`).
    pub(crate) fn parse_layout_decl(&mut self) -> Item {
        let start = self.peek_span();
        let name = self.expect_ident("layout name");

        let props = if self.eat(&Token::LParen).is_some() {
            let p = self.parse_params();
            self.expect(&Token::RParen, "`)`");
            p
        } else {
            vec![]
        };

        let body = self.parse_component_body();
        let span = self.span_from(start);
        Item::LayoutDecl(LayoutDecl {
            name: name.into(),
            props,
            body,
            span,
        })
    }

    /// Parse the body of a component or layout:
    ///
    /// ```text
    /// { head { ... } stmts... <template>...</template> }
    /// { stmts... <template>...</template> head { ... } }
    /// ```
    ///
    /// The `head` block may appear before or after the template.
    fn parse_component_body(&mut self) -> ComponentBody {
        let start = self.peek_span();
        self.expect(&Token::LBrace, "`{`");
        self.skip_newlines();

        // Head block can appear before the template (common convention)
        let mut head = None;
        if self.at(&Token::Head) {
            head = Some(self.parse_head_block());
            self.skip_newlines();
        }

        // Parse code statements (before template begins).
        // Stops when we see `}` (end of body), a template indicator (`<`, `when`,
        // `each`, `slot`, etc.), or another `head` block.
        let mut stmts = Vec::new();
        while !self.at(&Token::RBrace) && !self.eof() && !self.at_template_start() {
            stmts.push(self.parse_stmt());
            self.skip_newlines();
        }

        // Switch lexer to template mode and parse template nodes.
        // `enter_template_mode()` clears the peek buffer and rewinds the lexer
        // so that `<` (lexed as `Less` in Code mode) gets re-lexed as `HtmlOpen`.
        self.enter_template_mode();
        let template = self.parse_template_nodes();
        self.exit_template_mode();

        // Head block can also appear after the template
        self.skip_newlines();
        if head.is_none() && self.at(&Token::Head) {
            head = Some(self.parse_head_block());
        }

        self.skip_newlines();
        self.expect(&Token::RBrace, "`}`");
        let span = self.span_from(start);
        ComponentBody {
            stmts,
            template,
            head,
            span,
        }
    }

    /// Check if the current token starts a template (HTML tag or template keyword).
    ///
    /// In Code mode, `<` is tokenized as `Token::Less` rather than `HtmlOpen`.
    /// We include `Less` here because at a statement boundary inside a component
    /// body, `<` always indicates an HTML element start, not a comparison.
    fn at_template_start(&mut self) -> bool {
        matches!(
            self.peek(),
            Token::Less // `<` in Code mode = HTML tag start in component body
                | Token::HtmlOpen(_)
                | Token::HtmlText(_)
                | Token::ExprOpen
                | Token::When
                | Token::Each
                | Token::Suspend
                | Token::Slot
                | Token::Head
        )
    }

    /// Parse a head block: `head { key: value, ... }`
    fn parse_head_block(&mut self) -> HeadBlock {
        let start = self.peek_span();
        self.advance(); // consume `head`
        self.expect(&Token::LBrace, "`{`");
        let mut fields = Vec::new();
        self.skip_newlines();
        while !self.at(&Token::RBrace) && !self.eof() {
            let f_start = self.peek_span();
            let key = self.expect_ident("head field name");
            self.expect(&Token::Colon, "`:`");
            let value = self.parse_expr();
            let f_span = self.span_from(f_start);
            fields.push(HeadField {
                key: key.into(),
                value,
                span: f_span,
            });
            self.skip_newlines();
            if self.eat(&Token::Comma).is_none() {
                // Allow newline-separated too
            }
            self.skip_newlines();
        }
        self.expect(&Token::RBrace, "`}`");
        let span = self.span_from(start);
        HeadBlock { fields, span }
    }

    // ── API declaration ────────────────────────────────────────

    fn parse_api_decl(&mut self) -> Item {
        let start = self.peek_span();
        let name = self.expect_ident("API resource name");
        self.expect(&Token::LBrace, "`{`");
        let mut handlers = Vec::new();
        self.skip_newlines();
        while !self.at(&Token::RBrace) && !self.eof() {
            handlers.push(self.parse_api_handler());
            self.skip_newlines();
        }
        self.expect(&Token::RBrace, "`}`");
        let span = self.span_from(start);
        Item::ApiDecl(ApiDecl {
            name: name.into(),
            handlers,
            span,
        })
    }

    fn parse_api_handler(&mut self) -> ApiHandler {
        let start = self.peek_span();
        let method_str = self.expect_ident("HTTP method");
        let method = match method_str.as_str() {
            "get" => HttpMethod::Get,
            "post" => HttpMethod::Post,
            "put" => HttpMethod::Put,
            "patch" => HttpMethod::Patch,
            "delete" => HttpMethod::Delete,
            _ => {
                self.error(ParseError::unexpected(
                    "HTTP method (get, post, put, patch, delete)",
                    &Token::Ident(method_str.clone()),
                    start,
                ));
                HttpMethod::Get
            }
        };

        // Optional path params: `get[id]` or `get[year][month]`
        let mut path_params = Vec::new();
        while self.eat(&Token::LBracket).is_some() {
            let param = self.expect_ident("path parameter");
            path_params.push(SmolStr::new(param));
            self.expect(&Token::RBracket, "`]`");
        }

        // Optional typed params
        let params = if self.eat(&Token::LParen).is_some() {
            let p = self.parse_params();
            self.expect(&Token::RParen, "`)`");
            p
        } else {
            vec![]
        };

        let ret_ty = if self.eat(&Token::Arrow).is_some() {
            Some(self.parse_type_annotation())
        } else {
            None
        };

        let body = self.parse_block();
        let span = self.span_from(start);
        ApiHandler {
            method,
            path_params,
            params,
            ret_ty,
            body,
            span,
        }
    }
}

/// Internal enum for boundary block kind dispatch.
enum BoundaryKind {
    Server,
    Client,
    Shared,
}
