//! GaleX lexer — converts source text into a token stream.
//!
//! The lexer operates in multiple modes to handle GaleX's mixed code/template syntax:
//! - **Code** mode: default — keywords, operators, identifiers, literals
//! - **Template** mode: HTML tags, text nodes, template control flow
//! - **HtmlTag** mode: inside `<tag ...>` — attributes, directives
//! - **TemplateLiteral** mode: inside backtick strings with `${}` interpolation
//! - **TemplateExpr** mode: expressions inside template `{}` — tracks brace depth

mod code;
mod cursor;
mod number;
mod regex;
mod string;
mod template;

use crate::error::{LexError, LexResult};
use crate::span::Span;
use crate::token::{Token, TokenWithSpan};
use cursor::Cursor;

/// Lexer mode controlling how characters are interpreted.
#[derive(Debug, Clone, PartialEq)]
pub enum LexMode {
    /// Default mode — standard expression/statement lexing.
    Code,
    /// Inside a component body — HTML tags, text, template control flow.
    Template,
    /// Inside `<tag ...>` — attributes, directives, until `>` or `/>`.
    HtmlTag,
    /// Inside a backtick template literal.
    TemplateLiteral,
    /// Inside `{}` expression interpolation (in template or template literal).
    /// `depth` tracks nested braces to know when to pop.
    TemplateExpr { depth: u32 },
}

/// The GaleX lexer.
///
/// Converts source text into tokens on demand via [`next_token()`](Lexer::next_token).
/// Supports mode switching for mixed code/template syntax.
pub struct Lexer<'src> {
    cursor: Cursor<'src>,
    mode_stack: Vec<LexMode>,
    /// Whether the previous meaningful token can end an expression
    /// (for regex vs division disambiguation).
    prev_can_end_expr: bool,
    pub errors: Vec<LexError>,
    file_id: u32,
}

impl<'src> Lexer<'src> {
    /// Create a new lexer for the given source text.
    pub fn new(source: &'src str, file_id: u32) -> Self {
        Self {
            cursor: Cursor::new(source),
            mode_stack: vec![LexMode::Code],
            prev_can_end_expr: false,
            errors: Vec::new(),
            file_id,
        }
    }

    /// Get the current lexer mode.
    fn current_mode(&self) -> &LexMode {
        self.mode_stack.last().unwrap_or(&LexMode::Code)
    }

    /// Push a new mode onto the stack.
    pub fn push_mode(&mut self, mode: LexMode) {
        self.mode_stack.push(mode);
    }

    /// Pop the current mode, returning to the previous one.
    pub fn pop_mode(&mut self) {
        if self.mode_stack.len() > 1 {
            self.mode_stack.pop();
        }
    }

    /// Rewind the lexer to a previous byte offset.
    ///
    /// Used by the parser when switching modes requires re-lexing tokens.
    pub fn rewind_to(&mut self, byte_offset: usize) {
        self.cursor.rewind_to(byte_offset);
    }

    /// Get all errors accumulated during lexing.
    pub fn errors(&self) -> &[LexError] {
        &self.errors
    }

    /// Record a lex error.
    fn emit_error(&mut self, error: LexError) {
        self.errors.push(error);
    }

    /// Create a span from `start` byte offset to the cursor's current position.
    fn span_from(&self, start: usize, start_line: u32, start_col: u32) -> Span {
        Span::new(
            self.file_id,
            start as u32,
            self.cursor.pos() as u32,
            start_line,
            start_col,
        )
    }

    /// Produce the next token from the source.
    ///
    /// Returns [`Token::EOF`] when the source is exhausted.
    pub fn next_token(&mut self) -> TokenWithSpan {
        let mode = self.current_mode().clone();
        let result = match mode {
            LexMode::Code => self.lex_code(),
            LexMode::Template => self.lex_template(),
            LexMode::HtmlTag => self.lex_html_tag(),
            LexMode::TemplateLiteral => self.lex_template_literal_continuation(),
            LexMode::TemplateExpr { .. } => self.lex_template_expr(),
        };

        // Track whether previous token can end an expression (for regex disambiguation).
        // Skip whitespace/comments — they don't affect the decision.
        match &result.0 {
            Token::Newline | Token::Comment(_) | Token::BlockComment(_) => {}
            tok => self.prev_can_end_expr = tok.can_end_expression(),
        }

        result
    }

    /// Lex in TemplateExpr mode — same as code but tracks brace depth.
    fn lex_template_expr(&mut self) -> TokenWithSpan {
        // Peek to handle brace depth
        if let Some(ch) = self.cursor.peek() {
            match ch {
                '{' => {
                    // Nested brace — increment depth
                    if let Some(LexMode::TemplateExpr { depth }) = self.mode_stack.last_mut() {
                        *depth += 1;
                    }
                    let start = self.cursor.pos();
                    let line = self.cursor.line();
                    let col = self.cursor.col();
                    self.cursor.advance();
                    let span = self.span_from(start, line, col);
                    return (Token::LBrace, span);
                }
                '}' => {
                    let at_depth_zero =
                        matches!(self.current_mode(), LexMode::TemplateExpr { depth: 0 });
                    if at_depth_zero {
                        // Pop back to parent mode
                        let start = self.cursor.pos();
                        let line = self.cursor.line();
                        let col = self.cursor.col();
                        self.cursor.advance();
                        let span = self.span_from(start, line, col);
                        self.pop_mode();

                        // What we return depends on what mode we popped back to
                        return match self.current_mode() {
                            LexMode::TemplateLiteral => {
                                // Continue lexing the template literal
                                self.lex_template_literal_after_expr(start, line, col)
                            }
                            LexMode::Template | LexMode::HtmlTag => (Token::ExprClose, span),
                            _ => (Token::RBrace, span),
                        };
                    } else {
                        // Decrement depth
                        if let Some(LexMode::TemplateExpr { depth }) = self.mode_stack.last_mut() {
                            *depth -= 1;
                        }
                        let start = self.cursor.pos();
                        let line = self.cursor.line();
                        let col = self.cursor.col();
                        self.cursor.advance();
                        let span = self.span_from(start, line, col);
                        return (Token::RBrace, span);
                    }
                }
                _ => {}
            }
        }

        // Otherwise, lex as code
        self.lex_code()
    }

    /// Collect all tokens into a vec (convenience for testing and one-shot use).
    pub fn tokenize_all(&mut self) -> Vec<TokenWithSpan> {
        // Pre-allocate: rough estimate of ~1 token per 5 source bytes
        let estimated = (self.cursor.source_len() / 5) + 1;
        let mut tokens = Vec::with_capacity(estimated);
        loop {
            let tok = self.next_token();
            let is_eof = tok.0 == Token::EOF;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        tokens
    }
}

/// Lex source text in Code mode, returning all tokens and any errors.
///
/// This is the simplest entry point — for mixed code/template files,
/// use [`Lexer`] directly with mode switching.
pub fn lex(source: &str, file_id: u32) -> LexResult {
    let mut lexer = Lexer::new(source, file_id);
    let tokens = lexer.tokenize_all();
    let errors = std::mem::take(&mut lexer.errors);
    LexResult { tokens, errors }
}
