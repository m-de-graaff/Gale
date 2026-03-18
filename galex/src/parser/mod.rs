//! GaleX parser — transforms a token stream into an AST.
//!
//! The parser owns a [`Lexer`] and pulls tokens on demand, switching
//! lexer modes when entering template contexts (component/layout bodies).
//! This streaming approach is necessary because the lexer produces
//! different tokens in Code vs Template mode.
//!
//! # Architecture
//!
//! - **Pratt parser** for expressions (operator precedence via binding power)
//! - **Recursive descent** for statements, declarations, and templates
//! - **Error recovery** via `synchronize()` — skips to next statement boundary
//!
//! # Usage
//!
//! ```ignore
//! let result = galex::parser::parse(source, file_id);
//! if !result.errors().is_empty() {
//!     // report errors
//! }
//! let program = result.program;
//! ```

mod decl;
pub mod error;
mod expr;
mod stmt;
mod template;

use crate::ast::*;
use crate::error::LexError;
use crate::lexer::{LexMode, Lexer};
use crate::span::Span;
use crate::token::Token;
use error::ParseError;

// ── Public API ─────────────────────────────────────────────────────────

/// Result of parsing a GaleX source file.
pub struct ParseResult {
    /// The parsed AST.
    pub program: Program,
    /// Lexer errors encountered during tokenization.
    pub lex_errors: Vec<LexError>,
    /// Parser errors encountered during parsing.
    pub parse_errors: Vec<ParseError>,
}

impl ParseResult {
    /// All errors (lex + parse) combined.
    pub fn errors(&self) -> Vec<String> {
        let mut errs: Vec<String> = self.lex_errors.iter().map(|e| e.to_string()).collect();
        errs.extend(self.parse_errors.iter().map(|e| e.to_string()));
        errs
    }

    /// Whether parsing succeeded without errors.
    pub fn is_ok(&self) -> bool {
        self.lex_errors.is_empty() && self.parse_errors.is_empty()
    }
}

/// Parse a GaleX source file into an AST Program.
///
/// This is the main entry point for the parser. It creates a lexer,
/// parses the full token stream, and returns the AST along with any errors.
pub fn parse(source: &str, file_id: u32) -> ParseResult {
    let mut parser = Parser::new(source, file_id);
    let program = parser.parse_program();
    ParseResult {
        program,
        lex_errors: parser.lex_errors,
        parse_errors: parser.errors,
    }
}

// ── Parser struct ──────────────────────────────────────────────────────

/// The GaleX recursive descent parser.
///
/// Owns a [`Lexer`] and drives it forward, requesting tokens on demand.
/// A small lookahead buffer (`peeked`) avoids re-lexing.
pub struct Parser<'src> {
    lexer: Lexer<'src>,
    /// Lookahead buffer (at most 2 tokens).
    peeked: Vec<(Token, Span)>,
    /// Accumulated parse errors.
    errors: Vec<ParseError>,
    /// Accumulated lex errors (from the lexer).
    lex_errors: Vec<LexError>,
    /// File ID for span construction.
    file_id: u32,
}

impl<'src> Parser<'src> {
    /// Create a new parser for the given source text.
    pub fn new(source: &'src str, file_id: u32) -> Self {
        Self {
            lexer: Lexer::new(source, file_id),
            peeked: Vec::with_capacity(2),
            errors: Vec::new(),
            lex_errors: Vec::new(),
            file_id,
        }
    }

    // ── Token navigation ───────────────────────────────────────

    /// Fill the lookahead buffer to at least `n` tokens.
    fn fill_peek(&mut self, n: usize) {
        while self.peeked.len() < n {
            let tok = self.lexer.next_token();
            // Collect lex errors as they surface
            for err in std::mem::take(&mut self.lexer.errors) {
                self.lex_errors.push(err);
            }
            self.peeked.push(tok);
        }
    }

    /// Peek at the current token without consuming it.
    fn peek(&mut self) -> &Token {
        self.fill_peek(1);
        &self.peeked[0].0
    }

    /// Peek at the token `n` positions ahead (0-indexed).
    fn peek_nth(&mut self, n: usize) -> &Token {
        self.fill_peek(n + 1);
        &self.peeked[n].0
    }

    /// Get the span of the current token without consuming it.
    fn peek_span(&mut self) -> Span {
        self.fill_peek(1);
        self.peeked[0].1
    }

    /// Consume and return the current token.
    fn advance(&mut self) -> (Token, Span) {
        self.fill_peek(1);
        self.peeked.remove(0)
    }

    /// Check if the current token matches `expected`.
    fn at(&mut self, expected: &Token) -> bool {
        std::mem::discriminant(self.peek()) == std::mem::discriminant(expected)
    }

    /// Check if the current token is any of the given tokens.
    fn at_any(&mut self, tokens: &[Token]) -> bool {
        let current = std::mem::discriminant(self.peek());
        tokens.iter().any(|t| std::mem::discriminant(t) == current)
    }

    /// Check if we've reached EOF.
    fn eof(&mut self) -> bool {
        matches!(self.peek(), Token::EOF)
    }

    /// Consume the current token if it matches `expected`, returning its span.
    fn eat(&mut self, expected: &Token) -> Option<Span> {
        if self.at(expected) {
            Some(self.advance().1)
        } else {
            None
        }
    }

    /// Consume the current token if it matches `expected`, or report an error.
    fn expect(&mut self, expected: &Token, msg: &str) -> Span {
        if self.at(expected) {
            self.advance().1
        } else {
            let span = self.peek_span();
            let found = self.peek().clone();
            self.error(ParseError::unexpected(msg, &found, span));
            span
        }
    }

    /// Skip newlines (they are significant for statement termination but
    /// often need to be skipped between declarations).
    fn skip_newlines(&mut self) {
        while matches!(
            self.peek(),
            Token::Newline | Token::Comment(_) | Token::BlockComment(_)
        ) {
            self.advance();
        }
    }

    /// Skip an optional trailing newline or semicolon.
    fn skip_terminator(&mut self) {
        if matches!(self.peek(), Token::Newline | Token::Semicolon) {
            self.advance();
        }
    }

    // ── Span helpers ───────────────────────────────────────────

    /// Create a span from `start` to the current position.
    fn span_from(&self, start: Span) -> Span {
        if let Some(last) = self.peeked.first() {
            Span {
                file_id: start.file_id,
                start: start.start,
                end: last.1.start, // up to but not including the next token
                line: start.line,
                col: start.col,
            }
        } else {
            start
        }
    }

    /// Create a span covering from `start` to `end`.
    fn span_between(start: Span, end: Span) -> Span {
        Span {
            file_id: start.file_id,
            start: start.start,
            end: end.end,
            line: start.line,
            col: start.col,
        }
    }

    // ── Error handling ─────────────────────────────────────────

    /// Record a parse error.
    fn error(&mut self, err: ParseError) {
        self.errors.push(err);
    }

    /// Synchronize after an error — skip tokens until a recovery point.
    ///
    /// Recovery points: `}`, newline followed by a declaration keyword,
    /// or end of file.
    fn synchronize(&mut self) {
        loop {
            match self.peek() {
                Token::EOF => break,
                Token::RBrace => {
                    self.advance();
                    break;
                }
                Token::Newline => {
                    self.advance();
                    // Check if the next token starts a new declaration
                    if self.at_declaration_start() {
                        break;
                    }
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    /// Check if the current token can start a top-level declaration.
    fn at_declaration_start(&mut self) -> bool {
        matches!(
            self.peek(),
            Token::Let
                | Token::Mut
                | Token::Signal
                | Token::Derive
                | Token::Frozen
                | Token::Ref
                | Token::Fn
                | Token::Guard
                | Token::Action
                | Token::Query
                | Token::Store
                | Token::Channel
                | Token::Type
                | Token::Enum
                | Token::Test
                | Token::Effect
                | Token::Watch
                | Token::Use
                | Token::Out
                | Token::Server
                | Token::Client
                | Token::Shared
                | Token::Middleware
                | Token::Env
                | Token::If
                | Token::For
                | Token::Return
        )
    }

    // ── Lexer mode control ─────────────────────────────────────

    /// Push the lexer into template mode (for component/layout bodies).
    ///
    /// If there are peeked tokens (lexed in Code mode), rewinds the lexer
    /// to re-lex them in Template mode so that `<` becomes `HtmlOpen`
    /// instead of `Less`.
    fn enter_template_mode(&mut self) {
        if !self.peeked.is_empty() {
            let rewind_pos = self.peeked[0].1.start as usize;
            self.peeked.clear();
            self.lexer.rewind_to(rewind_pos);
        }
        self.lexer.push_mode(LexMode::Template);
    }

    /// Pop the lexer back to the previous mode.
    fn exit_template_mode(&mut self) {
        self.lexer.pop_mode();
    }

    // ── Program parsing ────────────────────────────────────────

    /// Parse an entire program (a complete .gx source file).
    pub fn parse_program(&mut self) -> Program {
        let start = self.peek_span();
        let mut items = Vec::new();

        self.skip_newlines();
        while !self.eof() {
            if let Some(item) = self.parse_item() {
                items.push(item);
            }
            self.skip_newlines();
        }

        let span = self.span_from(start);
        Program { items, span }
    }

    // ── Shared helpers ─────────────────────────────────────────

    /// Parse a comma-separated list of parameters: `name: Type = default`
    pub(crate) fn parse_params(&mut self) -> Vec<Param> {
        let mut params = Vec::new();
        self.skip_newlines();
        while !self.at(&Token::RParen) && !self.eof() {
            let start = self.peek_span();
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
            let span = self.span_from(start);
            params.push(Param {
                name: name.into(),
                ty_ann,
                default,
                span,
            });
            self.skip_newlines();
            if self.eat(&Token::Comma).is_none() {
                break;
            }
            self.skip_newlines();
        }
        params
    }

    /// Parse a type annotation.
    pub(crate) fn parse_type_annotation(&mut self) -> TypeAnnotation {
        let start = self.peek_span();
        let mut ty = self.parse_primary_type();

        // Handle `T[]` (array) and `T?` (optional)
        loop {
            if self.eat(&Token::LBracket).is_some() {
                self.expect(&Token::RBracket, "`]`");
                let span = self.span_from(start);
                ty = TypeAnnotation::Array {
                    element: Box::new(ty),
                    span,
                };
            } else if self.eat(&Token::Question).is_some() {
                let span = self.span_from(start);
                ty = TypeAnnotation::Optional {
                    inner: Box::new(ty),
                    span,
                };
            } else if self.at(&Token::Bar) {
                // Union type: `T | U | V`
                let mut types = vec![ty];
                while self.eat(&Token::Bar).is_some() {
                    types.push(self.parse_primary_type());
                }
                let span = self.span_from(start);
                ty = TypeAnnotation::Union { types, span };
                break; // Union is the outermost type form
            } else {
                break;
            }
        }
        ty
    }

    /// Parse a primary (non-compound) type.
    fn parse_primary_type(&mut self) -> TypeAnnotation {
        let start = self.peek_span();
        match self.peek().clone() {
            Token::Ident(name) => {
                self.advance();
                TypeAnnotation::Named {
                    name: name.into(),
                    span: self.span_from(start),
                }
            }
            Token::StringLit(value) => {
                self.advance();
                TypeAnnotation::StringLiteral {
                    value: value.into(),
                    span: self.span_from(start),
                }
            }
            Token::LParen => {
                // Tuple type: `(T, U, V)` or function type: `fn(T) -> U`
                self.advance();
                let mut elements = Vec::new();
                self.skip_newlines();
                while !self.at(&Token::RParen) && !self.eof() {
                    elements.push(self.parse_type_annotation());
                    self.skip_newlines();
                    if self.eat(&Token::Comma).is_none() {
                        break;
                    }
                    self.skip_newlines();
                }
                self.expect(&Token::RParen, "`)`");
                // Check for `-> RetType` (function type)
                if self.eat(&Token::Arrow).is_some() {
                    let ret = self.parse_type_annotation();
                    let span = self.span_from(start);
                    TypeAnnotation::Function {
                        params: elements,
                        ret: Box::new(ret),
                        span,
                    }
                } else {
                    let span = self.span_from(start);
                    TypeAnnotation::Tuple { elements, span }
                }
            }
            Token::LBrace => {
                // Object type: `{ key: Type, ... }`
                self.advance();
                let mut fields = Vec::new();
                self.skip_newlines();
                while !self.at(&Token::RBrace) && !self.eof() {
                    let fstart = self.peek_span();
                    let name = self.expect_ident("field name");
                    let optional = self.eat(&Token::Question).is_some();
                    self.expect(&Token::Colon, "`:`");
                    let ty = self.parse_type_annotation();
                    let fspan = self.span_from(fstart);
                    fields.push(ObjectTypeField {
                        name: name.into(),
                        ty,
                        optional,
                        span: fspan,
                    });
                    self.skip_newlines();
                    if self.eat(&Token::Comma).is_none() {
                        break;
                    }
                    self.skip_newlines();
                }
                self.expect(&Token::RBrace, "`}`");
                let span = self.span_from(start);
                TypeAnnotation::Object { fields, span }
            }
            Token::Fn => {
                // `fn(params) -> ret`
                self.advance();
                self.expect(&Token::LParen, "`(`");
                let mut params = Vec::new();
                self.skip_newlines();
                while !self.at(&Token::RParen) && !self.eof() {
                    params.push(self.parse_type_annotation());
                    self.skip_newlines();
                    if self.eat(&Token::Comma).is_none() {
                        break;
                    }
                    self.skip_newlines();
                }
                self.expect(&Token::RParen, "`)`");
                self.expect(&Token::Arrow, "`->`");
                let ret = self.parse_type_annotation();
                let span = self.span_from(start);
                TypeAnnotation::Function {
                    params,
                    ret: Box::new(ret),
                    span,
                }
            }
            _ => {
                // Fallback: treat as named type
                let name = self.expect_ident("type name");
                TypeAnnotation::Named {
                    name: name.into(),
                    span: self.span_from(start),
                }
            }
        }
    }

    /// Expect an identifier and return its name, or report error.
    pub(crate) fn expect_ident(&mut self, ctx: &str) -> String {
        match self.peek().clone() {
            Token::Ident(name) => {
                self.advance();
                name
            }
            _ => {
                let span = self.peek_span();
                let found = self.peek().clone();
                self.error(ParseError::unexpected(ctx, &found, span));
                // Return empty string as error recovery
                String::new()
            }
        }
    }

    /// Parse a block: `{ stmts... }`
    pub(crate) fn parse_block(&mut self) -> Block {
        let start = self.peek_span();
        self.expect(&Token::LBrace, "`{`");
        let mut stmts = Vec::new();
        self.skip_newlines();
        while !self.at(&Token::RBrace) && !self.eof() {
            stmts.push(self.parse_stmt());
            self.skip_newlines();
        }
        self.expect(&Token::RBrace, "`}`");
        let span = self.span_from(start);
        Block { stmts, span }
    }
}
